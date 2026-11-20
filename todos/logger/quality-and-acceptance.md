# Quality and acceptance

The governing [repository testing policy](../../docs/testing.md) takes precedence over this derived summary. Planning and moving documents do not authorize application changes or test execution.

## Test-first delivery

For each selected behavior-changing task:

1. Read its specification and derive cases from the requirement and an independent oracle, not current output.
2. Add the narrowly relevant tests, register modules explicitly in [lib.rs](../../core/src/lib.rs), and obtain justified RED in the required VM before production changes.
3. Implement the bounded change; obtain GREEN and required regression evidence.
4. If behavior already exists and a test starts GREEN, check its ability to detect a credible regression; do not break working code to force RED.
5. Stop dependent work for unavailable verification, an ambiguous oracle, or an unresolved contradiction. Preserve changes and report the blocker.

Use `test-gap-discovery`. Never weaken assertions, add skips, replace the integration under test with a mock, or run Rust/Playwright tests on the host to obtain acceptance.

## Execution and evidence

- Targeted Cargo commands in concept specs are **guest-side commands only**, executed as the test user through the existing VM mechanisms/lock/isolation policy. They are not host commands or new Make targets.
- Use one full required gate at the end of a coherent task, not after every edit. Do not additionally run `make test` when the selected gate already includes it.
- Rust behavior: `make validate-rust-migration`. Playwright integration/image: `make test-playwright`; combined work: `make test-all`. VM orchestration changes additionally require the documented host check.
- Preserve Rust 1.85 compatibility. At overall objective acceptance, verify actual `rustc`/Cargo 1.85.0 with `cargo build --manifest-path core/Cargo.toml --locked --release --bins` and `cargo test --manifest-path core/Cargo.toml --locked --all-targets` in the VM. Earlier evidence cannot be claimed for changed code. Tasks that change dependencies or use compiler-sensitive APIs need appropriate MSRV evidence before acceptance.
- Fixtures must use disposable checkout/project layouts and synthetic HOME/env/data. Do not write test logs into the working checkout, real HOME, or real databases; do not export diagnostic payloads containing user data as test reports.
- Record exact commands, outcomes, compiler versions and artifact paths on the task's entry in `roadmap.md`. `IMPLEMENTED` is not equivalent to verified completion.

## Shared behavioral acceptance

| Obligation | Discriminating observation |
| --- | --- |
| Lossless data | Reconstruct controlled bytes, including empty/binary/non-UTF-8 data and fragment boundaries; compare to independent fixtures and checksums. |
| Truthful flow | Match emitted starts/results/decisions to actual fixture operations and persisted state; no fabricated completion after exec or rollback after partial failure. |
| Correlation | Each attempt/operation/payload has stable identity and correct run/client/project association, including worker-thread events. |
| Semantic preservation | Compare observed bytes, exit/return values, stdout, cancellation/deadline behavior and cleanup with the governing application contract. |
| Failure isolation | Inject diagnostic failure separately from operation failure; the original result survives and missing diagnostics are explicit when reporting is possible. |
| Locality | Files stay under the recognized ignored local directory; no implicit external fallback or network export. |

Detailed negative/boundary cases belong to the concept owner, not a second global test backlog. No application tests are required solely to accept the documentation move and roadmap creation; validate content, source preservation, links and scope instead.
