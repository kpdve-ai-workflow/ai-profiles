# Observation after process handoff

**Roadmap coverage:** HANDOFF-OBSERVE. **Source:** [process handoff requirements](../logger.md#procesy-i-przekazanie-sterowania). Requirements: REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH, REQ-BOUNDARIES.

## Outcome and boundary

Observe relevant process diagnostics and actual termination after the Rust program currently calls exec. Existing pre-exec events are not sufficient evidence of later success/failure.

Known boundaries: [mcp/runtime.rs](../../../core/src/mcp/runtime.rs) `launch`; [playwright/runtime.rs](../../../core/src/playwright/runtime.rs) `main`; [logging/entry.rs](../../../core/src/logging/entry.rs) `handoff`. The Novamira guard already has a different owned-child lifecycle and must not be conflated with exec.

## Required contract before implementation

Resolve [HANDOFF-CONTRACT](../decisions-and-gaps.md#observation-after-exec). Select one concrete observer/collector/supervisor approach and establish ownership of lifetime, stdio, signals/process groups, exit/signal reporting, timeout/cancellation and container cleanup. A list of possible approaches is not an accepted implementation design.

Keep executable/project/client identity and a correlation identifier available on both observed sides. An external observer must not keep protocol pipes open accidentally, delay EOF, steal interactive input, change exit status, or kill unrelated processes/containers. Diagnostic failure cannot take over the business result.

## Acceptance

Use real controlled processes in the required VM, with independently known protocol bytes, exit code/signal, EOF and process cleanup. Verify ordinary success/failure, child-start failure, termination/cancellation, parent/child closure and relevant descendant/container ownership. Include the actual Playwright integration if its lifecycle changes. Do not substitute a mocked spawn call for evidence that signals and cleanup work.

Detailed fixture locations and commands must be fixed when the observation design is resolved; do not treat this deferred specification as an executable zero-discretion handoff. Until then, the task is not eligible. The foundation's accepted handoff/flush remains intact.
