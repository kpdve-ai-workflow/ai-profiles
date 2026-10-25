# Decisions, provenance, and open boundaries

This file owns decision rationale and unresolved questions, not task progress. See [roadmap.md](roadmap.md) for affected task statuses.

## Accepted and structural decisions

| Decision | Origin | Consequence |
| --- | --- | --- |
| Capture complete operation-related diagnostic data without default masking; keep it local and ignored by Git. | USER-APPROVED in the conversation; [source contract](logger.md#kontrakt-diagnostyczny). | Do not silently substitute sanitized summaries for payloads. |
| Preserve operation semantics and report diagnostic degradation separately. | APPROVED SOURCE; [accepted infrastructure decisions](logger.md#wiążące-decyzje-etapu-1-2026-10-25-bieżące-zadanie). | Capture design cannot change stream/protocol behavior without resolving the conflict. |
| Keep the existing synchronous session/payload infrastructure and settings. | Accepted foundation implementation and recorded verification. | Do not reopen a queue/Tokio/framework redesign while adding producers. |
| Move the original file unchanged, freeze it, and maintain current progress in the roadmap. | USER-APPROVED in the current request. | Old chronology is provenance, not a competing live plan. |
| Use numbered stages 1–5 as the main roadmap headings, with smaller tasks and completion/transition gates beneath them; retain stable task IDs and supporting concept specifications. | USER-APPROVED in the latest correction; supersedes the initial planner-selected concept-only execution layout. | Stage ownership and the current delivery boundary must be visible directly, not inferred from a compatibility table. |
| Use six bounded operation-instrumentation tasks. | PLANNER-PROPOSED in the preceding conversation, retained in response to the owner's request for smaller tasks. | Each task has its own result and acceptance, not a single all-core implementation invocation. |

## Stream observation boundary

**Gap ID: STREAM-BOUNDARY.** How will complete diagnostic capture coexist with `CaptureLimited`, `Null`, inherited stdin, and timeout paths without changing application semantics?

Evidence: [process.rs](../../core/src/process.rs) drops `Null` streams, reads at most `limit + 1` bytes for limited capture, inherits stdin in one mode, and deliberately avoids joining readers after timeout because descendants may retain pipes. Simply teeing all streams or continuing to drain can change pipe closure, SIGPIPE, waiting, or deadlines.

Affected task: PROCESS-DATA and any acceptance claim of complete shared-runner stream coverage. The implementer must not choose to remove limits, redirect inherited input, or wait indefinitely merely for logging. A bounded observation design must specify which bytes can be captured, how unavailable bytes are reported, and demonstrate semantic preservation. If this requires changing approved application behavior or reducing required capture, owner approval is required. No such alternative has been accepted in this migration.

## Novamira diagnostic channel

**Gap ID: NOVAMIRA-CHANNEL.** Which real channel in the pinned bridge/container exposes request/response bodies and diagnostics without contaminating MCP stdout?

Evidence: [novamira.rs](../../core/src/mcp/novamira.rs) currently removes `LOG_FILE`, disables upstream logging, and discards upstream stderr. The wrapper sees only its own protocol boundary; no inspected proof establishes a complete HTTP capture interface inside the bridge.

Affected task: NOVAMIRA-UPSTREAM. Unblock with a documented, verified integration contract for the actual pinned artifact/container. It must identify source ownership, byte fidelity, correlation, channel transport, failure behavior, and allowed required changes. Do not introduce host Node/npm or claim a synthetic bridge proves the real integration. Escalate a need to change the pinned upstream artifact or approved deployment contract to the owner.

## Observation after exec

**Gap ID: HANDOFF-CONTRACT.** What selected observation design preserves MCP/interactive stdio, process signals, exit status, deadlines, and Docker cleanup after the current exec boundary?

Evidence: [MCP runtime](../../core/src/mcp/runtime.rs) and [Playwright runtime](../../core/src/playwright/runtime.rs) replace themselves with the child program. Existing handoff/flush events cannot observe its later exit. The source suggests a supervisor or independent collector but does not accept one concrete design.

Affected task: HANDOFF-OBSERVE. Unblock with an approved/authorized concrete design and real-process acceptance contract. Do not silently convert exec to spawn/wait. Independent preparation/wrapper logging does not require this decision.

## Export authorization

**Gap ID: EXPORT-APPROVAL.** Is any export authorized, to which local/remote destination, and with what access/data contract?

The owner currently requires local-only logging. Future Grafana/Loki is an optional idea, not approval to transmit unmasked payloads. Only explicit owner authorization can activate EXPORT-OPTIONAL. Local objective acceptance does not depend on this decision.

## Historical inconsistencies

- The initial concept-only execution layout and its anti-stage skill rule were explicitly superseded by the owner's request for visible numbered stage headings. The scope, 11 stable task IDs, evidence and canonical task statuses are preserved.
- Older “foundation incomplete” statements were superseded by the later acceptance record, not deleted from the archive.
- Older possible asynchronous-writer designs are not active decisions; the accepted foundation is synchronous.
- The unrelated `todos/observability.md` proposal is not an authority for this logger's storage or masking policy.
