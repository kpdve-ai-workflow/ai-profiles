---
name: refine-plan
description: Refine an existing implementation plan with the user, one technical problem at a time, into an approved tasks/T-xx.md instruction. Use when the user wants to walk through a plan, discuss implementation options, settle exact technical choices, or expand a task so an executing agent does not have to decide how to implement it. Include a plain-language implementation walkthrough and explicit runtime paths. Extends make-plan; does not implement the task or replace planning from requirements.
---

# Refine Plan

## Mission and boundaries

Turn one existing plan task into a **jointly agreed, evidence-grounded implementation instruction**. The user and planning agent settle the solution here; the executing agent follows it rather than choosing an approach. Preserve the plan's approved scope, requirements, task IDs and safety boundaries.

Work as a technical partner: investigate facts yourself, explain meaningful options and recommend a solution, but discuss **one unresolved problem per turn** and wait for the user's answer. Finish with explicit approval of the complete task instruction; approval of individual choices is not approval of the whole handoff.

This skill writes planning documents, not production code, tests or permanent as-built documentation. It does not authorize test execution, experiments, installations, live connections, migrations or deployment. A request to refine an entire plan means sequential workshops, not permission to decide all tasks silently.

## Load only relevant context

1. Read governing project instructions, the supplied plan and its approved sources. Inspect Git state and preserve unrelated changes. Treat the supplied plan as context, not proof that its paths, findings or readiness are still current.
2. Read the project's `make-plan` instructions for shared provenance, readiness, blocker and test-first conventions. In this bundle they are [../make-plan/SKILL.md](../make-plan/SKILL.md). Do not restart plan creation or reorganize the whole plan.
3. Read the selected task's existing detail file, relevant shared contracts and hard dependencies. If no task was selected, recommend one based on the plan's next action and ask the user to confirm it; do not choose a different scope silently.
4. Inspect the smallest relevant source, configuration and test files to establish current touchpoints and constraints. For behavior-changing work, load the project's `test-gap-discovery` skill and use its `vision` methods to derive scenarios and independent oracles. Neither skill grants permission to write or run tests.
5. Read [references/task-layout.md](references/task-layout.md) before creating or revising the task detail file. Write documents and questions in the user's language; retain real code symbols and commands.

If a required source or skill is unavailable, identify the missing material and ask for access or an authorized resolution. Do not invent evidence or silently replace a required method.

## Document ownership

Use `<plan-directory>/tasks/T-xx.md`, preserving the exact existing task ID. For example, a plan at `todos/pi-performance/implementation-plan.md` has task details at `todos/pi-performance/tasks/T-02.md`. This example is a location convention, not a request to refine that task now.

- **Parent plan:** owns approved scope, requirements, shared decisions/contracts, dependency graph, execution queue and execution/verification status.
- **Task file:** owns the detailed task-specific solution, choice approvals, human walkthrough, technical contracts, test-first procedure and handoff approval.
- **Links:** the task file links to the parent task and applicable shared contracts; the parent task links back to the detail file. Link rather than duplicate normative detail. Do not create a second execution queue.

Keep workshop drafts explicitly non-binding. Do not replace a valid approved contract with an unapproved proposal. Once the full instruction is approved, update the selected parent entry and replace superseded task-specific prose with a link, preserving requirements and decision provenance. A shared-contract change requires explicit approval and an impact check of its consumers; do not quietly bury it in one task file.

If investigation reveals a genuine implementation-blocking gap in the current plan, record it immediately, mark the affected task and hard dependents `BLOCKED`, and update aggregate readiness using `make-plan`. Do not downgrade unrelated tasks merely because a new draft exists. Never treat an existing `READY` label as a reason to ignore newly discovered ambiguity.

## Workshop loop: one problem at a time

### Establish the focus

Briefly restate the selected task's outcome, scope boundaries, current execution state and existing binding choices. Respect completed or excluded tasks: do not turn them into pending work without an authorized new scope.

Privately map unresolved details and their dependencies. Start with the issue that most constrains the remaining design or poses the greatest risk. Do not dump a questionnaire or ask the user to select facts that repository inspection can establish.

### Investigate, explain and ask

For the current problem:

1. State the exact decision needed and why it affects implementation.
2. Separate inspected facts and their sources from unknowns and proposed design. Read code to locate seams, not to turn current behavior into the desired contract.
3. Present viable alternatives only when they involve a real choice. Explain their relevant consequences, such as compatibility, failure handling, complexity, performance or safety. If there is one sensible approach, explain it without manufacturing alternatives.
4. Recommend one concrete approach and explain why it fits the approved constraints. Make clear what would change and what would remain unchanged.
5. Ask **one answerable question about this problem**, then stop and wait. Do not append unrelated decisions or proceed as if a recommendation were accepted.

Existing approved decisions remain binding; explain and reuse them rather than reopening them for ceremony. Ask to revisit one only when new evidence exposes a conflict or the user explicitly requests a change. An ambiguous answer is not approval: clarify the same problem before moving on.

### Capture the answer and continue

After a clear answer, update the task draft when file edits are authorized:

- Record the selected rule, rationale where material, provenance and a concise reference or quotation identifying the user's approval. Use real message references only if available.
- Keep unapproved proposals marked `PROPOSED`; never relabel silence, a recommendation, a verified fact or an imported uncertain choice as `USER-APPROVED`.
- Update affected contracts and the walkthrough together so they describe the same solution.
- Preserve unrelated choices, observed verification evidence and task execution state.
- Move to the next unresolved problem in a later turn, or present the complete instruction for final approval when none remain.

For small mechanical details, propose concrete values in the draft and identify them in the final review rather than asking a separate question about every identifier. They still require approval as part of the complete instruction and must not be left for the executor to choose. If the user explicitly delegates a category of choices, record that authority and label resulting choices `PLANNER-SELECTED`, not `USER-APPROVED`; final whole-instruction approval still applies.

If the user interrupts, stop at a useful checkpoint: preserve confirmed choices, keep the document `DRAFT`, and leave exactly one next question. Do not infer the remaining answers to finish the file.

## Required result: human walkthrough plus exact instruction

Use the layout reference, tailoring details to the real task. Avoid empty database, API or deployment sections for components the project does not have.

### Explain the implementation to a human

Write a connected, plain-language walkthrough, not just a list of files or an architecture diagram. Explain separately:

- **How the change will be introduced:** starting state, ordered changes, when tests run, any required activation/deployment, what remains untouched and how completion is recognized.
- **How the resulting system will behave:** trigger/input → components or functions in order → decisions/state transitions → observable output and side effects.
- **Relevant alternative runtime paths:** for example failure, cancellation, existing data or repeated input, with their exact outcome. These are prescribed branches of the chosen solution, not design alternatives for the executor.
- **Failure during rollout:** the safe stopping point, preservation and recovery/rollback where applicable. Do not invent a deployment mechanism or promise that a lossy change is reversible.

Explain what the user will observe where relevant. Connect the prose to the technical steps and real touchpoints so a human can follow the process without knowing every symbol.

### Specify one binding technical solution

For applicable details, settle:

- Exact existing paths/symbols to reuse or change, with inspected evidence. Label new paths/symbols as **planned additions**, giving their exact names and responsibilities, not fabricated existing locations.
- Inputs, outputs, signatures/types, data formats, configuration keys/defaults and compatibility boundaries.
- Algorithm/control flow, ordering, state ownership, lifecycle and side effects. Use pseudocode, a schema or a state table when prose would permit different implementations.
- Error propagation, retries/timeouts, concurrency, idempotency, cancellation and cleanup where relevant, with specific rules rather than "handle errors appropriately".
- Permission, privacy, secret handling, existing-data preservation and activation/migration boundaries where relevant.
- Ordered changes with explicit before/after conditions, acceptance criteria and stop conditions.

Write selected behavior once and link to shared contracts. Remove unresolved "A or B", "choose a suitable library", "check during implementation", "etc." and similar decision delegation from a ready handoff. Do not attempt a full code patch disguised as a plan; specify what is necessary to eliminate implementation choices.

### Make testing executable without running it during refinement

Record the observable requirement and independent source of expected results, then derive valuable contrasting scenarios before borrowing expectations from implementation. Specify expected results, actual test location/framework/level, fixtures, isolation/mock boundaries, focused commands and regression scope. Identify plausible wrong behavior the assertions must distinguish.

For missing or faulty behavior, prescribe `test → justified RED → smallest production change → same tests GREEN → regression`. Define the intended RED signal and GREEN condition. For behavior-preserving refactoring, prescribe protection and equivalence checks first; never demand artificial RED. A test green immediately requires checking whether the behavior exists or the assertion is inadequate, not breaking working code.

Distinguish future commands and expected results from commands actually run and observed evidence. Mark unrun checks truthfully; preserve `NOT RUN — PROHIBITED` when applicable. Planning itself needs no executed RED. The executor must obtain the required evidence before dependent production changes. Missing oracle, required authority, feasible test seam or command is a blocker, not permission to improvise or skip test-first.

## Approval and readiness

Keep task-spec approval separate from execution and verification:

- `DRAFT`: the workshop or contract is incomplete.
- `AWAITING APPROVAL`: a complete revision is presented for explicit whole-instruction approval.
- `APPROVED`: the user explicitly approved that complete revision; record the actual approval reference.
- `STALE`: a material contract change or contradictory new evidence invalidated the previous approval; preserve its history and request approval of the new revision.

Number task-spec revisions so approval has a clear target. A newly approved individual choice does not automatically approve a revised instruction. Editorial corrections that do not change the contract need not restart approval.

Before requesting final approval, inspect the instruction as an executor:

1. Can every step be followed without choosing an approach, inferring behavior, guessing a path, inventing a test oracle or selecting a fallback?
2. Do the human walkthrough, technical contracts and test expectations agree, including relevant negative paths?
3. Are all choices either already binding or explicitly proposed for approval, with no hidden scope expansion?
4. Are dependencies, permissions and stop conditions explicit, and observed evidence distinct from planned verification?

If not, return to the next single unresolved problem. If yes, summarize the selected solution and any details proposed for whole-instruction approval, link the exact revision and ask one final approval question. **Wait for the answer.**

Only an approved, complete, unblocked instruction can be a `READY` refined handoff. Approval does not prove implementation, passing tests or deployment, nor override a prohibition. Keep parent-plan execution and verification statuses grounded in actual work. A plan may be `PARTIALLY READY` when only some tasks have safe, approved handoffs.

Any subsequent material change reopens the affected part of the workshop and makes that revision non-executable until approved again. Inspect changes to relevant sources on resumption; unchanged evidence need not be reapproved solely because time passed.

## Mandatory executor stop rule

Include this rule in every task instruction:

> Before dependent changes, check approval, readiness, blockers, source assumptions and required permissions. Do not execute a draft, stale or blocked instruction or a blocked task's hard dependents. If code, tests or evidence contradict the instruction, required verification cannot run, RED has an unexpected cause, or multiple plausible implementations remain, stop before dependent changes. Report the exact gap and evidence to the plan owner; do not choose, substitute, weaken assertions or expand scope. Preserve existing work. Continue only work explicitly established as independent and safe; if impact cannot be isolated, pause the handoff. Resume after an authorized, approved resolution.

## Close each interaction clearly

During the workshop, end with exactly one current decision question. At final review, end with the whole-instruction approval question. After approval, report the task file and parent-plan link, remaining blockers if any, and exactly one next action; approval of planning does not start implementation automatically.

Do not claim the underlying feature was implemented or tested. Do not create skill evaluations, benchmarks or extra report files unless separately requested. Follow the project's documentation checks and worklog rules for actual file changes without treating them as permission to run product tests.
