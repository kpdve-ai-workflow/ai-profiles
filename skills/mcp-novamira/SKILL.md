---
name: mcp-novamira
description: Read or change WordPress content, settings, menus, ACF data, or other site data through Novamira MCP. Use before Novamira-backed WordPress tasks; confirm the target site and inspect ability schemas first.
metadata:
  short-description: Use Novamira MCP safely for project WordPress data
---

# Novamira MCP workflow

Use this skill before reading or changing WordPress data through Novamira. It does not replace local theme-code editing or apply to unrelated MCP servers.

## Select the site

- Establish the intended site from the user's request and applicable project instructions **before** interpreting discovery results. Record the expected host and any explicitly specified blog ID. Never assume a domain or blog ID from another project. The configured MCP endpoint identifies a connection, not authorization to modify every site reachable through it.
- After discovery and schema inspection, verify the actual blog ID and home URL using read-only abilities. Compare them with the intended target; a site list is evidence about available sites, **not permission to choose a different target**.
- **Target mismatch is a hard stop for all writes.** If Novamira does not recognize the intended domain, returns another host/blog, or the target cannot be verified, report the expected host and observed host/blog ID and ask the user to resolve the discrepancy. Do not treat a redirect, canonical URL, similar domain, matching content/title, sole listed site, or blog ID `1` as an authorized fallback or proof that two domains are aliases. Continue only after explicit clarification authorizing the target and read-only verification of that target.
- In multisite, switch to another blog only on explicit user instruction. Do not silently replace the requested domain with a discovered blog ID. Do not repair URLs, switch connections, change credentials, or use SQL/WP-CLI to bypass a target mismatch.

Multisite blogs have separate content, options, menus, and media. Never assume data is shared.

## Before every write

- Confirm that the verified site still matches the authorized target. If the connection or blog context changes, verify again before writing. When an ability exposes a site/blog selector, pass the verified target explicitly according to its schema; when it does not, establish its effective site context read-only or stop.
- Read the target object on **that site** and verify its ID, type and relevant identifying fields against the requested operation. Object IDs are site-scoped; never reuse an ID obtained from another blog or infer the site from a matching ID/title.
- Moving a page/post to trash is a destructive write, not harmless inspection. The request must authorize that action on the verified site and object. If scope or identity is uncertain, ask before executing; availability of a delete/trash ability is not consent.

Example: project instructions name `preview.example.test`, but Novamira rejects that domain and lists only `main.example.test` as blog `1`. Report the mismatch and ask which site is authorized. **Do not trash or modify anything on blog `1`**, even if it contains a page with the expected title.

## Connect and inspect abilities

1. Verify that Novamira MCP tools are available in the active session. If they are missing, check the MCP connection/reload state; do not assume an OAuth failure or repeatedly initiate login.
2. Find the current server's `mcp-adapter-discover-abilities` tool and call it before WordPress work. Discover the client-visible name rather than assuming a prefix or naming convention.
3. Call its `mcp-adapter-get-ability-info` for each ability before using `mcp-adapter-execute-ability`, and follow the current input schema.
4. Use read-only operations for inspection. Perform a write only when the user explicitly requests it and the target-site and object checks above pass. A mismatch blocks every mutation, including PHP execution that writes data.
5. After a write, read the affected data back through Novamira and verify the result.

This profile uses the ai-profiles stdio adapter and a pinned WordPress bridge with `OAUTH_ENABLED=false`, targeting `/wp-json/mcp/novamira` (not `/novamira-oauth`). Novamira is enabled by default. The installer creates an empty `.env.novamira` template with mode `0600`, preserving any existing regular file. The user fills in the site endpoint, the WordPress account's login (`WP_API_USERNAME`), and its generated Application Password (`WP_API_PASSWORD`), **not the wp-admin login password**, then restarts the client/MCP. No local.json or activation command is required. The user must protect the env file against commits and HTTP access: no Git ignore rule is added automatically. Never request credentials in chat, print them, read secret files into tool output, or put them in versioned files. Do not switch to OAuth or alter credentials as routine troubleshooting. User-facing setup instructions are in `{{AI_PROFILE_ROOT}}/README.md`.

## Safety boundaries

- Do not modify, deactivate, uninstall, or delete the Novamira plugin; doing so can interrupt this connection.
- Do not revoke or delete the connected WordPress user's Application Password or other Novamira credentials.
- Do not edit WordPress core or perform direct database writes when the requested workflow belongs in WordPress/Novamira APIs.
- Treat code changes in the theme as repository work: use local file tools, not Novamira's remote file-writing abilities.
- For destructive or broad data changes, inspect the target records and scope first; request confirmation when the user's request does not clearly authorize the change.

## When the connection is unavailable

Distinguish a missing tool surface from an authorization or network error. The adapter rejects the bridge's failed initialize response, including `connectionFailed: {}`, through the client's normal MCP error handling. Confirmed HTTP 401 gets Application Password guidance and the actual configured env path; a failure with no confirmed HTTP status gets generic connection guidance. Never infer a wrong password from zero tools, TLS, DNS, timeouts or forbidden permissions. Respect deliberate local disablement and custom `env_file` in `{{AI_PROFILE_LOCAL}}`; configuration changes require `make update CLIENT={{AI_PROFILE_CLIENT}} PROJECT=<project>` and restart, but editing credentials alone only requires restarting the client/MCP. Check site availability, the intended host, plugin activation, AI Abilities, account permissions and the MCP connection state without exposing secrets. Do not disable TLS verification or repeatedly retry authentication. If the connection cannot be restored, report the blocking layer and stop before writing data.
