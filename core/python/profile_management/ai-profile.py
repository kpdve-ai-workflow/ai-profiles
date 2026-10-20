#!/usr/bin/env python3
"""Source-checkout command launcher."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from profile_management.context import RepositoryContext
from profile_management.cli import main

if __name__ == "__main__":
    raise SystemExit(main(context=RepositoryContext(ROOT)))
