from pathlib import Path
from .context import RepositoryContext
from .constants import GATE, IGNORE_LINE, PROFILES, RULE, STATE
from .errors import ProfileError
from .filesystem import file_path, json_bytes, read_json, sha, write_bytes
from .mcp import desired_mcp
from .project import config_for

def remove_gitignore_line(project: Path, old: dict) -> None:
    if not old.get("gitignore_added"):
        return
    p = file_path(project, ".gitignore")
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        if line.strip("\r\n") == IGNORE_LINE:
            del lines[i]
            break
    p.write_text("".join(lines), encoding="utf-8")

def migrate_legacy(context: RepositoryContext, project: Path, accept_existing: bool) -> None:
    """Explicit adoption of the original MVP's unversioned state.

    We cannot reconstruct original hashes, so this requires explicit confirmation.
    It records the present files as the baseline, without modifying their content.
    """
    if not accept_existing:
        raise ProfileError("Migracja starej instalacji wymaga --accept-existing po przeglądzie plików")
    state_path = file_path(project, STATE)
    if not state_path.exists():
        raise ProfileError("Brak starego profilu")
    old = read_json(state_path)
    if not isinstance(old, dict) or old.get("format") == 1:
        raise ProfileError("Nie wykryto starego formatu profilu (zainstalowany nowy lub niepoprawny)")
    profile = old.get("profile")
    if (profile not in PROFILES or old.get("skill") != PROFILES[profile]
            or old.get("mcp") not in desired_mcp(context, profile)):
        raise ProfileError("Nieznany format starego profilu")
    cfg, _ = config_for(project)
    mcp_name = old["mcp"]
    if mcp_name not in cfg.get("mcp", {}):
        raise ProfileError("Nie odnaleziono dotychczasowej konfiguracji MCP")
    rels = [RULE, ".opencode/plugins/approval-gate.js"]
    rels += [f".opencode/skills/{name}/SKILL.md" for name in
             ("implementation-breakdown", "refine-plan", old["skill"])]
    hashes = {}
    for rel in rels:
        path = file_path(project, rel)
        if not path.is_file():
            raise ProfileError(f"Brak pliku dotychczasowego profilu: {rel}")
        hashes[rel] = sha(path.read_bytes())
    old_state = {
        "format": 1, "profile": profile, "files": hashes,
        "mcp": {mcp_name: cfg["mcp"][mcp_name]},
        "had_config": True, "gitignore_added": False,
        "instructions_added": False,
    }
    # Retire any legacy approval marker during migration.
    write_bytes(state_path, json_bytes(old_state))
    file_path(project, GATE).unlink(missing_ok=True)
    print(f"OK: zaadoptowano stary profil {profile}; teraz uruchom update")
