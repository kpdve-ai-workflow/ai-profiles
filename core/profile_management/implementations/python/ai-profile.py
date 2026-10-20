#!/usr/bin/env python3
"""Source-checkout command launcher."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT))
from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.cli import main

if __name__ == "__main__":
    raise SystemExit(main(context=RepositoryContext(ROOT)))
