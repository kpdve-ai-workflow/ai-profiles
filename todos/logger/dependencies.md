# Dependencies and eligible work

Stage/task status and the next action are owned by [roadmap.md](roadmap.md). The graph below describes technical prerequisites. The roadmap separately owns the delivery stages and their completion/transition gates; a missing code-level dependency does not authorize bypassing an incomplete stage.

## Hard prerequisites

- Foundation APIs are prerequisites for all new local diagnostic producers.
- PROCESS-DATA requires the lifecycle identifiers introduced by PROCESS-LIFECYCLE and resolution of STREAM-BOUNDARY.
- NOVAMIRA-UPSTREAM requires the wrapper correlation contract and a selected, verified diagnostic channel for the pinned bridge/container.
- HANDOFF-OBSERVE requires a settled protocol/signal/cleanup observation contract. It does not automatically depend on completing every manager command.
- EXPORT-OPTIONAL requires explicit owner authorization. Approval is not implied by finishing local logging.

```text
foundation ──> runner lifecycle ──> process data <── stream-boundary decision
     ├───────> profile lifecycle / manager auxiliary commands
     ├───────> restore flow
     ├───────> MCP preparation
     ├───────> Novamira wrapper ──> bridge capture <── channel decision
     └───────> post-exec observation <── handoff-contract decision

export approval ──> optional collector/backend integration
```

## Interface dependencies and soft ordering

- Runner events make native activation and DDEV traces more useful; implementing the runner lifecycle before those consumers is a preference, not a reason to block their independent operation events.
- Operation-level events can be implemented without process-stream capture, but the overall objective is not fully covered until required data producers are connected.
- MCP preparation should precede detailed Novamira tracing so configuration and guard events use consistent context. The wrapper still needs its own observed data, not merely preparation events.
- Capture workers must preserve tracing dispatch and session activity across threads. A synchronous writer does not automatically propagate the caller's thread-local context.
- Correlation across exec is an interface requirement, not evidence that execution after handoff is observed today.

## Unresolved boundaries

See [decisions and gaps](decisions-and-gaps.md). A blocked capture boundary must be resolved before its dependent implementation; do not silently weaken complete-data requirements or change operational stream modes to obtain logs.

Independent tasks within the current stage may proceed only when selected by the owner/active roadmap. Beginning implementation in a later stage before acceptance of the current one requires explicit owner approval recorded in the roadmap. This does not authorize parallel editing agents or concurrent installers.
