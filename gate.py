#!/usr/bin/env python3
"""Human-only helper: approval marker is checked by the OpenCode hook."""
import json
import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) < 3 or sys.argv[2] not in ("approve", "lock", "status"):
        print("Usage: python3 gate.py /path/to/project approve TASK-001 | lock | status", file=sys.stderr)
        return 2
    project = Path(sys.argv[1]).expanduser().resolve()
    file = project / ".opencode" / "workflow-approval.json"
    if not project.is_dir() or not (project / ".opencode" / "plugins" / "approval-gate.js").exists():
        print("Error: Profile not installed in this project.", file=sys.stderr)
        return 2
    action = sys.argv[2]
    if action == "lock":
        file.unlink(missing_ok=True)
        print("LOCKED: file writes and bash blocked.")
        return 0
    if action == "approve":
        if len(sys.argv) != 4 or not re.fullmatch(r"TASK-[A-Za-z0-9_-]+", sys.argv[3]):
            print("Error: specify task ID e.g. TASK-001", file=sys.stderr)
            return 2
        file.write_text(json.dumps({"approved": True, "task": sys.argv[3]}, indent=2) + "\n")
        print(f"UNLOCKED for {sys.argv[3]}. Lock before the next task.")
        return 0
    if file.exists():
        try:
            data = json.loads(file.read_text())
            if data.get("approved") is True and re.fullmatch(r"TASK-[A-Za-z0-9_-]+", data.get("task", "")):
                print(f"UNLOCKED: {data['task']}")
                return 0
        except (ValueError, TypeError):
            pass
    print("LOCKED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
