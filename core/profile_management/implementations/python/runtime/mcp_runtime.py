#!/usr/bin/env python3
"""Shared MCP configuration and project-local stdio launcher (stdlib only)."""
from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import re
import shlex
import sys

LOCAL = ".opencode/ai-profile.local.json"
STATE = ".opencode/ai-profile-state.json"
LAUNCHER = ".opencode/ai-profile-mcp.py"
ENV_REFERENCE = re.compile(r"\{env:([A-Za-z_][A-Za-z0-9_]*)\}")
FIELDS = {"type", "command", "environment", "env_file", "enabled", "disabled",
           "cwd", "url", "headers", "oauth", "timeout", "codemode", "protocol", "access_mode"}
ACCESS_MODES = ("restricted", "unrestricted")


class ConfigError(Exception):
    pass


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ConfigError(f"Cannot read valid JSON: {path.name}") from error


def project_path(project: Path, relative: str) -> Path:
    if not isinstance(relative, str) or not relative or Path(relative).is_absolute() or ".." in Path(relative).parts:
        raise ConfigError("Expected a relative path inside the project")
    path = project
    for part in Path(relative).parts:
        path = path / part
        if path.is_symlink():
            raise ConfigError("Symlinks are not supported for MCP configuration")
    return path


def load_local(project: Path, servers: dict) -> dict:
    path = project_path(project, LOCAL)
    if not path.exists():
        return {}
    value = read_json(path)
    if not isinstance(value, dict) or set(value) - {"mcp"} or not isinstance(value.get("mcp", {}), dict):
        raise ConfigError("ai-profile.local.json must contain an mcp object")
    overrides = value.get("mcp", {})
    for name, entry in overrides.items():
        if name not in servers:
            raise ConfigError(f"Local override targets an unmanaged or removed MCP: {name}")
        if not isinstance(entry, dict) or set(entry) - FIELDS:
            raise ConfigError(f"Invalid local MCP fields: {name}")
        if "type" in entry and entry["type"] != servers[name].get("type"):
            raise ConfigError(f"Local override cannot change MCP transport type: {name}")
    return overrides


def merge(base: dict, override: dict) -> dict:
    result = copy.deepcopy(base)
    for key, value in override.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)
    return result


def split_access_mode(command: list[str]) -> tuple[list[str], str | None]:
    """Accept both CLI spellings; reject ambiguous or malformed mode arguments."""
    remaining = []
    modes = []
    index = 0
    while index < len(command):
        argument = command[index]
        if argument == "--access-mode":
            index += 1
            if index == len(command):
                raise ConfigError("Missing PostgreSQL --access-mode value")
            modes.append(command[index])
        elif argument.startswith("--access-mode="):
            modes.append(argument.split("=", 1)[1])
        else:
            remaining.append(argument)
        index += 1
    if len(modes) > 1:
        raise ConfigError("Duplicate PostgreSQL --access-mode arguments; specify exactly one mode")
    if modes and modes[0] not in ACCESS_MODES:
        raise ConfigError("PostgreSQL --access-mode must be restricted or unrestricted")
    return remaining, modes[0] if modes else None


def apply_access_mode(entry: dict) -> None:
    # The dedicated setting is the only authority. Old CLI arguments are removed
    # rather than silently retaining a different mode in project overrides.
    command, _ = split_access_mode(entry["command"])
    mode = entry.get("access_mode", "unrestricted")
    entry["command"] = [*command, f"--access-mode={mode}"]


def validate_servers(servers: dict, project: Path | None = None) -> None:
    for name, entry in servers.items():
        if not isinstance(entry, dict) or entry.get("type") not in ("local", "remote"):
            raise ConfigError(f"Invalid MCP type: {name}")
        if entry["type"] == "local":
            command = entry.get("command")
            if not isinstance(command, list) or not command or any(not isinstance(v, str) or not v for v in command):
                raise ConfigError(f"MCP command must be a nonempty string array: {name}")
        elif not isinstance(entry.get("url"), str) or not entry["url"]:
            raise ConfigError(f"Missing remote MCP URL: {name}")
        if "access_mode" in entry:
            if name != "postgres" or entry["type"] != "local":
                raise ConfigError("access_mode is supported only for local postgres MCP")
            if entry["access_mode"] not in ACCESS_MODES:
                raise ConfigError("PostgreSQL access_mode must be restricted or unrestricted")
        if name == "postgres" and entry["type"] == "local":
            split_access_mode(entry["command"])
        for field in ("environment", "headers"):
            value = entry.get(field, {})
            if not isinstance(value, dict) or any(not isinstance(v, str) for v in value.values()):
                raise ConfigError(f"MCP {field} must map names to strings: {name}")
        if any(not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key)
               for key in entry.get("environment", {})):
            raise ConfigError(f"Invalid environment variable name: {name}")
        for field in ("enabled", "disabled", "codemode"):
            if field in entry and not isinstance(entry[field], bool):
                raise ConfigError(f"MCP {field} must be boolean: {name}")
        if "cwd" in entry and (not isinstance(entry["cwd"], str) or not entry["cwd"]):
            raise ConfigError(f"Invalid MCP cwd: {name}")
        if "timeout" in entry:
            timeout = entry["timeout"]
            values = timeout.values() if isinstance(timeout, dict) else [timeout]
            if (isinstance(timeout, dict) and set(timeout) - {"startup", "catalog", "execution"}
                    or any(type(value) is not int or value <= 0 for value in values)):
                raise ConfigError(f"Invalid MCP timeout: {name}")
        if "oauth" in entry and not isinstance(entry["oauth"], (bool, dict)):
            raise ConfigError(f"Invalid MCP OAuth settings: {name}")
        if "protocol" in entry and entry["protocol"] not in ("legacy", "auto", "2026-07-28"):
            raise ConfigError(f"Invalid MCP protocol: {name}")
        if "env_file" in entry:
            if entry["type"] != "local":
                raise ConfigError(f"env_file is supported only for local MCP: {name}")
            project_path(project or Path.cwd(), entry["env_file"])


def resolve(project: Path, base: dict) -> tuple[dict, dict]:
    overrides = load_local(project, base)
    effective = {name: merge(entry, overrides.get(name, {})) for name, entry in base.items()}
    validate_servers(effective, project)
    for name, entry in effective.items():
        if name == "postgres" and entry["type"] == "local":
            apply_access_mode(entry)
    return effective, overrides


def render(effective: dict) -> dict:
    """Keep secrets out of generated OpenCode configuration and state."""
    result = copy.deepcopy(effective)
    for name, entry in result.items():
        if entry["type"] == "local":
            for field in ("env_file", "environment", "cwd", "access_mode"):
                entry.pop(field, None)
            # Absolute paths are supplied by the installer for nested workspaces.
            entry["command"] = ["python3", LAUNCHER, name]
    return result


def changed_override_paths(old: dict, new: dict, override: dict, prefix="") -> list[str]:
    paths = []
    for key, value in override.items():
        path = f"{prefix}.{key}" if prefix else key
        before, after = old.get(key), new.get(key)
        if isinstance(value, dict) and isinstance(before, dict) and isinstance(after, dict):
            paths.extend(changed_override_paths(before, after, value, path))
        elif before != after:
            paths.append(path)
    return paths


def dotenv(path: Path) -> dict[str, str]:
    """Literal single-line dotenv. Never execute shell or expand variables."""
    try:
        lines = path.read_text(encoding="utf-8-sig").splitlines()
    except OSError as error:
        raise ConfigError(f"Cannot read env file: {path.name}") from error
    values = {}
    for number, line in enumerate(lines, 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[7:].lstrip()
        key, separator, value = line.partition("=")
        key, value = key.strip(), value.strip()
        if not separator or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise ConfigError(f"Invalid env entry at line {number}")
        if value.startswith(("'", '"')):
            try:
                parts = shlex.split(value, comments=True, posix=True)
            except ValueError as error:
                raise ConfigError(f"Invalid env quoting at line {number}") from error
            if len(parts) != 1:
                raise ConfigError(f"Invalid env value at line {number}")
            value = parts[0]
        else:
            value = "" if value.startswith("#") else value.split(" #", 1)[0].rstrip()
        values[key] = value
    return values


def environment_for(project: Path, entry: dict, inherited: dict) -> dict:
    environment = dict(inherited)
    if "env_file" in entry:
        environment.update(dotenv(project_path(project, entry["env_file"])))
    # All mappings read from the same input, independently of dictionary order.
    def substitute(match):
        name = match.group(1)
        if not environment.get(name):
            raise ConfigError(f"Required environment variable is missing or empty: {name}")
        return environment[name]
    mapped = {key: ENV_REFERENCE.sub(substitute, value)
              for key, value in entry.get("environment", {}).items()}
    environment.update(mapped)
    return environment


def launch(project: Path, name: str) -> None:
    state = read_json(project_path(project, STATE))
    if (not isinstance(state, dict) or state.get("format") != 1
            or not isinstance(state.get("mcp_base"), dict) or not state["mcp_base"]):
        raise ConfigError("Missing or invalid mcp_base in profile state; run ai-profiles update")
    servers, _ = resolve(project, state["mcp_base"])
    if name not in servers or servers[name]["type"] != "local":
        raise ConfigError("Unknown local MCP server")
    entry = servers[name]
    if entry.get("disabled") is True or entry.get("enabled") is False:
        raise ConfigError("MCP server is disabled locally")
    environment = environment_for(project, entry, os.environ)
    cwd = Path(entry.get("cwd", "."))
    if not cwd.is_absolute():
        cwd = project / cwd
    os.chdir(cwd)
    # Preserve stdio and signals. No shell, no subprocess receiving secret argv.
    os.execvpe(entry["command"][0], entry["command"], environment)


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: ai-profile-mcp.py SERVER", file=sys.stderr)
        return 2
    try:
        launch(Path(__file__).resolve().parent.parent, sys.argv[1])
    except ConfigError as error:
        # ConfigError messages contain field names, never environment values.
        print(f"MCP startup failed: {error}", file=sys.stderr)
        return 1
    except (OSError, ValueError):
        # Raw OS errors can include executable paths or other sensitive values.
        print("MCP startup failed: check working directory and executable availability.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
