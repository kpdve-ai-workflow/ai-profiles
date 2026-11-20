# Requirement, task and document coverage

This matrix owns traceability, not task statuses. Use [roadmap.md](roadmap.md) for execution progress. Requirement definitions belong to [knowledge-base.md](knowledge-base.md#requirements).

| Task ID | Requirements | Primary specification | Acceptance/evidence owner |
| --- | --- | --- | --- |
| FOUNDATION-BASE | REQ-LOCAL, REQ-HEALTH, REQ-SEMANTICS, REQ-QUALITY | [Foundation](concepts/foundation.md) | Existing gate/MSRV logs linked from its roadmap entry; session/runtime/invocation regression coverage. |
| PROCESS-LIFECYCLE | REQ-FLOW, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH | [Processes: lifecycle](concepts/processes.md#runner-lifecycle) | New registered process-logging tests with real controlled outcomes; actual results go on the roadmap entry. |
| PROCESS-DATA | REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH | [Processes: data](concepts/processes.md#process-data) | Byte/env fixtures, real streams and deadline tests after STREAM-BOUNDARY resolution. |
| PROFILE-LIFECYCLE | REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS | [Profiles: lifecycle](concepts/profiles.md#client-lifecycle) | Fixture files/manifest, activation outcomes and diagnostic records. |
| MANAGER-AUX | REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS | [Profiles: other commands](concepts/profiles.md#other-manager-commands) | CLI/profile/validation/migration fixtures and unchanged user behavior. |
| RESTORE-FLOW | REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH | [Restore](concepts/restore.md) | Synthetic dump/plan/backup/state evidence and existing restore safeguards. |
| MCP-PREPARE | REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-BOUNDARIES | [MCP](concepts/mcp.md) | Configuration/dotenv/effective-child fixtures and protocol/handoff invariants. |
| NOVAMIRA-WRAPPER | REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-BOUNDARIES | [Novamira: wrapper](concepts/novamira.md#wrapper-diagnostics) | Controlled exact MCP messages, error classification and cleanup. |
| NOVAMIRA-UPSTREAM | REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-BOUNDARIES | [Novamira: bridge](concepts/novamira.md#bridge-and-http-diagnostics) | Actual pinned bridge/container and controlled HTTP after channel resolution. |
| HANDOFF-OBSERVE | REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH, REQ-BOUNDARIES | [Handoff](concepts/handoff.md) | Real process/protocol/signal/cleanup evidence after design resolution. |
| EXPORT-OPTIONAL | REQ-LOCAL, REQ-SEMANTICS | [Export](concepts/export.md) | Owner approval, future deployment contract and synthetic integration evidence. |

## Whole-objective acceptance coverage

| Obligation | Primary owner | Coverage |
| --- | --- | --- |
| Complete topic/source migration | [Concept map](concept-map.md) | Every original section is classified, including history, examples, exceptions and future scope. |
| Locality, settings and accepted storage contracts | [Working knowledge](knowledge-base.md#accepted-foundation-contracts) | Exact settings, identity rules, retention, nofollow, health and durability limitations. |
| Normal and failure flow | Producer specifications above | Per-operation/process/guard events, retained partial state and available causal errors. |
| Lossless bytes and bounded capture | [Process specification](concepts/processes.md#process-data) | Integrates existing storage; exposes unresolved observation modes rather than claiming full capture. |
| Optional/future scope does not become required | [Export specification](concepts/export.md) | Separate approval; local acceptance does not depend on export. |
| Test-first, required VM, regression and compatibility | [Quality and acceptance](quality-and-acceptance.md) | Applies to every implementation task; historical PASS is not new-code evidence. |
| Unresolved decisions and dependent work | [Decisions and gaps](decisions-and-gaps.md) | Four named boundaries; roadmap links affected tasks. |
| Single status owner and next action | [Roadmap](roadmap.md) | Only executable status ledger; other files route to it. |

No future verification in this matrix has been run by the documentation migration. Append actual implementation evidence to the corresponding roadmap entries, not as a second mutable status column here.
