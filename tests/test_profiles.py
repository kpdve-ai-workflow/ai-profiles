import json
import contextlib
import io
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

    def set_version(self, version):
        path = self.source / "profiles/backend-go/profile.json"
        metadata = json.loads(path.read_text())
        metadata["version"] = version
        path.write_text(json.dumps(metadata))

    def status_output(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            manager.status(self.project)
        return output.getvalue()

    def snapshot(self):
        return {p.relative_to(self.project).as_posix(): p.read_bytes()
                for p in self.project.rglob("*") if p.is_file()}

    def test_versions_install_update_and_idempotence(self):
        with patch.object(manager, "source_commit", return_value="a" * 40):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.0.0")
        self.assertEqual(self.state()["source_commit"], "a" * 40)
        for name in ("go-testing", "go-database"):
            skill = self.source / "profiles/backend-go/skills" / name
            skill.mkdir()
            (skill / "SKILL.md").write_text(f"---\nname: {name}\ndescription: Testing\n---\n")
        self.set_version("1.1.0")
        self.assertIn("UPDATE AVAILABLE", self.status_output())
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.1.0")
        for name in ("go-testing", "go-database"):
            self.assertTrue((self.project / f".opencode/skills/{name}/SKILL.md").exists())
        self.assertIn("UP TO DATE", self.status_output())
        before = self.snapshot()
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        shutil.rmtree(self.source / "profiles/backend-go/skills/go-testing")
        self.set_version("2.0.0")
        manager.apply_profile(self.project, "backend-go")
        self.assertFalse((self.project / ".opencode/skills/go-testing").exists())

    def test_unversioned_content_and_local_conflict(self):
        manager.apply_profile(self.project, "backend-go")
        source = self.source / "profiles/backend-go/skills/go-backend/SKILL.md"
        source.write_text(source.read_text() + "\nUpstream change\n")
        self.assertIn("CONTENT CHANGED WITHOUT VERSION BUMP", self.status_output())
        local = self.project / ".opencode/skills/go-backend/SKILL.md"
        local.write_text(local.read_text() + "\nUser change\n")
        self.assertIn(".opencode/skills/go-backend/SKILL.md", self.status_output())
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        local.write_bytes(source.read_bytes().replace(b"\nUpstream change\n", b""))
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(local.read_bytes(), source.read_bytes())
        self.assertIn("UP TO DATE", self.status_output())

    def test_old_hashed_state_migration(self):
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        del state["version"]
        del state["source_commit"]
        (self.project / manager.STATE).write_text(json.dumps(state))
        self.assertIn("VERSION UNKNOWN", self.status_output())
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.0.0")
        self.assertIn("source_commit", self.state())

    def test_invalid_version_numeric_comparison_and_downgrade(self):
        manager.apply_profile(self.project, "backend-go")
        before = self.snapshot()
        for version in ("1.0", "01.0.0", "1.0.0-beta", "invalid", None):
            self.set_version(version)
            with self.assertRaisesRegex(manager.ProfileError, "wersja"):
                manager.apply_profile(self.project, "backend-go")
            self.assertEqual(before, self.snapshot())
        self.set_version("1.9.0")
        manager.apply_profile(self.project, "backend-go")
        self.set_version("1.10.0")
        self.assertIn("UPDATE AVAILABLE", self.status_output())
        manager.apply_profile(self.project, "backend-go")
        self.assertIn("Content: UNCHANGED", self.status_output())
        self.set_version("1.9.0")
        self.assertIn("INSTALLED NEWER", self.status_output())
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Downgrade"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_mcp_content_and_conflict(self):
        manager.apply_profile(self.project, "backend-go")
        source = self.source / "profiles/backend-go/mcp.json"
        mcp = json.loads(source.read_text())
        mcp["postgres"]["enabled"] = False
        source.write_text(json.dumps(mcp))
        self.assertIn("CONTENT CHANGED WITHOUT VERSION BUMP", self.status_output())
        cfg = self.cfg()
        cfg["mcp"]["postgres"]["custom"] = True
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        self.assertIn("MCP:postgres", self.status_output())
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_new_profile_discovery_and_independent_versions(self):
        new = self.source / "profiles/custom-profile"
        shutil.copytree(self.source / "profiles/backend-go", new)
        (new / "profile.json").write_text(json.dumps({
            "name": "custom-profile", "version": "3.2.1", "description": "Custom profile"
        }))
        self.assertIn("custom-profile", manager.profile_names())
        manager.apply_profile(self.project, "custom-profile")
        self.assertEqual(self.state()["version"], "3.2.1")
        manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(self.state()["version"], "1.0.0")
        manager.uninstall(self.project)
        self.assertFalse((self.project / manager.STATE).exists())

    def test_path_collision_preflight(self):
        manager.apply_profile(self.project, "backend-go")
        self.set_version("1.1.0")
        asset = self.source / "profiles/backend-go/skills/go-backend/new-assets"
        asset.mkdir()
        (asset / "notes.md").write_text("new notes")
        (self.project / ".opencode/skills/go-backend/new-assets").write_text("user file")
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_update_common_plugin_and_mcp(self):
        manager.apply_profile(self.project, "backend-go")
        plugin = self.source / "common/plugins/approval-gate.js"
        plugin.write_text(plugin.read_text() + "\n// Updated plugin\n")
        mcp_path = self.source / "profiles/backend-go/mcp.json"
        mcp = json.loads(mcp_path.read_text())
        mcp["postgres"]["enabled"] = False
        mcp_path.write_text(json.dumps(mcp))
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual((self.project / ".opencode/plugins/approval-gate.js").read_bytes(),
                         plugin.read_bytes())
        self.assertEqual(self.cfg()["mcp"]["postgres"], mcp["postgres"])
        self.assertIn("UP TO DATE", self.status_output())

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
        skills = self.project / ".opencode/skills"
        legacy_skill = skills / "implementation-breakdown"
        legacy_skill.mkdir()
        (legacy_skill / "SKILL.md").write_text(
            "---\nname: implementation-breakdown\ndescription: Legacy planning skill\n---\n"
        )
        shutil.rmtree(skills / "make-plan")
        # The old MVP installed only SKILL.md, without supporting assets.
        for skill in (skills / "refine-plan", skills / "go-backend"):
            content = (skill / "SKILL.md").read_bytes()
            shutil.rmtree(skill)
            skill.mkdir()
            (skill / "SKILL.md").write_bytes(content)
        state_path = self.project / manager.STATE
        state_path.write_text(json.dumps({"profile": "backend-go", "skill": "go-backend", "mcp": "postgres"}))
        (self.project / ".opencode/ai-profile-gate.py").unlink()  # absent in old MVP
        with self.assertRaisesRegex(manager.ProfileError, "--accept-existing"):
            manager.migrate_legacy(self.project, accept_existing=False)
        manager.migrate_legacy(self.project, accept_existing=True)
        self.assertEqual(self.state()["format"], 1)
        manager.apply_profile(self.project, "backend-go")
        self.assertIn(".opencode/ai-profile-gate.py", self.state()["files"])
        self.assertTrue((skills / "make-plan/SKILL.md").is_file())
        self.assertFalse(legacy_skill.exists())

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
