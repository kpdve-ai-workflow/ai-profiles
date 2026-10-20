#!/usr/bin/env python3
"""Build the pinned official MariaDB MCP image once, then serve MCP on stdio."""
from __future__ import annotations

import fcntl
import hashlib
import io
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

COMMIT = "64876fb1a4cb81bf51564c46eb60401097c067cc"  # upstream 0.2.6
ARCHIVE_URL = f"https://codeload.github.com/MariaDB/mcp/tar.gz/{COMMIT}"
ARCHIVE_SHA256 = "0730bd3dd546bce18fb3cb4f940fd79980d6114b89909c72da1ebefc1d5fbbc0"
IMAGE = f"ai-profiles/mariadb-mcp:{COMMIT}"
SOURCE_LABEL = "ai-profiles.mariadb.source"
PLATFORM = "linux/amd64"  # upstream documents asyncmy SIGILL on native arm64
REQUIRED = ("DB_HOST", "DB_PORT", "DB_NAME", "DB_USER", "DB_PASSWORD")
CONTAINER_ENV = (*REQUIRED, "DB_CHARSET", "DB_SSL", "DB_SSL_VERIFY_CERT",
                 "DB_SSL_VERIFY_IDENTITY", "MCP_READ_ONLY", "MCP_BLOCK_SENSITIVE_SHOW",
                 "MCP_MAX_POOL_SIZE")


class StartupError(Exception):
    pass


def docker_environment(environment: dict[str, str]) -> dict[str, str]:
    """Build commands must never inherit database credentials or env-file values."""
    allowed = {"PATH", "HOME", "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "TMPDIR",
               "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "http_proxy", "https_proxy",
               "no_proxy", "SSL_CERT_FILE", "SSL_CERT_DIR", "SSH_AUTH_SOCK"}
    return {key: value for key, value in environment.items()
            if key in allowed or key.startswith("DOCKER_")}


def image_available(environment: dict[str, str]) -> bool:
    result = subprocess.run(
        ["docker", "image", "inspect", "--format",
         '{{ index .Config.Labels "' + SOURCE_LABEL + '" }}', IMAGE],
        env=environment, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30)
    return result.returncode == 0 and result.stdout.strip() == COMMIT


def fetch_source(destination: Path) -> None:
    with urllib.request.urlopen(ARCHIVE_URL, timeout=60) as response:
        data = response.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != ARCHIVE_SHA256:
        raise StartupError("MariaDB MCP source checksum mismatch; image was not built")
    # Extract only regular files/directories under the expected archive root.
    # No extractall, links, device files, absolute paths or parent traversal.
    root = f"mcp-{COMMIT}"
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            if (not parts or parts[0] != root or ".." in parts
                    or not (member.isfile() or member.isdir())):
                raise StartupError("Unsafe MariaDB MCP source archive")
            target = destination.joinpath(*parts[1:])
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as source:
                    target.write_bytes(source.read())
    if not (destination / "Dockerfile").is_file():
        raise StartupError("MariaDB MCP source archive has no Dockerfile")


def ensure_image(environment: dict[str, str]) -> None:
    build_env = docker_environment(environment)
    if image_available(build_env):
        return
    daemon = subprocess.run(["docker", "info", "--format", "{{.OSType}}"],
                            env=build_env, stdin=subprocess.DEVNULL,
                            capture_output=True, text=True, timeout=30)
    if daemon.returncode != 0 or daemon.stdout.strip() != "linux":
        raise StartupError("A running Linux Docker daemon is required")
    cache = Path(environment.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    cache = cache / "ai-profiles" / "mariadb"
    cache.mkdir(parents=True, exist_ok=True)
    # Different projects/sessions may request the same image concurrently.
    with (cache / f"{COMMIT}.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if image_available(build_env):
            return
        print("MariaDB MCP: building pinned image; first start can take several minutes "
              "and several GB of disk space. Progress is on stderr.", file=sys.stderr)
        with tempfile.TemporaryDirectory(prefix="ai-profiles-mariadb-") as temporary:
            source = Path(temporary)
            fetch_source(source)
            result = subprocess.run(
                ["docker", "build", "--platform", PLATFORM,
                 "--label", f"{SOURCE_LABEL}={COMMIT}", "--tag", IMAGE, str(source)],
                env=build_env, stdin=subprocess.DEVNULL,
                stdout=sys.stderr, stderr=sys.stderr, timeout=3600)
        if result.returncode != 0 or not image_available(build_env):
            raise StartupError("MariaDB MCP image build failed; check Docker build progress on stderr")


def launch(environment: dict[str, str]) -> None:
    for key in REQUIRED:
        if not environment.get(key):
            raise StartupError(f"Missing required MariaDB setting: {key}")
    if (not environment["DB_PORT"].isascii() or not environment["DB_PORT"].isdigit()
            or not 1 <= int(environment["DB_PORT"]) <= 65535):
        raise StartupError("DB_PORT must be a number between 1 and 65535")
    for key in ("MCP_READ_ONLY", "MCP_BLOCK_SENSITIVE_SHOW"):
        if environment.get(key, "false") not in ("true", "false"):
            raise StartupError(f"{key} must be true or false")
    ensure_image(environment)
    runtime_env = dict(environment)
    runtime_env.setdefault("MCP_READ_ONLY", "false")
    runtime_env.setdefault("MCP_BLOCK_SENSITIVE_SHOW", "false")
    args = ["docker", "run", "--interactive", "--rm", "--platform", PLATFORM,
            "--network=" + environment.get("MARIADB_MCP_NETWORK", "host")]
    for key in CONTAINER_ENV:
        if key in runtime_env:
            args.extend(["--env", key])  # values only in process environment, never argv
    args.extend([IMAGE, "python", "src/server.py", "--transport", "stdio"])
    os.execvpe("docker", args, runtime_env)


def main() -> int:
    try:
        launch(dict(os.environ))
    except StartupError as error:
        print(f"MariaDB MCP startup failed: {error}", file=sys.stderr)
        return 1
    except (OSError, ValueError, tarfile.TarError, subprocess.SubprocessError):
        # Never echo raw exceptions, environment values or credentials.
        print("MariaDB MCP startup failed: check Docker, network access, cache and disk space",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
