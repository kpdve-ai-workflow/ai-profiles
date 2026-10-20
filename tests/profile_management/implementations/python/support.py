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

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT))
from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.errors import ProfileError
from core.profile_management.implementations.python.manager.constants import STATE, RULE, GATE
from core.profile_management.implementations.python.manager.infrastructure.filesystem import sha
from core.profile_management.implementations.python.manager.infrastructure.repository import profile_names, profile_metadata, source_commit
from core.profile_management.implementations.python.manager.versioning import version_tuple
from core.profile_management.implementations.python.manager.actions.profiles import create_profile, bump_profile
from core.profile_management.implementations.python.manager.infrastructure.payload import desired_files
from core.profile_management.implementations.python.manager.infrastructure.project import selected_project, state_for, config_for
from core.profile_management.implementations.python.manager.infrastructure.mcp import desired_mcp, mcp_definition, configured_mcp, check_mcp
from core.profile_management.implementations.python.manager.actions.installation import apply_profile, uninstall
from core.profile_management.implementations.python.manager.actions.status import status
from core.profile_management.implementations.python.manager.actions.validation import validate_sources
from core.profile_management.implementations.python.manager.actions.migration import migrate_legacy
from core.profile_management.implementations.python.runtime import mcp_runtime


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

    def local_config(self, overrides):
        path = self.project / ".opencode" / "ai-profile.local.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"mcp": {"postgres": overrides}}))
        return path

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
