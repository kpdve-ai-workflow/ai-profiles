import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import profile as manager  # noqa: E402


class ProfileTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.source = self.base / "ai-profiles"
        shutil.copytree(ROOT, self.source, ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "tests"))
        self.project = self.base / "my-api"
        self.project.mkdir()
        self.override = patch.object(manager, "ROOT", self.source)
        self.override.start()
        self.addCleanup(self.override.stop)

    def state(self):
        return json.loads((self.project / manager.STATE).read_text())

    def cfg(self):
        return json.loads((self.project / "opencode.json").read_text())

    def test_install_update_add_remove_skills_preserve_custom_and_uninstall(self):
        custom_cfg = {
            "model": "test/model", "permission": {"read": "allow"},
            "mcp": {"unmanaged": {"type": "remote", "url": "https://example.org/mcp"}},
            "instructions": ["custom-rules.md"],
        }
        (self.project / "opencode.json").write_text(json.dumps(custom_cfg))
        (self.project / ".opencode/skills/user-skill").mkdir(parents=True)
        (self.project / ".opencode/skills/user-skill/SKILL.md").write_text("my skill")
        manager.apply_profile(self.project, "backend-go")
        self.assertIn("postgres", self.cfg()["mcp"])
        self.assertEqual(self.cfg()["model"], "test/model")
        self.assertEqual(self.cfg()["instructions"], ["custom-rules.md", manager.RULE])
        self.assertFalse((self.project / manager.GATE).exists())

        # Add two new skills and modify one previously installed skill upstream.
        for skill in ("go-database", "go-testing"):
            directory = self.source / "profiles/backend-go/skills" / skill
            directory.mkdir()
            (directory / "SKILL.md").write_text(f"---\nname: {skill}\ndescription: {skill}\n---\n\ncontent")
            (directory / "references").mkdir()
            (directory / "references/notes.md").write_text("new notes")
        source_go = self.source / "profiles/backend-go/skills/go-backend/SKILL.md"
        source_go.write_text(source_go.read_text() + "\nNew practice\n")
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual((self.project / ".opencode/skills/go-backend/SKILL.md").read_text(), source_go.read_text())
        for skill in ("go-database", "go-testing"):
            self.assertTrue((self.project / f".opencode/skills/{skill}/SKILL.md").exists())
            self.assertTrue((self.project / f".opencode/skills/{skill}/references/notes.md").exists())
        self.assertEqual(self.cfg()["mcp"]["unmanaged"], custom_cfg["mcp"]["unmanaged"])

        # Retire a managed skill -> update removes only files it owns.
        shutil.rmtree(self.source / "profiles/backend-go/skills/go-testing")
        manager.apply_profile(self.project, "backend-go")
        self.assertFalse((self.project / ".opencode/skills/go-testing").exists())
        self.assertTrue((self.project / ".opencode/skills/user-skill/SKILL.md").exists())

        # Switch Go -> React, preserve General and unrelated MCP.
        manager.apply_profile(self.project, "frontend-react")
        self.assertIn("figma", self.cfg()["mcp"])
        self.assertNotIn("postgres", self.cfg()["mcp"])
        self.assertTrue((self.project / ".opencode/skills/react-frontend/SKILL.md").exists())
        self.assertTrue((self.project / ".opencode/skills/refine-plan/SKILL.md").exists())
        manager.uninstall(self.project)
        self.assertFalse((self.project / manager.STATE).exists())
        self.assertFalse((self.project / ".opencode/skills/react-frontend").exists())
        self.assertTrue((self.project / ".opencode/skills/user-skill/SKILL.md").exists())
        self.assertEqual(self.cfg(), custom_cfg)
        self.assertNotIn(manager.GATE, (self.project / ".gitignore").read_text())

    def test_conflict_before_any_writes(self):
        manager.apply_profile(self.project, "backend-go")
        managed = self.project / ".opencode/skills/go-backend/SKILL.md"
        managed.write_text("edited manually")
        state_before = (self.project / manager.STATE).read_bytes()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(managed.read_text(), "edited manually")
        self.assertEqual((self.project / manager.STATE).read_bytes(), state_before)
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.uninstall(self.project)
        self.assertTrue((self.project / manager.STATE).exists())

    def test_collision_with_unmanaged_file(self):
        existing = self.project / ".opencode/skills/go-backend/SKILL.md"
        existing.parent.mkdir(parents=True)
        existing.write_text("unmanaged")
        with self.assertRaisesRegex(manager.ProfileError, "niezarządzanym"):
            manager.apply_profile(self.project, "backend-go")
        self.assertFalse((self.project / manager.STATE).exists())
        self.assertEqual(existing.read_text(), "unmanaged")

    def test_existing_jsonc_is_rejected(self):
        (self.project / "opencode.jsonc").write_text("{}")
        with self.assertRaisesRegex(manager.ProfileError, "opencode.jsonc"):
            manager.apply_profile(self.project, "backend-go")

    def test_symlink_protection(self):
        outside = self.base / "outside"
        outside.mkdir()
        (self.project / ".opencode").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(manager.ProfileError, "symlink"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(list(outside.iterdir()), [])

    def test_legacy_migration_then_update(self):
        manager.apply_profile(self.project, "backend-go")
        state_path = self.project / manager.STATE
        state_path.write_text(json.dumps({"profile": "backend-go", "skill": "go-backend", "mcp": "postgres"}))
        (self.project / ".opencode/ai-profile-gate.py").unlink()  # absent in old MVP
        with self.assertRaisesRegex(manager.ProfileError, "--accept-existing"):
            manager.migrate_legacy(self.project, accept_existing=False)
        manager.migrate_legacy(self.project, accept_existing=True)
        self.assertEqual(self.state()["format"], 1)
        manager.apply_profile(self.project, "backend-go")
        self.assertIn(".opencode/ai-profile-gate.py", self.state()["files"])

    def test_validate_example_mirrors_all_profiles(self):
        manager.validate_sources()
        example = self.source / "mcp/example.json"
        data = json.loads(example.read_text())
        data["mcp"].pop("figma")
        example.write_text(json.dumps(data))
        with self.assertRaisesRegex(manager.ProfileError, "mcp/example.json"):
            manager.validate_sources()

    def test_cli_and_makefile(self):
        cmd = [sys.executable, str(ROOT / "profile.py")]
        subprocess.run(cmd + ["install", "backend-go", "--project", str(self.project)], check=True, capture_output=True)
        subprocess.run(cmd + ["update", "--project", str(self.project)], check=True, capture_output=True)
        status = subprocess.run(cmd + ["status", "--project", str(self.project)], check=True, text=True, capture_output=True)
        self.assertIn("backend-go", status.stdout)
        subprocess.run(cmd + ["uninstall", "--project", str(self.project)], check=True, capture_output=True)
        self.assertFalse((self.project / manager.STATE).exists())
        make_result = subprocess.run(["make", "validate"], cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(make_result.returncode, 0, make_result.stdout + make_result.stderr)


if __name__ == "__main__":
    unittest.main()
