---
name: test-gap-discovery
description: Discover meaningful behavioral gaps by deriving diverse use cases and test oracles independently of the existing implementation. Analyze a repository, prioritize risk-based scenarios, and author tests that can expose real defects rather than merely turn green. Works without a multi-agent harness.
---

# Test Gap Discovery

## Purpose

Build a clear, repository-specific **testing vision** and, when authorized, turn that vision into useful behavioral tests. A test should discriminate correct behavior from plausible wrong behavior. Do not optimize for the number of tests, the number of findings, or a green dashboard.

This skill is generic: infer the language, test framework, user-facing boundaries, existing conventions and applicable safety policy from the target repository. Do not assume a particular architecture, harness, runtime, vendor, CI or orchestration system.

## When to use

- Understand what deserves testing in an unfamiliar or existing repository (`vision`).
- Explore a module for missed cases and add gap-seeking tests (`gaps`).
- Challenge weak regression tests around a change (`review`).
- Investigate a reported bug by deriving independent reproduction scenarios (`gaps`).

Do not use for formatting-only work or to simply rerun a known suite.

## Critical working principles

1. **Behavior before branches.** Discover public contracts, intended use cases, user-visible effects and invariants before deriving expected outcomes from the implementation. Looking at project layout to locate those contracts is fine. After drafting scenarios, inspect code and current tests to arrange execution and identify overlooked risks; do not copy their answers as the expected result.
2. **Independent oracle.** Ground each meaningful assertion in a contract, a requirement, a reviewed example, a domain invariant, a standard, a verified previous release, or a well-justified metamorphic relation. Existing code and snapshots of its output are observations, not proof of correctness. See `references/test-oracles.md`.
3. **Uncertainty stays uncertain.** When the desired outcome is ambiguous, ask only if the decision is blocking; otherwise leave that scenario unasserted or use an explicitly justified invariant. Never invent product semantics to manufacture a defect.
4. **Challenge plausible faults.** Think in terms of input classes, state transitions, sequences, retries, failure modes, concurrency, boundary values, permissions and recovery. Use the applicable dimensions, not a fixed checklist quota. See `references/case-derivation.md`.
5. **Tests must discriminate.** A green-on-first-run test can be valuable: it may protect a valid behavior. Check whether its assertion would catch a realistic regression with a safe mutation, perturbation, independent reference, or counterexample *when appropriate and authorized*. A red-first run is useful for an actual bug or TDD, not mandatory for every good test.
6. **Never make tests green by changing the question.** Do not weaken or delete legitimate assertions, over-mock collaborators, skip failures or change requirements to match code. A failure must be diagnosed: product defect, incorrect expectation, faulty fixture, nondeterminism, or infrastructure issue.
7. **No forced findings.** Zero real gaps is a valid outcome. Do not manufacture issues, fill placeholder sections or pretend untested cases failed. Do not optimize to a numerical gap, coverage or mutation score target.
8. **No mandatory report.** Default output is the tests and a short conversational summary of significant, *verified* observations. Do not create audit documents, gap registries, backlogs, YAML contracts, `test-vision.md`, or report files unless explicitly requested. If the user asks only for a vision, give a focused, actionable in-chat direction. Do not list speculative problems simply to fill a template.
9. **Respect project policy.** Read AGENTS.md, local instructions and applicable permissions before running commands. If running/writing tests is prohibited or requires approval, do read-only analysis and stop at that boundary. Never treat the existence of this skill as authorization to test production, modify services, write product code or bypass gates.
10. **Isolate risky actions.** Run mutation/perturbation experiments only in a disposable copy or isolated worktree, with explicit permission, no production access and verified cleanup. Never destructively reset an existing checkout. Do not automatically invoke arbitrary package-manager install scripts.

## Workflow

### Mode `vision` — decide what should be tested

1. Locate the project root and its local agent policies. Inspect repository structure, documentation, interfaces, contracts, test setup and a *representative* sample of existing tests. Read implementation as needed to understand system boundaries, but do not use implementation behavior to invent the expected outcome.
2. Map important user-visible behaviors and domain invariants to the actual entry points and external dependencies. Distinguish testable obligations from guesses.
3. Notice patterns in current tests: assertion of real effects vs internal call counts, empty/single happy paths, excessive mocks, snapshots, lack of error-path validation, ordering assumptions and integration seams. Note only patterns genuinely observed in files.
4. Derive an exploratory set of scenarios before writing tests. Vary inputs, ordering, state, failures, retries, concurrency, limits, recovery, compatibility, and adversarial cases **where relevant**.
5. Prioritize tests with the best expected defect-detection value and actionable oracles. Choose the appropriate test level, fixtures, data isolation, concurrency controls and mocking boundary.
6. Give the user a concise, repo-specific direction: which behaviors to test first, the most valuable contrasting use cases, and what a good assertion should observe. Reference real project paths only when observed. Do not create a document unless asked.

### Mode `gaps` — implement and run gap-seeking tests

1. Agree a useful scope from the request. Respect permitted files, test execution policy and time budget. If running is not allowed, propose cases without executing or modifying anything.
2. Derive independent oracles and draft candidate cases **before copying expectations from code or tests**. Use `references/case-derivation.md`. Focus on combinations and state transitions likely to reveal missed cases; no minimum count.
3. Choose a high-value subset based on severity if broken, real-world plausibility, independence from the implementation and execution cost.
4. Inspect implementation, existing tests and fixtures to select the narrowest appropriate test seam. Favor observable state, returned values, durable effects, API contracts and integration boundaries over internal interactions.
5. Write targeted tests using repository conventions. Avoid asserting values calculated by the same implementation path, tests that only verify mocks, and excessive snapshot reliance. Use controlled clocks, deterministic seeds and isolated resources where relevant.
6. Run targeted tests *only where authorized*. For an unexpected failure, independently validate the oracle, input setup and observed behavior; reproduce it when practicable. Do not quietly “fix” the test or product. Do not change product code unless separately assigned.
7. For important new assertions, consider an optional **sensitivity check** (manual counterexample, mutation tool or isolated perturbation). A passing original test is not suspicious on its own. A mutation that survives is evidence to investigate, not automatically a product gap. See `references/techniques.md` and `scripts/mutation-check.sh`.
8. Keep reproducible tests that encode legitimate behavior. If a confirmed bug is exposed, retain the failure and communicate the minimal reproduction. If no bug is found, keep only tests worth maintaining. Leave excluded or unsupported scenarios as optional next steps, not invented findings.
9. Conclude briefly with what changed and what was *actually* run/observed, or state why verification was not possible. Do not emit a formal report.

### Mode `review` — challenge existing tests

- Compare a change's user-visible obligations against tests written for it.
- Ask what plausible incorrect implementations would still pass.
- Look for missing boundary/state/failure cases, shared logic between expected and actual, excessive mocking, and assertion-free checks.
- Improve or propose a few meaningful cases where authorized. Do not rewrite tests to match the code or force a finding.

## Decision rules and test quality

- Prefer scenario coverage and independent oracles over line coverage or test quantity.
- A new test **need not fail on the current version** to be worthwhile. It should be capable of detecting an incorrect behavior and not merely mirror an implementation detail.
- For true defects, the expected/actual discrepancy must be grounded and reproducible; do not label an unverified hypothesis as a bug.
- A failing test may be invalid. Investigate before escalating.
- For race/timeout/resilience scenarios, provide deterministic synchronization or bounded simulation; avoid flaky sleeps or unsafe infrastructure experiments.
- A mock is acceptable for irrelevant external effects, but not as a substitute for the very integration boundary under investigation.
- Mutations and coverage statistics are optional diagnostics, not requirements or success metrics.
- Default to small, meaningful changes. Preserve existing unrelated files and avoid making a new test framework solely for this skill.

## Supporting material

Read only the references relevant to the task:

- `references/test-oracles.md` — independent expectations and ambiguity.
- `references/case-derivation.md` — deriving contrasting use cases and test selection.
- `references/techniques.md` — property, metamorphic, differential, state-machine, fuzz and mutation techniques.
- `references/anti-patterns.md` — traps of solution-mirroring tests.
- `references/gap-report.md` — optional, **only when an explicit formal artifact is requested**.
- `references/test-vision-template.md` — optional, **only when an explicit vision document is requested**.
- `scripts/inventory-tests.sh [repo]` — safe, heuristic inventory of test files and hints.
- `scripts/mutation-check.sh [repo]` — safe inspection of possible mutation tools; does not run or mutate anything.
- `scripts/coverage-map.sh [repo]` — safe, read-only coverage tool discovery; does not run tests.

## Environment and fallback

Works as a single agent using read/edit/shell tools. No Herdr, Pi worker launcher, special `project_*` commands, separate reviewer, working CI or mutation tool is required. If a local policy forbids test execution, remain in `vision` or read-only `review` until separately authorized. For a complete agent-workflow integration, follow the repository's real skill discovery paths, not an assumed target architecture.
