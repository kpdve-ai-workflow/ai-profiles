from pathlib import Path

from profile_management.constants import SCHEMA, STATE, IGNORE_LINE
from profile_management.context import RepositoryContext
from profile_management.errors import ProfileError
from profile_management.infrastructure.filesystem import file_path, read_json
from profile_management.infrastructure.repository import profile_names


def selected_project(context: RepositoryContext, project_option: Path | None) -> Path:
    project = (project_option or Path.cwd()).expanduser().resolve()
    if not project.is_dir():
        raise ProfileError(f"Projekt nie istnieje: {project}")
    if project == context.root.resolve() or context.root.resolve() in project.parents:
        raise ProfileError("Nie instaluj profilu wewnątrz repozytorium ai-profiles")
    return project


def state_for(context: RepositoryContext, project: Path) -> dict | None:
    path = file_path(project, STATE)
    if not path.exists():
        return None
    value = read_json(path)
    if (not isinstance(value, dict) or value.get("format") != 1
            or not isinstance(value.get("files"), dict) or not isinstance(value.get("mcp"), dict)
            or value.get("profile") not in profile_names(context)):
        raise ProfileError("Nieobsługiwany stan profilu. Nie nadpisuję obcych plików: " + str(path))
    if value.get("version") is not None:
        version_tuple(value["version"])
    return value


def config_for(project: Path) -> tuple[dict, bool]:
    if (project / "opencode.jsonc").exists():
        raise ProfileError("Wykryto opencode.jsonc. Żeby nie zgubić komentarzy ani "
                           "zmieszać dwóch konfiguracji, najpierw scal go z opencode.json.")
    path = file_path(project, "opencode.json")
    existed = path.exists()
    cfg = read_json(path) if existed else {"$schema": SCHEMA}
    if not isinstance(cfg, dict):
        raise ProfileError("opencode.json musi zawierać obiekt JSON")
    if not isinstance(cfg.get("mcp", {}), dict):
        raise ProfileError("opencode.json: mcp musi być obiektem")
    if not isinstance(cfg.get("instructions", []), list):
        raise ProfileError("opencode.json: instructions musi być tablicą")
    return cfg, existed

from profile_management.versioning import version_tuple


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
