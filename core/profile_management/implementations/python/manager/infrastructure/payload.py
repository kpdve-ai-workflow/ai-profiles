import re
from pathlib import Path

from core.profile_management.implementations.python.runtime import mcp_runtime

from core.profile_management.implementations.python.manager.context import RepositoryContext
from core.profile_management.implementations.python.manager.errors import ProfileError
from core.profile_management.implementations.python.manager.infrastructure.filesystem import file_path
from core.profile_management.implementations.python.manager.infrastructure.mcp import desired_mcp


def desired_files(context: RepositoryContext, profile: str) -> dict[str, bytes]:
    files: dict[str, bytes] = {}
    for folder in (context.root / "common" / "skills", context.root / "profiles" / profile / "skills"):
        if not folder.is_dir():
            raise ProfileError(f"Brak katalogu skills: {folder}")
        for skill in sorted(folder.iterdir()):
            if skill.name == ".gitkeep" and skill.is_file() and not skill.is_symlink():
                continue
            if not skill.is_dir() or skill.is_symlink():
                raise ProfileError(f"Nieprawidłowy katalog skilla: {skill}")
            skill_file = skill / "SKILL.md"
            if not skill_file.is_file() or skill_file.is_symlink():
                raise ProfileError(f"Brak lub nieprawidłowy SKILL.md w {skill}")
            frontmatter = skill_file.read_text(encoding="utf-8")
            if not frontmatter.startswith("---\n") or "\n---" not in frontmatter[4:]:
                raise ProfileError(f"Nieprawidłowy YAML frontmatter: {skill_file}")
            metadata = frontmatter.split("\n---", 1)[0]
            name_match = re.search(r'^name:\s*["\']?([a-z0-9]+(?:-[a-z0-9]+)*)["\']?\s*$', metadata, re.M)
            desc_match = re.search(r'^description:\s*.+$', metadata, re.M)
            if not name_match or name_match.group(1) != skill.name or not desc_match:
                raise ProfileError(f"SKILL.md musi mieć name={skill.name} i description: {skill_file}")
            for source in skill.rglob("*"):
                if source.is_symlink():
                    raise ProfileError(f"Skill używa symlinka: {source}")
                if source.is_file():
                    relative = (Path(".opencode/skills") / skill.name / source.relative_to(skill)).as_posix()
                    if relative in files:
                        raise ProfileError(f"Duplikat pliku skilla: {relative}")
                    files[relative] = source.read_bytes()
    files[mcp_runtime.LAUNCHER] = (context.root / "core/profile_management/implementations/python/runtime/mcp_runtime.py").read_bytes()
    for name, entry in desired_mcp(context, profile).items():
        launcher = file_path(context.root, f"mcp/launchers/{name}")
        if launcher.exists():
            if not launcher.is_dir():
                raise ProfileError(f"Nieprawidłowy katalog launchera MCP: {name}")
            for source in sorted(launcher.rglob("*")):
                if source.is_symlink():
                    raise ProfileError(f"Launcher MCP używa symlinka: {source}")
                if "__pycache__" in source.relative_to(launcher).parts or source.suffix == ".pyc":
                    continue
                if source.is_file():
                    relative = (Path(".opencode/mcp") / name / source.relative_to(launcher)).as_posix()
                    files[relative] = source.read_bytes()
        for argument in entry.get("command", []):
            if argument.startswith(".opencode/mcp/") and argument not in files:
                raise ProfileError(f"Brak zarządzanego pliku launchera MCP: {argument}")
    readme = file_path(context.root, f"profiles/{profile}/README.md")
    scripts = file_path(context.root, f"profiles/{profile}/scripts")
    if scripts.exists():
        if not scripts.is_dir():
            raise ProfileError(f"Nieprawidłowy katalog scripts: {profile}")
        for source in sorted(scripts.rglob("*")):
            if source.is_symlink():
                raise ProfileError(f"Skrypt profilu używa symlinka: {source}")
            if "__pycache__" in source.relative_to(scripts).parts or source.suffix == ".pyc":
                continue
            if source.is_file():
                files[(Path(".opencode/ai-profile/scripts") / source.relative_to(scripts)).as_posix()] = source.read_bytes()
    if readme.is_file():
        files[".opencode/ai-profile/README.md"] = readme.read_bytes()
    return files
