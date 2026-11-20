# Working knowledge and requirements

This is the bounded working reference for this objective. The archived input is linked for provenance, not required reading for routine tasks. Current progress belongs exclusively to [roadmap.md](roadmap.md).

## Requirements

| ID | Binding requirement | Provenance |
| --- | --- | --- |
| REQ-FLOW | Record successful and failing operation flow, decisions, skipped work, retries when they actually occur, timings, results, and partial retained state. | [Original diagnostic contract](logger.md#kontrakt-diagnostyczny); explicit owner request for complete development diagnostics. |
| REQ-DATA | Record complete operation-related arguments, environment, consumed dotenv/configuration, streams, SQL/dumps, MCP and accessible HTTP data. No default masking, silent truncation, or lossy conversion. Do not scan unrelated files. | [Original contract](logger.md#kontrakt-diagnostyczny). |
| REQ-CORRELATION | Correlate events and payloads with the run, operation, client/project when known, stage, stable event/error classification, and causal context. Preserve available error chains before conversion to text. | [Original contract](logger.md#kontrakt-diagnostyczny). |
| REQ-LOCAL | Store data only in the recognized checkout/project's ignored `.ai-profiles-logs/`. No implicit HOME/XDG/temp/CWD fallback or network export. | [Accepted storage decisions](logger.md#wiążące-decyzje-etapu-1-2026-10-25-bieżące-zadanie). |
| REQ-SEMANTICS | Do not change application bytes, stdout/protocols, user messages, codes, cancellation, deadlines, or cleanup for logging. Diagnostic failure does not replace an operation's result. | [Diagnostic contract](logger.md#kontrakt-diagnostyczny) and [handoff scope](logger.md#procesy-i-przekazanie-sterowania). |
| REQ-HEALTH | Distinguish operation outcome from diagnostic completeness. Report missing data and writer failures without pretending capture succeeded. | [Storage/completeness contract](logger.md#lokalizacja-retencja-i-kompletność). |
| REQ-BOUNDARIES | Wrapper visibility does not imply access to internal bridge HTTP or execution after exec. Close those gaps through explicit integrations, not invented observations. | [Novamira](logger.md#novamira-i-granice-obserwowalności) and [handoff](logger.md#procesy-i-przekazanie-sterowania). |
| REQ-QUALITY | Follow repository test-first and VM policy; preserve Rust 1.85 compatibility. Completion requires actual required verification. | [Repository testing policy](../../docs/testing.md), [Cargo manifest](../../core/Cargo.toml). |

## Accepted foundation contracts

- Manager location follows `RepositoryContext::discover`; installed tools follow their existing executable-relative project/client identity. Neither requires `.git`. A worktree is a separate local project.
- Shared settings: `AI_PROFILE_LOG_SEGMENT_BYTES=8388608`, `AI_PROFILE_LOG_SESSION_BYTES=1073741824`, `AI_PROFILE_LOG_RETAIN_BYTES=2147483648`, `AI_PROFILE_LOG_RETAIN_SECONDS=604800`. Values must be positive decimal integers. Invalid settings make diagnostics unavailable, not the operation invalid. TRACE is enabled.
- Each invocation owns a private `run-<pid>-<nanos>-<sequence>` package with format-v1 marker, activity lock, and `events-<index>.jsonl` segments. Atomic creation prevents overwrites.
- Rotate whole records; keep an oversized single record intact. The session budget includes events and payloads. Exceeding it explicitly degrades diagnostics.
- Payload storage accepts `Read`, uses a 64 KiB buffer and fragments no larger than 1 MiB, and records payload identity, indexes, offsets, lengths, SHA-256, and complete/incomplete outcome. This API stores bytes; it does not automatically intercept operations.
- Retention removes recognized inactive packages by age/target bytes while protecting active sessions and unknown resources. Retiring a package precedes deletion; interrupted cleanup can resume. Global target bytes are not a hard cap on concurrent active sessions.
- New directories/files use 0700/0600 and descriptor-relative nofollow. An existing public diagnostic directory is rejected rather than taken over. Legacy flat JSONL files are not automatically deleted.
- The root `.gitignore` rule is append-only and remains after uninstall. Diagnostic initialization ensures the rule before writing data.
- Diagnostic degradation is sticky. Use one stderr warning when that channel is not aliased to stdout, plus a best-effort private incomplete marker. Unavailable reporting channels do not imply completeness.
- `exec()` currently remains an execution handoff, with an event and synchronous flush. Do not emit a fabricated completion after successful exec. Flush does not guarantee fsync/crash durability.

## Current implementation boundaries

[Session storage](../../core/src/logging/session.rs), [entry integration](../../core/src/logging/entry.rs), and entry-point lifecycle are existing reusable infrastructure. Manager invocation context already preserves argv/environment/path bytes using explicit UTF-8 or hex representation.

The [shared runner](../../core/src/process.rs) still returns nonzero child exit statuses as `Ok(CommandOutput)`. Its `CaptureLimited(n)` retains at most `n + 1` bytes, `Null` discards a stream, and inherited stdin is not an application-owned byte buffer. Owner decision for STREAM-BOUNDARY: **process behavior remains unchanged**. PROCESS-DATA observes successful existing reads and writes only; it makes no claim to bytes outside that boundary. stdout/stderr full observation requires actual EOF; reaching CaptureLimited's existing `take(n + 1)` cap is `limited`, with neither known whole-stream length nor certain truncation. Null stdout/stderr and inherited stdin are explicitly unavailable; Null stdin is not applicable. stdin bytes submitted by the caller are distinct from bytes accepted by the pipe. No additional reads/writes/retries/proxies/joins, deadline or cleanup changes are allowed. This exception only limits stream observation, not argument completeness or available configuration data. See [the resolved boundary](decisions-and-gaps.md#stream-observation-boundary) and [process-data contract](concepts/processes.md#process-data).

Do not add a new retry, rollback, transaction, network dependency, Tokio runtime, or alternative logging framework merely to emit events.
