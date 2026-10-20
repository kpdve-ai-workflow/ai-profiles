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
        script.symlink_to(self.source / "profile.py")
        with self.assertRaisesRegex(ProfileError, "symlink"):
            desired_files(self.context, "wordpress")
