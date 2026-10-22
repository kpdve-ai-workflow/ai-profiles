---
name: mcp-mariadb
description: Inspect or change WordPress MariaDB data and schema through the MariaDB MCP server. Use for SQL, database diagnostics and explicitly requested database changes; confirm the database and multisite scope before executing queries.
---

# MariaDB MCP

The `mariadb` server is provided by ai-profiles and disabled until the user
configures an env file and enables it locally. Its managed launcher builds a
pinned official MariaDB MCP Docker image once and connects over stdio. Do not
change the managed launcher or registry to set project credentials.

## Connection and scope

- Inspect current tool availability; names may be normalized by the client.
- Confirm the intended database with a minimal read-only query before a task.
  Never infer a database, hostname, port or table prefix from another project.
- Inspect table schemas before querying or changing data. Avoid broad dumps:
  WordPress options, user data and plugin tables can contain secrets or PII.
- In multisite, one database can contain multiple blogs. Establish the actual
  table prefix and blog ID and keep the operation within the requested scope.
  `DB_NAME` sets a default database, not an access-control boundary.

## Full SQL access

The profile sets `MCP_READ_ONLY=false` and `MCP_BLOCK_SENSITIVE_SHOW=false`.
SQL writes and administrative SHOW commands are permitted by MCP; actual access
depends on the configured MariaDB account. Do not impose read-only settings
unless the user requests them. Full access is a capability, not authorization
to execute unrelated writes, grant privileges or delete data.

- For requested SQL changes, identify affected rows and schema first, follow
  project backup practices, then verify the result with a targeted read.
- Do not execute destructive operations or broad migrations without clear user
  authorization for the target and scope.
- SQL bypasses WordPress hooks, validation and cache invalidation. Prefer
  Novamira/WordPress APIs for ordinary content changes; use direct SQL when the
  task requires it. Do not corrupt serialized PHP values with naive replacements.
- Upstream disables multi-statement execution; submit individual statements.
  Do not assume a transaction persists across separate pooled tool calls.

## Secrets and diagnostics

Never print private env files, credentials, auth options or full environment
dumps. The first connection can spend several minutes building dependencies;
progress is sent to stderr, not the MCP protocol stream. Do not start repeated
builds or restart WordPress to diagnose a normal first build.

Distinguish disabled configuration, missing variables, Docker/image-build errors,
database connectivity and SQL permissions. Do not change database credentials,
grants or Docker networking without user authorization. User-facing setup
instructions are in the installed `{{AI_PROFILE_ROOT}}/README.md`.
