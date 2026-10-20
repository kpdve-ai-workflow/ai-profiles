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


class ProfileTestCase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.source = self.base / "ai-profiles"
        shutil.copytree(ROOT, self.source, ignore=shutil.ignore_patterns(
            ".git", "__pycache__", "*.pyc", "node_modules", "tests"))
        self.project = self.base / "my-api"
        self.project.mkdir()
        self.context = RepositoryContext(self.source)
        # Keep version scenarios independent of the current published release.
        self.set_version("1.0.0")
        metadata_path = self.source / "profiles/frontend-react/profile.json"
        metadata = json.loads(metadata_path.read_text())
        metadata["version"] = "1.0.0"
        metadata_path.write_text(json.dumps(metadata))

    def state(self):
        return json.loads((self.project / STATE).read_text())

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
            status(self.context, self.project)
        return output.getvalue()

    def snapshot(self):
        return {p.relative_to(self.project).as_posix(): p.read_bytes()
                for p in self.project.rglob("*") if p.is_file()}
