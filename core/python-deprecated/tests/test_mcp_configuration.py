from tests.support import *


class McpConfigurationTests(ProfileTestCase):

    def test_mcp_content_and_conflict(self):
        apply_profile(self.context, self.project, "backend-go")
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
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_update_common_rules_and_mcp(self):
        apply_profile(self.context, self.project, "backend-go")
        mcp_path = self.source / "mcp/servers/postgres.json"
        mcp = json.loads(mcp_path.read_text())
        mcp["enabled"] = False
        mcp_path.write_text(json.dumps(mcp))
        apply_profile(self.context, self.project, "backend-go")
        expected, _ = configured_mcp(self.project, {"postgres": mcp})
        self.assertEqual(self.cfg()["mcp"]["postgres"], expected["postgres"])
        self.assertIn("UP TO DATE", self.status_output())

    def test_registry_selects_only_explicit_servers_and_never_installs_everything(self):
        self.assertEqual(set(desired_mcp(self.context, "backend-go")), {"postgres"})
        self.assertEqual(set(desired_mcp(self.context, "frontend-react")), {"figma"})
        self.assertEqual(set(desired_mcp(self.context, "wordpress")), {"figma", "novamira", "mariadb"})
        self.assertEqual(desired_mcp(self.context, "wordpress")["figma"],
                         desired_mcp(self.context, "frontend-react")["figma"])
        create_profile(self.context, "no-mcp")
        self.assertEqual(desired_mcp(self.context, "no-mcp"), {})
        # An unused definition is available for selection, not installed implicitly.
        (self.source / "mcp/servers/unused.json").write_text(json.dumps({
            "type": "remote", "url": "https://unused.example/mcp",
        }))
        validate_sources(self.context, )
        apply_profile(self.context, self.project, "frontend-react")
        self.assertEqual(set(self.cfg()["mcp"]), {"figma"})
        apply_profile(self.context, self.project, "no-mcp")
        self.assertEqual(self.cfg().get("mcp", {}), {})

    def test_shared_registry_change_updates_only_consumers_preserving_local_settings(self):
        apply_profile(self.context, self.project, "frontend-react")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"figma": {"headers": {"X-Project": "project-only"}}}}))
        apply_profile(self.context, self.project, "frontend-react")
        local_before = local.read_bytes()
        path = self.source / "mcp/servers/figma.json"
        data = json.loads(path.read_text())
        data["url"] = "https://updated.example/mcp"
        path.write_text(json.dumps(data))
        self.assertIn("CONTENT CHANGED WITHOUT VERSION BUMP", self.status_output())
        for profile in ("frontend-react", "wordpress"):
            self.assertEqual(desired_mcp(self.context, profile)["figma"]["url"], data["url"])
            self.assertNotIn("headers", desired_mcp(self.context, profile)["figma"])
        self.assertNotIn("figma", desired_mcp(self.context, "backend-go"))
        apply_profile(self.context, self.project, "frontend-react")
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
                with self.assertRaises(ProfileError):
                    apply_profile(self.context, self.project, "frontend-react")
                self.assertEqual(self.snapshot(), {})

    def test_registry_rejects_symlinks_and_invalid_definitions(self):
        path = self.source / "mcp/servers/figma.json"
        for entry in ({}, [], {"type": "local", "command": []}):
            with self.subTest(entry=entry):
                path.write_text(json.dumps(entry))
                with self.assertRaises(mcp_runtime.ConfigError):
                    apply_profile(self.context, self.project, "frontend-react")
                self.assertEqual(self.snapshot(), {})
        path.unlink()
        path.symlink_to(self.source / "mcp/servers/novamira.json")
        with self.assertRaisesRegex(ProfileError, "symlink"):
            desired_mcp(self.context, "frontend-react")

    def test_existing_expanded_mcp_state_remains_updateable(self):
        # Installed state stores complete definitions, never registry references.
        apply_profile(self.context, self.project, "frontend-react")
        config = self.cfg()
        config["mcp"]["figma"].pop("disabled")
        config["mcp"]["figma"]["enabled"] = True
        (self.project / "opencode.json").write_text(json.dumps(config))
        state = self.state()
        state["mcp"]["figma"] = config["mcp"]["figma"]
        state["mcp_base"]["figma"] = config["mcp"]["figma"]
        (self.project / STATE).write_text(json.dumps(state))
        apply_profile(self.context, self.project, "frontend-react")
        self.assertFalse(self.cfg()["mcp"]["figma"]["disabled"])
        self.assertNotIn("servers", self.state()["mcp_base"])

    def test_postgres_command_has_no_dependency_workarounds_or_credentials(self):
        postgres = desired_mcp(self.context, "backend-go")["postgres"]
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
        apply_profile(self.context, self.project, "backend-go")
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

    def test_postgres_access_mode_default_and_local_settings(self):
        base = desired_mcp(self.context, "backend-go")
        effective, _ = mcp_runtime.resolve(self.project, base)
        self.assertEqual(effective["postgres"]["command"][-1], "--access-mode=unrestricted")
        for mode in ("restricted", "unrestricted"):
            with self.subTest(mode=mode):
                self.local_config({"access_mode": mode})
                effective, _ = mcp_runtime.resolve(self.project, base)
                flags = [arg for arg in effective["postgres"]["command"] if arg.startswith("--access-mode")]
                self.assertEqual(flags, [f"--access-mode={mode}"])
                apply_profile(self.context, self.project, "backend-go")
                self.assertNotIn("access_mode", self.cfg()["mcp"]["postgres"])

    def test_invalid_access_mode_never_writes_or_launches(self):
        apply_profile(self.context, self.project, "backend-go")
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
                    apply_profile(self.context, self.project, "backend-go")
                self.assertEqual(before, self.snapshot())
                with patch.object(mcp_runtime.os, "execvpe") as execute:
                    with self.assertRaises(mcp_runtime.ConfigError):
                        mcp_runtime.launch(self.project, "postgres")
                    execute.assert_not_called()

    def test_access_mode_only_supported_for_local_postgres(self):
        local = self.project / mcp_runtime.LOCAL
        local.parent.mkdir()
        local.write_text(json.dumps({"mcp": {"figma": {"access_mode": "unrestricted"}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "only for local postgres"):
            apply_profile(self.context, self.project, "frontend-react")
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
        apply_profile(self.context, self.project, "backend-go")
        old = self.state()
        self.assertEqual(old["mcp_base"]["postgres"], desired_mcp(self.context, "backend-go")["postgres"])
        mcp_path = self.source / "mcp/servers/postgres.json"
        base = json.loads(mcp_path.read_text())
        base["command"].append("--new-upstream-flag")
        base["environment"]["DATABASE_URI"] = "{env:UPSTREAM_DB}"
        base["enabled"] = False
        mcp_path.write_text(json.dumps(base))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            apply_profile(self.context, self.project, "backend-go")
        self.assertIn("postgres.command", output.getvalue())
        self.assertIn("postgres.environment.DATABASE_URI", output.getvalue())
        self.assertNotIn("local-secret", output.getvalue())
        self.assertFalse(self.cfg()["mcp"]["postgres"]["enabled"])
        effective, _ = mcp_runtime.resolve(self.project, self.state()["mcp_base"])
        self.assertEqual(effective["postgres"]["command"], ["custom-server", "--read-only", "--access-mode=unrestricted"])
        self.assertEqual((path.read_bytes(), env.read_bytes()), original)
        for rel in [STATE, "opencode.json"]:
            self.assertNotIn("local-secret", (self.project / rel).read_text())
        uninstall(self.context, self.project)
        self.assertEqual((path.read_bytes(), env.read_bytes()), original)

    def test_orphaned_override_blocks_switch_without_writes(self):
        self.local_config({"env_file": ".env.stage"})
        apply_profile(self.context, self.project, "backend-go")
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "removed MCP"):
            apply_profile(self.context, self.project, "frontend-react")
        self.assertEqual(before, self.snapshot())

    def test_remote_override_remains_native(self):
        path = self.project / mcp_runtime.LOCAL
        path.parent.mkdir()
        path.write_text(json.dumps({"mcp": {"figma": {
            "url": "https://example.org/custom-mcp", "headers": {"X-Key": "{env:FIGMA_KEY}"}
        }}}))
        apply_profile(self.context, self.project, "frontend-react")
        entry = self.cfg()["mcp"]["figma"]
        self.assertEqual(entry["url"], "https://example.org/custom-mcp")
        self.assertEqual(entry["headers"], {"X-Key": "{env:FIGMA_KEY}"})
        self.assertNotIn("command", entry)
        path.write_text(json.dumps({"mcp": {"figma": {"env_file": ".env"}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "only for local"):
            apply_profile(self.context, self.project, "frontend-react")
        self.assertEqual(before, self.snapshot())

    def test_missing_env_file_and_symlinks_fail_safely(self):
        self.local_config({"env_file": ".env.missing"})
        apply_profile(self.context, self.project, "backend-go")
        command = self.cfg()["mcp"]["postgres"]["command"]
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        outside = self.base / "private-env"
        outside.write_text("DATABASE_URI=private-secret\n")
        (self.project / ".env.missing").symlink_to(outside)
        before = self.snapshot()
        with self.assertRaisesRegex(mcp_runtime.ConfigError, "Symlinks"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())
        result = subprocess.run(command, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("private-secret", result.stderr)
