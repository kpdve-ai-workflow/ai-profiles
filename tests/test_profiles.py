import json
import contextlib
import io
import os
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
import mcp_runtime


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
        # Keep version scenarios independent of the current published release.
        self.set_version("1.0.0")
        metadata_path = self.source / "profiles/frontend-react/profile.json"
        metadata = json.loads(metadata_path.read_text())
        metadata["version"] = "1.0.0"
        metadata_path.write_text(json.dumps(metadata))

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

    def test_bump_semver_parts_and_profile_independence(self):
        frontend = self.source / "profiles/frontend-react/profile.json"
        other_before = frontend.read_bytes()
        path = self.source / "profiles/backend-go/profile.json"
        for part, expected in (("patch", "1.9.10"), ("minor", "1.10.0"), ("major", "2.0.0")):
            with self.subTest(part=part):
                self.set_version("1.9.9")
                metadata = json.loads(path.read_text())
                metadata["custom"] = {"keep": True}
                path.write_text(json.dumps(metadata))
                self.assertEqual(manager.bump_profile("backend-go", part), expected)
                changed = json.loads(path.read_text())
                self.assertEqual(changed, {**metadata, "version": expected})
                self.assertEqual(frontend.read_bytes(), other_before)
                self.assertEqual(list(path.parent.glob(".profile-*.tmp")), [])

    def test_create_profile_is_valid_installable_and_tracked(self):
        example = (self.source / "mcp/example.json").read_bytes()
        manager.create_profile("backend-python", "Python backend")
        self.assertEqual(manager.profile_metadata("backend-python"), {
            "name": "backend-python", "version": "1.0.0", "description": "Python backend",
        })
        self.assertTrue((self.source / "profiles/backend-python/skills/.gitkeep").is_file())
        self.assertEqual(manager.desired_mcp("backend-python"), {})
        self.assertEqual(json.loads((self.source / "profiles/backend-python/mcp.json").read_text()),
                         {"servers": []})
        manager.validate_sources()
        self.assertEqual((self.source / "mcp/example.json").read_bytes(), example)
        manager.apply_profile(self.project, "backend-python")
        self.assertEqual(self.state()["profile"], "backend-python")
        self.assertFalse((self.project / ".opencode/skills/.gitkeep").exists())
        manager.apply_profile(self.project, "backend-python")
        manager.uninstall(self.project)

    def test_create_rejects_invalid_existing_and_symlink_profiles(self):
        before = {p.relative_to(self.source): p.read_bytes()
                  for p in (self.source / "profiles").rglob("*") if p.is_file()}
        for name in ("backend-go", "../outside", "", "Backend", "bad_name", "a/b", "-bad", "bad--name"):
            with self.subTest(name=name), self.assertRaises(manager.ProfileError):
                manager.create_profile(name)
        with self.assertRaises(manager.ProfileError):
            manager.create_profile("blank", "  ")
        self.assertEqual(before, {p.relative_to(self.source): p.read_bytes()
                                for p in (self.source / "profiles").rglob("*") if p.is_file()})
        (self.source / "profiles/linked").symlink_to(self.project, target_is_directory=True)
        with self.assertRaisesRegex(manager.ProfileError, "symlink"):
            manager.create_profile("linked")
        self.assertEqual(self.snapshot(), {})

    def test_create_and_help_cli_and_make(self):
        cli = [sys.executable, str(self.source / "profile.py")]
        for args in (["create", "python-cli"],):
            subprocess.run(cli + args, cwd=self.source, check=True, capture_output=True, text=True)
        self.assertEqual(manager.profile_metadata("python-cli")["description"], "Profil python-cli")
        subprocess.run(["make", "create", "PROFILE=python-make", "DESCRIPTION=Python Make"],
                       cwd=self.source, check=True, capture_output=True, text=True)
        self.assertEqual(manager.profile_metadata("python-make")["description"], "Python Make")
        for command in (cli + ["help"], cli + ["--help"], cli + ["-h"], cli,
                        ["make", "help"], ["make"]):
            result = subprocess.run(command, cwd=self.source, check=True, capture_output=True, text=True)
            self.assertIn("ai-profiles — zarządzanie profilami\n", result.stdout)
            for noise in ("usage:", "positional arguments:", "options:", "Make:"):
                self.assertNotIn(noise, result.stdout)
            for action in ("create", "install", "switch", "status", "update", "pull-update",
                           "uninstall", "bump", "validate", "test", "help"):
                self.assertIn(f"make {action}", result.stdout)
            for text in ("create", "install", "switch", "bump", "pull-update",
                         "DESCRIPTION", "PART", "PROFILE", "PROJECT"):
                self.assertIn(text, result.stdout)
        for action in ("create", "install", "switch", "bump"):
            for command in (cli + [action], ["make", action, "PROJECT=" + str(self.project), "PROFILE="]):
                result = subprocess.run(command, cwd=self.source, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
        for args in (["create", "python-cli"], ["create", "../bad"],
                     ["create", "bad", "--project", str(self.project)],
                     ["create", "bad", "--part", "patch"],
                     ["validate", "unknown"], ["help", "--accept-existing"],
                     ["install", "unknown", "--project", str(self.project)],
                     ["bump", "python-cli", "--description", "bad"]):
            result = subprocess.run(cli + args, cwd=self.source, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.snapshot(), {})

    def test_bump_rejects_invalid_input_without_writes(self):
        path = self.source / "profiles/backend-go/profile.json"
        before = path.read_bytes()
        for profile, part in (("missing", "patch"), ("../backend-go", "major"), ("backend-go", "invalid")):
            with self.assertRaises(manager.ProfileError):
                manager.bump_profile(profile, part)
            self.assertEqual(path.read_bytes(), before)
        self.set_version("01.0.0")
        before = path.read_bytes()
        with self.assertRaisesRegex(manager.ProfileError, "wersja"):
            manager.bump_profile("backend-go")
        self.assertEqual(path.read_bytes(), before)

    def test_bump_write_failure_preserves_metadata(self):
        path = self.source / "profiles/backend-go/profile.json"
        before = path.read_bytes()
        with patch.object(Path, "replace", side_effect=OSError("Simulated write failure")):
            with self.assertRaises(OSError):
                manager.bump_profile("backend-go")
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(list(path.parent.glob(".profile-*.tmp")), [])

    def test_bump_cli_and_make_adapter(self):
        cmd = [sys.executable, str(self.source / "profile.py"), "bump"]
        result = subprocess.run(cmd + ["backend-go"], cwd=self.source,
                                check=True, capture_output=True, text=True)
        self.assertIn("1.0.0 -> 1.0.1", result.stdout)
        result = subprocess.run(["make", "bump", "PROFILE=backend-go", "PART=minor"],
                                cwd=self.source, check=True, capture_output=True, text=True)
        self.assertIn("1.0.1 -> 1.1.0", result.stdout)
        before = (self.source / "profiles/backend-go/profile.json").read_bytes()
        for args in ([], ["backend-go", "--part", "invalid"],
                     ["backend-go", "--project", str(self.project)]):
            result = subprocess.run(cmd + args, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.source / "profiles/backend-go/profile.json").read_bytes(), before)
        self.assertEqual(self.snapshot(), {})

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
        source = self.source / "mcp/servers/postgres.json"
        mcp = json.loads(source.read_text())
        mcp["enabled"] = False
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

    def test_update_common_rules_and_mcp(self):
        manager.apply_profile(self.project, "backend-go")
        mcp_path = self.source / "mcp/servers/postgres.json"
        mcp = json.loads(mcp_path.read_text())
        mcp["enabled"] = False
        mcp_path.write_text(json.dumps(mcp))
        manager.apply_profile(self.project, "backend-go")
        expected, _ = manager.configured_mcp(self.project, {"postgres": mcp})
        self.assertEqual(self.cfg()["mcp"]["postgres"], expected["postgres"])
        self.assertIn("UP TO DATE", self.status_output())

    def test_repository_local_skill_and_fresh_install_scope(self):
        local = self.source / ".opencode/skills/brain-to-implementer/SKILL.md"
        original = local.read_bytes()
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", manager.desired_files("backend-go"))
        manager.apply_profile(self.project, "backend-go")
        self.assertNotIn(manager.RULE, self.state()["files"])
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", self.state()["files"])
        self.assertFalse((self.project / manager.RULE).exists())
        self.assertFalse((self.project / ".opencode/skills/brain-to-implementer").exists())
        self.assertNotIn("instructions", self.cfg())
        self.assertEqual(local.read_bytes(), original)

    def test_retire_old_rules_and_skill_hash_checked_idempotently(self):
        manager.apply_profile(self.project, "backend-go")
        retired_skill = ".opencode/skills/brain-to-implementer/SKILL.md"
        rules = self.project / manager.RULE
        rules.write_text("old managed rules\n")
        skill = self.project / retired_skill
        skill.parent.mkdir(parents=True)
        skill.write_text("old managed skill\n")
        cfg = self.cfg()
        cfg["instructions"] = ["keep-me.md", manager.RULE]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        state = self.state()
        state["files"][manager.RULE] = manager.sha(rules.read_bytes())
        state["files"][retired_skill] = manager.sha(skill.read_bytes())
        state["instructions_added"] = True
        (self.project / manager.STATE).write_text(json.dumps(state))
        manager.apply_profile(self.project, "backend-go")
        self.assertFalse(rules.exists())
        self.assertFalse(skill.exists())
        self.assertEqual(self.cfg()["instructions"], ["keep-me.md"])
        self.assertFalse(self.state()["instructions_added"])
        snapshot = self.snapshot()
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(snapshot, self.snapshot())

    def test_retired_file_conflicts_abort_without_partial_changes(self):
        manager.apply_profile(self.project, "backend-go")
        rules = self.project / manager.RULE
        rules.write_text("managed old rules\n")
        state = self.state()
        state["files"][manager.RULE] = manager.sha(rules.read_bytes())
        state["instructions_added"] = True
        (self.project / manager.STATE).write_text(json.dumps(state))
        cfg = self.cfg()
        cfg["instructions"] = [manager.RULE]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        rules.write_text("locally modified\n")
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_uninstall_removes_hash_verified_retired_files_and_owned_instruction(self):
        manager.apply_profile(self.project, "backend-go")
        for rel, content in ((manager.RULE, b"old rules\n"),
                             (".opencode/skills/brain-to-implementer/SKILL.md", b"old skill\n")):
            path = self.project / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        state = self.state()
        state["files"][manager.RULE] = manager.sha(b"old rules\n")
        state["files"][".opencode/skills/brain-to-implementer/SKILL.md"] = manager.sha(b"old skill\n")
        state["instructions_added"] = True
        (self.project / manager.STATE).write_text(json.dumps(state))
        cfg = self.cfg()
        cfg["instructions"] = [manager.RULE, "my-instructions.md"]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        manager.uninstall(self.project)
        self.assertFalse((self.project / manager.RULE).exists())
        self.assertFalse((self.project / ".opencode/skills/brain-to-implementer/SKILL.md").exists())
        self.assertEqual(self.cfg()["instructions"], ["my-instructions.md"])

    def test_user_owned_retired_instruction_and_untracked_files_survive(self):
        cfg = {"instructions": [manager.RULE, "other.md"]}
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(self.cfg()["instructions"], cfg["instructions"])
        untracked = self.project / manager.RULE
        untracked.parent.mkdir(parents=True, exist_ok=True)
        untracked.write_text("user file\n")
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(untracked.read_text(), "user file\n")

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
        self.assertEqual(self.cfg()["instructions"], ["custom-rules.md"])
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
        ignore = self.project / ".gitignore"
        self.assertNotIn(manager.GATE, ignore.read_text() if ignore.exists() else "")

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
        # These were not part of the legacy MVP's installed skill set.
        for name in ("changelogs", "make-roadmap", "test-gap-discovery"):
            shutil.rmtree(skills / name)
        # The old MVP installed only SKILL.md, without supporting assets.
        for skill in (skills / "refine-plan", skills / "go-backend"):
            content = (skill / "SKILL.md").read_bytes()
            shutil.rmtree(skill)
            skill.mkdir()
            (skill / "SKILL.md").write_bytes(content)
        state_path = self.project / manager.STATE
        state_path.write_text(json.dumps({"profile": "backend-go", "skill": "go-backend", "mcp": "postgres"}))
        (self.project / manager.RULE).parent.mkdir(parents=True, exist_ok=True)
        (self.project / manager.RULE).write_text("legacy workflow rules\n")
        plugin = self.project / ".opencode/plugins/approval-gate.js"
        plugin.parent.mkdir(parents=True)
        plugin.write_text("// Historical approval gate fixture\n")
        (self.project / mcp_runtime.LAUNCHER).unlink()
        with self.assertRaisesRegex(manager.ProfileError, "--accept-existing"):
            manager.migrate_legacy(self.project, accept_existing=False)
        manager.migrate_legacy(self.project, accept_existing=True)
        self.assertEqual(self.state()["format"], 1)
        manager.apply_profile(self.project, "backend-go")
        self.assertNotIn(".opencode/ai-profile-gate.py", self.state()["files"])
        self.assertFalse(plugin.exists())
        self.assertTrue((skills / "make-plan/SKILL.md").is_file())
        self.assertFalse(legacy_skill.exists())

    def test_install_and_update_do_not_require_approval(self):
        manager.apply_profile(self.project, "backend-go")
        for relative in (".opencode/plugins/approval-gate.js", ".opencode/ai-profile-gate.py", manager.GATE):
            self.assertFalse((self.project / relative).exists())
        manager.apply_profile(self.project, "backend-go")
        self.assertNotIn("LOCKED", self.status_output())
        self.assertIn("Approval gate: not installed", self.status_output())

    def test_update_retires_owned_gate_and_preserves_unrelated_plugins(self):
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        for relative in (".opencode/plugins/approval-gate.js", ".opencode/ai-profile-gate.py"):
            path = self.project / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("historical gate\n")
            state["files"][relative] = manager.sha(path.read_bytes())
        (self.project / manager.GATE).write_text('{"approved": true, "task": "TASK-001"}\n')
        (self.project / ".gitignore").write_text("custom-entry\n" + manager.GATE + "\n")
        state["gitignore_added"] = True
        (self.project / manager.STATE).write_text(json.dumps(state))
        custom = self.project / ".opencode/plugins/custom.js"
        custom.write_text("// user plugin\n")
        self.assertIn("legacy installation", self.status_output())
        manager.apply_profile(self.project, "backend-go")
        self.assertTrue(custom.exists())
        self.assertFalse((self.project / ".opencode/plugins/approval-gate.js").exists())
        self.assertFalse((self.project / ".opencode/ai-profile-gate.py").exists())
        self.assertFalse((self.project / manager.GATE).exists())
        self.assertEqual((self.project / ".gitignore").read_text(), "custom-entry\n")
        self.assertFalse(self.state()["gitignore_added"])
        manager.apply_profile(self.project, "backend-go")
        self.assertIn("Approval gate: not installed", self.status_output())

    def test_validate_one_profile_cli_and_make(self):
        cli = [sys.executable, str(self.source / "profile.py"), "validate"]
        for command in (cli + ["backend-go"], ["make", "validate", "PROFILE=backend-go"]):
            result = subprocess.run(command, cwd=self.source, check=True, capture_output=True, text=True)
            self.assertIn("OK: profil backend-go", result.stdout)
        for name in ("missing", "../backend-go"):
            with self.assertRaisesRegex(manager.ProfileError, "Nieznany profil"):
                manager.validate_sources(name)
        # A broken unrelated profile must not block scoped validation.
        (self.source / "profiles/frontend-react/profile.json").write_text("{}")
        manager.validate_sources("backend-go")
        with self.assertRaises(manager.ProfileError):
            manager.validate_sources()

    def test_validate_one_profile_checks_shared_skills_and_mcp_example(self):
        path = self.source / "mcp/example.json"
        example = json.loads(path.read_text())
        name = next(iter(manager.desired_mcp("backend-go")))
        del example["mcp"][name]
        path.write_text(json.dumps(example))
        with self.assertRaisesRegex(manager.ProfileError, "mcp/example.json"):
            manager.validate_sources("backend-go")
        example["mcp"][name] = {"type": "local", "command": ["wrong"]}
        path.write_text(json.dumps(example))
        with self.assertRaisesRegex(manager.ProfileError, "mcp/example.json"):
            manager.validate_sources("backend-go")
        example["mcp"][name] = manager.desired_mcp("backend-go")[name]
        path.write_text(json.dumps(example))
        manager.create_profile("empty-profile")
        manager.validate_sources("empty-profile")
        skill = next((self.source / "common/skills").glob("*/SKILL.md"))
        skill.write_text("invalid frontmatter")
        with self.assertRaisesRegex(manager.ProfileError, "frontmatter"):
            manager.validate_sources("empty-profile")

    def test_validate_example_mirrors_all_profiles(self):
        manager.validate_sources()
        example = self.source / "mcp/example.json"
        data = json.loads(example.read_text())
        data["mcp"].pop("figma")
        example.write_text(json.dumps(data))
        with self.assertRaisesRegex(manager.ProfileError, "mcp/example.json"):
            manager.validate_sources()

    def test_wordpress_install_delivers_mcp_skills_docs_without_plugin_changes(self):
        plugin = self.project / "wp-content/plugins/novamira/novamira.php"
        plugin.parent.mkdir(parents=True)
        plugin.write_text("existing user-owned plugin")
        manager.apply_profile(self.project, "wordpress")
        self.assertEqual(set(self.cfg()["mcp"]), {"novamira", "figma", "mariadb"})
        self.assertTrue(self.cfg()["mcp"]["novamira"]["disabled"])
        self.assertFalse(self.cfg()["mcp"]["figma"]["disabled"])
        for skill in ("mcp-novamira", "mcp-figma", "mcp-mariadb", "acf", "styling"):
            self.assertTrue((self.project / f".opencode/skills/{skill}/SKILL.md").is_file())
        self.assertEqual((self.project / ".opencode/ai-profile/README.md").read_bytes(),
                         (self.source / "profiles/wordpress/README.md").read_bytes())
        for skill in ("mcp-novamira", "mcp-mariadb"):
            self.assertFalse(any((self.project / f".opencode/skills/{skill}/examples").glob("*")))
        self.assertFalse((self.project / mcp_runtime.LOCAL).exists())
        self.assertFalse((self.project / ".env.novamira").exists())
        with patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "disabled"):
                mcp_runtime.launch(self.project, "novamira")
            execute.assert_not_called()
        manager.apply_profile(self.project, "wordpress")
        manager.uninstall(self.project)
        self.assertEqual(plugin.read_text(), "existing user-owned plugin")

    def test_wordpress_private_credentials_launch_and_survive_update_uninstall(self):
        manager.apply_profile(self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {
            "disabled": False, "env_file": ".env.novamira",
        }}}))
        private = self.project / ".env.novamira"
        secret = "abcd efgh test-secret ijkl"
        private.write_text('WP_API_URL=https://example.test/wp-json/mcp/novamira\n'
                           'WP_API_USERNAME=operator\n'
                           f'WP_API_PASSWORD="{secret}"\n')
        private.chmod(0o600)
        local_before, env_before = local.read_bytes(), private.read_bytes()
        manager.apply_profile(self.project, "wordpress")
        command = ["npx", "-y", "@automattic/mcp-wordpress-remote@0.4.0"]
        self.assertEqual(self.state()["mcp_base"]["novamira"]["command"], command)
        self.assertFalse(self.cfg()["mcp"]["novamira"]["disabled"])
        self.assertNotIn("environment", self.cfg()["mcp"]["novamira"])
        with patch.dict(mcp_runtime.os.environ, {}, clear=True), \
                patch.object(mcp_runtime.os, "chdir"), \
                patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "novamira")
        executable, args, environment = execute.call_args.args
        self.assertEqual((executable, args), ("npx", command))
        self.assertEqual(environment["WP_API_PASSWORD"], secret)
        self.assertEqual(environment["WP_API_USERNAME"], "operator")
        self.assertEqual(environment["WP_API_URL"], "https://example.test/wp-json/mcp/novamira")
        self.assertEqual(environment["OAUTH_ENABLED"], "false")
        self.assertEqual(environment["NODE_OPTIONS"], "--use-system-ca")
        for rel in (*self.state()["files"], manager.STATE, "opencode.json"):
            self.assertNotIn(secret.encode(), (self.project / rel).read_bytes())
        manager.apply_profile(self.project, "wordpress")
        manager.uninstall(self.project)
        self.assertEqual(local.read_bytes(), local_before)
        self.assertEqual(private.read_bytes(), env_before)
        self.assertEqual(private.stat().st_mode & 0o777, 0o600)

    def test_wordpress_update_retires_managed_skill_templates_preserving_private_files(self):
        old_templates = (
            "mcp-novamira/examples/ai-profile.local.example.json",
            "mcp-novamira/examples/novamira.env.example",
            "mcp-mariadb/examples/ai-profile.local.example.json",
            "mcp-mariadb/examples/mariadb.env.example",
        )
        for relative in old_templates:
            source = self.source / "profiles/wordpress/skills" / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("old managed template\n")
        manager.apply_profile(self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {"env_file": ".env.novamira"},
                                             "mariadb": {"env_file": ".env.mariadb"}}}))
        private = [local, self.project / ".env.novamira", self.project / ".env.mariadb"]
        for path in private[1:]:
            path.write_text("PROJECT_SECRET=synthetic-secret\n")
        user_file = self.project / ".opencode/skills/mcp-novamira/examples/my-notes.txt"
        user_file.write_text("user-owned notes\n")
        private.append(user_file)
        before = {path: path.read_bytes() for path in private}
        for relative in old_templates:
            (self.source / "profiles/wordpress/skills" / relative).unlink()
        manager.apply_profile(self.project, "wordpress")
        for relative in old_templates:
            self.assertFalse((self.project / ".opencode/skills" / relative).exists())
        self.assertEqual({path: path.read_bytes() for path in private}, before)
        for skill in ("mcp-novamira", "mcp-mariadb"):
            self.assertFalse(any(rel.startswith(f".opencode/skills/{skill}/examples/")
                                 for rel in self.state()["files"]))

    def test_wordpress_missing_credentials_fail_before_bridge_starts(self):
        manager.apply_profile(self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {"disabled": False, "env_file": ".env.novamira"}}}))
        private = self.project / ".env.novamira"
        with patch.dict(mcp_runtime.os.environ, {}, clear=True), \
                patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "Cannot read env file"):
                mcp_runtime.launch(self.project, "novamira")
            private.write_text('WP_API_URL=https://example.test/wp-json/mcp/novamira\nWP_API_USERNAME=operator\n')
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "WP_API_PASSWORD"):
                mcp_runtime.launch(self.project, "novamira")
            execute.assert_not_called()

    def test_wordpress_mariadb_installed_launcher_runs_with_private_env(self):
        manager.apply_profile(self.project, "wordpress")
        script = self.project / ".opencode/mcp/mariadb/launch.py"
        self.assertTrue(script.is_file())
        self.assertTrue(self.cfg()["mcp"]["mariadb"]["disabled"])
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"mariadb": {
            "disabled": False, "env_file": ".env.mariadb",
        }}}))
        private = self.project / ".env.mariadb"
        secret = "synthetic-db-secret"
        private.write_text('DB_HOST=127.0.0.1\nDB_PORT=62335\nDB_NAME=db\n'
                           f'DB_USER=operator\nDB_PASSWORD="{secret}"\n')
        manager.apply_profile(self.project, "wordpress")
        # Exercise both installed launchers in a child process; Docker is replaced
        # by a fake executable. No Docker/network/database access occurs.
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        docker = bin_dir / "docker"
        docker.write_text(f'#!{sys.executable}\n'
                          'import json, os, sys\n'
                          'if sys.argv[1:3] == ["image", "inspect"]:\n'
                          '    print("64876fb1a4cb81bf51564c46eb60401097c067cc")\n'
                          'elif sys.argv[1] == "run":\n'
                          '    print(json.dumps({"args": sys.argv[1:], "env": dict(os.environ)}))\n'
                          'else:\n'
                          '    raise SystemExit(9)\n')
        docker.chmod(0o700)
        command = [sys.executable, str(self.project / mcp_runtime.LAUNCHER), "mariadb"]
        result = subprocess.run(command, cwd=self.base, check=True, capture_output=True, text=True,
                                env={"PATH": str(bin_dir) + os.pathsep + os.defpath,
                                     "HOME": str(self.base)})
        invoked = json.loads(result.stdout)
        self.assertEqual(invoked["env"]["DB_PASSWORD"], secret)
        self.assertEqual(invoked["env"]["MCP_READ_ONLY"], "false")
        self.assertEqual(invoked["env"]["MCP_BLOCK_SENSITIVE_SHOW"], "false")
        self.assertIn("--network=host", invoked["args"])
        self.assertNotIn(secret, " ".join(invoked["args"]))
        for rel in (*self.state()["files"], manager.STATE, "opencode.json"):
            self.assertNotIn(secret.encode(), (self.project / rel).read_bytes())
        before = local.read_bytes(), private.read_bytes()
        manager.apply_profile(self.project, "wordpress")
        manager.uninstall(self.project)
        self.assertEqual(before, (local.read_bytes(), private.read_bytes()))
        self.assertFalse(script.exists())

    def test_mcp_launcher_assets_are_selected_protected_and_validated(self):
        self.assertNotIn(".opencode/mcp/mariadb/launch.py", manager.desired_files("frontend-react"))
        self.assertNotIn(".opencode/mcp/mariadb/launch.py", manager.desired_files("backend-go"))
        manager.apply_profile(self.project, "wordpress")
        script = self.project / ".opencode/mcp/mariadb/launch.py"
        original = script.read_bytes()
        script.write_bytes(original + b"\n# local change\n")
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "wordpress")
        self.assertEqual(self.snapshot(), before)
        script.write_bytes(original)
        manager.apply_profile(self.project, "frontend-react")
        self.assertFalse(script.exists())
        source = self.source / "mcp/launchers/mariadb/launch.py"
        source.unlink()
        with self.assertRaisesRegex(manager.ProfileError, "Brak zarządzanego pliku launchera"):
            manager.desired_files("wordpress")
        source.symlink_to(self.source / "profile.py")
        with self.assertRaisesRegex(manager.ProfileError, "symlink"):
            manager.desired_files("wordpress")

    def test_wordpress_scripts_are_installed_without_running_and_protected(self):
        relative = ".opencode/ai-profile/scripts/restore-updraft.py"
        self.assertNotIn(relative, manager.desired_files("frontend-react"))
        manager.apply_profile(self.project, "wordpress")
        script = self.project / relative
        self.assertTrue(script.is_file())
        for helper in ("local-guard.php", "local-safety.php"):
            self.assertTrue((script.parent / helper).is_file())
        result = subprocess.run([sys.executable, str(script), "--help"], cwd=self.base,
                                capture_output=True, text=True, check=True)
        self.assertIn("--apply", result.stdout)
        self.assertIn("--map", result.stdout)
        self.assertFalse((self.project / "wp-content/mu-plugins/ai-profile-local-safety.php").exists())
        original = script.read_bytes()
        script.write_bytes(original + b"\n# user change\n")
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "wordpress")
        self.assertEqual(self.snapshot(), before)
        script.write_bytes(original)
        manager.apply_profile(self.project, "frontend-react")
        self.assertFalse(script.exists())

    def test_profile_scripts_reject_symlinks(self):
        script = self.source / "profiles/wordpress/scripts/local-guard.php"
        script.unlink()
        script.symlink_to(self.source / "profile.py")
        with self.assertRaisesRegex(manager.ProfileError, "symlink"):
            manager.desired_files("wordpress")

    def test_validate_profiles_resolve_shared_figma_and_example_detects_changes(self):
        manager.validate_sources()
        manager.validate_sources("wordpress")
        manager.validate_sources("frontend-react")
        path = self.source / "mcp/servers/figma.json"
        data = json.loads(path.read_text())
        data["url"] = "https://different.example/mcp"
        path.write_text(json.dumps(data))
        with self.assertRaisesRegex(manager.ProfileError, "mcp/example.json"):
            manager.validate_sources()

    def test_switch_from_react_to_wordpress_preserves_shared_figma(self):
        manager.apply_profile(self.project, "frontend-react")
        manager.apply_profile(self.project, "wordpress")
        self.assertEqual(set(self.cfg()["mcp"]), {"novamira", "figma", "mariadb"})
        manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(set(self.cfg()["mcp"]), {"figma"})
        self.assertFalse((self.project / ".opencode/ai-profile/README.md").exists())

    def test_registry_selects_only_explicit_servers_and_never_installs_everything(self):
        self.assertEqual(set(manager.desired_mcp("backend-go")), {"postgres"})
        self.assertEqual(set(manager.desired_mcp("frontend-react")), {"figma"})
        self.assertEqual(set(manager.desired_mcp("wordpress")), {"figma", "novamira", "mariadb"})
        self.assertEqual(manager.desired_mcp("wordpress")["figma"],
                         manager.desired_mcp("frontend-react")["figma"])
        manager.create_profile("no-mcp")
        self.assertEqual(manager.desired_mcp("no-mcp"), {})
        # An unused definition is available for selection, not installed implicitly.
        (self.source / "mcp/servers/unused.json").write_text(json.dumps({
            "type": "remote", "url": "https://unused.example/mcp",
        }))
        manager.validate_sources()
        manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(set(self.cfg()["mcp"]), {"figma"})
        manager.apply_profile(self.project, "no-mcp")
        self.assertEqual(self.cfg().get("mcp", {}), {})

    def test_shared_registry_change_updates_only_consumers_preserving_local_settings(self):
        manager.apply_profile(self.project, "frontend-react")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"figma": {"headers": {"X-Project": "project-only"}}}}))
        manager.apply_profile(self.project, "frontend-react")
        local_before = local.read_bytes()
        path = self.source / "mcp/servers/figma.json"
        data = json.loads(path.read_text())
        data["url"] = "https://updated.example/mcp"
        path.write_text(json.dumps(data))
        self.assertIn("CONTENT CHANGED WITHOUT VERSION BUMP", self.status_output())
        for profile in ("frontend-react", "wordpress"):
            self.assertEqual(manager.desired_mcp(profile)["figma"]["url"], data["url"])
            self.assertNotIn("headers", manager.desired_mcp(profile)["figma"])
        self.assertNotIn("figma", manager.desired_mcp("backend-go"))
        manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(self.cfg()["mcp"]["figma"]["url"], data["url"])
        self.assertEqual(self.cfg()["mcp"]["figma"]["headers"], {"X-Project": "project-only"})
        self.assertEqual(local.read_bytes(), local_before)
        self.assertEqual(json.loads(path.read_text()), data)

    def test_mcp_manifest_rejects_invalid_references_without_project_writes(self):
        path = self.source / "profiles/frontend-react/mcp.json"
        for value in ({}, [], {"servers": "figma"}, {"servers": ["figma", "figma"]},
                      {"servers": ["missing"]}, {"servers": ["../figma"]},
                      {"servers": ["/figma"]}, {"servers": [None]},
                      {"servers": [{}]}, {"servers": ["figma"], "figma": {}},
                      {"figma": {"type": "remote", "url": "https://inline.example/mcp"}}):
            with self.subTest(value=value):
                path.write_text(json.dumps(value))
                with self.assertRaises(manager.ProfileError):
                    manager.apply_profile(self.project, "frontend-react")
                self.assertEqual(self.snapshot(), {})

    def test_registry_rejects_symlinks_and_invalid_definitions(self):
        path = self.source / "mcp/servers/figma.json"
        for entry in ({}, [], {"type": "local", "command": []}):
            with self.subTest(entry=entry):
                path.write_text(json.dumps(entry))
                with self.assertRaises(mcp_runtime.ConfigError):
                    manager.apply_profile(self.project, "frontend-react")
                self.assertEqual(self.snapshot(), {})
        path.unlink()
        path.symlink_to(self.source / "mcp/servers/novamira.json")
        with self.assertRaisesRegex(manager.ProfileError, "symlink"):
            manager.desired_mcp("frontend-react")

    def test_scoped_validation_ignores_unselected_registry_definitions(self):
        path = self.source / "mcp/servers/postgres.json"
        path.write_text("{}")
        manager.validate_sources("wordpress")
        with self.assertRaises(mcp_runtime.ConfigError):
            manager.validate_sources()

    def test_existing_expanded_mcp_state_remains_updateable(self):
        # Installed state stores complete definitions, never registry references.
        manager.apply_profile(self.project, "frontend-react")
        config = self.cfg()
        config["mcp"]["figma"].pop("disabled")
        config["mcp"]["figma"]["enabled"] = True
        (self.project / "opencode.json").write_text(json.dumps(config))
        state = self.state()
        state["mcp"]["figma"] = config["mcp"]["figma"]
        state["mcp_base"]["figma"] = config["mcp"]["figma"]
        (self.project / manager.STATE).write_text(json.dumps(state))
        manager.apply_profile(self.project, "frontend-react")
        self.assertFalse(self.cfg()["mcp"]["figma"]["disabled"])
        self.assertNotIn("servers", self.state()["mcp_base"])

    def test_postgres_command_has_no_dependency_workarounds_or_credentials(self):
        postgres = manager.desired_mcp("backend-go")["postgres"]
        self.assertEqual(postgres["command"], [
            "docker", "run", "-i", "--rm", "-e", "DATABASE_URI",
            "crystaldba/postgres-mcp"
        ])
        self.assertEqual(postgres["access_mode"], "unrestricted")
        self.assertEqual(postgres["environment"], {"DATABASE_URI": "{env:DATABASE_URI}"})

    def test_docker_network_override_passes_connection_only_in_environment(self):
        example = json.loads((self.source / "mcp/local.example.json").read_text())
        local = self.project / mcp_runtime.LOCAL
        local.parent.mkdir()
        local.write_text(json.dumps(example))
        uri = "postgresql://user:test-secret@postgres:5432/testdb"
        (self.project / ".env.stage").write_text(f"STAGE_DATABASE_URL={uri}\n")
        manager.apply_profile(self.project, "backend-go")
        with patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "postgres")
        executable, command, environment = execute.call_args.args
        self.assertEqual(executable, "docker")
        self.assertEqual(command[command.index("--network") + 1], "my-project_default")
        self.assertLess(command.index("--network"), command.index("crystaldba/postgres-mcp"))
        self.assertEqual(command[command.index("-e") + 1], "DATABASE_URI")
        self.assertEqual(environment["DATABASE_URI"], uri)
        self.assertNotIn("test-secret", " ".join(command))
        self.assertNotIn(".env.stage", " ".join(command))

    def local_config(self, postgres):
        path = self.project / mcp_runtime.LOCAL
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"mcp": {"postgres": postgres}}))
        return path

    def test_postgres_access_mode_default_and_local_settings(self):
        base = manager.desired_mcp("backend-go")
        effective, _ = mcp_runtime.resolve(self.project, base)
        self.assertEqual(effective["postgres"]["command"][-1], "--access-mode=unrestricted")
        for mode in ("restricted", "unrestricted"):
            with self.subTest(mode=mode):
                self.local_config({"access_mode": mode})
                effective, _ = mcp_runtime.resolve(self.project, base)
                flags = [arg for arg in effective["postgres"]["command"] if arg.startswith("--access-mode")]
                self.assertEqual(flags, [f"--access-mode={mode}"])
                manager.apply_profile(self.project, "backend-go")
                self.assertNotIn("access_mode", self.cfg()["mcp"]["postgres"])

    def test_legacy_command_mode_is_replaced_by_default(self):
        for arguments in (["--access-mode=restricted"], ["--access-mode", "restricted"],
                          ["--access-mode=unrestricted"], []):
            with self.subTest(arguments=arguments):
                self.local_config({"command": ["postgres-mcp", *arguments]})
                effective, _ = mcp_runtime.resolve(self.project, manager.desired_mcp("backend-go"))
                self.assertEqual(effective["postgres"]["command"], ["postgres-mcp", "--access-mode=unrestricted"])
        self.local_config({"access_mode": "restricted", "command": ["postgres-mcp", "--access-mode", "restricted"]})
        effective, _ = mcp_runtime.resolve(self.project, manager.desired_mcp("backend-go"))
        self.assertEqual(effective["postgres"]["command"], ["postgres-mcp", "--access-mode=restricted"])

    def test_access_mode_overrides_legacy_command_argument(self):
        for mode, old_argument in (("unrestricted", "restricted"), ("restricted", "unrestricted")):
            with self.subTest(mode=mode):
                self.local_config({"access_mode": mode, "command": ["postgres-mcp", "--access-mode", old_argument]})
                manager.apply_profile(self.project, "backend-go")
                with patch.dict(mcp_runtime.os.environ, {"DATABASE_URI": "test-connection"}), \
                        patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
                    mcp_runtime.launch(self.project, "postgres")
                self.assertEqual(execute.call_args.args[1], ["postgres-mcp", f"--access-mode={mode}"])

    def test_invalid_access_mode_never_writes_or_launches(self):
        manager.apply_profile(self.project, "backend-go")
        invalid = [
            {"access_mode": value} for value in ("invalid", "", None, 1, True, [], {})
        ] + [
            {"command": ["postgres-mcp", "--access-mode=invalid"]},
            {"command": ["postgres-mcp", "--access-mode"]},
            {"command": ["postgres-mcp", "--access-mode=restricted", "--access-mode=unrestricted"]},
            {"command": ["postgres-mcp", "--access-mode=restricted", "--access-mode=restricted"]},
        ]
        for override in invalid:
            with self.subTest(override=override):
                self.local_config(override)
                before = self.snapshot()
                with self.assertRaises(mcp_runtime.ConfigError):
                    manager.apply_profile(self.project, "backend-go")
                self.assertEqual(before, self.snapshot())
                with patch.object(mcp_runtime.os, "execvpe") as execute:
                    with self.assertRaises(mcp_runtime.ConfigError):
                        mcp_runtime.launch(self.project, "postgres")
                    execute.assert_not_called()

    def test_access_mode_survives_update_and_runtime_receives_single_flag(self):
        local = self.local_config({"access_mode": "restricted"})
        before = local.read_bytes()
        manager.apply_profile(self.project, "backend-go")
        mcp_path = self.source / "mcp/servers/postgres.json"
        base = json.loads(mcp_path.read_text())
        base["command"].append("--transport=stdio")
        mcp_path.write_text(json.dumps(base))
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(local.read_bytes(), before)
        with patch.dict(mcp_runtime.os.environ, {"DATABASE_URI": "test-connection"}), \
                patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "postgres")
        command = execute.call_args.args[1]
        self.assertEqual(command[-1], "--access-mode=restricted")
        self.assertEqual(sum(arg.startswith("--access-mode") for arg in command), 1)
        self.assertIn("--transport=stdio", command)

    def test_access_mode_only_supported_for_local_postgres(self):
        local = self.project / mcp_runtime.LOCAL
        local.parent.mkdir()
        local.write_text(json.dumps({"mcp": {"figma": {"access_mode": "unrestricted"}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "only for local postgres"):
            manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(before, self.snapshot())

    def test_local_overrides_survive_update_and_uninstall(self):
        path = self.local_config({
            "env_file": ".env.stage",
            "environment": {"DATABASE_URI": "{env:STAGE_DB}"},
            "command": ["custom-server", "--read-only"],
        })
        env = self.project / ".env.stage"
        env.write_text("STAGE_DB=postgresql://local-secret\n")
        original = path.read_bytes(), env.read_bytes()
        manager.apply_profile(self.project, "backend-go")
        old = self.state()
        self.assertEqual(old["mcp_base"]["postgres"], manager.desired_mcp("backend-go")["postgres"])
        mcp_path = self.source / "mcp/servers/postgres.json"
        base = json.loads(mcp_path.read_text())
        base["command"].append("--new-upstream-flag")
        base["environment"]["DATABASE_URI"] = "{env:UPSTREAM_DB}"
        base["enabled"] = False
        mcp_path.write_text(json.dumps(base))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            manager.apply_profile(self.project, "backend-go")
        self.assertIn("postgres.command", output.getvalue())
        self.assertIn("postgres.environment.DATABASE_URI", output.getvalue())
        self.assertNotIn("local-secret", output.getvalue())
        self.assertFalse(self.cfg()["mcp"]["postgres"]["enabled"])
        effective, _ = mcp_runtime.resolve(self.project, self.state()["mcp_base"])
        self.assertEqual(effective["postgres"]["command"], ["custom-server", "--read-only", "--access-mode=unrestricted"])
        self.assertEqual((path.read_bytes(), env.read_bytes()), original)
        for rel in [manager.STATE, "opencode.json"]:
            self.assertNotIn("local-secret", (self.project / rel).read_text())
        manager.uninstall(self.project)
        self.assertEqual((path.read_bytes(), env.read_bytes()), original)

    def test_local_changes_are_not_managed_conflicts(self):
        manager.apply_profile(self.project, "backend-go")
        self.local_config({"environment": {"DATABASE_URI": "{env:OTHER_DATABASE}"}})
        self.assertIn("MCP local configuration: changed", self.status_output())
        self.assertIn("Local modifications: none", self.status_output())
        manager.apply_profile(self.project, "backend-go")
        self.assertNotIn("MCP local configuration: changed", self.status_output())
        self.assertIn("MCP local overrides: postgres", self.status_output())

    def test_invalid_local_settings_fail_before_writes(self):
        manager.apply_profile(self.project, "backend-go")
        for override in ({"command": []}, {"environment": {"DATABASE_URI": 5}},
                         {"env_file": "../outside.env"}, {"env_file": "/tmp/env"},
                         {"env_file": None}, {"env_flie": ".env"}, {"enabled": "yes"},
                         {"environment": {"INVALID-NAME": "value"}},
                         {"timeout": "slow"}, {"timeout": {"startup": -1}},
                         {"type": "remote", "url": "https://example.org/mcp"}):
            self.local_config(override)
            before = self.snapshot()
            with self.assertRaises(mcp_runtime.ConfigError):
                manager.apply_profile(self.project, "backend-go")
            self.assertEqual(before, self.snapshot())
        path = self.project / mcp_runtime.LOCAL
        path.write_text(json.dumps({"mcp": {"unknown": {"enabled": False}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "unmanaged"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_previous_state_without_base_migrates(self):
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        state["mcp"] = state.pop("mcp_base")
        state.pop("mcp_local_hash")
        for rel in (mcp_runtime.LAUNCHER,):
            del state["files"][rel]
            (self.project / rel).unlink()
        cfg = self.cfg()
        cfg["mcp"] = state["mcp"]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        (self.project / manager.STATE).write_text(json.dumps(state))
        self.local_config({"env_file": ".env.stage", "environment": {"DATABASE_URI": "{env:STAGE_URL}"}})
        manager.apply_profile(self.project, "backend-go")
        self.assertIn("mcp_base", self.state())
        self.assertTrue((self.project / mcp_runtime.LAUNCHER).exists())

    def test_update_removes_managed_manifest_and_launches_from_state(self):
        self.local_config({"command": ["test-server"], "environment": {"DATABASE_URI": "{env:TEST_DB}"}})
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        relative = ".opencode/ai-profile-mcp.json"
        obsolete = self.project / relative
        obsolete.write_text(json.dumps({"servers": state["mcp_base"]}))
        state["files"][relative] = manager.sha(obsolete.read_bytes())
        (self.project / manager.STATE).write_text(json.dumps(state))
        local_before = (self.project / mcp_runtime.LOCAL).read_bytes()
        manager.apply_profile(self.project, "backend-go")
        self.assertFalse(obsolete.exists())
        self.assertNotIn(relative, self.state()["files"])
        self.assertEqual((self.project / mcp_runtime.LOCAL).read_bytes(), local_before)
        with patch.dict(mcp_runtime.os.environ, {"TEST_DB": "test-value"}), \
                patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "postgres")
        executable, command, environment = execute.call_args.args
        self.assertEqual((executable, command), ("test-server", ["test-server", "--access-mode=unrestricted"]))
        self.assertEqual(environment["DATABASE_URI"], "test-value")
        before = self.snapshot()
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_modified_obsolete_manifest_still_protected(self):
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        relative = ".opencode/ai-profile-mcp.json"
        obsolete = self.project / relative
        obsolete.write_text('{}\n')
        state["files"][relative] = manager.sha(obsolete.read_bytes())
        (self.project / manager.STATE).write_text(json.dumps(state))
        obsolete.write_text('{"user": "changed"}\n')
        before = self.snapshot()
        with self.assertRaisesRegex(manager.ProfileError, "Konflikt"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_runtime_rejects_state_without_base_instead_of_using_rendered_command(self):
        manager.apply_profile(self.project, "backend-go")
        state = self.state()
        del state["mcp_base"]
        (self.project / manager.STATE).write_text(json.dumps(state))
        with patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "run ai-profiles update"):
                mcp_runtime.launch(self.project, "postgres")
        execute.assert_not_called()

    def test_runtime_loads_custom_env_and_maps_names_without_shell(self):
        self.local_config({
            "env_file": ".env.stage",
            "environment": {"DATABASE_URI": "{env:STAGE_URL}"},
            "command": [sys.executable, "-c",
                        "import os,sys,json; print(json.dumps([os.environ['DATABASE_URI'], sys.stdin.readline().strip(), os.getcwd()]))"],
        })
        (self.project / ".env.stage").write_text('STAGE_URL="literal-$(never-execute)#value"\n')
        manager.apply_profile(self.project, "backend-go")
        command = self.cfg()["mcp"]["postgres"]["command"]
        # Launch from outside the project, with an unrelated inherited value.
        result = subprocess.run(command, cwd=self.base, input="stdio-preserved\n",
                                text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout),
                         ["literal-$(never-execute)#value", "stdio-preserved", str(self.project)])
        # Runtime reads the local file again, without requiring update.
        (self.project / ".env.stage").write_text("STAGE_URL=changed\n")
        result = subprocess.run(command, input="next\n", text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout)[0], "changed")
        self.local_config({"env_file": ".env.stage", "environment": {"DATABASE_URI": "{env:MISSING_TEST_VARIABLE}"}})
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertNotIn("literal-", result.stderr)

    def test_runtime_dotenv_precedence_and_validation(self):
        path = self.project / ".env.custom"
        path.write_text("# comment\nexport CUSTOM='from-file' # comment\nEMPTY=\nURI=postgresql://host/db#fragment\n")
        entry = {"env_file": ".env.custom", "environment": {"DATABASE_URI": "{env:CUSTOM}"}}
        inherited = {"CUSTOM": "inherited", "KEEP": "keep"}
        result = mcp_runtime.environment_for(self.project, entry, inherited)
        self.assertEqual(result["DATABASE_URI"], "from-file")
        self.assertEqual(result["KEEP"], "keep")
        self.assertEqual(result["URI"], "postgresql://host/db#fragment")
        self.assertEqual(inherited["CUSTOM"], "inherited")
        path.write_text('CUSTOM="secret-unclosed\n')
        with self.assertRaises(mcp_runtime.ConfigError) as error:
            mcp_runtime.environment_for(self.project, entry, {})
        self.assertNotIn("secret-unclosed", str(error.exception))

    def test_local_merge_arrays_and_idempotence(self):
        self.local_config({"command": ["replacement"], "environment": {"EXTRA": "extra"}})
        base = manager.desired_mcp("backend-go")
        effective, _ = mcp_runtime.resolve(self.project, base)
        self.assertEqual(effective["postgres"]["command"], ["replacement", "--access-mode=unrestricted"])
        self.assertEqual(effective["postgres"]["environment"], {
            "DATABASE_URI": "{env:DATABASE_URI}", "EXTRA": "extra"
        })
        self.assertEqual(base["postgres"]["command"][0], "docker")
        manager.apply_profile(self.project, "backend-go")
        before = self.snapshot()
        manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_orphaned_override_blocks_switch_without_writes(self):
        self.local_config({"env_file": ".env.stage"})
        manager.apply_profile(self.project, "backend-go")
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "removed MCP"):
            manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(before, self.snapshot())

    def test_remote_override_remains_native(self):
        path = self.project / mcp_runtime.LOCAL
        path.parent.mkdir()
        path.write_text(json.dumps({"mcp": {"figma": {
            "url": "https://example.org/custom-mcp", "headers": {"X-Key": "{env:FIGMA_KEY}"}
        }}}))
        manager.apply_profile(self.project, "frontend-react")
        entry = self.cfg()["mcp"]["figma"]
        self.assertEqual(entry["url"], "https://example.org/custom-mcp")
        self.assertEqual(entry["headers"], {"X-Key": "{env:FIGMA_KEY}"})
        self.assertNotIn("command", entry)
        path.write_text(json.dumps({"mcp": {"figma": {"env_file": ".env"}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "only for local"):
            manager.apply_profile(self.project, "frontend-react")
        self.assertEqual(before, self.snapshot())

    def test_missing_env_file_and_symlinks_fail_safely(self):
        self.local_config({"env_file": ".env.missing"})
        manager.apply_profile(self.project, "backend-go")
        command = self.cfg()["mcp"]["postgres"]["command"]
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        outside = self.base / "private-env"
        outside.write_text("DATABASE_URI=private-secret\n")
        (self.project / ".env.missing").symlink_to(outside)
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "Symlinks"):
            manager.apply_profile(self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("private-secret", result.stderr)

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
