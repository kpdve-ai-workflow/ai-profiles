# Implementation Blueprint: Output Contract

Read when drafting or restructuring an actual plan. This is an **information architecture**, not a demand to print every section for every small feature. The English headings and labels below are illustrative; write and localize the generated plan in the user's language.

## Recommended structure

```md
# <Feature / milestone> — Implementation Plan

## Status and next action

**Plan:** READY | PARTIALLY READY | BLOCKED | RESEARCH ONLY
**Scope:** One or two sentences.
**Current state:** What is already implemented versus not implemented.
**Validation:** What is verified, what needs manual acceptance.
**Next action:** Exactly one T-xx task or blocker-resolution step.

| Task | Outcome | Implementation | Verification (observed) |
| --- | --- | --- | --- |
| T-01 | ... | IMPLEMENTED | MANUAL PENDING |
| T-02 | ... | TODO | NOT RUN |

## Requirements and user decisions

### Objective and scope
The business outcome, IN/OUT and hard constraints.

### Requirements
| ID | Requirement / acceptance condition | Source |
| --- | --- | --- |
| BR-01 | ... | approved spec / explicit user message |

### User-approved choices
| Choice | Binding rule | Provenance |
| --- | --- | --- |
| ... | ... | explicit user instruction / approved source |

## Verified technical findings

| Finding | Evidence | Relevance / limitations |
| --- | --- | --- |
| ... | checked file / command / upstream version | ... |

## Technical approach

### System flow
Short description or diagram, without repeating all tasks.

### Engineering decisions
| Choice | Selected approach | Reason | Origin |
| --- | --- | --- | --- |
| ... | ... | ... | PLANNER-SELECTED / APPROVED SPEC / USER-APPROVED |

### Concept implementation contracts
Give every approved in-scope concept, capability, or behavior its own subsection. A general architecture summary is not a substitute for these contracts. Include the applicable, evidence-supported details needed to implement each concept without inventing behavior:

#### <Concept name>
**Requirements / decisions:** ...
**Behavior and data/control flow:** inputs, outputs, side effects, component interactions.
**Exact touchpoints:** verified repository paths and symbols; what to add, change, or reuse.
**Interfaces / data / configuration:** types, fields, formats, keys/defaults, compatibility.
**State / lifecycle / failures:** relevant transitions, edge cases, retries, idempotency, recovery.
**Safety / existing data:** relevant permissions, privacy, security, migration, preservation.
**Test-first contract:** link selected scenarios and an independent oracle/source to expected results; include relevant risk-driven negatives/boundaries/state/failures, exact test location/framework/level, fixtures/isolation/mock boundary, focused command, expected RED signal (for missing or faulty behavior), GREEN condition, and later regression scope. Keep expected and observed outcomes separate. Mark `NOT RUN` only for checks not yet performed; preserve previously documented results with their execution context (command, code/state, and outcome) when updating an existing plan.

Omit genuinely irrelevant details rather than inventing unsupported components. Add a signature, schema, example, or pseudocode where prose leaves multiple plausible implementations. Link implementation tasks to these contracts instead of duplicating them. Resolve choices in the plan with provenance; leave one binding approach, never options for the implementer. If a necessary choice or fact cannot be resolved within the planner's authority/evidence, block the affected work.

## Implementation tasks

### T-01 — <Outcome>  [IMPLEMENTED / MANUAL PENDING]
Short record of real artifacts, known checks and pending acceptance.
Do not include a to-do procedure for already implemented work.

### T-02 — <Outcome>  [TODO / NOT RUN]
**Depends on:** ...
**Contract:** link to the concept contract above.
**Change locations:** exact verified paths and symbols.
**Goal:** ...
**Steps:**
1. Establish requirement, observable contract, independent oracle and scenarios (see contract above).
2. Add tests at the specified layer using the existing framework and fixtures.
3. Run focused tests if authorized; for missing/faulty behavior confirm a failure for the intended reason. Record observed outcome separately from expected RED.
4. If required test writing/running is prohibited, unavailable, or fails to produce the justified RED, mark this task and dependent implementation `BLOCKED` and stop. Do not proceed to production changes unless an authorized exception explicitly permits it.
5. Once the required RED is confirmed (or an authorized exception is recorded), make the minimal production change.
6. Re-run the same tests and relevant regression checks; record observed results.
7. If required verification cannot be completed or GREEN is not established, keep the task `BLOCKED`/incomplete and stop dependent work. Refactor only with working test protection.

A test green on first run may protect existing correct behavior. Determine whether behavior already exists or the assertion lacks sensitivity; never break code to manufacture RED. Test prohibitions remain in force.
**Failure/safety behavior:** ...
**Verification:** focused command, expected result, and separately the actual command/state/result when run; otherwise explicit status.
**Done when:** observable acceptance is established and required test protection and verification are complete with truthful evidence. An unrun required check, prohibition, or unresolved blocker means the task is `BLOCKED`/incomplete—not done—unless an authorized exception explicitly waives that requirement; record the exception and its authority.

<!-- Include this section whenever implementation-relevant information, approval, or a conflict is unresolved; omit it only when there is no such gap. Localize the heading to the plan's language. -->
## Missing information and blockers

| Missing information / exact question | Why needed and evidence checked | Affected concepts / tasks | Resolver / authority | Unblocks when |
| --- | --- | --- | --- | --- |
| ... | ... | ... (`BLOCKED`) | ... | ... |

The implementer must stop blocked tasks and hard dependents, and must not guess or choose an alternative. If an unforeseen ambiguity appears, stop before dependent changes, add/report the gap using these fields, and wait for authorized resolution. Continue only explicitly independent and safe tasks; if impact cannot be isolated, pause the whole handoff.

## Open items

### Assumptions
Only unverified inputs relied upon, with verification. Do not use assumptions to hide implementation-blocking gaps.

### Risks
| Risk | Consequence | Mitigation / owner |
| --- | --- | --- |

## Deferred and excluded scope
Only intentional exclusions and conditions for revisiting them.

## Next action
Exactly one ready T-xx or blocker-resolution action. If already
prominent at the top, avoid repeating details here; a short link is enough.
```

## Placement rules

- **Requirements:** what is required, without implementation pseudo-steps.
- **User decisions:** choices explicitly made/approved by the user; list separately from planner-selected engineering decisions.
- **Verified findings:** facts observed during analysis; each important fact has a source and relevant version/scope. Upstream docs are evidence of upstream docs, not proof of installed runtime behavior.
- **Engineering decisions:** only nontrivial selected approaches. Small mechanical choices can be documented inline with the technical approach.
- **Tasks:** only executable changes or independently requested validations. `T-xx` is the task namespace. Decision identifiers, if present, use `D-xx` and do not look like task identifiers.
- **Progress:** implementation status never proves acceptance; verification has its own status. Distinguish expected RED/GREEN from what was actually run and observed, including command/state and why RED is valid.
- **Test-first contract:** missing independent oracle, primary scenarios, test location/command, or transition condition from test to implementation is a blocker. Planning does not require an already executed RED.
- **Concept contracts:** give each approved in-scope concept its own technically specific, evidence-grounded contract linked to requirements and tasks.
- **Readiness and blockers:** mark a plan `READY` only if each concept is covered and tasks require no implementer choices, behavior inference, or file guessing. Put unresolved blockers in a dedicated section localized to the plan's language; mark affected tasks `BLOCKED` and require the implementer to stop. A possible failure is not an existing failure, and an unknown is not a user decision.

## When refactoring an existing plan

1. Build a private mapping of each existing paragraph to exactly one canonical destination, leaving a short cross-reference where necessary.
2. Extract explicit user choices from mixed 'facts/assumptions/risks' sections. Preserve provenance and do not infer missing approvals.
3. Convert `TD-xx` ledgers into a compact Engineering Decisions table. Preserve IDs only when existing references depend on them; otherwise replace with descriptive decision names and update references consistently.
4. Convert finished `T-xx` to historical status/evidence summaries; retain full steps only for pending work.
5. Detect stale contradictions between the plan and current repository/docs, mark affected claims CURRENT, SUPERSEDED, or UNRESOLVED. Never rewrite history as if newer architecture had always been used.
6. Remove repetition, but preserve actual requirements, limitations, safety controls and pending work. Link to an authoritative reference where possible.

## Information density test

The opening view should answer, without scrolling through task details:
- What feature is being delivered?
- Which parts are done and which are waiting?
- What choices came from the user?
- What did the agent independently verify?
- What is the next actionable step?

If the document cannot answer these, rework its hierarchy before adding more text.
