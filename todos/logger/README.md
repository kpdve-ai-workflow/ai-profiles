# Core Rust diagnostic logging

This is the documentation entry point and global roadmap router for local diagnostic logging in `core/rust`. The objective is to reconstruct operations, decisions, data flow, failures, and retained state without changing application behavior.

**Current progress and the next eligible task:** [roadmap.md](roadmap.md). There is no second execution queue in this README.

## Reading route

| Document | Responsibility |
| --- | --- |
| [Roadmap](roadmap.md) | Main execution plan: five explicit numbered stage headings, smaller tasks within each stage, completion/transition criteria, evidence, and the next action. |
| [Knowledge base](knowledge-base.md) | Working requirements and accepted invariants, derived from the original input and subsequent instructions. |
| [Concept map](concept-map.md) | Complete topic coverage and ownership, including source-section migration. |
| [Dependencies](dependencies.md) | Hard prerequisites, interface dependencies, and suggested ordering. |
| [Coverage matrix](coverage-matrix.md) | Requirements and tasks mapped to specifications and evidence. |
| [Quality and acceptance](quality-and-acceptance.md) | Shared test-first, isolation, regression, and completion rules. |
| [Decisions and gaps](decisions-and-gaps.md) | Decision provenance and unresolved implementation boundaries. |
| [Migration validation](validation-report.md) | Documentation migration checks, not a new application test run. |

## Concept navigation

| Concept | Outcome and specification |
| --- | --- |
| Foundation and storage | [Local sessions, configuration, payload storage, and delivery health](concepts/foundation.md) |
| Process diagnostics | [Runner lifecycle and process data](concepts/processes.md) |
| Profile operations | [Client-scoped lifecycle and remaining manager commands](concepts/profiles.md) |
| WordPress restore | [Plan, backup, import, localization, and recovery diagnostics](concepts/restore.md) |
| MCP preparation | [Configuration, dotenv, environment, and launch preparation](concepts/mcp.md) |
| Novamira integration | [Wrapper diagnostics and bridge/HTTP visibility](concepts/novamira.md) |
| Process handoff | [Observation after exec without protocol or cleanup regressions](concepts/handoff.md) |
| Optional export | [Future Alloy/Loki/Grafana integration, not currently authorized](concepts/export.md) |

## Ownership and updates

- Update `roadmap.md` after each bounded implementation task, including actual verification evidence.
- Select only one eligible task per agent invocation unless the owner explicitly requests otherwise.
- Identify the current stage in the roadmap before selecting work. Do not skip an incomplete stage without explicit owner approval recorded there; technical independence alone is not permission to change the delivery order.
- Keep concepts and their contracts separate from mutable progress. The specs and coverage matrix do not duplicate task statuses.
- The original [logger.md](logger.md) was moved without changing its contents. It is retained only as an archive of the original knowledge and work history. Future work must use this package, not that file.
- Source-derived requirements are distinguished from planner-selected task boundaries. Unknown integration decisions remain explicit blockers; optional export has not become a required deliverable.
- The separate [observability proposal](../observability.md) concerns whole-agent sessions and is **not** merged into this logging objective.
