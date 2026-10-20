"""Implementation-independent temporary fixture helpers."""
import json
import shutil
from pathlib import Path

def copy_repository(source: Path, destination: Path) -> Path:
    return shutil.copytree(source, destination, ignore=shutil.ignore_patterns(".git", "__pycache__", "*.pyc", "node_modules", "tests"))

def write_json(path: Path, value) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value))
    return path

def file_snapshot(root: Path) -> dict[str, bytes]:
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}
