from tests.support import *


class CliTests(ProfileTestCase):

    def test_cli_and_makefile(self):
        cmd = [sys.executable, str(self.source / "core/python/profile_management/ai-profile.py")]
        subprocess.run(cmd + ["install", "backend-go", "--project", str(self.project)], cwd=self.project, check=True, capture_output=True)
        subprocess.run(cmd + ["update", "--project", str(self.project)], cwd=self.project, check=True, capture_output=True)
        status = subprocess.run(cmd + ["status", "--project", str(self.project)], cwd=self.project, check=True, text=True, capture_output=True)
        self.assertIn("backend-go", status.stdout)
        subprocess.run(cmd + ["uninstall", "--project", str(self.project)], cwd=self.project, check=True, capture_output=True)
        self.assertFalse((self.project / STATE).exists())
        make_result = subprocess.run(["make", "validate"], cwd=self.source, text=True, capture_output=True)
        self.assertEqual(make_result.returncode, 0, make_result.stdout + make_result.stderr)

    def test_nested_launcher_shows_help_from_another_directory(self):
        entrypoint = self.source / "core/python/profile_management/ai-profile.py"
        result = subprocess.run([str(entrypoint), "help"], cwd=self.project,
                                check=True, capture_output=True, text=True)
        self.assertIn("ai-profiles — zarządzanie profilami", result.stdout)

    def test_default_source_root_is_independent_of_working_directory(self):
        result = subprocess.run([sys.executable, str(self.source / "core/python/profile_management/ai-profile.py"), "validate"],
                                cwd=self.project, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_importing_package_modules_has_no_cli_side_effects(self):
        code = "import profile_management.cli, profile_management.actions.installation, profile_management.actions.status, profile_management.actions.validation"
        result = subprocess.run([sys.executable, "-c", code], cwd=self.source,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")

    def test_create_and_help_cli_and_make(self):
        cli = [sys.executable, str(self.source / "core/python/profile_management/ai-profile.py")]
        for args in (["create", "python-cli"],):
            subprocess.run(cli + args, cwd=self.source, check=True, capture_output=True, text=True)
        self.assertEqual(profile_metadata(self.context, "python-cli")["description"], "Profil python-cli")
        subprocess.run(["make", "create", "PROFILE=python-make", "DESCRIPTION=Python Make"],
                       cwd=self.source, check=True, capture_output=True, text=True)
        self.assertEqual(profile_metadata(self.context, "python-make")["description"], "Python Make")
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

    def test_bump_cli_and_make_adapter(self):
        cmd = [sys.executable, str(self.source / "core/python/profile_management/ai-profile.py"), "bump"]
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
