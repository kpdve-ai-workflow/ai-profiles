from pathlib import Path

from core.profile_management.implementations.python.manager.constants import STATE
from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.infrastructure.filesystem import file_path, sha, json_bytes
from core.profile_management.implementations.python.manager.infrastructure.mcp import configured_mcp, desired_mcp
from core.profile_management.implementations.python.manager.infrastructure.payload import desired_files
from core.profile_management.implementations.python.manager.infrastructure.repository import profile_metadata
from core.profile_management.implementations.python.manager.infrastructure.project import config_for, state_for


def status(context: RepositoryContext, project: Path) -> None:
    old = state_for(context, project)
    if not old:
        print(f"{project}: brak profilu")
        return
    mismatches = []
    for relative, expected in old["files"].items():
        path = file_path(project, relative)
        if not path.is_file() or sha(path.read_bytes()) != expected:
            mismatches.append(relative)
    available = profile_metadata(context, old["profile"])["version"]
    installed = old.get("version")
    files = desired_files(context, old["profile"])
    base = desired_mcp(context, old["profile"])
    content_changed = (old["files"] != {key: sha(value) for key, value in files.items()}
                       or old.get("mcp_base", old["mcp"]) != base)
    _, overrides = configured_mcp(project, base)
    cfg, _ = config_for(project)
    mcp_modified = [key for key, expected in old["mcp"].items() if cfg.get("mcp", {}).get(key) != expected]
    if installed is None:
        label = "VERSION UNKNOWN (legacy installation)"
    elif version_tuple(installed) < version_tuple(available):
        label = "UPDATE AVAILABLE"
    elif version_tuple(installed) > version_tuple(available):
        label = "INSTALLED NEWER (downgrade blocked)"
    elif content_changed:
        label = "CONTENT CHANGED WITHOUT VERSION BUMP"
    else:
        label = "UP TO DATE"
    print(f"Profile: {old['profile']}")
    print(f"Installed: {installed or 'UNKNOWN'}")
    print(f"Available: {available}")
    print(f"Status: {label}")
    print(f"Source commit: {old.get('source_commit') or 'UNKNOWN'}")
    print(f"Content: {'CHANGED' if content_changed else 'UNCHANGED'}")
    print("Local modifications: " + (", ".join(mismatches + [f"MCP:{key}" for key in mcp_modified]) or "none"))
    print("MCP local overrides: " + (", ".join(sorted(overrides)) or "none"))
    if old.get("mcp_local_hash") != sha(json_bytes(overrides)):
        print("MCP local configuration: changed; run update to synchronize generated settings")
    legacy_gate = ".opencode/plugins/approval-gate.js" in old["files"]
    print("Approval gate: " + ("legacy installation; removed on update" if legacy_gate else "not installed"))

from core.profile_management.implementations.python.manager.versioning import version_tuple
