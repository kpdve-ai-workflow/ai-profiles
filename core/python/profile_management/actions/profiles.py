import re
import subprocess
import tempfile
from pathlib import Path

from ..context import RepositoryContext
from ..errors import ProfileError
from ..infrastructure.filesystem import file_path, json_bytes, read_json


def profile_names(context: RepositoryContext) -> list[str]:
    return sorted(path.name for path in (context.root / "profiles").iterdir() if path.is_dir())


def version_tuple(version: str) -> tuple[int, int, int]:
    if not isinstance(version, str) or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ProfileError(f"Nieprawidłowa wersja MAJOR.MINOR.PATCH: {version!r}")
    return tuple(map(int, version.split(".")))


def profile_metadata(context: RepositoryContext, profile: str) -> dict:
    metadata = read_json(context.root / "profiles" / profile / "profile.json")
    if (not isinstance(metadata, dict) or metadata.get("name") != profile
            or not isinstance(metadata.get("description"), str)
            or not metadata["description"].strip()):
        raise ProfileError(f"Nieprawidłowy profile.json: {profile}")
    version_tuple(metadata.get("version"))
    return metadata


def create_profile(context: RepositoryContext, profile: str, description: str | None = None) -> None:
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", profile):
        raise ProfileError("Nazwa profilu musi zawierać małe litery/cyfry rozdzielone pojedynczymi myślnikami")
    if description is not None and not description.strip():
        raise ProfileError("Opis profilu nie może być pusty")
    destination = file_path(context.root, f"profiles/{profile}")
    if destination.exists():
        raise ProfileError(f"Profil już istnieje: {profile}")
    with tempfile.TemporaryDirectory(dir=destination.parent, prefix=".create-") as temporary:
        staged = Path(temporary) / profile
        (staged / "skills").mkdir(parents=True)
        (staged / "profile.json").write_bytes(json_bytes({
            "name": profile, "version": "1.0.0",
            "description": description.strip() if description is not None else f"Profil {profile}",
        }))
        (staged / "mcp.json").write_bytes(json_bytes({"servers": []}))
        (staged / "skills" / ".gitkeep").touch()
        destination.mkdir()
        try:
            for source in staged.iterdir():
                source.rename(destination / source.name)
        except OSError as error:
            raise ProfileError(f"Nie udało się utworzyć profilu: {destination}; sprawdź jego zawartość") from error
    print(f"OK: utworzono profil {profile} (1.0.0)")


def bump_profile(context: RepositoryContext, profile: str, part: str = "patch") -> str:
    if profile not in profile_names(context):
        raise ProfileError(f"Nieznany profil: {profile}")
    if part not in ("major", "minor", "patch"):
        raise ProfileError(f"Nieprawidłowy rodzaj bump: {part}")
    path = file_path(context.root, f"profiles/{profile}/profile.json")
    metadata = profile_metadata(context, profile)
    previous = metadata["version"]
    major, minor, patch = version_tuple(previous)
    version = (f"{major + 1}.0.0" if part == "major" else
               f"{major}.{minor + 1}.0" if part == "minor" else f"{major}.{minor}.{patch + 1}")
    metadata["version"] = version
    temporary_path = None
    try:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=".profile-", suffix=".tmp", delete=False) as output:
            temporary_path = Path(output.name)
            output.write(json_bytes(metadata))
        temporary_path.chmod(path.stat().st_mode & 0o777)
        temporary_path.replace(path)
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)
    print(f"OK: {profile}: {previous} -> {version}")
    return version

