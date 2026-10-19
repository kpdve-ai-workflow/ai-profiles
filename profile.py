#!/usr/bin/env python3
"""Local OpenCode profile manager: install, update, switch and uninstall.
Uses only the Python standard library (Python 3.10+).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
PROFILES = {"backend-go": "go-backend", "frontend-react": "react-frontend"}
RULE = ".opencode/workflow-rules.md"
GATE = ".opencode/workflow-approval.json"
STATE = ".opencode/ai-profile-state.json"
IGNORE_LINE = GATE
SCHEMA = "https://opencode.ai/config.json"


class ProfileError(Exception):
    pass


def sha(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        raise ProfileError(f"Nie mogę odczytać {path}: {e}") from e


def selected_project(project_option: Path | None) -> Path:
    project = (project_option or Path.cwd()).expanduser().resolve()
    if not project.is_dir():
        raise ProfileError(f"Projekt nie istnieje: {project}")
    if project == ROOT or ROOT in project.parents:
        raise ProfileError("Nie instaluj profilu wewnątrz repozytorium ai-profiles")
    return project


def file_path(project: Path, relative: str) -> Path:
    if relative.startswith("/") or ".." in Path(relative).parts:
        raise ProfileError(f"Nieprawidłowa ścieżka zarządzana: {relative}")
    destination = project / relative
    # Protect against pre-existing symlinked directories or files.
    p = project
    for part in Path(relative).parts:
        p = p / part
        if p.is_symlink():
            raise ProfileError(f"Odmowa operacji przez symlink: {p}")
    return destination


def state_for(project: Path) -> dict | None:
    p = file_path(project, STATE)
    if not p.exists():
        return None
    value = read_json(p)
    if (not isinstance(value, dict) or value.get("format") != 1
            or not isinstance(value.get("files"), dict) or not isinstance(value.get("mcp"), dict)
            or value.get("profile") not in PROFILES):
        raise ProfileError("Nieobsługiwany stan profilu. Nie nadpisuję obcych plików: " + str(p))
    return value


def config_for(project: Path) -> tuple[dict, bool]:
    if (project / "opencode.jsonc").exists():
        raise ProfileError("Wykryto opencode.jsonc. Żeby nie zgubić komentarzy ani "
                           "zmieszać dwóch konfiguracji, najpierw scal go z opencode.json.")
    p = file_path(project, "opencode.json")
    existed = p.exists()
    cfg = read_json(p) if existed else {"$schema": SCHEMA}
    if not isinstance(cfg, dict):
        raise ProfileError("opencode.json musi zawierać obiekt JSON")
    if not isinstance(cfg.get("mcp", {}), dict):
        raise ProfileError("opencode.json: mcp musi być obiektem")
    if not isinstance(cfg.get("instructions", []), list):
        raise ProfileError("opencode.json: instructions musi być tablicą")
    return cfg, existed


def desired_files(profile: str) -> dict[str, bytes]:
    """Collect every skill, including skills newly added upstream (and their assets)."""
    files: dict[str, bytes] = {}
    for folder in (ROOT / "common" / "skills", ROOT / "profiles" / profile / "skills"):
        if not folder.is_dir():
            raise ProfileError(f"Brak katalogu skills: {folder}")
        for skill in sorted(folder.iterdir()):
            if not skill.is_dir() or skill.is_symlink():
                raise ProfileError(f"Nieprawidłowy katalog skilla: {skill}")
            skill_file = skill / "SKILL.md"
            if not skill_file.is_file() or skill_file.is_symlink():
                raise ProfileError(f"Brak lub nieprawidłowy SKILL.md w {skill}")
            frontmatter = skill_file.read_text(encoding="utf-8")
            if not frontmatter.startswith("---\n") or "\n---" not in frontmatter[4:]:
                raise ProfileError(f"Nieprawidłowy YAML frontmatter: {skill_file}")
            metadata = frontmatter.split("\n---", 1)[0]
            name_match = re.search(r'^name:\s*[\"\']?([a-z0-9]+(?:-[a-z0-9]+)*)[\"\']?\s*$', metadata, re.M)
            desc_match = re.search(r'^description:\s*.+$', metadata, re.M)
            if not name_match or name_match.group(1) != skill.name or not desc_match:
                raise ProfileError(f"SKILL.md musi mieć name={skill.name} i description: {skill_file}")
            for source in skill.rglob("*"):
                if source.is_symlink():
                    raise ProfileError(f"Skill używa symlinka: {source}")
                if source.is_file():
                    rel = (Path(".opencode/skills") / skill.name / source.relative_to(skill)).as_posix()
                    if rel in files:
                        raise ProfileError(f"Duplikat pliku skilla: {rel}")
                    files[rel] = source.read_bytes()
    files[".opencode/plugins/approval-gate.js"] = (ROOT / "common/plugins/approval-gate.js").read_bytes()
    files[RULE] = (ROOT / "common/workflow-rules.md").read_bytes()
    files[".opencode/ai-profile-gate.py"] = (ROOT / "gate.py").read_bytes()
    return files


def desired_mcp(profile: str) -> dict:
    value = read_json(ROOT / "profiles" / profile / "mcp.json")
    if not isinstance(value, dict) or not value:
        raise ProfileError(f"Nieprawidłowe MCP dla {profile}")
    return value


def check_files(project: Path, old: dict | None, next_files: dict[str, bytes]) -> None:
    owned = old.get("files", {}) if old else {}
    for rel, old_hash in owned.items():
        p = file_path(project, rel)
        if not p.is_file() or sha(p.read_bytes()) != old_hash:
            raise ProfileError(f"Konflikt: zarządzany plik zmieniono/usunięto: {rel}. "
                               "Zrób backup i przywróć zarządzaną wersję przed update/remove.")
    for rel in next_files:
        p = file_path(project, rel)
        if p.exists() and rel not in owned:
            raise ProfileError(f"Konflikt z istniejącym niezarządzanym plikiem: {rel}")


def check_mcp(cfg: dict, old: dict | None, new: dict) -> None:
    current = cfg.get("mcp", {})
    prior = old.get("mcp", {}) if old else {}
    for key, expected in prior.items():
        if current.get(key) != expected:
            raise ProfileError(f"Konflikt: konfiguracja MCP '{key}' była zmieniona. "
                               "Nie nadpisuję jej automatycznie.")
    for key in new:
        if key in current and key not in prior:
            raise ProfileError(f"MCP '{key}' już istnieje i nie należy do managera.")


def ensure_gitignore(project: Path, old: dict | None) -> bool:
    ignore = file_path(project, ".gitignore")
    original = ignore.read_text(encoding="utf-8") if ignore.exists() else ""
    if IGNORE_LINE in original.splitlines():
        return bool(old and old.get("gitignore_added"))
    trailing = "\n" if original and not original.endswith("\n") else ""
    ignore.write_text(original + trailing + IGNORE_LINE + "\n", encoding="utf-8")
    return True


def remove_gitignore_line(project: Path, old: dict) -> None:
    if not old.get("gitignore_added"):
        return
    p = file_path(project, ".gitignore")
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        if line.strip("\r\n") == IGNORE_LINE:
            del lines[i]
            break
    p.write_text("".join(lines), encoding="utf-8")


def write_bytes(path: Path, contents: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)


def cleanup_empty_directories(project: Path, paths: list[str]) -> None:
    for rel in paths:
        p = file_path(project, rel).parent
        while p != project and p != project / ".opencode":
            try:
                p.rmdir()
            except OSError:
                break
            p = p.parent
    try:
        (project / ".opencode").rmdir()
    except OSError:
        pass


def apply_profile(project: Path, profile: str) -> None:
    old = state_for(project)
    files = desired_files(profile)
    mcp = desired_mcp(profile)
    cfg, existed = config_for(project)
    check_files(project, old, files)
    check_mcp(cfg, old, mcp)
    current = dict(cfg.get("mcp", {}))
    for k in (old.get("mcp", {}) if old else {}):
        current.pop(k)
    current.update(mcp)
    if current:
        cfg["mcp"] = current
    else:
        cfg.pop("mcp", None)
    rules = list(cfg.get("instructions", []))
    originally_missing_rule = RULE not in rules
    if originally_missing_rule:
        rules.append(RULE)
    cfg["instructions"] = rules

    for rel in (old.get("files", {}) if old else {}):
        if rel not in files:
            file_path(project, rel).unlink()
    for rel, content in files.items():
        write_bytes(file_path(project, rel), content)
    write_bytes(file_path(project, "opencode.json"), json_bytes(cfg))
    ignore_added = ensure_gitignore(project, old)
    updated_state = {
        "format": 1,
        "profile": profile,
        "files": {key: sha(value) for key, value in files.items()},
        "mcp": mcp,
        "had_config": old.get("had_config", existed) if old else existed,
        "gitignore_added": ignore_added,
        "instructions_added": old.get("instructions_added", originally_missing_rule) if old else originally_missing_rule,
    }
    write_bytes(file_path(project, STATE), json_bytes(updated_state))
    file_path(project, GATE).unlink(missing_ok=True)  # force fresh human approval
    if old:
        cleanup_empty_directories(project, [k for k in old["files"] if k not in files])
    print(f"OK: profil {profile} w {project}; gate LOCKED. Zrestartuj OpenCode.")


def uninstall(project: Path) -> None:
    old = state_for(project)
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


def status(project: Path) -> None:
    old = state_for(project)
    if not old:
        print(f"{project}: brak profilu")
        return
    mismatches = []
    for rel, expected in old["files"].items():
        p = file_path(project, rel)
        if not p.is_file() or sha(p.read_bytes()) != expected:
            mismatches.append(rel)
    print(f"{project}: {old['profile']}, plików zarządzanych: {len(old['files'])}")
    print("Zmodyfikowane/usunięte pliki: " + (", ".join(mismatches) if mismatches else "brak"))
    gate = file_path(project, GATE)
    if gate.exists():
        try:
            approval = read_json(gate)
            print(f"Approval: {approval.get('task') if approval.get('approved') else 'LOCKED'}")
        except (ProfileError, AttributeError):
            print("Approval: LOCKED")
    else:
        print("Approval: LOCKED")


def validate_sources() -> None:
    """Verify source layout and that mcp/example.json mirrors actual profile MCPs."""
    all_mcp: dict = {}
    for profile in PROFILES:
        desired_files(profile)
        mcp = desired_mcp(profile)
        for name, entry in mcp.items():
            if name in all_mcp:
                raise ProfileError(f"Powielona nazwa MCP: {name}")
            all_mcp[name] = entry
    example = read_json(ROOT / "mcp/example.json")
    if example != {"mcp": all_mcp}:
        raise ProfileError("mcp/example.json różni się od profiles/*/mcp.json")
    print("OK: wszystkie skille i definicje MCP są spójne")


def migrate_legacy(project: Path, accept_existing: bool) -> None:
    """Explicit adoption of the original MVP's unversioned state.

    We cannot reconstruct original hashes, so this requires explicit confirmation.
    It records the present files as the baseline, without modifying their content.
    """
    if not accept_existing:
        raise ProfileError("Migracja starej instalacji wymaga --accept-existing po przeglądzie plików")
    state_path = file_path(project, STATE)
    if not state_path.exists():
        raise ProfileError("Brak starego profilu")
    old = read_json(state_path)
    if not isinstance(old, dict) or old.get("format") == 1:
        raise ProfileError("Nie wykryto starego formatu profilu (zainstalowany nowy lub niepoprawny)")
    profile = old.get("profile")
    if (profile not in PROFILES or old.get("skill") != PROFILES[profile]
            or old.get("mcp") not in desired_mcp(profile)):
        raise ProfileError("Nieznany format starego profilu")
    cfg, _ = config_for(project)
    mcp_name = old["mcp"]
    if mcp_name not in cfg.get("mcp", {}):
        raise ProfileError("Nie odnaleziono dotychczasowej konfiguracji MCP")
    rels = [RULE, ".opencode/plugins/approval-gate.js"]
    rels += [f".opencode/skills/{name}/SKILL.md" for name in
             ("implementation-breakdown", "refine-plan", old["skill"])]
    hashes = {}
    for rel in rels:
        path = file_path(project, rel)
        if not path.is_file():
            raise ProfileError(f"Brak pliku dotychczasowego profilu: {rel}")
        hashes[rel] = sha(path.read_bytes())
    old_state = {
        "format": 1, "profile": profile, "files": hashes,
        "mcp": {mcp_name: cfg["mcp"][mcp_name]},
        "had_config": True, "gitignore_added": False,
        "instructions_added": False,
    }
    # Existing gate is left locked for the subsequent update.
    write_bytes(state_path, json_bytes(old_state))
    file_path(project, GATE).unlink(missing_ok=True)
    print(f"OK: zaadoptowano stary profil {profile}; teraz uruchom update")


def main() -> int:
    parser = argparse.ArgumentParser(description="Zarządzanie profilami OpenCode z lokalnego ai-profiles")
    parser.add_argument("action", choices=["install", "update", "uninstall", "switch", "status", "validate", "migrate-legacy"])
    parser.add_argument("profile", nargs="?", choices=list(PROFILES))
    parser.add_argument("--project", type=Path, help="Katalog projektu (domyślnie bieżący katalog)")
    parser.add_argument("--accept-existing", action="store_true", help="Świadomie przejmij pliki starego instalatora")
    opts = parser.parse_args()
    try:
        if opts.action == "validate":
            validate_sources()
            return 0
        project = selected_project(opts.project)
        if opts.action in ("install", "switch"):
            if not opts.profile:
                parser.error("install/switch wymagają nazwy profilu")
            old = state_for(project)
            if opts.action == "install" and old:
                raise ProfileError("Profil już zainstalowany; użyj update albo switch")
            if opts.action == "switch" and not old:
                raise ProfileError("Brak profilu do zmiany; użyj install")
            apply_profile(project, opts.profile)
        elif opts.action == "update":
            old = state_for(project)
            if not old:
                raise ProfileError("Brak zainstalowanego profilu. Użyj install")
            apply_profile(project, old["profile"])
        elif opts.action == "uninstall":
            uninstall(project)
        elif opts.action == "migrate-legacy":
            migrate_legacy(project, opts.accept_existing)
        else:
            status(project)
        return 0
    except (ProfileError, OSError, ValueError, TypeError, KeyError) as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
