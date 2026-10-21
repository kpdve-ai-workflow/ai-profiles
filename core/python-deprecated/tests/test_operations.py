from tests.support import *


class OperationsTests(ProfileTestCase):

    def test_versions_install_update_and_idempotence(self):
        with patch("profile_management.actions.installation.source_commit", return_value="a" * 40):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.0.0")
        self.assertEqual(self.state()["source_commit"], "a" * 40)
        for name in ("go-testing", "go-database"):
            skill = self.source / "profiles/backend-go/skills" / name
            skill.mkdir()
            (skill / "SKILL.md").write_text(f"---\nname: {name}\ndescription: Testing\n---\n")
        self.set_version("1.1.0")
        self.assertIn("UPDATE AVAILABLE", self.status_output())
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(self.state()["version"], "1.1.0")
        for name in ("go-testing", "go-database"):
            self.assertTrue((self.project / f".opencode/skills/{name}/SKILL.md").exists())
        self.assertIn("UP TO DATE", self.status_output())
        before = self.snapshot()
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        shutil.rmtree(self.source / "profiles/backend-go/skills/go-testing")
        self.set_version("2.0.0")
        apply_profile(self.context, self.project, "backend-go")
        self.assertFalse((self.project / ".opencode/skills/go-testing").exists())

    def test_invalid_version_numeric_comparison_and_downgrade(self):
        apply_profile(self.context, self.project, "backend-go")
        before = self.snapshot()
        for version in ("1.0", "01.0.0", "1.0.0-beta", "invalid", None):
            self.set_version(version)
            with self.assertRaisesRegex(ProfileError, "wersja"):
                apply_profile(self.context, self.project, "backend-go")
            self.assertEqual(before, self.snapshot())
        self.set_version("1.9.0")
        apply_profile(self.context, self.project, "backend-go")
        self.set_version("1.10.0")
        self.assertIn("UPDATE AVAILABLE", self.status_output())
        apply_profile(self.context, self.project, "backend-go")
        self.assertIn("Content: UNCHANGED", self.status_output())
        self.set_version("1.9.0")
        self.assertIn("INSTALLED NEWER", self.status_output())
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Downgrade"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_install_update_add_remove_skills_preserve_custom_and_uninstall(self):
        custom_cfg = {
            "model": "test/model", "permission": {"read": "allow"},
            "mcp": {"unmanaged": {"type": "remote", "url": "https://example.org/mcp"}},
            "instructions": ["custom-rules.md"],
        }
        (self.project / "opencode.json").write_text(json.dumps(custom_cfg))
        (self.project / ".opencode/skills/user-skill").mkdir(parents=True)
        (self.project / ".opencode/skills/user-skill/SKILL.md").write_text("my skill")
        apply_profile(self.context, self.project, "backend-go")
        self.assertIn("postgres", self.cfg()["mcp"])
        self.assertEqual(self.cfg()["model"], "test/model")
        self.assertEqual(self.cfg()["instructions"], ["custom-rules.md"])
        self.assertFalse((self.project / GATE).exists())

        # Add two new skills and modify one previously installed skill upstream.
        for skill in ("go-database", "go-testing"):
            directory = self.source / "profiles/backend-go/skills" / skill
            directory.mkdir()
            (directory / "SKILL.md").write_text(f"---\nname: {skill}\ndescription: {skill}\n---\n\ncontent")
            (directory / "references").mkdir()
            (directory / "references/notes.md").write_text("new notes")
        source_go = self.source / "profiles/backend-go/skills/go-backend/SKILL.md"
        source_go.write_text(source_go.read_text() + "\nNew practice\n")
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual((self.project / ".opencode/skills/go-backend/SKILL.md").read_text(), source_go.read_text())
        for skill in ("go-database", "go-testing"):
            self.assertTrue((self.project / f".opencode/skills/{skill}/SKILL.md").exists())
            self.assertTrue((self.project / f".opencode/skills/{skill}/references/notes.md").exists())
        self.assertEqual(self.cfg()["mcp"]["unmanaged"], custom_cfg["mcp"]["unmanaged"])

        # Retire a managed skill -> update removes only files it owns.
        shutil.rmtree(self.source / "profiles/backend-go/skills/go-testing")
        apply_profile(self.context, self.project, "backend-go")
        self.assertFalse((self.project / ".opencode/skills/go-testing").exists())
        self.assertTrue((self.project / ".opencode/skills/user-skill/SKILL.md").exists())

        # Switch Go -> React, preserve General and unrelated MCP.
        apply_profile(self.context, self.project, "frontend-react")
        self.assertIn("figma", self.cfg()["mcp"])
        self.assertNotIn("postgres", self.cfg()["mcp"])
        self.assertTrue((self.project / ".opencode/skills/react-frontend/SKILL.md").exists())
        self.assertTrue((self.project / ".opencode/skills/refine-plan/SKILL.md").exists())
        uninstall(self.context, self.project)
        self.assertFalse((self.project / STATE).exists())
        self.assertFalse((self.project / ".opencode/skills/react-frontend").exists())
        self.assertTrue((self.project / ".opencode/skills/user-skill/SKILL.md").exists())
        self.assertEqual(self.cfg(), custom_cfg)
        ignore = self.project / ".gitignore"
        self.assertNotIn(GATE, ignore.read_text() if ignore.exists() else "")

    def test_conflict_before_any_writes(self):
        apply_profile(self.context, self.project, "backend-go")
        managed = self.project / ".opencode/skills/go-backend/SKILL.md"
        managed.write_text("edited manually")
        state_before = (self.project / STATE).read_bytes()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(managed.read_text(), "edited manually")
        self.assertEqual((self.project / STATE).read_bytes(), state_before)
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            uninstall(self.context, self.project)
        self.assertTrue((self.project / STATE).exists())

    def test_existing_jsonc_is_rejected(self):
        (self.project / "opencode.jsonc").write_text("{}")
        with self.assertRaisesRegex(ProfileError, "opencode.jsonc"):
            apply_profile(self.context, self.project, "backend-go")

    def test_install_and_update_do_not_require_approval(self):
        apply_profile(self.context, self.project, "backend-go")
        for relative in (".opencode/plugins/approval-gate.js", ".opencode/ai-profile-gate.py", GATE):
            self.assertFalse((self.project / relative).exists())
        apply_profile(self.context, self.project, "backend-go")
        self.assertNotIn("LOCKED", self.status_output())
        self.assertIn("Approval gate: not installed", self.status_output())

    def test_local_changes_are_not_managed_conflicts(self):
        apply_profile(self.context, self.project, "backend-go")
        self.local_config({"environment": {"DATABASE_URI": "{env:OTHER_DATABASE}"}})
        self.assertIn("MCP local configuration: changed", self.status_output())
        self.assertIn("Local modifications: none", self.status_output())
        apply_profile(self.context, self.project, "backend-go")
        self.assertNotIn("MCP local configuration: changed", self.status_output())
        self.assertIn("MCP local overrides: postgres", self.status_output())

    def test_invalid_local_settings_fail_before_writes(self):
        apply_profile(self.context, self.project, "backend-go")
        for override in ({"command": []}, {"environment": {"DATABASE_URI": 5}},
                         {"env_file": "../outside.env"}, {"env_file": "/tmp/env"},
                         {"env_file": None}, {"env_flie": ".env"}, {"enabled": "yes"},
                         {"environment": {"INVALID-NAME": "value"}},
                         {"timeout": "slow"}, {"timeout": {"startup": -1}},
                         {"type": "remote", "url": "https://example.org/mcp"}):
            self.local_config(override)
            before = self.snapshot()
            with self.assertRaises(mcp_runtime.ConfigError):
                apply_profile(self.context, self.project, "backend-go")
            self.assertEqual(before, self.snapshot())
        path = self.project / mcp_runtime.LOCAL
        path.write_text(json.dumps({"mcp": {"unknown": {"enabled": False}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "unmanaged"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_update_removes_managed_manifest_and_launches_from_state(self):
        self.local_config({"command": ["test-server"], "environment": {"DATABASE_URI": "{env:TEST_DB}"}})
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        relative = ".opencode/ai-profile-mcp.json"
        obsolete = self.project / relative
        obsolete.write_text(json.dumps({"servers": state["mcp_base"]}))
        state["files"][relative] = sha(obsolete.read_bytes())
        (self.project / STATE).write_text(json.dumps(state))
        local_before = (self.project / mcp_runtime.LOCAL).read_bytes()
        apply_profile(self.context, self.project, "backend-go")
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
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_modified_obsolete_manifest_still_protected(self):
        apply_profile(self.context, self.project, "backend-go")
        state = self.state()
        relative = ".opencode/ai-profile-mcp.json"
        obsolete = self.project / relative
        obsolete.write_text('{}\n')
        state["files"][relative] = sha(obsolete.read_bytes())
        (self.project / STATE).write_text(json.dumps(state))
        obsolete.write_text('{"user": "changed"}\n')
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_local_merge_arrays_and_idempotence(self):
        self.local_config({"command": ["replacement"], "environment": {"EXTRA": "extra"}})
        base = desired_mcp(self.context, "backend-go")
        effective, _ = mcp_runtime.resolve(self.project, base)
        self.assertEqual(effective["postgres"]["command"], ["replacement", "--access-mode=unrestricted"])
        self.assertEqual(effective["postgres"]["environment"], {
            "DATABASE_URI": "{env:DATABASE_URI}", "EXTRA": "extra"
        })
        self.assertEqual(base["postgres"]["command"][0], "docker")
        apply_profile(self.context, self.project, "backend-go")
        before = self.snapshot()
        apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
