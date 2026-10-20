from tests.profile_management.implementations.python.support import *


class ProfileTests(ProfileTestCase):
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
                self.assertEqual(bump_profile(self.context, "backend-go", part), expected)
                changed = json.loads(path.read_text())
                self.assertEqual(changed, {**metadata, "version": expected})
                self.assertEqual(frontend.read_bytes(), other_before)
                self.assertEqual(list(path.parent.glob(".profile-*.tmp")), [])

    def test_create_profile_is_valid_installable_and_tracked(self):
        example = (self.source / "mcp/example.json").read_bytes()
        create_profile(self.context, "backend-python", "Python backend")
        self.assertEqual(profile_metadata(self.context, "backend-python"), {
            "name": "backend-python", "version": "1.0.0", "description": "Python backend",
        })
        self.assertTrue((self.source / "profiles/backend-python/skills/.gitkeep").is_file())
        self.assertEqual(desired_mcp(self.context, "backend-python"), {})
        self.assertEqual(json.loads((self.source / "profiles/backend-python/mcp.json").read_text()),
                         {"servers": []})
        validate_sources(self.context, )
        self.assertEqual((self.source / "mcp/example.json").read_bytes(), example)
        apply_profile(self.context, self.project, "backend-python")
        self.assertEqual(self.state()["profile"], "backend-python")
        self.assertFalse((self.project / ".opencode/skills/.gitkeep").exists())
        apply_profile(self.context, self.project, "backend-python")
        uninstall(self.context, self.project)

    def test_create_rejects_invalid_existing_and_symlink_profiles(self):
        before = {p.relative_to(self.source): p.read_bytes()
                  for p in (self.source / "profiles").rglob("*") if p.is_file()}
        for name in ("backend-go", "../outside", "", "Backend", "bad_name", "a/b", "-bad", "bad--name"):
            with self.subTest(name=name), self.assertRaises(ProfileError):
                create_profile(self.context, name)
        with self.assertRaises(ProfileError):
            create_profile(self.context, "blank", "  ")
        self.assertEqual(before, {p.relative_to(self.source): p.read_bytes()
                                for p in (self.source / "profiles").rglob("*") if p.is_file()})
        (self.source / "profiles/linked").symlink_to(self.project, target_is_directory=True)
        with self.assertRaisesRegex(ProfileError, "symlink"):
            create_profile(self.context, "linked")
        self.assertEqual(self.snapshot(), {})

    def test_bump_rejects_invalid_input_without_writes(self):
        path = self.source / "profiles/backend-go/profile.json"
        before = path.read_bytes()
        for profile, part in (("missing", "patch"), ("../backend-go", "major"), ("backend-go", "invalid")):
            with self.assertRaises(ProfileError):
                bump_profile(self.context, profile, part)
            self.assertEqual(path.read_bytes(), before)
        self.set_version("01.0.0")
        before = path.read_bytes()
        with self.assertRaisesRegex(ProfileError, "wersja"):
            bump_profile(self.context, "backend-go")
        self.assertEqual(path.read_bytes(), before)

    def test_bump_write_failure_preserves_metadata(self):
        path = self.source / "profiles/backend-go/profile.json"
        before = path.read_bytes()
        with patch.object(Path, "replace", side_effect=OSError("Simulated write failure")):
            with self.assertRaises(OSError):
                bump_profile(self.context, "backend-go")
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(list(path.parent.glob(".profile-*.tmp")), [])

    def test_new_profile_discovery_and_independent_versions(self):
        new = self.source / "profiles/custom-profile"
        shutil.copytree(self.source / "profiles/backend-go", new)
        (new / "profile.json").write_text(json.dumps({
            "name": "custom-profile", "version": "3.2.1", "description": "Custom profile"
        }))
        self.assertIn("custom-profile", profile_names(self.context, ))
        apply_profile(self.context, self.project, "custom-profile")
        self.assertEqual(self.state()["version"], "3.2.1")
        apply_profile(self.context, self.project, "frontend-react")
        self.assertEqual(self.state()["version"], "1.0.0")
        uninstall(self.context, self.project)
        self.assertFalse((self.project / STATE).exists())
