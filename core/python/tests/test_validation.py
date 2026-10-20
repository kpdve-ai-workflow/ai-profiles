from tests.support import *


class ValidationTests(ProfileTestCase):

    def test_validate_one_profile_cli_and_make(self):
        cli = [sys.executable, str(self.source / "core/python/profile_management/ai-profile.py"), "validate"]
        for command in (cli + ["backend-go"], ["make", "validate", "PROFILE=backend-go"]):
            result = subprocess.run(command, cwd=self.source, check=True, capture_output=True, text=True)
            self.assertIn("OK: profil backend-go", result.stdout)
        for name in ("missing", "../backend-go"):
            with self.assertRaisesRegex(ProfileError, "Nieznany profil"):
                validate_sources(self.context, name)
        # A broken unrelated profile must not block scoped validation.
        (self.source / "profiles/frontend-react/profile.json").write_text("{}")
        validate_sources(self.context, "backend-go")
        with self.assertRaises(ProfileError):
            validate_sources(self.context, )

    def test_validate_one_profile_checks_shared_skills_and_mcp_example(self):
        path = self.source / "mcp/example.json"
        example = json.loads(path.read_text())
        name = next(iter(desired_mcp(self.context, "backend-go")))
        del example["mcp"][name]
        path.write_text(json.dumps(example))
        with self.assertRaisesRegex(ProfileError, "mcp/example.json"):
            validate_sources(self.context, "backend-go")
        example["mcp"][name] = {"type": "local", "command": ["wrong"]}
        path.write_text(json.dumps(example))
        with self.assertRaisesRegex(ProfileError, "mcp/example.json"):
            validate_sources(self.context, "backend-go")
        example["mcp"][name] = desired_mcp(self.context, "backend-go")[name]
        path.write_text(json.dumps(example))
        create_profile(self.context, "empty-profile")
        validate_sources(self.context, "empty-profile")
        skill = next((self.source / "common/skills").glob("*/SKILL.md"))
        skill.write_text("invalid frontmatter")
        with self.assertRaisesRegex(ProfileError, "frontmatter"):
            validate_sources(self.context, "empty-profile")

    def test_validate_example_mirrors_all_profiles(self):
        validate_sources(self.context, )
        example = self.source / "mcp/example.json"
        data = json.loads(example.read_text())
        data["mcp"].pop("figma")
        example.write_text(json.dumps(data))
        with self.assertRaisesRegex(ProfileError, "mcp/example.json"):
            validate_sources(self.context, )

    def test_mcp_launcher_assets_are_selected_protected_and_validated(self):
        self.assertNotIn(".opencode/mcp/mariadb/launch.py", desired_files(self.context, "frontend-react"))
        self.assertNotIn(".opencode/mcp/mariadb/launch.py", desired_files(self.context, "backend-go"))
        apply_profile(self.context, self.project, "wordpress")
        script = self.project / ".opencode/mcp/mariadb/launch.py"
        original = script.read_bytes()
        script.write_bytes(original + b"\n# local change\n")
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "wordpress")
        self.assertEqual(self.snapshot(), before)
        script.write_bytes(original)
        apply_profile(self.context, self.project, "frontend-react")
        self.assertFalse(script.exists())
        source = self.source / "mcp/launchers/mariadb/launch.py"
        source.unlink()
        with self.assertRaisesRegex(ProfileError, "Brak zarządzanego pliku launchera"):
            desired_files(self.context, "wordpress")
        source.symlink_to(self.source / "core/python/profile_management/ai-profile.py")
        with self.assertRaisesRegex(ProfileError, "symlink"):
            desired_files(self.context, "wordpress")

    def test_validate_profiles_resolve_shared_figma_and_example_detects_changes(self):
        validate_sources(self.context, )
        validate_sources(self.context, "wordpress")
        validate_sources(self.context, "frontend-react")
        path = self.source / "mcp/servers/figma.json"
        data = json.loads(path.read_text())
        data["url"] = "https://different.example/mcp"
        path.write_text(json.dumps(data))
        with self.assertRaisesRegex(ProfileError, "mcp/example.json"):
            validate_sources(self.context, )

    def test_scoped_validation_ignores_unselected_registry_definitions(self):
        path = self.source / "mcp/servers/postgres.json"
        path.write_text("{}")
        validate_sources(self.context, "wordpress")
        with self.assertRaises(mcp_runtime.ConfigError):
            validate_sources(self.context, )

    def test_repository_contexts_are_independent(self):
        from profile_management.infrastructure.mcp import desired_mcp

        second_source = self.base / "second-source"
        shutil.copytree(self.source, second_source)
        second_context = RepositoryContext(second_source)
        (second_source / "profiles").joinpath("context-only").mkdir()
        (second_source / "profiles/context-only/profile.json").write_text(json.dumps({
            "name": "context-only", "version": "1.0.0", "description": "test"
        }))
        (second_source / "profiles/context-only/mcp.json").write_text('{"servers": []}\n')
        (second_source / "profiles/context-only/skills").mkdir()
        self.assertNotIn("context-only", profile_names(self.context))
        self.assertIn("context-only", profile_names(second_context))
        self.assertEqual(desired_mcp(self.context, "backend-go"),
                         desired_mcp(second_context, "backend-go"))

    def test_source_repository_and_descendants_are_not_install_targets(self):
        descendant = self.source / "nested-project"
        descendant.mkdir()
        with self.assertRaisesRegex(ProfileError, "wewnątrz repozytorium"):
            selected_project(self.context, descendant)
