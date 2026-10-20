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


class WordpressProfileTests(ProfileTestCase):

    def test_wordpress_install_delivers_mcp_skills_docs_without_plugin_changes(self):
        plugin = self.project / "wp-content/plugins/novamira/novamira.php"
        plugin.parent.mkdir(parents=True)
        plugin.write_text("existing user-owned plugin")
        apply_profile(self.context, self.project, "wordpress")
        self.assertEqual(set(self.cfg()["mcp"]), {"novamira", "figma", "mariadb"})
        self.assertTrue(self.cfg()["mcp"]["novamira"]["disabled"])
        self.assertFalse(self.cfg()["mcp"]["figma"]["disabled"])
        for skill in ("mcp-novamira", "mcp-figma", "mcp-mariadb", "acf", "styling"):
            self.assertTrue((self.project / f".opencode/skills/{skill}/SKILL.md").is_file())
        self.assertEqual((self.project / ".opencode/ai-profile/README.md").read_bytes(),
                         (self.source / "profiles/wordpress/README.md").read_bytes())
        for skill in ("mcp-novamira", "mcp-mariadb"):
            self.assertFalse(any((self.project / f".opencode/skills/{skill}/examples").glob("*")))
        self.assertFalse((self.project / mcp_runtime.LOCAL).exists())
        self.assertFalse((self.project / ".env.novamira").exists())
        with patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "disabled"):
                mcp_runtime.launch(self.project, "novamira")
            execute.assert_not_called()
        apply_profile(self.context, self.project, "wordpress")
        uninstall(self.context, self.project)
        self.assertEqual(plugin.read_text(), "existing user-owned plugin")

    def test_wordpress_private_credentials_launch_and_survive_update_uninstall(self):
        apply_profile(self.context, self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {
            "disabled": False, "env_file": ".env.novamira",
        }}}))
        private = self.project / ".env.novamira"
        secret = "abcd efgh test-secret ijkl"
        private.write_text('WP_API_URL=https://example.test/wp-json/mcp/novamira\n'
                           'WP_API_USERNAME=operator\n'
                           f'WP_API_PASSWORD="{secret}"\n')
        private.chmod(0o600)
        local_before, env_before = local.read_bytes(), private.read_bytes()
        apply_profile(self.context, self.project, "wordpress")
        command = ["npx", "-y", "@automattic/mcp-wordpress-remote@0.4.0"]
        self.assertEqual(self.state()["mcp_base"]["novamira"]["command"], command)
        self.assertFalse(self.cfg()["mcp"]["novamira"]["disabled"])
        self.assertNotIn("environment", self.cfg()["mcp"]["novamira"])
        with patch.dict(mcp_runtime.os.environ, {}, clear=True), \
                patch.object(mcp_runtime.os, "chdir"), \
                patch.object(mcp_runtime.os, "execvpe") as execute:
            mcp_runtime.launch(self.project, "novamira")
        executable, args, environment = execute.call_args.args
        self.assertEqual((executable, args), ("npx", command))
        self.assertEqual(environment["WP_API_PASSWORD"], secret)
        self.assertEqual(environment["WP_API_USERNAME"], "operator")
        self.assertEqual(environment["WP_API_URL"], "https://example.test/wp-json/mcp/novamira")
        self.assertEqual(environment["OAUTH_ENABLED"], "false")
        self.assertEqual(environment["NODE_OPTIONS"], "--use-system-ca")
        for rel in (*self.state()["files"], STATE, "opencode.json"):
            self.assertNotIn(secret.encode(), (self.project / rel).read_bytes())
        apply_profile(self.context, self.project, "wordpress")
        uninstall(self.context, self.project)
        self.assertEqual(local.read_bytes(), local_before)
        self.assertEqual(private.read_bytes(), env_before)
        self.assertEqual(private.stat().st_mode & 0o777, 0o600)

    def test_wordpress_update_retires_managed_skill_templates_preserving_private_files(self):
        old_templates = (
            "mcp-novamira/examples/ai-profile.local.example.json",
            "mcp-novamira/examples/novamira.env.example",
            "mcp-mariadb/examples/ai-profile.local.example.json",
            "mcp-mariadb/examples/mariadb.env.example",
        )
        for relative in old_templates:
            source = self.source / "profiles/wordpress/skills" / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("old managed template\n")
        apply_profile(self.context, self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {"env_file": ".env.novamira"},
                                             "mariadb": {"env_file": ".env.mariadb"}}}))
        private = [local, self.project / ".env.novamira", self.project / ".env.mariadb"]
        for path in private[1:]:
            path.write_text("PROJECT_SECRET=synthetic-secret\n")
        user_file = self.project / ".opencode/skills/mcp-novamira/examples/my-notes.txt"
        user_file.write_text("user-owned notes\n")
        private.append(user_file)
        before = {path: path.read_bytes() for path in private}
        for relative in old_templates:
            (self.source / "profiles/wordpress/skills" / relative).unlink()
        apply_profile(self.context, self.project, "wordpress")
        for relative in old_templates:
            self.assertFalse((self.project / ".opencode/skills" / relative).exists())
        self.assertEqual({path: path.read_bytes() for path in private}, before)
        for skill in ("mcp-novamira", "mcp-mariadb"):
            self.assertFalse(any(rel.startswith(f".opencode/skills/{skill}/examples/")
                                 for rel in self.state()["files"]))

    def test_wordpress_missing_credentials_fail_before_bridge_starts(self):
        apply_profile(self.context, self.project, "wordpress")
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"novamira": {"disabled": False, "env_file": ".env.novamira"}}}))
        private = self.project / ".env.novamira"
        with patch.dict(mcp_runtime.os.environ, {}, clear=True), \
                patch.object(mcp_runtime.os, "execvpe") as execute:
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "Cannot read env file"):
                mcp_runtime.launch(self.project, "novamira")
            private.write_text('WP_API_URL=https://example.test/wp-json/mcp/novamira\nWP_API_USERNAME=operator\n')
            with self.assertRaisesRegex(mcp_runtime.ConfigError, "WP_API_PASSWORD"):
                mcp_runtime.launch(self.project, "novamira")
            execute.assert_not_called()

    def test_wordpress_mariadb_installed_launcher_runs_with_private_env(self):
        apply_profile(self.context, self.project, "wordpress")
        script = self.project / ".opencode/mcp/mariadb/launch.py"
        self.assertTrue(script.is_file())
        self.assertTrue(self.cfg()["mcp"]["mariadb"]["disabled"])
        local = self.project / mcp_runtime.LOCAL
        local.write_text(json.dumps({"mcp": {"mariadb": {
            "disabled": False, "env_file": ".env.mariadb",
        }}}))
        private = self.project / ".env.mariadb"
        secret = "synthetic-db-secret"
        private.write_text('DB_HOST=127.0.0.1\nDB_PORT=62335\nDB_NAME=db\n'
                           f'DB_USER=operator\nDB_PASSWORD="{secret}"\n')
        apply_profile(self.context, self.project, "wordpress")
        # Exercise both installed launchers in a child process; Docker is replaced
        # by a fake executable. No Docker/network/database access occurs.
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        docker = bin_dir / "docker"
        docker.write_text(f'#!{sys.executable}\n'
                          'import json, os, sys\n'
                          'if sys.argv[1:3] == ["image", "inspect"]:\n'
                          '    print("64876fb1a4cb81bf51564c46eb60401097c067cc")\n'
                          'elif sys.argv[1] == "run":\n'
                          '    print(json.dumps({"args": sys.argv[1:], "env": dict(os.environ)}))\n'
                          'else:\n'
                          '    raise SystemExit(9)\n')
        docker.chmod(0o700)
        command = [sys.executable, str(self.project / mcp_runtime.LAUNCHER), "mariadb"]
        result = subprocess.run(command, cwd=self.base, check=True, capture_output=True, text=True,
                                env={"PATH": str(bin_dir) + os.pathsep + os.defpath,
                                     "HOME": str(self.base)})
        invoked = json.loads(result.stdout)
        self.assertEqual(invoked["env"]["DB_PASSWORD"], secret)
        self.assertEqual(invoked["env"]["MCP_READ_ONLY"], "false")
        self.assertEqual(invoked["env"]["MCP_BLOCK_SENSITIVE_SHOW"], "false")
        self.assertIn("--network=host", invoked["args"])
        self.assertNotIn(secret, " ".join(invoked["args"]))
        for rel in (*self.state()["files"], STATE, "opencode.json"):
            self.assertNotIn(secret.encode(), (self.project / rel).read_bytes())
        before = local.read_bytes(), private.read_bytes()
        apply_profile(self.context, self.project, "wordpress")
        uninstall(self.context, self.project)
        self.assertEqual(before, (local.read_bytes(), private.read_bytes()))
        self.assertFalse(script.exists())

    def test_wordpress_scripts_are_installed_without_running_and_protected(self):
        relative = ".opencode/ai-profile/scripts/restore-updraft.py"
        self.assertNotIn(relative, desired_files(self.context, "frontend-react"))
        apply_profile(self.context, self.project, "wordpress")
        script = self.project / relative
        self.assertTrue(script.is_file())
        for helper in ("local-guard.php", "local-safety.php"):
            self.assertTrue((script.parent / helper).is_file())
        result = subprocess.run([sys.executable, str(script), "--help"], cwd=self.base,
                                capture_output=True, text=True, check=True)
        self.assertIn("--apply", result.stdout)
        self.assertIn("--map", result.stdout)
        self.assertFalse((self.project / "wp-content/mu-plugins/ai-profile-local-safety.php").exists())
        original = script.read_bytes()
        script.write_bytes(original + b"\n# user change\n")
        before = self.snapshot()
        with self.assertRaisesRegex(ProfileError, "Konflikt"):
            apply_profile(self.context, self.project, "wordpress")
        self.assertEqual(self.snapshot(), before)
        script.write_bytes(original)
        apply_profile(self.context, self.project, "frontend-react")
        self.assertFalse(script.exists())

    def test_switch_from_react_to_wordpress_preserves_shared_figma(self):
        apply_profile(self.context, self.project, "frontend-react")
        apply_profile(self.context, self.project, "wordpress")
        self.assertEqual(set(self.cfg()["mcp"]), {"novamira", "figma", "mariadb"})
        apply_profile(self.context, self.project, "frontend-react")
        self.assertEqual(set(self.cfg()["mcp"]), {"figma"})
        self.assertFalse((self.project / ".opencode/ai-profile/README.md").exists())
