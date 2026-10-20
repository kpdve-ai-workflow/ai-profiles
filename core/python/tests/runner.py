"""Adapter from contract scenarios to the currently selected CLI."""
from pathlib import Path
import sys


def cli_argv(source: Path, *arguments: str) -> list[str]:
    launcher = source / "core/python/profile_management/ai-profile.py"
    return [sys.executable, str(launcher), *arguments]
