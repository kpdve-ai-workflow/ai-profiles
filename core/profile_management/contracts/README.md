# Profile management contract

This document specifies observable behavior, independent of implementation
language, module names, and function signatures.

## CLI

The commands are `create`, `install`, `switch`, `status`, `update`, `uninstall`,
`validate`, `migrate-legacy`, `bump`, and `help`; omitted action means help.
`--project` selects an existing project directory for project actions; omission
selects the caller's current directory. `create` accepts a profile name and
optional `--description`; `bump` accepts `--part major|minor|patch`; legacy
migration accepts `--accept-existing`. Help and success use stdout and exit 0,
argument errors use the command parser's nonzero usage exit, and operational
errors print `ERROR:` to stderr and exit 1.

## Source data

Profiles are selected by `profiles/<name>/profile.json`, `mcp.json`, optional
profile skills, and optional profile README/scripts. Common skills under
`common/skills/` are delivered to every profile. MCP definitions come only from
the registry entries explicitly selected by the profile. Repository-local
`.opencode/skills/` workflows are excluded from installed payloads.

## Installation and ownership

Managed paths include `.opencode/skills/`, `.opencode/ai-profile-mcp.py`,
selected `.opencode/mcp/` launchers, optional profile scripts/readme beneath
`.opencode/ai-profile/`, state `.opencode/ai-profile-state.json`, and generated
managed MCP entries in `opencode.json`. Synchronization is shared by install,
update, and switch. Uninstall removes managed content while preserving unrelated
files and project-local `.opencode/ai-profile.local.json`. Repeating an unchanged
update yields the same managed snapshot.

## State

Current state is JSON format 1 with the selected profile/version/source commit,
managed file SHA-256 values, managed MCP base entries, and ownership data
including whether the installer added its instruction. Earlier format-1 states
without version and source commit are accepted; older hashed states without MCP base are migrated
after checking the existing configuration. MVP states without hashes require
explicit legacy adoption. State values and managed hashes are compatibility data.

## Safety

Managed-file and MCP conflicts are checked before mutation. Modified managed
files block update/uninstall; unmanaged files and local configuration are
preserved. Symlinks and path collisions are rejected, downgrade of the same
profile is blocked, and `opencode.jsonc` is rejected rather than rewritten.
Retired installer-owned instructions/files are removed only when their recorded
ownership/hash matches; user-owned instructions remain. Writes are not guaranteed
to be transactionally recoverable from interruption or disk failure.

## MCP runtime

The installed entrypoint is `.opencode/ai-profile-mcp.py`. It reads MCP base
configuration from state and overrides from `.opencode/ai-profile.local.json`.
Environment precedence is inherited process environment, optional local env
file, then explicit environment mapping. Local commands run without a shell;
the runtime forwards stdio and replaces the process with the server. Remote MCP
remains native to the host application. Errors do not reveal secret values.

## Compatibility

A replacement must preserve CLI behavior, payload bytes and paths, state
read/write compatibility, ownership semantics, conflict-before-mutation safety,
and installed runtime behavior. Changing managed paths, state fields/meaning,
serialization, supported old states, conflict rules, or runtime inputs requires
an explicit compatibility migration. Source-checkout launcher paths are not
installed-project compatibility guarantees.
