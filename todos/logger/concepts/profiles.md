# Profile-operation diagnostics

**Roadmap coverage:** PROFILE-LIFECYCLE and MANAGER-AUX. **Source:** [flow and partial-outcome requirements](../logger.md#kontrakt-diagnostyczny). Requirements: REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH.

## Client lifecycle

Touchpoints: [scoped.rs](../../../core/rust/src/profile_management/scoped.rs), `run_with_runner`, `prepare`, `Prepared::commit`, `Manifest::save`, `preflight_remove`, `remove`; [native.rs](../../../core/rust/src/profile_management/native.rs), activation/configuration helpers; [payload.rs](../../../core/rust/src/profile_management/payload.rs), payload preparation; production helpers in [installation.rs](../../../core/rust/src/profile_management/installation.rs).

Instrument install/update/switch/uninstall as operations with per-client and per-stage context. Record selected/effective profile, previous state, preparation, decisions/skip reasons, planned and actual file/configuration changes, activation status, persisted state and final outcome. Store operation-related raw content through the existing payload API where needed; do not scan unrelated files or duplicate the whole repository into logs.

Preserve the existing read-only preparation and sequential commit boundaries. A preparation conflict before commit must not be reported as a write; a successful filesystem call is not proof that the final manifest was saved. Distinguish `prepared`, `pending`, `synchronized`, and `failed`. Report retained state after partial failure, never an invented cross-client rollback. Capture available errors before aggregation reduces them to client names/text.

Extend [lifecycle.rs](../../../core/rust/tests/lifecycle.rs) with scoped diagnostic capture or a registered concept-specific test module. Independent oracles: the owner's flow contract, fixture filesystem/state, and controlled activation results. Cases: success, same-profile skip/no-op, preflight conflict with no commits, activation pending, later-client failure preserving earlier work, and removal failure. Assert both truthful logs and actual persisted state; preserve user-facing output assertions. Inject native runners only where real CLI execution is not the behavior under test.

## Other manager commands

Touchpoints: [status.rs](../../../core/rust/src/profile_management/status.rs) `status`; [validation.rs](../../../core/rust/src/profile_management/validation.rs) `validate_sources`; [migration.rs](../../../core/rust/src/profile_management/migration.rs) `migrate_legacy`; [profiles.rs](../../../core/rust/src/profile_management/profiles.rs) create/rename/bump; [cli.rs](../../../core/rust/src/profile_management/cli.rs) dispatch and `operation_error`.

Record inputs actually used, checks/decisions, transformations, skips, writes and results for these commands. Preserve available causal errors before final CLI flattening without replacing the domain's error types with a generic string logger error. Status confirms prepared configuration, not a live authenticated MCP connection. Read-only commands must not become mutating operations through instrumentation.

Extend [profiles.rs tests](../../../core/rust/tests/profiles.rs), [CLI tests](../../../core/rust/tests/cli.rs), and appropriate existing lifecycle/config tests. Cases: successful read-only status/validation, invalid input/configuration, guarded migration, create conflict, rename failure/recovery path and version change. Observe fixture files and exact CLI behavior as well as events.

## Shared acceptance

Use one operation identifier under the existing run, with client/stage context on events and payload references. Log an error at its actual handling boundary; distinct process, client and final summary events are allowed, but repeated propagation is not a new failure. Follow [quality rules](../quality-and-acceptance.md); select the focused registered test module for RED/GREEN, then the required Rust gate. These tasks do not instrument restore, MCP runtime internals, post-exec observation or optional export.
