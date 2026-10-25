# Shared process diagnostics

**Roadmap coverage:** PROCESS-LIFECYCLE and PROCESS-DATA. **Source:** [diagnostic contract](../logger.md#kontrakt-diagnostyczny), [process boundaries](../logger.md#procesy-i-przekazanie-sterowania). Requirements: REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH.

Primary touchpoints: [process.rs](../../../core/rust/src/process.rs), `SystemProcessRunner::run`, `CommandSpec`, `CommandOutput`, reader/writer/wait helpers; [error.rs](../../../core/rust/src/error.rs), `ProcessError`; existing session/entry APIs. Test modules must be explicitly registered in [lib.rs](../../../core/rust/src/lib.rs).

## Runner lifecycle

Planner-selected contract for the next bounded task:

- Emit `process.started` for every runner attempt, including cancellation before spawn. Allocate a diagnostic `process_id` unique within the run; it is not the OS child PID.
- Emit `process.spawned` only after successful spawn, including the actual child PID.
- Emit exactly one terminal `process.finished` for `Ok(CommandOutput)` or `process.failed` for an existing `ProcessError` return. Correlate all events through the run and process identity, including concurrent attempts.
- `finished` records elapsed duration and the actual exit code or terminating signal when available. Nonzero child status remains `Ok(CommandOutput)`; do not reinterpret it as a runner error.
- `failed` records duration, the observed stage, available causal details, and stable classification: `process.spawn`, `process.timeout`, `process.cancelled`, `process.io`, or `process.stream_thread`, matching existing variants. Do not log a normal finish after failure.
- Preparation/capture failure in diagnostics does not change the runner result. Preserve its cancellation, timeout, kill/wait and reader-join contracts.
- Do not add full argv/environment/stdin/stdout/stderr capture in this task, change stream modes, or retrofit exec call sites. That is separate work.

### Test-first contract

Add `tests/process_logging.rs` and register it as `process_logging_tests`. Oracle: these event rules, controlled real-child outcomes, and existing runner semantics, not a snapshot of emitted current output.

Select cases for successful exit, known nonzero exit, missing executable, cancellation before spawn, controlled in-flight cancellation/timeout, and concurrent attempts. Assert correlation, event order/cardinality, actual returned status/error, available PID and cause, and no spawn event for pre-cancel/spawn failure. Use bounded fixture processes and synchronization; do not rely on arbitrary sleeps to manufacture races. Test that unavailable diagnostics leaves the returned result unchanged.

Expected RED: a correctly exercised runner produces no required lifecycle records before implementation. GREEN: the new observations match the independently controlled process outcome and existing semantics still hold. Guest-side focused command: `cargo test --manifest-path core/rust/Cargo.toml --locked --lib process_logging -- --nocapture`. Follow the [shared acceptance rules](../quality-and-acceptance.md) for the required final gate.

## Process data

Capture exact command argument boundaries, effective child environment (honoring inherited values, explicit overrides, and `clear_env`), working directory, and the actual observable stream bytes. Use the foundation payload API rather than buffering arbitrary diagnostic copies. Retain binary/non-UTF-8 values without masking or lossy display-only encoding.

Before implementation, resolve [STREAM-BOUNDARY](../decisions-and-gaps.md#stream-observation-boundary). The specification must not equate a caller's limited retained output with the complete child stream, fabricate inherited input, or redirect null streams without proving unchanged behavior. Worker logging must retain the active dispatch/session and process identity. Partial writes, cancellation, timeout and diagnostic failure require truthful payload completeness; submitted stdin is not proof that the child received every byte.

Extend `tests/process_logging.rs` after that contract is resolved. Independent fixtures must cover environment precedence, exact stdin/output bytes, simultaneous stdout/stderr, binary/large data, thread correlation, operational capture limits, timeout/descendant-held pipes, and diagnostic I/O failure. Verify both reconstructed payloads and the unchanged caller-visible results. A fake runner cannot prove actual pipe or process behavior.
