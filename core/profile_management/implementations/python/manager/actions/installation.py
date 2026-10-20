from pathlib import Path
from core.profile_management.implementations.python.runtime import mcp_runtime
from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.constants import GATE, RULE, STATE, SCHEMA
from core.profile_management.implementations.python.manager.errors import ProfileError
from core.profile_management.implementations.python.manager.infrastructure.filesystem import check_files, cleanup_empty_directories, file_path, json_bytes, sha, write_bytes
from core.profile_management.implementations.python.manager.infrastructure.project import remove_gitignore_line
from core.profile_management.implementations.python.manager.infrastructure.mcp import check_mcp, configured_mcp, desired_mcp
from core.profile_management.implementations.python.manager.infrastructure.payload import desired_files
from core.profile_management.implementations.python.manager.infrastructure.repository import profile_metadata, source_commit
from core.profile_management.implementations.python.manager.infrastructure.project import config_for, state_for

def apply_profile(context: RepositoryContext, project: Path, profile: str) -> None:
    old = state_for(context, project)
    metadata = profile_metadata(context, profile)
    if (old and old["profile"] == profile and old.get("version") is not None
            and version_tuple(old["version"]) > version_tuple(metadata["version"])):
        raise ProfileError("Downgrade niedozwolony: zainstalowana wersja jest wyższa od dostępnej")
    files = desired_files(context, profile)
    base_mcp = desired_mcp(context, profile)
    mcp, overrides = configured_mcp(project, base_mcp)
    cfg, existed = config_for(project)
    check_files(project, old, files)
    check_mcp(cfg, old, mcp)
    prior_base = old.get("mcp_base", old.get("mcp", {})) if old else {}
    for name, override in overrides.items():
        if name in prior_base:
            for path in mcp_runtime.changed_override_paths(prior_base[name], base_mcp[name], override):
                print(f"WARNING: upstream changed overridden field {name}.{path}; local override wins.")
    # Preflight every destination before changing any managed file.
    for rel in [*files, STATE, GATE, ".gitignore", "opencode.json"]:
        destination = file_path(project, rel)
        if destination.exists() and not destination.is_file():
            raise ProfileError(f"Konflikt: ścieżka nie jest plikiem: {rel}")
        for parent in destination.parents:
            if parent == project:
                break
            if parent.exists() and not parent.is_dir():
                raise ProfileError(f"Konflikt: ścieżka nie jest katalogiem: {parent}")
    if file_path(project, ".gitignore").exists():
        file_path(project, ".gitignore").read_text(encoding="utf-8")
    commit = source_commit(context, )
    current = dict(cfg.get("mcp", {}))
    for k in (old.get("mcp", {}) if old else {}):
        current.pop(k)
    current.update(mcp)
    if current:
        cfg["mcp"] = current
    else:
        cfg.pop("mcp", None)
    rules = list(cfg.get("instructions", []))
    instructions_added = bool(old and old.get("instructions_added"))
    if instructions_added:
        rules = [item for item in rules if item != RULE]
        instructions_added = False
    if rules:
        cfg["instructions"] = rules
    else:
        cfg.pop("instructions", None)

    for rel in (old.get("files", {}) if old else {}):
        if rel not in files:
            file_path(project, rel).unlink()
    for rel, content in files.items():
        write_bytes(file_path(project, rel), content)
    write_bytes(file_path(project, "opencode.json"), json_bytes(cfg))
    if old:
        remove_gitignore_line(project, old)
    updated_state = {
        "format": 1,
        "profile": profile,
        "version": metadata["version"],
        "source_commit": commit,
        "files": {key: sha(value) for key, value in files.items()},
        "mcp": mcp,
        "mcp_base": base_mcp,
        "mcp_local_hash": sha(json_bytes(overrides)),
        "had_config": old.get("had_config", existed) if old else existed,
        "gitignore_added": False,
        "instructions_added": instructions_added,
    }
    if old:
        file_path(project, GATE).unlink(missing_ok=True)  # retire the old gate marker
    if old:
        cleanup_empty_directories(project, [k for k in old["files"] if k not in files])
    write_bytes(file_path(project, STATE), json_bytes(updated_state))
    print(f"OK: profil {profile} w {project}. Zrestartuj OpenCode.")
    if ".opencode/ai-profile/README.md" in files:
        print(f"Instrukcja konfiguracji: {project / '.opencode/ai-profile/README.md'}")

def uninstall(context: RepositoryContext, project: Path) -> None:
    old = state_for(context, project)
    if not old:
        raise ProfileError("Brak zainstalowanego profilu w tym projekcie")
    cfg, _ = config_for(project)
    check_files(project, old, {})
    check_mcp(cfg, old, {})
    for rel in old["files"]:
        file_path(project, rel).unlink()
    mcp = dict(cfg.get("mcp", {}))
    for key in old["mcp"]:
        mcp.pop(key, None)
    if mcp:
        cfg["mcp"] = mcp
    else:
        cfg.pop("mcp", None)
    instructions = list(cfg.get("instructions", []))
    if old.get("instructions_added"):
        instructions = [item for item in instructions if item != RULE]
    if instructions:
        cfg["instructions"] = instructions
    else:
        cfg.pop("instructions", None)
    config_path = file_path(project, "opencode.json")
    if not old.get("had_config") and cfg == {"$schema": SCHEMA}:
        config_path.unlink()
    else:
        write_bytes(config_path, json_bytes(cfg))
    file_path(project, GATE).unlink(missing_ok=True)
    file_path(project, STATE).unlink()
    remove_gitignore_line(project, old)
    cleanup_empty_directories(project, list(old["files"]) + [STATE, GATE])
    print(f"OK: profil usunięty z {project}. Pozostałe pliki projektu zachowane.")

from core.profile_management.implementations.python.manager.versioning import version_tuple
