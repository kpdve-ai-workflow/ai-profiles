# Shared process diagnostics

**Roadmap coverage:** PROCESS-LIFECYCLE and PROCESS-DATA. **Source:** [diagnostic contract](../logger.md#kontrakt-diagnostyczny), [process boundaries](../logger.md#procesy-i-przekazanie-sterowania). Requirements: REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH.

Primary touchpoints: [process.rs](../../../core/src/process.rs), `SystemProcessRunner::run`, `CommandSpec`, `CommandOutput`, reader/writer/wait helpers; [error.rs](../../../core/src/error.rs), `ProcessError`; existing session/entry APIs. Test modules must be explicitly registered in [lib.rs](../../../core/src/lib.rs).

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

Expected RED: a correctly exercised runner produces no required lifecycle records before implementation. GREEN: the new observations match the independently controlled process outcome and existing semantics still hold. Guest-side focused command: `cargo test --manifest-path core/Cargo.toml --locked --lib process_logging -- --nocapture`. Follow the [shared acceptance rules](../quality-and-acceptance.md) for the required final gate.

## Process data

Owner decision for STREAM-BOUNDARY: **process behavior remains unchanged**. Observe all bytes returned by existing reads and accepted by existing writes. Explicitly mark data outside those operations as not captured. This stream-boundary exception does not reduce argument completeness or available configuration data.

Emit schema-version-1 `process.context.started`, `process.context.program`, one indexed `process.context.arg` per argument, one `process.context.env` per effective environment key/value, `process.context.cwd`, and `process.context.finished`, with explicit argument and environment counts. Encode byte values as UTF-8 when valid, otherwise hex; include encoding and byte length, preserving empty, binary, and non-UTF-8 values without lossy conversion, masking, or default truncation. Snapshot effective environment immediately before spawn preparation: `spec.env` only when `clear_env`, otherwise inherited `vars_os` with `spec.env` overrides. Label it `snapshot_at_spawn_preparation`; it is not a readback of the child environment. Keep `Command.env_clear/envs` behavior unchanged. Report configured cwd and available inherited context; do not canonicalize or introduce filesystem errors. If diagnostic `current_dir` is unavailable, say so explicitly. Cancellation before spawn preparation reports `not_prepared` and performs no log-only process preparation.

Use Read/Write observers inside existing reader/writer threads. Read observes successful reads; Write observes only bytes accepted by successful writes. Preserve operational I/O errors and do not add reads, writes, or retries. Send borrowed pieces of at most 64 KiB to `Session::payload`; do not buffer a second full stream. A payload fragment is correlated by process, stream, scope, fragment index, and observed-byte offset. Never give the operational pipe itself to the payload API: diagnostic failure must not stop application I/O. Offset advances by bytes observed even when storage fails.

Stream observations have explicit states: `complete`, `limited`, `unavailable`, `partial`, `not_started`, `not_applicable`. Capture stdout/stderr is complete only after actual EOF. CaptureLimited keeps the existing `take(n + 1)` and places observation below Take: stopping at that cap is `limited`, including possible exact `n + 1`, and does not establish total length or certain truncation. Null stays `Stdio::null` and is unavailable (`redirected_to_null`). Inherited stdin stays inherited and is unavailable (`inherited_stdin`). Null stdin is `not_applicable` (`no_input_configured`). Bytes stdin has separate `requested_input` data and `written_to_pipe` observations; successful writes do not prove child reads. Transfers are `not_started` after failed spawn or earlier cancellation.

Emit `process.data.chunk` with process ID, stream, scope, index, offset, observed length, payload ID, stored length, and payload completeness. After observer work emit `process.stream.closed` with scope, state, reason, and counters. Before the lifecycle terminal event, emit a `process.stream.snapshot` for every stream at `boundary=runner_return`. Keep state in a short-lock shared structure and never hold its lock during diagnostic I/O. At timeout/cancellation do not add joins; unfinished observers are `partial` in the return-time snapshot, and may later emit chunks/closed. Storage failure uses existing diagnostic incompleteness reporting; it does not change process I/O or result and does not create warnings for Null/Inherit/limit boundaries. Preserve `process_id` and correlate payload IDs to the right run and process under concurrency.

Extend `tests/process_logging.rs` test-first. Independent fixtures cover exact args, environment precedence and clear-env through real children, stdin/output, simultaneous stdout/stderr, binary/large payload reconstruction, capture limits, Null/inherited streams, writer partial/error behavior, timeout/descendant-held pipes, concurrent correlation, absent sessions and diagnostic storage failure. Use a controlled test Write for partial-write semantics, not as proof of process integration; use real child processes for pipes and child behavior.
