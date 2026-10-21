---
name: mcp-novamira
description: Read or change WordPress content, settings, menus, ACF data, or other site data through Novamira MCP. Use before Novamira-backed WordPress tasks; confirm the target site and inspect ability schemas first.
metadata:
  short-description: Use Novamira MCP safely for project WordPress data
---

# Novamira MCP workflow

Use this skill before reading or changing WordPress data through Novamira. It does not replace local theme-code editing or apply to unrelated MCP servers.

## Select the site

- Use project instructions and the configured Novamira connection to establish the intended host. Never assume a domain or blog ID from another project.
- Verify the actual blog ID and home URL using a read-only ability after discovery and schema inspection.
- In multisite, switch to another blog only on explicit user instruction. If the intended site is ambiguous, ask before writing.

Multisite blogs have separate content, options, menus, and media. Never assume data is shared.

## Connect and inspect abilities

1. Verify that Novamira MCP tools are available in the active session. If they are missing, check the MCP connection/reload state; do not assume an OAuth failure or repeatedly initiate login.
2. Find the current server's `mcp-adapter-discover-abilities` tool and call it before WordPress work. Discover the client-visible name rather than assuming a prefix or naming convention.
3. Call its `mcp-adapter-get-ability-info` for each ability before using `mcp-adapter-execute-ability`, and follow the current input schema.
4. Use read-only operations for inspection. Perform a write only when the user explicitly requests it.
5. After a write, read the affected data back through Novamira and verify the result.

This profile uses the ai-profiles stdio adapter and a pinned WordPress bridge with `OAUTH_ENABLED=false`, targeting `/wp-json/mcp/novamira` (not `/novamira-oauth`). The user configures a WordPress Application Password in a private env file. Never request credentials in chat, print them, read secret files into tool output, or put them in versioned files. Do not switch to OAuth or alter credentials as routine troubleshooting. User-facing setup instructions are in `.opencode/ai-profile/README.md`.

## Safety boundaries

- Do not modify, deactivate, uninstall, or delete the Novamira plugin; doing so can interrupt this connection.
- Do not revoke or delete the connected WordPress user's Application Password or other Novamira credentials.
- Do not edit WordPress core or perform direct database writes when the requested workflow belongs in WordPress/Novamira APIs.
- Treat code changes in the theme as repository work: use local file tools, not Novamira's remote file-writing abilities.
- For destructive or broad data changes, inspect the target records and scope first; request confirmation when the user's request does not clearly authorize the change.

## When the connection is unavailable

Distinguish a missing tool surface from an authorization or network error. Novamira is disabled until the user configures and enables it in `.opencode/ai-profile.local.json`, runs update and restarts OpenCode. Check site availability, the intended host, plugin activation, AI Abilities, account permissions and the MCP connection state without exposing secrets. Do not disable TLS verification or repeatedly retry authentication. If the connection cannot be restored, report the blocking layer and stop before writing data.
