import contextlib
import hashlib
import io
from pathlib import Path
import subprocess
import tarfile
import tempfile
import types
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / "mcp/launchers/mariadb/launch.py"


class MariaDBLauncherTests(unittest.TestCase):
    def setUp(self):
        # Execute without producing __pycache__ inside managed launcher assets.
        self.launcher = types.ModuleType("mariadb_launcher")
        exec(compile(SOURCE.read_bytes(), str(SOURCE), "exec"), self.launcher.__dict__)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.env = {
            "PATH": "/bin", "HOME": str(self.base), "XDG_CACHE_HOME": str(self.base / "cache"),
            "DB_HOST": "127.0.0.1", "DB_PORT": "62335", "DB_NAME": "db",
            "DB_USER": "test-user", "DB_PASSWORD": "not-a-real-secret",
            "WP_API_PASSWORD": "unrelated-secret", "DOCKER_CONTEXT": "test-context",
        }

    def result(self, code=0, stdout=""):
        return subprocess.CompletedProcess([], code, stdout, "")

    def archive(self, name="Dockerfile", kind=tarfile.REGTYPE):
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            info = tarfile.TarInfo(f"mcp-{self.launcher.COMMIT}/{name}")
            info.type = kind
            if kind == tarfile.REGTYPE:
                data = b"FROM python:3.11-slim\n"
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            else:
                info.linkname = "/outside"
                archive.addfile(info)
        return buffer.getvalue()

    def test_existing_verified_image_skips_download_and_build(self):
        with patch.object(self.launcher.subprocess, "run", return_value=self.result(stdout=self.launcher.COMMIT)) as run, \
                patch.object(self.launcher, "fetch_source") as fetch:
            self.launcher.ensure_image(self.env)
        self.assertEqual(run.call_count, 1)
        self.assertEqual(run.call_args.args[0][:3], ["docker", "image", "inspect"])
        self.assertEqual(run.call_args.kwargs["env"]["DOCKER_CONTEXT"], "test-context")
        self.assertNotIn("DB_PASSWORD", run.call_args.kwargs["env"])
        fetch.assert_not_called()

    def test_missing_image_downloads_builds_once_then_verifies(self):
        with patch.object(self.launcher, "image_available", side_effect=[False, False, True]), \
                patch.object(self.launcher, "fetch_source") as fetch, \
                patch.object(self.launcher.subprocess, "run", side_effect=[self.result(stdout="linux"), self.result()]) as run, \
                contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.launcher.ensure_image(self.env)
        fetch.assert_called_once()
        build = run.call_args
        args = build.args[0]
        self.assertEqual(args[:4], ["docker", "build", "--platform", "linux/amd64"])
        self.assertIn(self.launcher.IMAGE, args)
        self.assertIn(f"{self.launcher.SOURCE_LABEL}={self.launcher.COMMIT}", args)
        self.assertNotIn("DB_PASSWORD", build.kwargs["env"])
        self.assertNotIn("WP_API_PASSWORD", build.kwargs["env"])
        self.assertIs(build.kwargs["stdout"], stderr)
        self.assertIs(build.kwargs["stderr"], stderr)
        self.assertFalse(fetch.call_args.args[0].exists())  # temporary build context removed

    def test_concurrent_builder_recheck_reuses_image(self):
        with patch.object(self.launcher, "image_available", side_effect=[False, True]), \
                patch.object(self.launcher.subprocess, "run", return_value=self.result(stdout="linux")), \
                patch.object(self.launcher, "fetch_source") as fetch:
            self.launcher.ensure_image(self.env)
        fetch.assert_not_called()

    def test_docker_unavailable_fails_before_download(self):
        with patch.object(self.launcher, "image_available", return_value=False), \
                patch.object(self.launcher.subprocess, "run", return_value=self.result(code=1)), \
                patch.object(self.launcher, "fetch_source") as fetch:
            with self.assertRaisesRegex(self.launcher.StartupError, "Docker daemon"):
                self.launcher.ensure_image(self.env)
        fetch.assert_not_called()

    def test_failed_build_does_not_report_image_ready(self):
        with patch.object(self.launcher, "image_available", return_value=False), \
                patch.object(self.launcher, "fetch_source"), \
                patch.object(self.launcher.subprocess, "run", side_effect=[self.result(stdout="linux"), self.result(code=1)]), \
                contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(self.launcher.StartupError, "build failed"):
                self.launcher.ensure_image(self.env)

    def test_checksum_verified_before_extracting_source(self):
        data = self.archive()
        target = self.base / "source"
        target.mkdir()
        with patch.object(self.launcher.urllib.request, "urlopen", return_value=io.BytesIO(data)):
            with self.assertRaisesRegex(self.launcher.StartupError, "checksum"):
                self.launcher.fetch_source(target)
        self.assertEqual(list(target.iterdir()), [])
        with patch.object(self.launcher, "ARCHIVE_SHA256", hashlib.sha256(data).hexdigest()), \
                patch.object(self.launcher.urllib.request, "urlopen", return_value=io.BytesIO(data)):
            self.launcher.fetch_source(target)
        self.assertTrue((target / "Dockerfile").is_file())

    def test_archive_rejects_traversal_and_links(self):
        for name, kind in (("../outside", tarfile.REGTYPE), ("link", tarfile.SYMTYPE),
                           ("hardlink", tarfile.LNKTYPE)):
            with self.subTest(name=name):
                data = self.archive(name, kind)
                with patch.object(self.launcher, "ARCHIVE_SHA256", hashlib.sha256(data).hexdigest()), \
                        patch.object(self.launcher.urllib.request, "urlopen", return_value=io.BytesIO(data)):
                    with self.assertRaisesRegex(self.launcher.StartupError, "Unsafe"):
                        self.launcher.fetch_source(self.base)
                self.assertFalse((self.base.parent / "outside").exists())

    def test_run_uses_full_access_stdio_and_no_secret_arguments(self):
        with patch.object(self.launcher, "ensure_image") as ensure, \
                patch.object(self.launcher.os, "execvpe") as execute:
            self.launcher.launch(self.env)
        ensure.assert_called_once()
        executable, args, environment = execute.call_args.args
        self.assertEqual(executable, "docker")
        self.assertIn("--network=host", args)
        self.assertEqual(args[-5:], [self.launcher.IMAGE, "python", "src/server.py", "--transport", "stdio"])
        self.assertNotIn("--tty", args)
        self.assertNotIn("--publish", args)
        self.assertEqual(environment["DB_PASSWORD"], "not-a-real-secret")
        self.assertEqual(environment["MCP_READ_ONLY"], "false")
        self.assertEqual(environment["MCP_BLOCK_SENSITIVE_SHOW"], "false")
        self.assertIn("DB_PASSWORD", args)
        self.assertNotIn("WP_API_PASSWORD", args)
        self.assertNotIn("EMBEDDING_PROVIDER", args)
        self.assertNotIn("not-a-real-secret", " ".join(args))
        self.assertNotIn("unrelated-secret", " ".join(args))

    def test_local_network_and_flags_remain_overridable(self):
        self.env.update(MARIADB_MCP_NETWORK="ddev_default", DB_HOST="ddev-project-db", DB_PORT="3306",
                        MCP_READ_ONLY="true", MCP_BLOCK_SENSITIVE_SHOW="true")
        with patch.object(self.launcher, "ensure_image"), patch.object(self.launcher.os, "execvpe") as execute:
            self.launcher.launch(self.env)
        self.assertIn("--network=ddev_default", execute.call_args.args[1])
        self.assertEqual(execute.call_args.args[2]["MCP_READ_ONLY"], "true")

    def test_invalid_settings_fail_before_docker(self):
        for key, value in (("DB_PASSWORD", ""), ("DB_NAME", ""), ("DB_PORT", ""),
                           ("DB_PORT", "0"), ("DB_PORT", "65536"), ("DB_PORT", "abc"),
                           ("MCP_READ_ONLY", "invalid"), ("MCP_BLOCK_SENSITIVE_SHOW", "invalid")):
            with self.subTest(key=key, value=value), patch.object(self.launcher, "ensure_image") as ensure:
                with self.assertRaises(self.launcher.StartupError):
                    self.launcher.launch({**self.env, key: value})
                ensure.assert_not_called()

    def test_main_sanitizes_external_errors(self):
        with patch.object(self.launcher, "launch", side_effect=OSError("secret-value")), \
                contextlib.redirect_stderr(io.StringIO()) as stderr, \
                contextlib.redirect_stdout(io.StringIO()) as stdout:
            self.assertEqual(self.launcher.main(), 1)
        self.assertEqual(stdout.getvalue(), "")
        self.assertNotIn("secret-value", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
