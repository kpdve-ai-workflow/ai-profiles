# Foundation and diagnostic storage

**Roadmap coverage:** FOUNDATION-BASE. **Source:** [accepted storage decisions](../logger.md#wiążące-decyzje-etapu-1-2026-10-25-bieżące-zadanie), [recorded scope and evidence](../logger.md#domknięcie-etapu-1--2026-10-25). Current status is only in [Stage 1 of the roadmap](../roadmap.md#etap-1-fundament-loggera).

## Responsibility and interfaces

Provide local storage, configuration, session identity, payload persistence, rotation, retention and delivery health to diagnostic producers. The accepted settings and exception behavior are in the [working knowledge base](../knowledge-base.md#accepted-foundation-contracts).

Existing implementation touchpoints:

- [logging.rs](../../../core/rust/src/logging.rs): shared logging module.
- [session.rs](../../../core/rust/src/logging/session.rs): `Config::from_env`, `Session::open`, `Session::payload`, `complete`, `flush`, `subscriber`.
- [entry.rs](../../../core/rust/src/logging/entry.rs): shared location/Git-ignore preparation, `with_diagnostics`, `diagnostics_complete`, `handoff`, `runtime_entry`.
- [bin entry points](../../../core/rust/src/bin): manager, MCP adapter, Playwright launcher, restore and test inventory.

## Boundaries and failure behavior

This infrastructure stores what producers supply. It does not intercept subprocesses, dumps, HTTP or MCP merely by being initialized. A diagnostic session remains distinct from the outcome of the application operation. Active packages must remain protected during retention and while payloads/subscribers retain them.

Do not restore old flat-JSONL-only assumptions, reopen storage location, or change accepted limits as a side effect of producer instrumentation. A required infrastructure correction needs its own explicit rationale and tests.

## Acceptance and evidence ownership

Evidence pointers and accepted platform belong to FOUNDATION-BASE in the roadmap. Existing coverage includes [logging_session.rs](../../../core/rust/tests/logging_session.rs), [runtime_logging.rs](../../../core/rust/tests/runtime_logging.rs), and [manager_logging.rs](../../../core/rust/tests/manager_logging.rs).

Regression-sensitive invariants: exact binary reconstruction, complete records across rotation, explicit limits/I/O failure, safe interrupted retention, no active/unknown-resource deletion, no external fallback, and unchanged stdout/protocol/handoff behavior. Later tasks must reuse these contracts rather than reassert completion solely from an existing API.
