#!/usr/bin/env python3
"""Local OpenCode profile manager: install, update, switch and uninstall.
Uses only the Python standard library (Python 3.10+).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

import mcp_runtime

ROOT = Path(__file__).resolve().parent
PROFILES = {"backend-go": "go-backend", "frontend-react": "react-frontend"}
RULE = ".opencode/workflow-rules.md"
GATE = ".opencode/workflow-approval.json"
STATE = mcp_runtime.STATE
IGNORE_LINE = GATE
SCHEMA = "https://opencode.ai/config.json"


class ProfileError(Exception):
    pass


def profile_names() -> list[str]:
    return sorted(p.name for p in (ROOT / "profiles").iterdir() if p.is_dir())


def create_profile(profile: str, description: str | None = None) -> None:
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", profile):
        raise ProfileError("Nazwa profilu musi zawierać małe litery/cyfry rozdzielone pojedynczymi myślnikami")
    if description is not None and not description.strip():
        raise ProfileError("Opis profilu nie może być pusty")
    destination = file_path(ROOT, f"profiles/{profile}")
    if destination.exists():
        raise ProfileError(f"Profil już istnieje: {profile}")
    # Stage the complete scaffold; never merge into an existing profile.
    with tempfile.TemporaryDirectory(dir=destination.parent, prefix=".create-") as temporary:
        staged = Path(temporary) / profile
        (staged / "skills").mkdir(parents=True)
        (staged / "profile.json").write_bytes(json_bytes({
            "name": profile, "version": "1.0.0",
            "description": description.strip() if description is not None else f"Profil {profile}",
        }))
        (staged / "mcp.json").write_bytes(json_bytes({"servers": []}))
        # Git does not track empty directories; preserve skills after cloning.
        (staged / "skills" / ".gitkeep").touch()
        destination.mkdir()
        try:
            for source in staged.iterdir():
                source.rename(destination / source.name)
        except OSError:
            raise ProfileError(f"Nie udało się utworzyć profilu: {destination}; sprawdź jego zawartość")
    print(f"OK: utworzono profil {profile} (1.0.0)")


def version_tuple(version: str) -> tuple[int, int, int]:
    if not isinstance(version, str) or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ProfileError(f"Nieprawidłowa wersja MAJOR.MINOR.PATCH: {version!r}")
    return tuple(map(int, version.split(".")))


def profile_metadata(profile: str) -> dict:
    metadata = read_json(ROOT / "profiles" / profile / "profile.json")
    if (not isinstance(metadata, dict) or metadata.get("name") != profile
            or not isinstance(metadata.get("description"), str)
            or not metadata["description"].strip()):
        raise ProfileError(f"Nieprawidłowy profile.json: {profile}")
    version_tuple(metadata.get("version"))
    return metadata


def bump_profile(profile: str, part: str = "patch") -> str:
    """Explicit release step; never triggered by install/update or file changes."""
    if profile not in profile_names():
        raise ProfileError(f"Nieznany profil: {profile}")
    if part not in ("major", "minor", "patch"):
        raise ProfileError(f"Nieprawidłowy rodzaj bump: {part}")
    path = file_path(ROOT, f"profiles/{profile}/profile.json")
    metadata = profile_metadata(profile)
    previous = metadata["version"]
    major, minor, patch = version_tuple(previous)
    if part == "major":
        version = f"{major + 1}.0.0"
    elif part == "minor":
        version = f"{major}.{minor + 1}.0"
    else:
        version = f"{major}.{minor}.{patch + 1}"
    metadata["version"] = version
    # Publish the complete metadata in one replacement, preserving other fields.
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".profile-", suffix=".tmp", delete=False) as output:
            temporary = Path(output.name)
            output.write(json_bytes(metadata))
        temporary.chmod(path.stat().st_mode & 0o777)
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
    print(f"OK: {profile}: {previous} -> {version}")
    return version


def source_commit() -> str | None:
    try:
        result = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"],
                                check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


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
            or value.get("profile") not in profile_names()):
        raise ProfileError("Nieobsługiwany stan profilu. Nie nadpisuję obcych plików: " + str(p))
    if value.get("version") is not None:
        version_tuple(value["version"])
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
            if skill.name == ".gitkeep" and skill.is_file() and not skill.is_symlink():
                continue
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
    files[RULE] = (ROOT / "common/workflow-rules.md").read_bytes()
    files[mcp_runtime.LAUNCHER] = (ROOT / "mcp_runtime.py").read_bytes()
    # Optional launcher assets travel only with profiles selecting that MCP.
    for name, entry in desired_mcp(profile).items():
        launcher = file_path(ROOT, f"mcp/launchers/{name}")
        if launcher.exists():
            if not launcher.is_dir():
                raise ProfileError(f"Nieprawidłowy katalog launchera MCP: {name}")
            for source in sorted(launcher.rglob("*")):
                if source.is_symlink():
                    raise ProfileError(f"Launcher MCP używa symlinka: {source}")
                if "__pycache__" in source.relative_to(launcher).parts or source.suffix == ".pyc":
                    continue
                if source.is_file():
                    rel = (Path(".opencode/mcp") / name / source.relative_to(launcher)).as_posix()
                    files[rel] = source.read_bytes()
        for argument in entry.get("command", []):
            if argument.startswith(".opencode/mcp/") and argument not in files:
                raise ProfileError(f"Brak zarządzanego pliku launchera MCP: {argument}")
    readme = file_path(ROOT, f"profiles/{profile}/README.md")
    scripts = file_path(ROOT, f"profiles/{profile}/scripts")
    if scripts.exists():
        if not scripts.is_dir():
            raise ProfileError(f"Nieprawidłowy katalog scripts: {profile}")
        for source in sorted(scripts.rglob("*")):
            if source.is_symlink():
                raise ProfileError(f"Skrypt profilu używa symlinka: {source}")
            if "__pycache__" in source.relative_to(scripts).parts or source.suffix == ".pyc":
                continue
            if source.is_file():
                rel = (Path(".opencode/ai-profile/scripts") / source.relative_to(scripts)).as_posix()
                files[rel] = source.read_bytes()
    if readme.is_file():
        files[".opencode/ai-profile/README.md"] = readme.read_bytes()
    return files


def mcp_definition(name: str) -> dict:
    if not isinstance(name, str) or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name):
        raise ProfileError("Nieprawidłowa nazwa definicji MCP; użyj małych liter/cyfr i pojedynczych myślników")
    path = file_path(ROOT, f"mcp/servers/{name}.json")
    if not path.is_file():
        raise ProfileError(f"Brak definicji MCP: {name} (mcp/servers/{name}.json)")
    entry = read_json(path)
    mcp_runtime.validate_servers({name: entry})
    return entry


def desired_mcp(profile: str) -> dict:
    value = read_json(file_path(ROOT, f"profiles/{profile}/mcp.json"))
    if (not isinstance(value, dict) or set(value) != {"servers"}
            or not isinstance(value["servers"], list)):
        raise ProfileError(f"Nieprawidłowe MCP dla {profile}: oczekiwano listy servers z nazwami z mcp/servers/")
    servers = {}
    for name in value["servers"]:
        entry = mcp_definition(name)
        if name in servers:
            raise ProfileError(f"Powielona nazwa MCP w profilu {profile}: {name}")
        servers[name] = entry
    return servers


def configured_mcp(project: Path, base: dict) -> tuple[dict, dict]:
    effective, overrides = mcp_runtime.resolve(project, base)
    rendered = mcp_runtime.render(effective)
    for name, entry in rendered.items():
        if effective[name]["type"] == "local":
            entry["command"][1] = str(project / mcp_runtime.LAUNCHER)
    return rendered, overrides


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
    metadata = profile_metadata(profile)
    if (old and old["profile"] == profile and old.get("version") is not None
            and version_tuple(old["version"]) > version_tuple(metadata["version"])):
        raise ProfileError("Downgrade niedozwolony: zainstalowana wersja jest wyższa od dostępnej")
    files = desired_files(profile)
    base_mcp = desired_mcp(profile)
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
    commit = source_commit()
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
        "instructions_added": old.get("instructions_added", originally_missing_rule) if old else originally_missing_rule,
    }
    if old:
        file_path(project, GATE).unlink(missing_ok=True)  # retire the old gate marker
    if old:
        cleanup_empty_directories(project, [k for k in old["files"] if k not in files])
    write_bytes(file_path(project, STATE), json_bytes(updated_state))
    print(f"OK: profil {profile} w {project}. Zrestartuj OpenCode.")
    if ".opencode/ai-profile/README.md" in files:
        print(f"Instrukcja konfiguracji: {project / '.opencode/ai-profile/README.md'}")


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
    available = profile_metadata(old["profile"])["version"]
    installed = old.get("version")
    files = desired_files(old["profile"])
    content_changed = (old["files"] != {key: sha(value) for key, value in files.items()}
                       or old.get("mcp_base", old["mcp"]) != desired_mcp(old["profile"]))
    _, overrides = configured_mcp(project, desired_mcp(old["profile"]))
    cfg, _ = config_for(project)
    mcp_modified = [key for key, expected in old["mcp"].items()
                    if cfg.get("mcp", {}).get(key) != expected]
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


def validate_sources(profile: str | None = None) -> None:
    """Validate one profile, or the complete source layout and MCP union."""
    if profile is not None and profile not in profile_names():
        raise ProfileError(f"Nieznany profil: {profile}")
    if profile is None:
        registry = file_path(ROOT, "mcp/servers")
        for path in sorted(registry.iterdir()):
            if path.suffix == ".json":
                mcp_definition(path.stem)
    all_mcp: dict = {}
    for selected_profile in ([profile] if profile is not None else profile_names()):
        profile_metadata(selected_profile)
        desired_files(selected_profile)
        mcp = desired_mcp(selected_profile)
        all_mcp.update(mcp)
    example = read_json(ROOT / "mcp/example.json")
    if (not isinstance(example, dict) or set(example) != {"mcp"}
            or not isinstance(example["mcp"], dict)
            or any(not isinstance(entry, dict) for entry in example["mcp"].values())):
        raise ProfileError("Nieprawidłowy mcp/example.json")
    if profile is not None:
        if any(name not in example["mcp"] or example["mcp"][name] != entry
               for name, entry in all_mcp.items()):
            raise ProfileError(f"mcp/example.json różni się od MCP wybranych przez profil {profile}")
        print(f"OK: profil {profile}, wspólne skille i definicje MCP są spójne")
        return
    if example != {"mcp": all_mcp}:
        raise ProfileError("mcp/example.json różni się od definicji MCP wybranych przez profile")
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
    # Retire any legacy approval marker during migration.
    write_bytes(state_path, json_bytes(old_state))
    file_path(project, GATE).unlink(missing_ok=True)
    print(f"OK: zaadoptowano stary profil {profile}; teraz uruchom update")


COMMAND_HELP = """ai-profiles — zarządzanie profilami

TWORZENIE PROFILU
  make create PROFILE=NAZWA
    Tworzy profil 1.0.0 z pustym MCP i katalogiem skilli. Nie nadpisuje plików.
    Opcjonalny opis: DESCRIPTION="Opis profilu"

PRACA Z PROJEKTEM
  make install PROFILE=NAZWA PROJECT=SCIEZKA_DO_PROJEKTU
    Instaluje profil w projekcie, który jeszcze go nie ma.
  make switch PROFILE=NAZWA PROJECT=SCIEZKA_DO_PROJEKTU
    Zmienia zainstalowany profil na inny.
  make status PROJECT=SCIEZKA_DO_PROJEKTU
    Pokazuje wersje i lokalne zmiany.
  make update PROJECT=SCIEZKA_DO_PROJEKTU
    Aktualizuje profil z lokalnych źródeł, bez pobierania zmian z sieci.
  make pull-update PROJECT=SCIEZKA_DO_PROJEKTU
    Pobiera źródła przez git pull --ff-only, potem aktualizuje profil.
  make uninstall PROJECT=SCIEZKA_DO_PROJEKTU
    Usuwa profil, zachowując własne pliki i ustawienia użytkownika.

UTRZYMANIE REPOZYTORIUM
  make bump PROFILE=NAZWA
    Zwiększa wersję profilu. Opcjonalnie: PART=patch|minor|major (domyślnie patch).
  make validate
    Sprawdza wszystkie profile, skille i konfigurację MCP.
  make validate PROFILE=NAZWA
    Sprawdza tylko wybrany profil, wspólne skille i jego wpisy w mcp/example.json.
    Cały rejestr MCP i nadmiarowe wpisy sprawdza pełne make validate.
  make test
    Uruchamia testy w tymczasowych kopiach repozytorium.
  make help
    Pokazuje pomoc. Samo make również pokazuje pomoc.

ARGUMENTY
  PROFILE      Wymagany tam, gdzie podano. Nie ma domyślnego profilu.
               create: nowa nazwa, małe litery/cyfry i pojedyncze myślniki.
               install/switch/bump/validate: nazwa istniejącego profilu.
               validate: PROFILE jest opcjonalny; bez niego sprawdza wszystko.
  PROJECT      Wymagany tam, gdzie podano: istniejący katalog poza ai-profiles.
  DESCRIPTION  Opcjonalny opis dla create; domyślnie „Profil NAZWA”.
  PART         Opcjonalny rodzaj zmiany wersji dla bump.

"""


class ProfileArgumentParser(argparse.ArgumentParser):
    def format_help(self) -> str:
        return COMMAND_HELP


def main() -> int:
    parser = ProfileArgumentParser(
        description="Zarządzanie profilami OpenCode z lokalnego ai-profiles",
        usage="%(prog)s KOMENDA [NAZWA] [opcje] (pomoc: make help)")
    parser.add_argument("action", nargs="?", choices=["create", "help", "install", "update", "uninstall", "switch", "status", "validate", "migrate-legacy", "bump"], help="Komenda (domyślnie help)")
    parser.add_argument("profile", nargs="?", help="Nazwa profilu; wymagana przez create/install/switch/bump")
    parser.add_argument("--project", type=Path, help="Katalog projektu (domyślnie bieżący katalog)")
    parser.add_argument("--accept-existing", action="store_true", help="Świadomie przejmij pliki starego instalatora")
    parser.add_argument("--part", choices=["major", "minor", "patch"], help="Składnik wersji zwiększany przez bump (domyślnie patch)")
    parser.add_argument("--description", help="Opis nowego profilu (tylko create)")
    opts = parser.parse_args()
    if opts.part is not None and opts.action != "bump":
        parser.error("--part jest dostępne tylko dla bump")
    if opts.description is not None and opts.action != "create":
        parser.error("--description jest dostępne tylko dla create")
    if opts.profile is not None and opts.action not in ("create", "install", "switch", "bump", "validate"):
        parser.error("Ta komenda nie przyjmuje nazwy profilu")
    if opts.project is not None and opts.action in (None, "help", "create", "bump", "validate"):
        parser.error("Ta komenda nie używa --project")
    if opts.accept_existing and opts.action != "migrate-legacy":
        parser.error("--accept-existing jest dostępne tylko dla migrate-legacy")
    try:
        if opts.action in (None, "help"):
            parser.print_help()
            return 0
        if opts.action in ("create", "install", "switch", "bump") and not opts.profile:
            parser.error(f"{opts.action} wymaga nazwy profilu")
        if opts.action in ("install", "switch", "bump") and opts.profile not in profile_names():
            parser.error(f"Nieznany profil: {opts.profile}")
        if opts.action == "create":
            create_profile(opts.profile, opts.description)
            return 0
        if opts.action == "bump":
            if not opts.profile or opts.project is not None or opts.accept_existing:
                parser.error("bump wymaga nazwy profilu źródłowego; nie używa --project ani --accept-existing")
            bump_profile(opts.profile, opts.part or "patch")
            return 0
        if opts.action == "validate":
            validate_sources(opts.profile)
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
    except (ProfileError, mcp_runtime.ConfigError, OSError, ValueError, TypeError, KeyError) as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
