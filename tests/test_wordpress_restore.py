import contextlib
import copy
import gzip
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import types
import unittest
from unittest.mock import Mock, patch


ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "profiles/wordpress/scripts"


def sql_dump(prefix="wp_", multisite=False):
    names = ["options", "posts", "users", "usermeta"]
    if multisite:
        names += ["blogs", "site", "sitemeta", "2_options"]
    return ("# WordPress database backup\n" + "\n".join(
        f"CREATE TABLE `{prefix}{name}` (`id` bigint PRIMARY KEY);" for name in names)).encode()


class RestoreTests(unittest.TestCase):
    def setUp(self):
        self.module = types.ModuleType("restore_updraft")
        source = SCRIPTS / "restore-updraft.py"
        exec(compile(source.read_bytes(), str(source), "exec"), self.module.__dict__)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.project = self.base / "my project"
        self.project.mkdir()
        (self.project / ".ddev").mkdir()
        (self.project / ".ddev/config.yaml").write_text("name: demo\ntype: wordpress\n")
        (self.project / "wp-config.php").write_text("<?php // fixture\n")
        (self.project / "wp-content").mkdir()
        shutil.copytree(SCRIPTS, self.project / self.module.SCRIPTS)
        self.dump = self.base / "Updraft backup-db.gz"
        self.dump.write_bytes(gzip.compress(sql_dump()))
        self.mapping = self.module.mappings(["https://source.example=https://demo.ddev.site"])
        self.info = {"name": "demo", "approot": str(self.project), "status": "running",
                     "type": "wordpress", "docroot": "", "urls": ["https://demo.ddev.site"]}
        self.current = {"sites": [{"id": "1", "domain": "", "path": "/", "options": "wp_options",
                                   "home": "https://demo.ddev.site", "siteurl": "https://demo.ddev.site"}],
                        "networks": [], "multisite": False}
        self.ddev = Mock()
        self.ddev.docroot = ""
        self.ddev.describe.return_value = self.info
        self.ddev.wp.return_value = "wp_"
        self.ddev.query.return_value = [["container-db", "db"]]
        self.commands = []

        def run(args):
            self.commands.append(args)
            if args[0] == "export-db":
                path = Path(next(arg[7:] for arg in args if arg.startswith("--file=")))
                path.write_bytes(gzip.compress(sql_dump()))
            return "container-db\tdb" if args[0] == "mysql" else ""
        self.ddev.run.side_effect = run
        env = patch.dict(os.environ, {"XDG_STATE_HOME": str(self.base / "private state")})
        env.start()
        self.addCleanup(env.stop)

    def preflight(self):
        with patch.object(self.module, "topology", return_value=self.current):
            return self.module.preflight(self.project, self.dump, self.mapping, self.ddev)

    def workflow(self, apply=True, answer="demo", localize_error=None):
        with patch.object(self.module, "Ddev", return_value=self.ddev), \
                patch.object(self.module, "topology", return_value=self.current), \
                patch.object(self.module, "localize", side_effect=localize_error), \
                patch("builtins.input", return_value=answer), \
                contextlib.redirect_stdout(io.StringIO()):
            return self.module.restore(self.project, self.dump, self.mapping, apply)

    def test_plain_and_gzip_dump_validation_and_prefix_detection(self):
        result = self.module.inspect_dump(self.dump)
        self.assertEqual(result["prefix"], "wp_")
        self.assertTrue(result["gzip"])
        self.assertFalse(result["multisite"])
        plain = self.base / "plain.sql"
        plain.write_bytes(sql_dump("custom_", True))
        result = self.module.inspect_dump(plain)
        self.assertEqual(result["prefix"], "custom_")
        self.assertTrue(result["multisite"])
        self.assertFalse(result["gzip"])

    def test_rejects_encrypted_incomplete_corrupted_or_database_switching_dumps(self):
        for suffix, data in ((".crypt", b"encrypted"), (".sql", b"not SQL"),
                             (".sql", sql_dump() + b"\nUSE production;"),
                             (".sql", sql_dump() + b"\nCREATE DATABASE production;"),
                             (".sql", sql_dump() + b"\n\\! echo bad"),
                             (".gz", gzip.compress(sql_dump())[:-5])):
            with self.subTest(suffix=suffix, data=data[:15]):
                path = self.base / ("bad" + suffix)
                path.write_bytes(data)
                with self.assertRaises((self.module.RestoreError, OSError, EOFError)):
                    self.module.inspect_dump(path)

    def test_mapping_requires_explicit_unambiguous_origins(self):
        for values in ([], ["bad"], ["https://a.test=https://a.test"],
                       ["https://a.test/path=https://demo.ddev.site"],
                       ["https://u:p@a.test=https://demo.ddev.site"],
                       ["https://a.test=https://demo.ddev.site?x=1"],
                       ["https://a.test=https://demo.ddev.site"] * 2,
                       ["https://a.test=https://b.test", "https://b.test=https://demo.ddev.site"]):
            with self.subTest(values=values), self.assertRaises(self.module.RestoreError):
                self.module.mappings(values)
        mapping = self.module.mappings(["http://source.example=https://demo.ddev.site",
                                       "https://source.example=https://demo.ddev.site"])
        self.assertEqual(self.module.mapped_url("http://source.example/sub/path", mapping),
                         "https://demo.ddev.site/sub/path")

    def test_plan_does_not_export_import_or_write_project(self):
        before = {path.relative_to(self.project): path.read_bytes()
                  for path in self.project.rglob("*") if path.is_file()}
        self.assertIsNone(self.workflow(apply=False))
        self.assertFalse(any(args[0] in ("export-db", "import-db") for args in self.commands))
        self.assertFalse((self.base / "private state").exists())
        self.assertEqual(before, {path.relative_to(self.project): path.read_bytes()
                                  for path in self.project.rglob("*") if path.is_file()})

    def test_cancelled_confirmation_does_not_backup_or_write(self):
        with self.assertRaisesRegex(self.module.RestoreError, "Anulowano"):
            self.workflow(answer="wrong-project")
        self.assertFalse((self.base / "private state").exists())
        self.assertFalse(any(args[0] in ("export-db", "import-db") for args in self.commands))

    def test_backup_precedes_import_is_private_and_retained(self):
        backup = self.workflow()
        self.assertEqual([args[0] for args in self.commands if args[0] != "mysql"],
                         ["export-db", "import-db"])
        self.assertTrue(backup.is_file())
        self.assertEqual(backup.stat().st_mode & 0o777, 0o600)
        self.assertEqual(backup.parent.stat().st_mode & 0o777, 0o700)
        self.assertFalse(any(backup.parent.glob("incoming*")))
        self.assertTrue((self.project / "wp-content/mu-plugins/ai-profile-local-safety.php").is_file())

    def test_backup_failure_blocks_import_and_safety_install(self):
        self.ddev.run.side_effect = lambda args: "container-db\tdb" if args[0] == "mysql" else (_ for _ in ()).throw(self.module.RestoreError("export failed"))
        with self.assertRaisesRegex(self.module.RestoreError, "export failed"):
            self.workflow()
        self.assertFalse((self.project / "wp-content/mu-plugins").exists())
        self.assertFalse(any((self.base / "private state").rglob("incoming*")))

    def test_corrupt_backup_blocks_import(self):
        def run(args):
            self.commands.append(args)
            if args[0] == "export-db":
                Path(next(arg[7:] for arg in args if arg.startswith("--file="))).write_bytes(b"not a backup")
            return "container-db\tdb" if args[0] == "mysql" else ""
        self.ddev.run.side_effect = run
        with self.assertRaises(self.module.RestoreError):
            self.workflow()
        self.assertFalse(any(args[0] == "import-db" for args in self.commands))

    def test_post_import_failure_preserves_backup_and_prints_manual_recovery(self):
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            with self.assertRaisesRegex(self.module.RestoreError, "mapping failed"):
                self.workflow(localize_error=self.module.RestoreError("mapping failed"))
        backups = list((self.base / "private state").rglob("before-import.sql.gz"))
        self.assertEqual(len(backups), 1)
        self.assertIn("ddev import-db --database=db --file=", stderr.getvalue())
        self.assertIn(str(backups[0]), stderr.getvalue())
        self.assertFalse(any(backups[0].parent.glob("incoming*")))

    def test_preflight_rejects_wrong_project_status_hosts_prefix_and_database(self):
        for change in ({"status": "stopped"}, {"approot": str(self.base)}, {"type": "generic"},
                       {"urls": ["https://other.ddev.site"]}):
            with self.subTest(change=change):
                self.ddev.describe.return_value = {**self.info, **change}
                with self.assertRaises(self.module.RestoreError):
                    self.preflight()
        self.ddev.describe.return_value = self.info
        self.ddev.wp.return_value = "different_"
        with self.assertRaisesRegex(self.module.RestoreError, "prefiks"):
            self.preflight()
        self.ddev.wp.return_value = "wp_"
        self.ddev.query.return_value = [["remote-db", "db"]]
        with self.assertRaisesRegex(self.module.RestoreError, "tej samej"):
            self.preflight()

    def test_preflight_does_not_overwrite_existing_safety_file_or_follow_symlinks(self):
        safety = self.project / "wp-content/mu-plugins/ai-profile-local-safety.php"
        safety.parent.mkdir()
        safety.write_text("user-owned content")
        with self.assertRaisesRegex(self.module.RestoreError, "nie nadpisuję"):
            self.preflight()
        safety.unlink()
        safety.symlink_to(self.project / "wp-config.php")
        with self.assertRaisesRegex(self.module.RestoreError, "symlink"):
            self.preflight()

    def test_wp_cli_always_runs_in_ddev_with_guard_and_no_shell(self):
        result = subprocess.CompletedProcess([], 0, "wp_\n", "")
        with patch.object(self.module.subprocess, "run", return_value=result) as run:
            ddev = self.module.Ddev(self.project)
            self.assertEqual(ddev.wp("db", "prefix"), "wp_")
        args = run.call_args.args[0]
        self.assertEqual(args[:2], ["ddev", "exec"])
        self.assertIn("wp", args)
        self.assertIn("--require=/var/www/html/.opencode/ai-profile/scripts/local-guard.php", args)
        self.assertIn("--skip-plugins", args)
        self.assertNotIn("shell", run.call_args.kwargs)

    def test_ddev_json_parsing_and_sanitized_errors(self):
        with patch.object(self.module.subprocess, "run", return_value=subprocess.CompletedProcess(
                [], 0, json.dumps({"raw": self.info}), "")):
            self.assertEqual(self.module.Ddev(self.project).describe(), self.info)
        with patch.object(self.module.subprocess, "run", return_value=subprocess.CompletedProcess(
                [], 1, "secret-output", "secret-password")):
            with self.assertRaises(self.module.RestoreError) as error:
                self.module.Ddev(self.project).wp("db", "prefix")
        self.assertNotIn("secret", str(error.exception))

    def test_localize_uses_serialization_aware_wpcli_and_verifies_multisite(self):
        imported = {"multisite": True, "networks": [["1", "source.example", "/"]], "sites": [
            {"id": "1", "domain": "source.example", "path": "/", "options": "wp_options",
             "home": "https://source.example", "siteurl": "https://source.example"},
            {"id": "2", "domain": "bip.source.example", "path": "/", "options": "wp_2_options",
             "home": "https://bip.source.example", "siteurl": "https://bip.source.example"},
        ]}
        mapping = self.module.mappings(["https://source.example=https://demo.ddev.site",
                                        "https://bip.source.example=https://bip-demo.ddev.site"])
        final = copy.deepcopy(imported)
        for site in final["sites"]:
            site["home"] = mapping[site["home"]]
            site["siteurl"] = mapping[site["siteurl"]]
            site["domain"] = site["home"].split("//")[1]
        final["networks"][0][1] = "demo.ddev.site"
        self.ddev.wp.return_value = "0"
        with patch.object(self.module, "topology", return_value=final):
            self.module.localize(self.ddev, "wp_", mapping, imported)
        calls = [call.args for call in self.ddev.wp.call_args_list if "search-replace" in call.args]
        self.assertEqual(len(calls), 4)  # write and verification dry-run for each origin
        for args in calls:
            self.assertIn("--precise", args)
            self.assertIn("--all-tables-with-prefix", args)
            self.assertIn("--skip-columns=guid", args)
        sql = "\n".join(call.args[0] for call in self.ddev.query.call_args_list)
        self.assertIn("`wp_blogs`", sql)
        self.assertIn("`wp_site`", sql)
        self.assertIn("`wp_2_options`", sql)
        self.assertNotIn("REPLACE(", sql)

    def test_missing_multisite_mapping_fails_before_localization_writes(self):
        imported = {"multisite": True, "networks": [], "sites": [
            {"id": "2", "domain": "bip.source.example", "path": "/", "options": "wp_2_options",
             "home": "https://bip.source.example", "siteurl": "https://bip.source.example"}]}
        with self.assertRaises(self.module.RestoreError):
            self.module.localize(self.ddev, "wp_", self.mapping, imported)
        self.ddev.query.assert_not_called()
        self.ddev.wp.assert_not_called()


if __name__ == "__main__":
    unittest.main()
