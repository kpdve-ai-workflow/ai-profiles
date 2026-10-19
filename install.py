#!/usr/bin/env python3
"""Install one GitHub-hosted OpenCode profile into a local project.

Preserves unrelated MCP/config and switches the previously managed profile.
"""
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PROFILES = {"backend-go": "go-backend", "frontend-react": "react-frontend"}


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in PROFILES:
        print("Usage: python3 install.py backend-go|frontend-react /path/to/project", file=sys.stderr)
        return 2
    name = sys.argv[1]
    project = Path(sys.argv[2]).expanduser().resolve()
    if not project.is_dir():
        print(f"Project directory does not exist: {project}", file=sys.stderr)
        return 2
    if (project / "opencode.jsonc").exists():
        print("Project has opencode.jsonc. MVP installer does not edit JSONC; migrate or merge manually.", file=sys.stderr)
        return 2
    target_config = project / "opencode.json"
    try:
        config = json.loads(target_config.read_text()) if target_config.exists() else {"$schema": "https://opencode.ai/config.json"}
        if not isinstance(config, dict):
            raise ValueError("opencode.json must be a JSON object")
        existing_mcp = config.get("mcp", {})
        if not isinstance(existing_mcp, dict):
            raise ValueError("opencode.json mcp must be a JSON object")
        existing_instructions = config.get("instructions", [])
        if not isinstance(existing_instructions, list):
            raise ValueError("opencode.json instructions must be a list")
        mcp = json.loads((ROOT / "profiles" / name / "mcp.json").read_text())
        if not isinstance(mcp, dict):
            raise ValueError("profile mcp must be a JSON object")
    except (OSError, json.JSONDecodeError, ValueError) as e:
        print(f"Configuration error: {e}", file=sys.stderr)
        return 2

    opendir = project / ".opencode"
    state_file = opendir / "ai-profile-state.json"
    try:
        state = json.loads(state_file.read_text()) if state_file.exists() else {}
    except json.JSONDecodeError:
        print("Invalid previous ai-profile-state.json; refusing to overwrite unknown files.", file=sys.stderr)
        return 2
    old_mcp = state.get("mcp")
    old_skill = state.get("skill")
    if old_mcp and old_mcp not in ("postgres", "figma"):
        print("Unexpected managed MCP state; refusing to continue.", file=sys.stderr)
        return 2
    if old_skill and old_skill not in PROFILES.values():
        print("Unexpected managed skill state; refusing to continue.", file=sys.stderr)
        return 2

    # Install shared workflow and selected skill.
    skill_dir = opendir / "skills"
    plugin_dir = opendir / "plugins"
    skill_dir.mkdir(parents=True, exist_ok=True)
    plugin_dir.mkdir(parents=True, exist_ok=True)
    for shared in ("implementation-breakdown", "refine-plan"):
        dst = skill_dir / shared
        dst.mkdir(exist_ok=True)
        shutil.copy2(ROOT / "common" / "skills" / shared / "SKILL.md", dst / "SKILL.md")
    selected_skill = PROFILES[name]
    if old_skill and old_skill != selected_skill:
        shutil.rmtree(skill_dir / old_skill, ignore_errors=True)
    (skill_dir / selected_skill).mkdir(exist_ok=True)
    shutil.copy2(ROOT / "profiles" / name / "skills" / selected_skill / "SKILL.md", skill_dir / selected_skill / "SKILL.md")
    shutil.copy2(ROOT / "common" / "plugins" / "approval-gate.js", plugin_dir / "approval-gate.js")
    shutil.copy2(ROOT / "common" / "workflow-rules.md", opendir / "workflow-rules.md")

    if old_mcp:
        existing_mcp.pop(old_mcp, None)
    # Only own the selected MCP. Preserve all other MCP names not managed here.
    if any(key in existing_mcp for key in mcp):
        print("Warning: replacing an existing MCP with the same name as the selected profile.", file=sys.stderr)
    existing_mcp.update(mcp)
    config["mcp"] = existing_mcp
    rule = ".opencode/workflow-rules.md"
    if rule not in existing_instructions:
        existing_instructions.append(rule)
    config["instructions"] = existing_instructions
    target_config.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n")
    state_file.write_text(json.dumps({"profile": name, "skill": selected_skill, "mcp": next(iter(mcp))}, indent=2) + "\n")
    (opendir / "workflow-approval.json").unlink(missing_ok=True)
    gitignore = project / ".gitignore"
    line = ".opencode/workflow-approval.json"
    content = gitignore.read_text() if gitignore.exists() else ""
    if line not in content.splitlines():
        with gitignore.open("a") as f:
            f.write(("\n" if content and not content.endswith("\n") else "") + line + "\n")
    print(f"Installed {name} in {project}. Gate: LOCKED. Restart OpenCode.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
