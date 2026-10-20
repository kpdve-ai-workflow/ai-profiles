from tests.profile_management.implementations.python.support import *


class FilesystemTests(ProfileTestCase):

    def test_path_collision_preflight(self):
        apply_profile(self.context, self.project, "backend-go")
        self.set_version("1.1.0")
        asset = self.source / "profiles/backend-go/skills/go-backend/new-assets"
        asset.mkdir()
        (asset / "notes.md").write_text("new notes")
        (self.project / ".opencode/skills/go-backend/new-assets").write_text("user file")
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(before, self.snapshot())

    def test_collision_with_unmanaged_file(self):
        existing = self.project / ".opencode/skills/go-backend/SKILL.md"
        existing.parent.mkdir(parents=True)
        existing.write_text("unmanaged")
        with self.assertRaisesRegex(ProfileError, "niezarządzanym"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertFalse((self.project / STATE).exists())
        self.assertEqual(existing.read_text(), "unmanaged")

    def test_symlink_protection(self):
        outside = self.base / "outside"
        outside.mkdir()
        (self.project / ".opencode").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ProfileError, "symlink"):
            apply_profile(self.context, self.project, "backend-go")
        self.assertEqual(list(outside.iterdir()), [])

    def test_profile_scripts_reject_symlinks(self):
        script = self.source / "profiles/wordpress/scripts/local-guard.php"
        script.unlink()
        script.symlink_to(self.source / "core/profile_management/implementations/python/ai-profile.py")
        with self.assertRaisesRegex(ProfileError, "symlink"):
            desired_files(self.context, "wordpress")
