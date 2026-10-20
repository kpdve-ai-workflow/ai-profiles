"""Black-box contract test support; intentionally imports no production code."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]


class ContractCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = self.base / "checkout with spaces"
        import shutil
        shutil.copytree(ROOT, self.source, ignore=shutil.ignore_patterns(
            ".git", "__pycache__", "*.pyc", "node_modules", "tests"))
        self.project = self.base / "project"
        self.project.mkdir()

    def run_cli(self, *args, cwd=None):
        from tests.profile_management.runner import cli_argv
        return subprocess.run(cli_argv(self.source, *args), cwd=cwd or self.project,
                              capture_output=True, text=True)
