import hashlib
import json
from pathlib import Path
from typing import Any

from profile_management.errors import ProfileError


def sha(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ProfileError(f"Nie mogę odczytać {path}: {error}") from error


def file_path(project: Path, relative: str) -> Path:
    if relative.startswith("/") or ".." in Path(relative).parts:
        raise ProfileError(f"Nieprawidłowa ścieżka zarządzana: {relative}")
    destination = project / relative
    path = project
    for part in Path(relative).parts:
        path = path / part
        if path.is_symlink():
            raise ProfileError(f"Odmowa operacji przez symlink: {path}")
    return destination


def check_files(project: Path, old: dict | None, next_files: dict[str, bytes]) -> None:
    owned = old.get("files", {}) if old else {}
    for relative, old_hash in owned.items():
        path = file_path(project, relative)
        if not path.is_file() or sha(path.read_bytes()) != old_hash:
            raise ProfileError(f"Konflikt: zarządzany plik zmieniono/usunięto: {relative}. "
                               "Zrób backup i przywróć zarządzaną wersję przed update/remove.")
    for relative in next_files:
        path = file_path(project, relative)
        if path.exists() and relative not in owned:
            raise ProfileError(f"Konflikt z istniejącym niezarządzanym plikiem: {relative}")


def write_bytes(path: Path, contents: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)


def cleanup_empty_directories(project: Path, paths: list[str]) -> None:
    for relative in paths:
        path = file_path(project, relative).parent
        while path != project and path != project / ".opencode":
            try:
                path.rmdir()
            except OSError:
                break
            path = path.parent
    try:
        (project / ".opencode").rmdir()
    except OSError:
        pass
