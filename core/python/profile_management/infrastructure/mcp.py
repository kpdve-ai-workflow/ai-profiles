import re
from pathlib import Path

from profile_management import mcp_runtime

from profile_management.context import RepositoryContext
from profile_management.errors import ProfileError
from profile_management.infrastructure.filesystem import file_path, read_json


def mcp_definition(context: RepositoryContext, name: str) -> dict:
    if not isinstance(name, str) or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name):
        raise ProfileError("Nieprawidłowa nazwa definicji MCP; użyj małych liter/cyfr i pojedynczych myślników")
    path = file_path(context.root, f"mcp/servers/{name}.json")
    if not path.is_file():
        raise ProfileError(f"Brak definicji MCP: {name} (mcp/servers/{name}.json)")
    entry = read_json(path)
    mcp_runtime.validate_servers({name: entry})
    return entry


def desired_mcp(context: RepositoryContext, profile: str) -> dict:
    value = read_json(file_path(context.root, f"profiles/{profile}/mcp.json"))
    if (not isinstance(value, dict) or set(value) != {"servers"}
            or not isinstance(value["servers"], list)):
        raise ProfileError(f"Nieprawidłowe MCP dla {profile}: oczekiwano listy servers z nazwami z mcp/servers/")
    servers = {}
    for name in value["servers"]:
        entry = mcp_definition(context, name)
        if name in servers:
            raise ProfileError(f"Powielona nazwa MCP w profilu {profile}: {name}")
        servers[name] = entry
    return servers


def configured_mcp(project: Path, base: dict) -> tuple[dict, dict]:
    effective, overrides = mcp_runtime.resolve(project, base)
    rendered = mcp_runtime.render(effective)
    for name, entry in rendered.items():
        if effective[name]["type"] == "local":
            entry["command"][1] = str(project / mcp_runtime.LAUNCHER)
    return rendered, overrides


def check_mcp(cfg: dict, old: dict | None, new: dict) -> None:
    current = cfg.get("mcp", {})
    prior = old.get("mcp", {}) if old else {}
    for key, expected in prior.items():
        if current.get(key) != expected:
            raise ProfileError(f"Konflikt: konfiguracja MCP '{key}' była zmieniona. "
                               "Nie nadpisuję jej automatycznie.")
    for key in new:
        if key in current and key not in prior:
            raise ProfileError(f"MCP '{key}' już istnieje i nie należy do managera.")
