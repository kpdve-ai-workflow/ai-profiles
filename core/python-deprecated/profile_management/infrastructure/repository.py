import re
import subprocess
import tempfile
from pathlib import Path

from profile_management.context import RepositoryContext
from profile_management.errors import ProfileError
from profile_management.infrastructure.filesystem import file_path, json_bytes, read_json
from profile_management.versioning import version_tuple


def profile_names(context: RepositoryContext) -> list[str]:
    return sorted(path.name for path in (context.root / "profiles").iterdir() if path.is_dir())


def profile_metadata(context: RepositoryContext, profile: str) -> dict:
    metadata = read_json(context.root / "profiles" / profile / "profile.json")
    if (not isinstance(metadata, dict) or metadata.get("name") != profile
            or not isinstance(metadata.get("description"), str)
            or not metadata["description"].strip()):
        raise ProfileError(f"Nieprawidłowy profile.json: {profile}")
    version_tuple(metadata.get("version"))
    return metadata



def source_commit(context: RepositoryContext) -> str | None:
    try:
        result = subprocess.run(["git", "-C", str(context.root), "rev-parse", "HEAD"],
                                check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None
