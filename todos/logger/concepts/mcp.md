# MCP preparation diagnostics

**Roadmap coverage:** MCP-PREPARE. **Source:** [diagnostic contract](../logger.md#kontrakt-diagnostyczny), [visibility boundaries](../logger.md#novamira-i-granice-obserwowalności). Requirements: REQ-FLOW, REQ-DATA, REQ-CORRELATION, REQ-SEMANTICS, REQ-HEALTH, REQ-BOUNDARIES.

## Scope and code

Observe local configuration and launch preparation up to the existing guard/exec boundary:

- [mcp/runtime.rs](../../../core/src/mcp/runtime.rs): `launch`, identity, selected server, executable/cwd and `apply_environment`.
- [mcp/config.rs](../../../core/src/mcp/config.rs): consumed state/local overrides, merge/resolve and effective configuration.
- [mcp/environment.rs](../../../core/src/mcp/environment.rs): `dotenv`, `environment_for`, substitution and validation.
- [playwright/runtime.rs](../../../core/src/playwright/runtime.rs): launch planning and the existing handoff.

## Contract

Record selected client/project/server, consumed configuration, override/merge decisions, disabled/rejected states, actual dotenv bytes, environment precedence, effective command/environment/cwd, launch-plan output and preparation errors. Preserve available causes before returning a user-facing summary. Use binary payload storage for full contents; do not load a file solely because its name resembles `.env`.

The diagnostic observer must not evaluate dotenv as a shell, change interpolation/precedence, broaden accepted configuration, or mutate the parent environment. Respect executable-relative identity and reserved adapter metadata. Local preparation success is not proof of an MCP connection, authentication or server readiness.

Reuse the foundation's handoff/flush and entry-point lifecycle. Do not replace exec, forward diagnostic text on protocol stdout, or claim the later server process was observed. Novamira guard/bridge internals have their own responsibility.

## Test-first acceptance

Extend [mcp_config.rs tests](../../../core/tests/mcp_config.rs), [mcp_runtime.rs tests](../../../core/tests/mcp_runtime.rs), [playwright.rs tests](../../../core/tests/playwright.rs), and [runtime_logging.rs](../../../core/tests/runtime_logging.rs) as appropriate.

Independent fixtures specify exact input configuration/dotenv, expected precedence, child environment and returned errors. Cover a valid launch plan, overrides, disabled server, missing/malformed env, reserved metadata, wrong location and diagnostic failure. Assert original protocol/stdout/codes and configuration semantics, plus exact captured bytes and absence of invented post-exec completion.

Use test-first/focused checks and the [gate appropriate to actual changed scope](../quality-and-acceptance.md). A Rust-only preparation change does not itself require rebuilding an unchanged Playwright image; an image/integration change does require its proper integration gate.
