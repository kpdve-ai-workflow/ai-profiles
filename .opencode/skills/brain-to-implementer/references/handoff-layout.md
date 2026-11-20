# Persistent handoff layout

Read this reference before generating a handoff. These are templates, not a live handoff. Replace all illustrative angle-bracket fields with inspected, request-specific content; do not leave placeholders, alternatives, or template commentary in generated files. Localize prose to the user's language while preserving paths, symbols, commands, IDs, and status tokens.

## Placement and ownership

Create in the **target project's root**, not relative to this reference:

```text
todos/<slug>/
├── plan.md
├── implementer-prompt.md
└── tasks/
    ├── T-01.md
    ├── T-02.md
    └── ...
```

Use a short lowercase ASCII kebab-case slug. A new request must use an unused directory: if the base exists, try `-2`, `-3`, etc., selecting the first available suffix. Only revise an existing package when explicitly identified by the user; preserve IDs, progress, historical evidence, and approved scope. Brain writes only the handoff Markdown, not feature source/tests. Do not create empty tasks to match this illustrative tree: one task is enough for a small request.

`plan.md` alone owns statuses, dependencies, completion evidence, and the single next action. Task files own detailed procedures; they must not acquire duplicate mutable status/dependency/evidence ledgers. The reusable prompt points at the actual package path. No runner or extra dependency is needed.

Before deriving test scenarios, Brain must apply `test-gap-discovery`, loading the skill when available. If unavailable, report it and retain the methodology in `../SKILL.md` and the target project's testing policy. Inspect that policy and cite its actual paths in the handoff. Infer required environments, commands, isolation, and acceptance gates from the target project; never impose this repository's infrastructure on another project.

## `plan.md` template

```markdown
# <Request title> — handoff

## Current state
Plan readiness: READY
Progress: <Brief implementation and verification summary; initially no implementation performed.>
Next action: Execute [T-01](tasks/T-01.md) only.

## Objective and scope
<Observable outcome, in-scope surfaces, explicit exclusions, preservation constraints.>

## Requirements and user decisions
- R-01: <Requirement and measurable acceptance condition; source in user request/approved spec.>
- <Explicit user-selected decision and its source, or “No additional user decisions.”>

## Verified findings
| Finding | Source inspected | Relevance / limits |
| --- | --- | --- |
| <Fact, not a design choice> | <Exact repository path/symbol, versioned reference, or actual check> | <What this establishes and does not establish> |

## Brain-selected technical decisions
| Decision | Binding approach and rationale | Origin |
| --- | --- | --- |
| <Named choice> | <One resolved approach within approved scope> | BRAIN-SELECTED |

## Shared contracts
### Testing policy and methodology
<Actual target-project testing instruction paths, required execution environments,
permissions/isolation and verification gates. State that Brain and Implementer apply
test-gap-discovery, load its skill when available, and retain the methodology if absent.
Keep task-specific scenarios and commands in task files unless shared.>

### <Named contract needed by multiple tasks>
<Linked requirements and decisions; exact interfaces, behavior, data/control flow,
state, failure/security/compatibility rules and verification expectations where relevant.
Keep each shared contract here once; put task-specific procedures in task files.>

## Ordered execution queue
- [ ] [T-01 — <Bounded outcome>](tasks/T-01.md)
  - Execution: TODO
  - Verification: NOT RUN
  - Depends on: none
  - Evidence: none
- [ ] [T-02 — <Next bounded outcome>](tasks/T-02.md)
  - Execution: TODO
  - Verification: NOT RUN
  - Depends on: T-01
  - Evidence: none

## Execution evidence
<Initially “None — implementation has not started.” Later, per task, retain dated/session
records of changed files, actual commands or manual checks, observed results and relevant
working-tree state, remaining problems, and acceptance evidence. Link queue entries here
when details are longer. Preserve history; never replace observed results with expectations.>

## Blockers
<Initially “None.” For each gap: exact question/problem, affected task IDs and hard
dependents, evidence inspected, why execution is unsafe, resolver/authority, and the
specific answer/check/permission needed to unblock.>
```

Adapt the queue length, dependencies, readiness, and next action to actual evidence. Do not copy the two-task example mechanically. If essential facts or verification are missing, mark affected tasks and hard dependents `BLOCKED`; readiness is `PARTIALLY READY` if some work is executable, otherwise `BLOCKED`. Use `READY` only when every task has an executable contract, not to imply tests were run. A blocker is not a task asking the Implementer to design the missing solution.

There must be **exactly one next action**. Initially select the first unchecked task, or a precise blocker-resolution step if execution cannot safely proceed. On completion point to the next unchecked task, not permission to start it. Pending manual acceptance keeps the pointer on resolving that acceptance. When all tasks are checked, the next action is “Report completion; no implementation work remains.” Do not add work automatically.

### Status and checkbox contract

- Execution values: `TODO`, `IN PROGRESS`, `IMPLEMENTED`, `BLOCKED`.
- Verification values: `NOT RUN`, `PASSED`, `FAILED`, `MANUAL PENDING`, `NOT APPLICABLE`.
- Use `- [ ]` until all completion criteria and required verification are satisfied; then `- [x]`. `NOT APPLICABLE` needs a task-specific explanation proving verification genuinely does not apply. It cannot waive unavailable tests or required documentation checks.
- Implementation complete but manual acceptance outstanding: `IMPLEMENTED` / `MANUAL PENDING`, **unchecked**. Required test unavailable: record `BLOCKED` with `NOT RUN` (or `FAILED` if actually attempted and failed), preserve partial progress in evidence, leave unchecked, and do not advance. For a failure after some successful checks, record each check's real outcome rather than overwriting history.
- Readiness, implementation progress, and acceptance are distinct. Update the summary and readiness when new evidence/blockers change them. Only the plan holds mutable execution records; task procedures remain specifications, not claims they have run.

## `tasks/T-xx.md` template

```markdown
# T-01 — <One bounded, independently verifiable outcome>

## Objective and traceability
<Observable outcome and link to requirement in ../plan.md.>

## Required context and governing decisions
<Links to relevant ../plan.md sections, inspected sources and decisions only.
Do not repeat statuses, dependencies or evidence: read those in the plan.>

## Change locations
- Existing: <Exact inspected file paths/symbols and purpose of change.>
- Add: <Exact new paths and their intended responsibilities.>

## Implementation contract
<Concrete inputs/outputs, interfaces, behavior and relevant edge/failure/security/
compatibility cases. Link shared contracts rather than repeating them.>

## Scope boundaries
Allowed: <Bounded edits needed for this outcome.>
Non-goals: <What must remain unchanged; no unrelated refactors or speculative work.>

## Ordered procedure
1. <Establish independent expectation and source for selected scenarios.>
2. <Write focused tests at inspected location with existing framework/fixtures.>
3. <Run inspected focused command; expected RED for the specific missing/faulty behavior.>
4. <After justified RED, make specified minimal implementation.>
5. <Run the same focused check for GREEN, then the specified relevant regression checks.>

## Verification and expected results
<Link the target-project testing policy. For behavior changes specify independent
expectation sources, risk-based scenarios, discriminating assertions and plausible faults,
test level, locations, fixtures/data, isolation and mock boundaries. Give exact inspected
commands/check procedures, expected RED reason for new/repaired behavior, GREEN result,
regression scope, acceptance criteria, and manual acceptance if required.
These are expectations, not execution evidence. Actual results belong in ../plan.md.>

## Completion criteria
<Observable acceptance conditions and required checks; update the plan only on evidence.
Implementation without pending required acceptance resolved is not checkbox completion.>

## Stop conditions
<Task-specific hazards plus missing permission, unavailable required checks, unexpected
failure, ambiguity, or contradiction. Preserve partial changes and record the exact blocker
in ../plan.md; do not redesign, weaken tests, skip ahead, clean up speculatively, or revert.>
```

For documentation-only tasks, **replace** the test/RED/implementation/GREEN sequence with content-expectation review, bounded documentation edits, and concrete content/link/consistency checks with expected results. Do not invent tests. For behavior tasks, identify the independent oracle, test seam, intended failure, minimal change, and regression scope before marking ready. The ordered test-first sequence applies to new or repaired behavior. A test protecting existing correct behavior may be GREEN immediately: adapt that sequence without requiring RED. If initial tests are GREEN, inspect whether behavior exists or the test lacks sensitivity; do not force RED by breaking code. Unresolved or unexpected outcomes require stopping, not guessing or unilaterally changing the testing contract. Missing essential locations or verification blocks the task; do not leave “figure it out” instructions.

## `implementer-prompt.md` template

Substitute the actual project-relative handoff path everywhere below. This prompt must be directly reusable in a fresh session without chat history.

```markdown
# Implementer: one task per invocation

You are the implementation agent working in the target project. The handoff is at
`todos/<slug>/`. Follow this procedure for exactly one task, then report and stop.
“Execute the plan” or “continue” authorizes one task, not the entire queue.

1. Read `todos/<slug>/plan.md`, including the progress summary, blockers, queue,
   and its single next action. It is the sole authority for statuses, dependencies,
   and completion evidence. Do not rely on chat history. If all tasks are checked,
   report completion and stop; do not repeat tasks or invent work.
2. Follow the next-action pointer. If it requires blocker resolution or pending
   acceptance, address/report that action and stop; do not select unrelated work.
   Otherwise open only the selected task specification and context/source files
   needed for it. Do not load every future task by default.
3. Confirm dependencies are checked complete and scope and verification are executable.
   Read the target project's testing instructions identified in the plan and applicable
   local policies. When adding, correcting, or reviewing tests, apply test-gap-discovery,
   loading its skill when available. Report absence, but retain the methodology below
   and the project's rules; skill availability does not waive them.
   Inspect actual working-tree changes before editing; preserve unrelated work.
   Resume an interrupted task using its recorded state and existing partial changes,
   not a blind restart. Never repeat checked tasks automatically.
4. Mark the selected task IN PROGRESS in `plan.md` before implementation.
5. Perform only that task within its contract, including its verification sequence.
   Do not redesign, change approved scope, or perform the next task.
6. Update `plan.md`: execution and verification statuses separately, changed files,
   actual checks/commands and outcomes, partial progress, acceptance evidence,
   remaining problems and exact blockers. Preserve historical evidence. Task files
   remain specifications, not a second mutable status ledger.
7. Only when completion criteria and required verification are satisfied, check the
   task's checkbox and point the single next action at the next unchecked task.
   Do not start it. If all are checked, point to reporting completion. Otherwise
   keep the pointer on resuming this task or resolving its blocker/acceptance.
   Refresh the progress summary and plan readiness as evidence requires.
8. Report this invocation's result, changed files, actual checks and outcomes,
   and any blocker or pending acceptance. Then stop.

Execution: TODO, IN PROGRESS, IMPLEMENTED, BLOCKED.
Verification: NOT RUN, PASSED, FAILED, MANUAL PENDING, NOT APPLICABLE.
A checkbox is checked only when completion criteria are met and required verification
has passed. NOT APPLICABLE requires a task-specific justification for genuinely
inapplicable checks; it is not an escape from unavailable tests. IMPLEMENTED with
MANUAL PENDING remains unchecked: preserve progress without claiming acceptance.
Unavailable required verification leaves the task unaccepted and unchecked: record
BLOCKED execution, NOT RUN (or FAILED if attempted and failed), completed edits and
actual limitations. Do not advance or relabel required checks NOT APPLICABLE.

Testing methodology: establish observable requirements and independent expectation
sources before copying expectations from implementation. Derive risk-relevant scenarios;
select the test level, data, isolation and mock boundaries. Identify plausible incorrect
behavior and discriminating assertions. Current output alone is not an oracle; mocking
the behavior under test or checking orchestration does not prove real integration.
Use test-first and justified RED before implementation for new or repaired behavior,
then GREEN and required regression checks in the project's approved environment.
Diagnose failures: product defect, wrong expectation, faulty fixture, nondeterminism,
or infrastructure. No required bug finding, case quota, mutation experiment, or separate
per-test report. Mutations need explicit permission and safe disposable isolation.

Stop on missing permission, unavailable required verification, unexpected test failure,
ambiguous requirements, or contradictions affecting this task. Mark blocked execution
when appropriate; record partial changes and the exact blocker and actual verification
state. Do not skip a blocked selected task to choose unrelated work. Preserve changes;
no speculative cleanup or reverts. Never weaken tests or fabricate results to finish.
Do not unilaterally change the testing contract, requirements, or legitimate assertions,
add skips to get GREEN, omit required checks, or bypass isolation. Stop and report any
unresolved contradiction; preserve completed edits without claiming acceptance.
If a focused test is GREEN initially, inspect existing behavior and test sensitivity;
tests of existing correct behavior may be GREEN from the first run. Never break code
to manufacture RED. Stop if the discrepancy cannot be resolved
within the binding task contract.

These are workflow instructions, not a runtime-enforced permission boundary.
```

## Publication review

Resolve every relative link from its containing file; check task IDs/queue links, requirement/decision anchors, and actual source paths. Confirm no accidental template placeholders remain, no status ledger is duplicated, and the next action names exactly one action. Do not claim expected checks have run. Consult [behavior-checks.md](behavior-checks.md) for static regression scenarios; no behavioral harness is implied.

After saving, Brain's final response contains only the actual directory and a short copy-paste-ready prompt telling the Implementer to read the actual `implementer-prompt.md` and execute one task, then stop. Do not paste the full plan or launch execution. If access is unavailable/denied, explicitly say nothing was saved and return proposed contents with intended paths instead; never advertise nonexistent files as a usable package. For partial write failure, distinguish saved from unsaved artifacts and state that the package is incomplete.
