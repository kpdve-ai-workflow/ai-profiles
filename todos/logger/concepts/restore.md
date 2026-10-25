# WordPress restore diagnostics

**Roadmap coverage:** RESTORE-FLOW. **Source:** [complete diagnostic data/flow contract](../logger.md#kontrakt-diagnostyczny). Requirements: REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH.

## Outcome and touchpoints

Reconstruct plan/apply, backup, safety guards, import, localization, verification and interrupted/recovery state without changing database safety behavior.

Primary code: [restore.rs](../../../core/rust/src/wordpress/restore.rs) `restore`/`cli_main`; [preflight.rs](../../../core/rust/src/wordpress/preflight.rs) `preflight`; [dump.rs](../../../core/rust/src/wordpress/dump.rs) `inspect_dump`; [localize.rs](../../../core/rust/src/wordpress/localize.rs) `localize`; [ddev.rs](../../../core/rust/src/wordpress/ddev.rs) DDEV/WP-CLI boundary. Reuse process correlation/capture when available, not a second process logger.

## Contract

- Record actual plan/apply mode, mapping decisions, checks and their results, lock/confirmation/cancellation boundaries, backup preparation and verification, mutation steps and final verification.
- Capture consumed dump/SQL and relevant configuration/data through the existing payload API at actual use boundaries; do not read unrelated databases or dump files for logging.
- Distinguish planned work from performed writes. Record when a local backup exists and when database mutation has started.
- On failure, record what really happened and the available recovery information. There is no automatic rollback unless the operation actually performs one.
- Diagnostic failure must not bypass a confirmation, lock, preflight rejection or safety guard, and must not turn plan mode into apply mode.
- Original backup location/security is application behavior; do not move backups into the log directory or change it merely to align diagnostic storage.

## Test-first acceptance

Extend [wordpress_restore.rs](../../../core/rust/tests/wordpress_restore.rs), [wordpress_preflight.rs](../../../core/rust/tests/wordpress_preflight.rs), and relevant existing dump/localize tests. Use synthetic dumps/projects and isolated controlled runners, never a real user database.

Oracles: the fixture plan/mapping, actual temporary files, controlled DDEV outcomes, and the existing no-mutation-before-confirmation invariant. Cover plan-only, rejected preflight, cancelled confirmation, successful apply, failure before/after database mutation, and diagnostic degradation. Compare events/payloads to independent input bytes and real retained fixture state. Mocking DDEV orchestration is not proof of a real import; retain existing integration protection where applicable.

Select existing focused modules for RED/GREEN and finish with the [required Rust gate](../quality-and-acceptance.md). This responsibility does not include Novamira bridge HTTP, post-exec supervision, or a redesign of restore.
