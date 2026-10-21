# Profile management

Python source and tests live under `core/python/`, grouped by functional area.
The project deliberately avoids component → implementations → language
nesting; future languages may have sibling directories and independent
dependencies.

`actions/` contains lifecycle, validation, status, and migration operations;
`infrastructure/` contains repository, project, filesystem, payload, and MCP
access. The standalone `mcp_runtime.py` is copied byte-for-byte into projects.
Run the launcher from any working directory with
`python3 /path/to/repository/core/python/profile_management/ai-profile.py`.

The implementation preserves observable CLI behavior, installed payload bytes
and paths, state compatibility, ownership semantics, conflict checks, and MCP
runtime behavior. See [profile management guidance](../../../docs/profile-management.md).
