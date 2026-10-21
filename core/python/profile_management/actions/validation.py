from profile_management.context import RepositoryContext
from profile_management.errors import ProfileError
from profile_management.infrastructure.filesystem import file_path, read_json
from profile_management.infrastructure.mcp import desired_mcp, mcp_definition
from profile_management.infrastructure.payload import desired_files
from profile_management.infrastructure.repository import profile_metadata, profile_names


def validate_sources(context: RepositoryContext, profile: str | None = None) -> None:
    if profile is not None and profile not in profile_names(context):
        raise ProfileError(f"Nieznany profil: {profile}")
    if profile is None:
        for path in sorted(file_path(context.root, "mcp/servers").iterdir()):
            if path.suffix == ".json":
                mcp_definition(context, path.stem)
    all_mcp: dict = {}
    for selected in ([profile] if profile is not None else profile_names(context)):
        profile_metadata(context, selected)
        desired_files(context, selected)
        all_mcp.update(desired_mcp(context, selected))
    example = read_json(context.root / "mcp/example.json")
    if (not isinstance(example, dict) or set(example) != {"mcp"}
            or not isinstance(example["mcp"], dict)
            or any(not isinstance(entry, dict) for entry in example["mcp"].values())):
        raise ProfileError("Nieprawidłowy mcp/example.json")
    if profile is not None:
        if any(name not in example["mcp"] or example["mcp"][name] != entry
               for name, entry in all_mcp.items()):
            raise ProfileError(f"mcp/example.json różni się od MCP wybranych przez profil {profile}")
        print(f"OK: profil {profile}, wspólne skille i definicje MCP są spójne")
        return
    if example != {"mcp": all_mcp}:
        raise ProfileError("mcp/example.json różni się od definicji MCP wybranych przez profile")
    print("OK: wszystkie skille i definicje MCP są spójne")
