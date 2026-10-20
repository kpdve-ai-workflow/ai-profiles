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
from ai_profiles.context import RepositoryContext
from ai_profiles.errors import ProfileError
from ai_profiles.constants import STATE, RULE, GATE
from ai_profiles.filesystem import sha
from ai_profiles.profiles import profile_names, create_profile, version_tuple, profile_metadata, bump_profile, source_commit
from ai_profiles.payload import desired_files
from ai_profiles.project import selected_project, state_for, config_for
from ai_profiles.mcp import desired_mcp, mcp_definition, configured_mcp, check_mcp
from ai_profiles.operations import apply_profile, uninstall
from ai_profiles.status import status
from ai_profiles.validation import validate_sources
from ai_profiles.legacy import migrate_legacy
import mcp_runtime


from tests.support import ProfileTestCase


class LegacyTests(ProfileTestCase):

    def test_unversioned_content_and_local_conflict(self):
        apply_profile(self.context, self.project, "backend-go")
        source = self.source / "profiles/backend-go/skills/go-backend/SKILL.md"
        source.write_text(source.read_text() + "\nUpstream change\n")
        self.assertIn("CONTENT CHANGED WITHOUT VERSION BUMP", self.status_output())
        local = self.project / ".opencode/skills/go-backend/SKILL.md"
        local.write_text(local.read_text() + "\nUser change\n")
        self.assertIn(".opencode/skills/go-backend/SKILL.md", self.status_output())
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        local.write_bytes(source.read_bytes().replace(b"\nUpstream change\n", b""))
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(local.read_bytes(), source.read_bytes())
        self.assertIn("UP TO DATE", self.status_output())

    def test_old_hashed_state_migration(self):
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        del state["version"]
        del state["source_commit"]
        (self.project / STATE).write_text(json.dumps(state))
        self.assertIn("VERSION UNKNOWN", self.status_output())
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.0.0")
        self.assertIn("source_commit", self.state())

    def test_retire_old_rules_and_skill_hash_checked_idempotently(self):
        apply_profile(self.context, self.project, "backend-go")
        retired_skill = ".opencode/skills/brain-to-implementer/SKILL.md"
        rules = self.project / RULE
        rules.write_text("old managed rules\n")
        skill = self.project / retired_skill
        skill.parent.mkdir(parents=True)
        skill.write_text("old managed skill\n")
        cfg = self.cfg()
        cfg["instructions"] = ["keep-me.md", RULE]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        state = self.state()
        state["files"][RULE] = sha(rules.read_bytes())
        state["files"][retired_skill] = sha(skill.read_bytes())
        state["instructions_added"] = True
        (self.project / STATE).write_text(json.dumps(state))
        apply_profile(self.context, self.project, "backend-go")
        self.assertFalse(rules.exists())
        self.assertFalse(skill.exists())
        self.assertEqual(self.cfg()["instructions"], ["keep-me.md"])
        self.assertFalse(self.state()["instructions_added"])
        snapshot = self.snapshot()
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(snapshot, self.snapshot())

    def test_retired_file_conflicts_abort_without_partial_changes(self):
        apply_profile(self.context, self.project, "backend-go")
        rules = self.project / RULE
        rules.write_text("managed old rules\n")
        state = self.state()
        state["files"][RULE] = sha(rules.read_bytes())
        state["instructions_added"] = True
        (self.project / STATE).write_text(json.dumps(state))
        cfg = self.cfg()
        cfg["instructions"] = [RULE]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        rules.write_text("locally modified\n")
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_uninstall_removes_hash_verified_retired_files_and_owned_instruction(self):
        apply_profile(self.context, self.project, "backend-go")
        for rel, content in ((RULE, b"old rules\n"),
                             (".opencode/skills/brain-to-implementer/SKILL.md", b"old skill\n")):
            path = self.project / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        state = self.state()
        state["files"][RULE] = sha(b"old rules\n")
        state["files"][".opencode/skills/brain-to-implementer/SKILL.md"] = sha(b"old skill\n")
        state["instructions_added"] = True
        (self.project / STATE).write_text(json.dumps(state))
        cfg = self.cfg()
        cfg["instructions"] = [RULE, "my-instructions.md"]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        uninstall(self.context, self.project)
        self.assertFalse((self.project / RULE).exists())
        self.assertFalse((self.project / ".opencode/skills/brain-to-implementer/SKILL.md").exists())
        self.assertEqual(self.cfg()["instructions"], ["my-instructions.md"])

    def test_user_owned_retired_instruction_and_untracked_files_survive(self):
        cfg = {"instructions": [RULE, "other.md"]}
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(self.cfg()["instructions"], cfg["instructions"])
        untracked = self.project / RULE
        untracked.parent.mkdir(parents=True, exist_ok=True)
        untracked.write_text("user file\n")
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(untracked.read_text(), "user file\n")

    def test_legacy_migration_then_update(self):
        apply_profile(self.context, self.project, "backend-go")
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
        state_path = self.project / STATE
        state_path.write_text(json.dumps({"profile": "backend-go", "skill": "go-backend", "mcp": "postgres"}))
        (self.project / RULE).parent.mkdir(parents=True, exist_ok=True)
        (self.project / RULE).write_text("legacy workflow rules\n")
        plugin = self.project / ".opencode/plugins/approval-gate.js"
        plugin.parent.mkdir(parents=True)
        plugin.write_text("// Historical approval gate fixture\n")
        (self.project / mcp_runtime.LAUNCHER).unlink()
        with self.assertRaisesRegex(ProfileError, "--accept-existing"):
            migrate_legacy(self.context, self.project, accept_existing=False)
        migrate_legacy(self.context, self.project, accept_existing=True)
        self.assertEqual(self.state()["format"], 1)
        apply_profile(self.context, self.project, "backend-go")
        self.assertNotIn(".opencode/ai-profile-gate.py", self.state()["files"])
        self.assertFalse(plugin.exists())
        self.assertTrue((skills / "make-plan/SKILL.md").is_file())
        self.assertFalse(legacy_skill.exists())

    def test_update_retires_owned_gate_and_preserves_unrelated_plugins(self):
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        for relative in (".opencode/plugins/approval-gate.js", ".opencode/ai-profile-gate.py"):
            path = self.project / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("historical gate\n")
            state["files"][relative] = sha(path.read_bytes())
        (self.project / GATE).write_text('{"approved": true, "task": "TASK-001"}\n')
        (self.project / ".gitignore").write_text("custom-entry\n" + GATE + "\n")
        state["gitignore_added"] = True
        (self.project / STATE).write_text(json.dumps(state))
        custom = self.project / ".opencode/plugins/custom.js"
        custom.write_text("// user plugin\n")
        self.assertIn("legacy installation", self.status_output())
        apply_profile(self.context, self.project, "backend-go")
        self.assertTrue(custom.exists())
        self.assertFalse((self.project / ".opencode/plugins/approval-gate.js").exists())
        self.assertFalse((self.project / ".opencode/ai-profile-gate.py").exists())
        self.assertFalse((self.project / GATE).exists())
        self.assertEqual((self.project / ".gitignore").read_text(), "custom-entry\n")
        self.assertFalse(self.state()["gitignore_added"])
        apply_profile(self.context, self.project, "backend-go")
        self.assertIn("Approval gate: not installed", self.status_output())

    def test_legacy_command_mode_is_replaced_by_default(self):
        for arguments in (["--access-mode=restricted"], ["--access-mode", "restricted"],
                          ["--access-mode=unrestricted"], []):
            with self.subTest(arguments=arguments):
                self.local_config({"command": ["postgres-mcp", *arguments]})
                effective, _ = mcp_runtime.resolve(self.project, desired_mcp(self.context, "backend-go"))
                self.assertEqual(effective["postgres"]["command"], ["postgres-mcp", "--access-mode=unrestricted"])
        self.local_config({"access_mode": "restricted", "command": ["postgres-mcp", "--access-mode", "restricted"]})
        effective, _ = mcp_runtime.resolve(self.project, desired_mcp(self.context, "backend-go"))
        self.assertEqual(effective["postgres"]["command"], ["postgres-mcp", "--access-mode=restricted"])

    def test_access_mode_overrides_legacy_command_argument(self):
        for mode, old_argument in (("unrestricted", "restricted"), ("restricted", "unrestricted")):
            with self.subTest(mode=mode):
                self.local_config({"access_mode": mode, "command": ["postgres-mcp", "--access-mode", old_argument]})
                apply_profile(self.context, self.project, "backend-go")
                with patch.dict(mcp_runtime.os.environ, {"DATABASE_URI": "test-connection"}), \
                        patch.object(mcp_runtime.os, "chdir"), patch.object(mcp_runtime.os, "execvpe") as execute:
                    mcp_runtime.launch(self.project, "postgres")
                self.assertEqual(execute.call_args.args[1], ["postgres-mcp", f"--access-mode={mode}"])

    def test_previous_state_without_base_migrates(self):
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        state["mcp"] = state.pop("mcp_base")
        state.pop("mcp_local_hash")
        for rel in (mcp_runtime.LAUNCHER,):
            del state["files"][rel]
            (self.project / rel).unlink()
        cfg = self.cfg()
        cfg["mcp"] = state["mcp"]
        (self.project / "opencode.json").write_text(json.dumps(cfg))
        (self.project / STATE).write_text(json.dumps(state))
        self.local_config({"env_file": ".env.stage", "environment": {"DATABASE_URI": "{env:STAGE_URL}"}})
        apply_profile(self.context, self.project, "backend-go")
        self.assertIn("mcp_base", self.state())
        self.assertTrue((self.project / mcp_runtime.LAUNCHER).exists())
