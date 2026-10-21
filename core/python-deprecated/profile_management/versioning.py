from __future__ import annotations

import re

from profile_management.errors import ProfileError


def version_tuple(version: str) -> tuple[int, int, int]:
    if not isinstance(version, str) or not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ProfileError(f"Nieprawidłowa wersja MAJOR.MINOR.PATCH: {version!r}")
    return tuple(map(int, version.split(".")))
