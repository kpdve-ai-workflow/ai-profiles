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


class PayloadTests(ProfileTestCase):

    def test_repository_local_skill_and_fresh_install_scope(self):
        local = self.source / ".opencode/skills/brain-to-implementer/SKILL.md"
        original = local.read_bytes()
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", desired_files(self.context, "backend-go"))
        apply_profile(self.context, self.project, "backend-go")
        self.assertNotIn(RULE, self.state()["files"])
        self.assertNotIn(".opencode/skills/brain-to-implementer/SKILL.md", self.state()["files"])
        self.assertFalse((self.project / RULE).exists())
        self.assertFalse((self.project / ".opencode/skills/brain-to-implementer").exists())
        self.assertNotIn("instructions", self.cfg())
        self.assertEqual(local.read_bytes(), original)
