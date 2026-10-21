---
name: make-plan
description: Create or refactor evidence-grounded technical implementation plans from approved requirements. Use when a user needs clear ownership of decisions, verified findings, concrete technical contracts, task status, dependencies, and an executable next step, without mixing research notes with user choices or confusing decisions with tasks.
---

# Make Plan

## Mission

Turn approved requirements and project evidence into a **readable, executable test-first implementation handoff**, not a plan that appends tests after implementation. A human must understand the current situation within the first screen; an implementation agent must find the governing decisions, exact work, and verification without rereading repeated narratives. For behavior-changing plans, load `.agents/skills/test-gap-discovery/SKILL.md` before deriving scenarios and expected outcomes; use its methods without copying its full methodology here.

This skill writes **plans and handoffs**, not permanent feature documentation. For as-built component documentation, use the project's documentation skill (for example `make-docs`) instead. Planning does not authorize code changes or test execution; the planner specifies an executable test-first contract but need not write or run tests unless separately assigned.

## Load context selectively

1. Read only the task's governing instructions, approved requirements, existing plan (if any), and relevant repository entry points.
2. Inspect the smallest relevant set of source files, tests, configs and version-specific contracts needed to verify the proposed work.
3. Expand inspection only to resolve a material uncertainty or conflict. Never read all `docs/` or scan the entire repository by default.
4. Before writing or refactoring a plan, read [references/plan-layout.md](references/plan-layout.md). The example in `examples/` is optional and should be opened only when a concrete formatting example is useful.
5. When revising an existing plan, preserve IDs and approved scope. Preserve useful evidence, but reorganize it by information type rather than by where it originally appeared.

## Five distinct kinds of information

Never combine these categories under a heading such as "Facts, assumptions and risks".

| Kind | Meaning | Where it belongs |
| --- | --- | --- |
| **User-approved requirements and decisions** | The user, an approved spec, or another authorized source explicitly requires or selected something | `Requirements and user decisions` |
| **Verified findings** | Repository/runtime/upstream facts the planner actually inspected | `Verified findings` or relevant technical context |
| **Planner's technical choices** | Engineering decisions made by the planning agent within already approved authority | `Technical approach and decisions` |
| **Assumptions, risks, blockers** | Unknowns, consequences, limitations or unresolved conflicts | Separate `Assumptions`, `Risks`, and `Blockers`, only when present |
| **Implementation tasks** | Work that someone must execute to change project artifacts | `Implementation tasks` only |

**Provenance is mandatory for decisions**. Distinguish `USER-APPROVED`, `APPROVED SPEC`, and `PLANNER-SELECTED`. Do not attribute a choice to the user unless the provided material supports that attribution. If provenance is unclear, say `ORIGIN UNCONFIRMED` and do not silently elevate it to a user decision. A planner-selected routine choice may be binding within an authorized handoff; this does **not** make it user-approved.

**Verification is independent of decision authority.** `VERIFIED` means a fact was checked, not that a design was approved. `DECIDED` means a choice is binding, not that runtime behavior was tested. Never treat these as interchangeable status labels.

## What constitutes a decision, task, fact, or risk?

- **Decision:** A selected rule or design that governs implementation. Write it as a short declarative statement, explain the *reason* only when material, and identify its origin. Example: `GitHub tools must be read-only — USER-APPROVED`.
- **Fact:** An observation that can be checked. Include evidence and scope, such as `Pi CLI supports ... (checked via pi mcp add --help, version X)`. Facts do not tell an implementer to do something.
- **Task:** An actionable outcome requiring repository changes or a separately requested validation activity. Only tasks use `T-01`, `T-02`, ... and have steps, dependencies and completion criteria.
- **Risk:** An undesirable possible outcome with cause, impact and mitigation/handling; it is not a required feature or a new task by itself.
- **Assumption:** An unverified assertion that planning temporarily relies on; include a narrow verification path. Do not hide product, security or destructive-operation choices as assumptions.
- **Blocker:** A missing approval, conflict or essential fact that prevents safe execution; state exactly which task it blocks.

Avoid a mandatory list of `TD-01`, `TD-02`, ... records. For a few choices, use a **compact decision table**. Use a detailed decision record only for an architecture-significant choice with a real trade-off. Decision records are informative/normative **context**, never implementation tasks. Do not include step-by-step implementation instructions inside decision records.

## Authority and conflict handling

Precedence: explicit current user direction → approved business scope → project-wide normative rules → verified checked-in code/runtime → previous decisions/ADRs → official versioned upstream documentation → explicitly marked assumptions.

If current code or documentation contradicts an earlier plan, **do not merge both as if simultaneously valid**. Record `CURRENT`, `SUPERSEDED`, or `UNRESOLVED` with the evidence. The latest verified implementation is a current-state fact, but does not automatically authorize changing the approved target behavior. If resolving a conflict alters user-facing behavior, access, privacy, data loss, supported scope, or an explicit approval boundary: mark affected tasks `BLOCKED` and request the missing decision.

When evidence is limited to a pasted document, write `Reported by source document; not independently verified` rather than claiming runtime verification. Never invent a version, file path, command, successful test or user approval.

## Concept-level implementation contracts and zero-discretion handoff

For every approved concept, capability, or behavior in scope, give it a distinct subsection under `Technical approach` with its own technical implementation contract. A general architecture summary is not a substitute for describing each concept. Map each contract to the relevant requirements and implementation tasks; tasks should link to the contract rather than duplicate it.

For each concept, specify the applicable, evidence-supported details needed to implement it without inventing behavior:

- Observable behavior, inputs, outputs, side effects, and the end-to-end data/control flow.
- Exact repository paths and symbols to add, change, or reuse, grounded in inspected source and tests.
- Interfaces and data/configuration contracts: types, fields, formats, serialization, configuration keys/defaults, and compatibility where relevant.
- State and lifecycle, error handling, edge cases, and relevant retries, idempotency, cancellation, recovery, or cleanup behavior.
- Security, permissions, privacy, secrets, and effects on existing data or compatibility where relevant.
- A test-first contract: observable requirement; independent oracle/source for expected results; selected scenarios and expected outcomes; relevant negative, boundary, stateful, or failure cases; exact test locations, existing framework and level; fixtures, isolation and mocking boundaries; focused command; expected RED signal for missing behavior/a bug; GREEN condition and later regression scope. Tailor selection to contract and risk, not a mechanical checklist. Do not derive correctness merely by copying current implementation output. Reading code to locate seams is allowed.

Tailor the contract to the actual project; do not invent a database, API, persistence layer, or other unsupported component. Include a small example, signature, schema, or pseudocode only when prose would leave more than one plausible implementation. Do not leave a ready task's file locations as `TO VERIFY`; if inspection cannot establish the necessary touchpoints or contract, record the gap and block the affected task.

All implementation-affecting choices must be resolved in the plan by the planner within their authority and recorded with provenance. Give the implementer one binding approach, not alternatives to choose from. If the planner lacks authority or evidence to select a behavior, architecture, interface, security rule, compatibility behavior, or other material contract, block the affected work and request resolution from the authorized source. Do not create an implementation task that delegates a decision to the implementer.

**Plan-readiness gate:** Before marking a plan `READY`, ask: “Can an implementer complete every task without selecting an approach, inferring behavior, or guessing files, behavior, or test contracts?” A plan is not `READY` if the implementer must invent expected behavior or its oracle, primary scenarios, test locations/how to run them, or the condition for proceeding from test to implementation. Do not require RED already executed during planning: readiness means the planned sequence is safe to execute, not that future verification is complete. Link each approved concept contract to requirements and tasks. Otherwise mark `PARTIALLY READY` or `BLOCKED` and identify exactly what is missing.

When an implementation-relevant fact, approval, choice, or conflict remains unresolved, include a dedicated section for missing information/blockers in the plan, using a heading localized to the plan's language. For each gap, state the exact answerable question, why it matters, evidence checked and what remains unknown, affected concepts/requirements/tasks/dependents, who or what authority can resolve it, and what answer/evidence unblocks the work. Do not hide a blocker in assumptions or offer it as an implementer choice. Mark affected tasks and hard dependents `BLOCKED`; mark the plan `BLOCKED` if no safe independent task can proceed, otherwise `PARTIALLY READY`. The next action must resolve the blocker.

**Mandatory implementer stop rule:** Before starting, check the blocker section and task statuses. Do not start blocked tasks or their hard dependents. Stop before dependent implementation for an ambiguous oracle, missing permissions, inability to run the required test, unexpected RED, or mismatch between requirement and planned expectation. If a test is green immediately, determine whether the behavior already exists or the test is inadequate; do not assume either or break code to force RED. If an unrecorded ambiguity or contradiction appears, stop before making any change that depends on its resolution. Stop rather than choosing an interpretation whenever the plan, code, or evidence leaves multiple plausible implementations or omits a detail needed to follow the task exactly. Do not improvise, substitute an approach, change scope or contracts, or claim blocked work is complete. Add the exact gap and evidence to the plan if authorized; otherwise report it to the plan owner and wait for authorized resolution. Continue only tasks explicitly identified as independent and safe. If impact cannot be isolated, pause all implementation in the handoff. Preserve already-made changes and report the current state; do not make speculative cleanup or reverts.

## Required reader-first narrative

Organize the document around reader questions, not the planner's discovery chronology:

1. **Where are we now?** One brief summary of scope, implementation/verification progress, blockers and next action.
2. **What was required or chosen by the user?** Approved goal, boundaries, requirements and explicit user decisions; no agent discoveries mixed in.
3. **What did the analysis establish?** Only relevant verified findings; note their evidence, version, and freshness where meaningful.
4. **How will it work?** Chosen technical approach, responsibilities, interfaces/contracts, and significant planner decisions. Explain the flow once.
5. **What must be done?** Numbered implementation tasks in execution order, with independent acceptance and validation.
6. **What remains uncertain or risky?** Document blockers in a dedicated section using the plan's language; keep assumptions, risks and deferred scope distinct.

Do not create a massive front-loaded references section. Link sources near the claims they support or in one compact source table. Use headings (`##`, `###`) for genuine sections and named topics; use lists for items **within** a section, not as a substitute for document structure.

## Status: never conflate progress with approval

Keep three dimensions separate:

- **Plan readiness:** `READY`, `PARTIALLY READY`, `BLOCKED`, or `RESEARCH ONLY`.
- **Task execution:** `TODO`, `IN PROGRESS`, `IMPLEMENTED`, `BLOCKED`, or `NOT APPLICABLE`.
- **Verification/acceptance:** `NOT RUN`, `PASSED`, `FAILED`, `MANUAL PENDING`, or `NOT APPLICABLE`.

Use observed evidence, not prose optimism. `IMPLEMENTED` + `MANUAL PENDING` is valid and **must not** be flattened into "done" or "not done". A completed task gets a concise implementation/evidence summary; do not leave its old step-by-step instructions written as if they were future work. Preserve historical implementation detail in the worklog, Git, or an explicit collapsed/linked historical appendix only if needed.

## Tasks are the only execution queue

Use `T-xx` only for actual technical tasks. Each pending task must specify:

- **Outcome:** observable behavior after completion.
- **Traceability:** relevant requirement and governing decision IDs if used.
- **Status:** execution and verification independently.
- **Dependencies:** only real hard predecessors.
- **Touchpoints:** exact repository paths and symbols, verified during analysis. If a necessary location cannot be established, document the gap and block affected work; never leave a ready task's location as `TO VERIFY`.
- **Steps:** an ordered, runnable procedure. For behavior changes, show `scenarios and oracle → test → RED → implementation → GREEN → regression/refactoring` visibly. Use one vertical task or separate test and implementation tasks where useful; the latter must have a hard dependency on test preparation and required RED check. Do not create one task per test or defer all tests until after the feature.
- **Failure and safety behavior:** relevant edge cases and invariants.
- **Verification:** exact feasible commands/checks and expected outcomes; never claim unrun checks passed.
- **Done when:** inspectable acceptance conditions.

Do not create separate tasks for choosing, approving, or writing down decisions in an `IMPLEMENTATION HANDOFF`. Resolve routine technical choices within approved constraints during planning. Put explicitly requested research/spikes in a clearly separate `Research or decision work` section, **not** in the implementation queue. Block implementation if a material authority decision cannot be resolved.

Traceability should be practical, not bureaucratic. Preserve `BR-xx` and `T-xx`; add decision IDs only when cross-references would genuinely help. Avoid duplicating detailed contracts across requirements, decisions, tasks, acceptance matrix and deployment notes. State each contract once and link to it from tasks.

## Design quality, boundaries, and validation

- Fit the actual project type. Never assume a database, HTTP API, Docker, service layers, or persistence unless supported by the task.
- Prefer existing native/project capabilities over new services, proxies, registries, schemas or frameworks.
- Keep safety, secrets, permissions, failure modes, retries, lifecycle and recovery explicit where they affect behavior.
- Distinguish configured vs authenticated vs connected vs tested; planned vs implemented vs accepted.
- Describe concrete inputs, outputs, side effects and errors for meaningful interfaces; show example configuration only when it helps implementation.
- When tests are prohibited, mark them `NOT RUN — PROHIBITED` and preserve the prohibition. Do not replace verification with a claim that the design works.
- End with **exactly one recommended next action**, a task or a precise blocker-resolution step.

## Output and final review

Follow [references/plan-layout.md](references/plan-layout.md); adapt optional sections to complexity. Write in the user's language; keep true code symbols and commands unchanged. Prefer short paragraphs, compact tables, and task-focused numbered procedures over walls of bullet-point text.

Before publishing, verify:

1. A reader can identify current state, user decisions, completed work and next task within the opening section.
2. No user choice is mixed into `Verified findings`, no inference is mislabeled as user-approved, and no fact is mislabeled as a decision.
3. Decision entries describe **what governs**, while `T-xx` entries alone describe **what to do**.
4. A previously completed task is not simultaneously presented as an unexecuted procedure.
5. Current and superseded contracts do not silently coexist.
6. Every approved in-scope concept has its own evidence-grounded implementation contract linked to requirements and tasks; every pending task is executable without inventing material behavior and has precise verification.
7. For behavior changes, tests precede production code; expected results have independent justification; the contract identifies expected RED and GREEN, and scenarios detect plausible faults. Exceptions and blockers are explicit, and planned tests are not presented as run.
8. No claim of passing tests, working OAuth, successful connections or approval is made without evidence.
9. Only relevant files were read; the plan does not copy the whole project's documentation into itself.
10. There is exactly one concrete next action.
11. Every unresolved implementation-blocking gap is documented in a section localized to the plan's language, affected tasks are blocked, and the implementer is explicitly instructed to stop rather than decide or improvise.

**Success criterion:** A human can distinguish intent, evidence, decisions, remaining implementation work and progress at a glance; a coding agent can execute the next task safely without re-deciding the architecture.
