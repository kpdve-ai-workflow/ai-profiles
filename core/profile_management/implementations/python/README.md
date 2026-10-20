# Python implementation

`manager/cli.py` implements the CLI; `manager/actions/` owns lifecycle,
validation, status, and migration operations; `manager/infrastructure/` owns
repository, project, filesystem, payload, and MCP access. Context, constants,
errors, and version comparison are in `manager/`. `runtime/mcp_runtime.py` is
standalone and is copied byte-for-byte into projects. Run the source launcher
from any directory with `python3 core/profile_management/implementations/python/ai-profile.py`.
