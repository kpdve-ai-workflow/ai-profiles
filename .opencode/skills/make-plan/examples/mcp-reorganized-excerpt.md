# F1-3 — MCP: Example of a Clearer Plan Opening

> **Illustrative restructuring of the provided plan, not an independently verified description of the current repository.** This example intentionally covers only the opening sections, not the full technical plan. Any user-attribution below is based on what the supplied plan explicitly reports.

## Status and next action

**Scope:** Configure GitHub, Figma, PostgreSQL and Grafana MCP in the user-level Pi profile; no automatic OAuth or server connections during installation.

**Current implementation:** T-01 (GitHub/Figma installer and adapter integration) is reported as implemented; manual acceptance is still pending. T-02 (PostgreSQL) and T-03 (Grafana) are reported as not yet implemented.

| Task | Result | Implementation | Verification |
| --- | --- | --- | --- |
| T-01 | MCP installer, GitHub/Figma, adapter | IMPLEMENTED | MANUAL PENDING |
| T-02 | PostgreSQL MCP and local Docker connectivity | TODO | NOT RUN |
| T-03 | Grafana MCP and local Docker connectivity | TODO | NOT RUN |

**Next action:** Manually validate T-01 in the user's Pi profile before starting T-02.

## Requirements and user decisions

### Intended behavior

The installer adds managed MCP entries to the user profile, preserves unrelated configuration, does not store secrets in the repository and does not authenticate or connect to MCP servers automatically.

### Decisions explicitly attributed to the user in the source plan

| Choice | Required outcome | Source confidence |
| --- | --- | --- |
| Extend F1-3 scope | Include PostgreSQL and Grafana, without changing the official MVP roadmap | Reported in the source plan as user-requested |
| PostgreSQL and Grafana access | Make available full read/write operations, constrained by real account permissions | Reported in the source plan as user-required |
| Figma OAuth workaround | Use `pi-mcp-adapter` and `pi-figma-remote-auth` after the reported native OAuth 403 | Reported in the source plan as user-selected |
| Validation boundary | Do not add automated MCP configuration/connection tests; user performs manual checks | Reported in the plan's scope, precise approval origin to confirm if necessary. This is a historical scope constraint in the source example, not permission to add automated tests; apply the current test-first policy only within authorized test boundaries. |

Do not put the above choices under `Verified technical findings`: they are requirements/choices, not discoveries about the environment.

## Verified technical findings

The source plan **reports** that:

- A Pi CLI version and `pi mcp add --help` were inspected in an earlier planning environment.
- The Postgres MCP and Grafana MCP upstream documentation described their corresponding Docker/stdio options.
- The documented Docker host-gateway mechanism can expose host-published services to an MCP container.

These are historical statements in the provided plan. Do not assert current installed versions or working connections without fresh repository/runtime evidence.

## Technical approach and decisions

Describe GitHub/Figma adapter configuration, user-level file preservation and Postgres/Grafana stdio Docker targets once, in this section. A decision may say `Use Docker stdio` with its reason; it must not repeat an entire step-by-step task.

**Freshness issue requiring review:** The source plan's older GitHub HTTP/token decision conflicts with the later MCP documentation's GitHub Docker/device OAuth flow. Treat this as a potentially superseded contract, not as two simultaneously valid choices. Inspect current code before specifying future work.

## Implementation tasks

Only `T-02` and `T-03` need implementation instructions if T-01 is already implemented. T-01 should contain links to changed artifacts, validation evidence and the pending manual acceptance step—not a full 'create these files' to-do list.
