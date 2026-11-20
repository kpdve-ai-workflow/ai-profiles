---
name: make-roadmap
description: Transform one comprehensive knowledge file into a traceable roadmap with explicit numbered stage headings, bounded tasks, acceptance gates, and navigable supporting specifications. Use for new projects, large knowledge-to-plan migrations, active-objective packages, or roadmap/documentation audits when readers need to see what is complete, which stage is current, and what comes next.
---

# Make Roadmap Architect

Use this skill to turn **one global knowledge file** about any topic into a coherent, dependency-aware roadmap and a documentation system that humans and AI agents can navigate without repeatedly loading the whole source. The topic may concern software, research, operations, business processes, products, or any other substantial initiative.

**Organize the authoritative execution roadmap by explicit numbered stages as its primary execution headings.** Use `## Stage 1: <outcome>`, `## Stage 2: <outcome>`, and so on, localized to the owner's language, with smaller tasks beneath each stage. Do not replace those headings with concept groups or hide stages in a compatibility table. Concepts and responsibilities organize the supporting specifications, not the primary execution hierarchy.

The output must be more than a prioritized checklist: it needs a first-screen stage overview, visibly numbered stage sections, bounded tasks, explicit transition/acceptance gates and dependencies, plus navigable concept specifications for the current objective. A reader must not need to infer which tasks belong to which stage.

## 1. Core contract

**Required input:** one comprehensive document containing the available knowledge about a single topic, called `global-knowledge.md` below. Its actual filename and location may differ.

**Optional input:** a stated current objective, existing repository or documentation structure, implementation/delivery artifacts, verifiable progress, accepted decisions, and a target documentation directory.

**Required outputs:**

1. A navigable documentation root and AI reading instructions.
2. A short global roadmap router.
3. A concept map covering the entire known subject, with responsibilities and boundaries.
4. Concept-oriented global roadmap files that describe intended outcomes, dependencies, deliverables, risks, and acceptance conditions.
5. A dependency map separating technical prerequisites from stage-transition rules and explicitly authorized parallel work.
6. An active-objective package containing its README, bounded knowledge base, **one authoritative task-status roadmap**, coverage matrix, and concept/responsibility-based delivery documents.
7. Decision records, conflict/uncertainty records, and links to the global knowledge source where useful.
8. A validation report checking knowledge coverage, file ownership, links, and unresolved gaps.

**Language:** use the owner's requested language or the established language of the conversation/project; use English only when neither establishes a preference. Localize stage headings and task descriptions while preserving exact domain identifiers, paths, and stable task IDs.

**Working modes:**

- `create` — build the architecture from the global knowledge file.
- `update` — incorporate source changes or verified progress without overwriting history or accepted decisions.
- `audit` — examine and, when permitted, repair missing traceability, broken routing, competing status owners, and incomplete coverage.

If the full global knowledge file is unavailable, do not fabricate content or claim full coverage. Mark what is unknown and report which input is missing.

## 2. Non-negotiable principles

1. **One canonical global knowledge source.** The supplied source file owns topic meaning, requirements, constraints, and stated behavior. Keep it intact as one file unless its owner expressly authorizes a change.
2. **Stages first, concepts underneath.** The execution roadmap must have numbered stage headings, with each task under one stage. Preserve owner-approved stage numbers/names. Supporting specs may use responsibility-based filenames and stable concept-derived task IDs; these must not obscure the stage structure. Display numbers such as `2.1` may accompany stable IDs without replacing them.
3. **No arbitrary fragmentation.** Split a concept only when its responsibilities are genuinely separable; combine small, tightly coupled concepts when separate files would introduce needless navigation.
4. **Derive; do not duplicate.** Roadmap plans link to original knowledge sections. Optional topic indexes and scoped summaries remain explicitly derived and non-authoritative.
5. **Separate knowledge, planning, execution, and rationale.** Global knowledge says *what is known*. Concept roadmaps explain *what needs to be delivered and what depends on what*. Active specifications explain *how*. ADRs preserve *why* a decision was taken.
6. **One owner per mutable fact.** Each active objective's `roadmap.md` exclusively owns its task statuses and executable task order. ADRs own accepted decision rationale. Other files link to these sources rather than maintaining copies.
7. **Evidence before completion.** A documented capability is not necessarily implemented. Mark work complete only when the required evidence has been verified.
8. **No silent inventions.** Clearly distinguish sourced facts, derived structure, proposals, assumptions, unknowns, and unresolved conflicts.
9. **Explicit stage progression and dependencies.** Every stage has prerequisites, an observable outcome, completion criteria and a transition rule. Do not advance past an incomplete stage merely because later work appears technically independent. An exception requires explicit owner approval recorded in the roadmap. Separate this workflow rule from actual technical dependencies; do not invent code-level dependencies to justify numbering.
10. **Complete bounded active scope.** Document every responsibility needed for the current objective, including prerequisites, failure paths, acceptance, and handoff—not just the next task.
11. **Preserve history and decisions.** Do not discard existing requirements, accepted decisions, completed work, or historical plans. Archive superseded plans and mark them non-operative.
12. **Topic-neutral architecture.** Do not assume a language, stack, company process, release cadence, or particular set of concepts without evidence.

## 3. Recommended documentation structure

Respect an existing project layout where appropriate. Otherwise use:

```text
<project>/
├── AGENTS.md                         # navigation/reading instructions only
├── README.md                         # documentation entry point
└── docs/
    ├── global-knowledge.md           # ONE canonical global knowledge file
    ├── roadmap.md                    # short global roadmap router
    ├── knowledge/                     # OPTIONAL non-authoritative indexes
    │   └── README.md
    ├── roadmap/
    │   ├── concept-map.md            # global conceptual coverage and ownership
    │   ├── dependencies.md           # cross-concept prerequisites and ordering
    │   ├── quality-and-acceptance.md # shared delivery/validation constraints
    │   ├── <concept-a>.md             # one coherent global planning concept
    │   ├── <concept-b>.md             # another concept; NOT a stage
    │   └── legacy/                   # historical versions only
    ├── decisions/                     # cross-objective ADRs, when appropriate
    └── <active-objective>/
        ├── README.md                 # bounded objective and navigation
        ├── knowledge-base.md         # relevant source-derived invariants
        ├── roadmap.md                # ONLY task statuses and execution order
        ├── coverage-matrix.md        # requirements -> tasks -> documents/evidence
        ├── <concept-a>.md            # implementation/delivery responsibility
        ├── <concept-b>.md            # another coherent responsibility
        ├── references/              # auxiliary context, never status authority
        │   └── future-compatibility.md
        └── adr/                      # objective-specific decision records
```

**The placeholder concept files are not mandatory.** Generate only files justified by the source. Use human-readable names based on actual concepts: for example, `identity-and-access.md`, `data-contracts.md`, `error-recovery.md`, `experiment-design.md`, or `customer-feedback.md`. These are examples of naming style, **not** a predefined taxonomy.

Keep supporting filenames descriptive rather than creating one file per tiny step. A concept may support several stages; link its specification from each relevant task without duplicating mutable status. The primary execution roadmap still uses explicit numbered stages and tasks. An active objective remains centered on **one declared outcome**.

For non-software topics, treat “implementation documents” as research specifications, operating procedures, service designs, campaign briefs, or comparable delivery documents. Preserve traceability and evidence requirements.

## 4. Sources of truth and responsibilities

| Document | Owns | Must not own |
| --- | --- | --- |
| `docs/global-knowledge.md` | Topic knowledge, requirements, constraints, explicitly stated goals | Invented progress claims |
| `AGENTS.md` | Reading/navigation rules for agents | Domain facts, decisions, task status |
| Root `README.md` | Documentation map and update workflow | A competing task backlog |
| `docs/roadmap.md` | Short routing overview and a dependency-aware summary | Mutable execution status or long knowledge copies |
| `docs/roadmap/concept-map.md` | Canonical map of planned concepts, scope, and document ownership | Execution checkboxes |
| `docs/roadmap/dependencies.md` | Global dependencies, prerequisite relationships, and sequencing constraints | A total order without justification; live task status |
| `docs/roadmap/quality-and-acceptance.md` | Shared requirements for quality and verification | Concept-specific implementation details |
| `docs/roadmap/<concept>.md` | Planning for one concept: deliverables, boundaries, risks, and acceptance conditions | Duplicated domain definitions or implementation status |
| Active `README.md` | Outcome, scope, non-goals, document route and pointer to next action | Independently maintained next-task queue |
| Active `knowledge-base.md` | Relevant derived rules and invariants | An alternative global knowledge source |
| Active `roadmap.md` | **Only mutable task/stage status, numbered stage headings, task placement, transition gates and executable order** for that objective | Canonical domain definitions and full design contracts |
| Active `coverage-matrix.md` | Traceability across source, tasks, specs and evidence | A second status ledger |
| Active `<concept>.md` | How to deliver/test an objective-specific concept or responsibility | Mutable task-status copies |
| `adr/` and `decisions/` | Decision status, alternatives, rationale, consequences | Claims of completion without evidence |
| `references/` | Auxiliary and future-compatibility context | Scope, active status, accepted decisions |
| `legacy/` | Historical/non-operative records | Live instructions or current task plans |

If source meaning changes, edit the global knowledge file **only with permission**, then reconcile derived files. Without authorization, flag the proposed source change and record the mismatch. If an accepted decision changes, create or supersede an ADR only with appropriate authority.

## 5. Extract global knowledge with provenance

Before creating the roadmap, inventory **the entire available knowledge file**: headings, subsections, appendices, examples, tables, constraints, and exceptions. Process large sources in chunks, but maintain one complete extraction ledger.

Recommended ledger fields:

| Field | Meaning |
| --- | --- |
| `source` | Stable link to original heading/range |
| `statement` | Concise sourced fact, rule, or requirement |
| `type` | Goal, concept, requirement, invariant, dependency, risk, constraint, quality attribute, decision, example, future idea, non-goal, unknown |
| `authority` | Explicit/accepted, proposed, illustrative, conflicting, unknown |
| `concept` | Primary concept/responsibility owning the statement |
| `objective` | Related active objective, future work, or global-only context |
| `destination` | Primary documentation file/section |
| `verification` | Observable criterion or “not yet specified” |

Rules:

- Separate requirements from examples, hypotheses, options, aspirations, and historical notes; examples are not automatically commitments.
- Preserve critical domain distinctions, edge cases, invariants, identifiers, and accepted contracts.
- Consider functional behavior, errors, lifecycle, security, privacy, accessibility, operational constraints, compatibility, testing, and handoff when the source supports them.
- Surface contradictions explicitly and maintain both source references until resolved.
- Use relative Markdown links and, where needed, stable identifiers such as `REQ-001` that do **not** encode delivery stage.
- Do not invent deadlines, costs, team sizes, technology decisions, or completion status; label inferred proposals clearly.
- Every source section must be assigned to a concept or deliberately classified as context, example, history, duplicate, or future reference.

**Ownership rule:** each sourced statement gets a single primary concept owner. Other documents may reference it, but should not claim competing authority.

## 6. Build stage navigation and concept specifications

### Stage structure before file expansion

Extract the owner's existing stages first. Preserve their numbers, names, optionality, approved boundaries and verified completion. If no stage structure exists, propose a small outcome-based structure and label its origin; do not attribute it to the owner. A single bounded task may use one stage, without inventing a multi-stage program.

At the beginning of the authoritative execution roadmap, provide a short overview linking every stage to its actual `## Stage N: ...` heading. Show the current stage and distinguish implemented from verified completion. Keep summary status derived from the canonical task entries in this same file, not independently maintained in routers or specs.

Each stage must state its outcome, included tasks, entry conditions, completion evidence, and rule for moving on. Put supporting technical design in concept specs. Never present a whole large stage as one mandatory implementation invocation: split genuinely separable responsibilities into bounded tasks without fragmenting every field/event into its own task.

### 6.1 Discover concepts and responsibilities

Identify domain concepts from source semantics, not from heading length, chronology, or a generic implementation lifecycle. A concept is a useful file boundary when it has:

1. One clear responsibility or closely related responsibilities.
2. A distinct set of source-linked requirements or planning obligations.
3. Explicit inputs/outputs, dependencies, or interfaces with other concepts.
4. Deliverables and observable acceptance conditions, when specified or responsibly proposed.
5. Enough substance to merit a separate document without creating duplication.

If a concept is too large, split it along genuine responsibility boundaries. If two concepts cannot be explained separately without copying large sections, consider one document. Prefer descriptive slugs, e.g. `dependency-resolution.md`, `session-persistence.md`, `validation-and-recovery.md`.

### 6.2 Create `roadmap/concept-map.md`

Map the **complete known subject**, including work not relevant to the current objective:

| Concept | Responsibility/outcome | Source | Primary global plan | Related concepts | Current objective relevance |
| --- | --- | --- | --- | --- | --- |
| `<concept>` | `<bounded purpose>` | `[GK §...](../global-knowledge.md#...)` | `[plan](<concept>.md)` | `<links>` | In scope / dependency / future / unknown |

The concept map owns the global catalog and file ownership. Avoid treating the table order as execution order. It can be sorted alphabetically or by domain grouping.

### 6.3 Build the dependency graph

Use `roadmap/dependencies.md` to describe relationships among concepts:

- **Hard prerequisites:** one concept cannot be delivered or validated without another artifact/decision.
- **Interface dependencies:** concepts need compatible contracts without necessarily being delivered sequentially.
- **Soft preferences:** useful proposed ordering, not a correctness constraint.
- **Parallel opportunities:** concepts with independent prerequisites.
- **Validation conditions:** evidence required to accept a deliverable or activate a dependent concept.
- **Cross-cutting obligations:** safety, privacy, quality and operational constraints across multiple concepts.
- **Future compatibility:** constraints that should be respected now without implementing deferred work.

Identify circular or uncertain dependencies explicitly and propose investigation or a decision record. Keep the technical dependency graph accurate even when the owner chooses a stricter staged delivery order. Identify owner-approved parallel/cross-stage exceptions explicitly; technical independence alone does not authorize skipping an incomplete stage. For example:

```text
shared contract ──> consuming integration ──> integration verification
       │
       └──────────> persistence behavior
independent monitoring ────────────────────── (parallel where valid)
```

This diagram is illustrative, not a template to impose.

### 6.4 Write one global plan per substantial concept

Each `roadmap/<concept>.md` should describe:

1. **Purpose/outcome:** why the concept matters and what it should enable.
2. **Scope/non-goals:** boundaries and explicitly deferred concerns.
3. **Requirements/source links:** precise source references, without copying the corpus.
4. **Prerequisites and relationships:** required inputs and related concepts.
5. **Work/deliverables:** bounded intended results and links to the stages/tasks they support, not a competing status queue.
6. **Contracts and interfaces:** essential boundaries supported by the source.
7. **Risks/unknowns:** uncertain requirements, open decisions, safety conditions.
8. **Acceptance:** verifiable concept criteria and expected evidence; link to the authoritative roadmap for stage-exit gates.
9. **Ownership links:** associated active-objective specs, ADRs and shared quality rules.

A large concept may have several coherent sub-concept documents; a small one may be covered by the map and a shared document. Do not produce empty files to conform to a template.

### 6.5 Keep planning separate from execution

The global router, concept map, dependency graph, and concept plans show **what exists in the proposed plan and why**. They do not own live checkboxes or task order for the active objective. `docs/<active-objective>/roadmap.md` is authoritative for executable tasks, statuses, and the next eligible action.

Global planning may include future and optional concepts. Their appearance does not imply approval, implementation, or commitment.

## 7. Build the active-objective package with staged execution

Use an objective declared by the owner, if supplied. Otherwise propose the smallest coherent objective that addresses significant uncertainty or delivers useful value and label it **PROPOSED** until approved.

The package must cover the **entire objective**, including prerequisites, shared contracts, normal and error behavior, testing, acceptance and handoff. It must not cover only the next incomplete task. The execution roadmap is stage-first; its supporting work documents are concept-based and bounded to the active outcome.

### 7.1 Bounded knowledge base

`knowledge-base.md` contains the source-backed concepts, rules, invariants and interfaces needed for this objective. Link to exact global source sections, expose assumptions, and distinguish current obligations from future compatibility. Do not reproduce the global corpus or silently introduce new requirements.

### 7.2 One authoritative active task roadmap

`roadmap.md` contains:

- Declared outcome, boundaries and non-goals.
- A first-screen overview linking all stages, their derived progress, and the current stage.
- A real `## Stage N: <outcome>` heading for every stage, localized to the owner's language; a table alone is insufficient.
- For each stage: outcome, entry conditions, smaller tasks, completion criteria/evidence, and transition rule. Optional stages remain explicitly optional and approval-gated.
- Tasks **nested under their stage**, with `### Task N.M: <outcome>` subheadings when several tasks are present and stable IDs such as `DATA-001`, `VALIDATION-002`, or `REPORTING-003`.
- Dependencies between tasks and links to related concept specifications.
- Exactly one canonical status per task, with verification pointers. Stage summaries are derived from those task entries in this same file; a stage cannot be complete while a required task or required verification remains incomplete.
- Acceptance criteria for tasks and the overall objective.
- A **Next eligible action** derived from status and dependencies, rather than copied to other files.

Preserve existing stable IDs. Display stage/task numbers for navigation without silently renumbering stable cross-reference IDs. New IDs may be concept-derived or neutral (`TASK-001`); explain mappings when migrating an existing numbering scheme.

Do not silently redefine a stage to make it appear complete, move unfinished tasks to later stages, promote optional work to required scope, or use technical independence to override stage gates. Record any owner-approved change in the roadmap update log, including its effect on scope and dependencies.

| Marker | Meaning |
| --- | --- |
| `[ ]` | Not started; no verified completion evidence |
| `[~]` | In progress; verified work still remains |
| `[x]` | Completed; acceptance evidence verified |
| `[!]` | Blocked by a named issue, dependency or decision |
| `[?]` | Unknown; status/evidence requires inspection |

Do not claim `[x]` from a plan, mockup, or assignment alone. If execution evidence is not available, start with `[ ]` or `[?]` as applicable. Record invalid test attempts separately where needed.

### 7.3 Active work specifications

Name each file for the **concept or responsibility**, such as `data-model-and-validation.md`, `adapter-boundaries.md`, `error-handling.md`, or `test-strategy.md`. The names must reflect the actual knowledge source and objective, not a predefined methodology.

A specification should state:

1. Covered active roadmap task IDs and exact source knowledge links.
2. Concept responsibility, scope, non-goals and prerequisites.
3. Inputs, outputs, interaction/data boundaries and invariants.
4. Normal behavior and lifecycle, including handoffs.
5. Exceptions, unknown/invalid conditions, cancellation, timeouts and recovery when relevant.
6. Persistence, security, privacy, safety, accessibility and operational implications as applicable.
7. Test/review approach, fixtures or evidence artifacts, and acceptance criteria.
8. Known limitations, unresolved issues, ADRs and related concepts.

A specification may own several related task IDs. Do not duplicate mutable status checkboxes here; link to the active `roadmap.md`.

### 7.4 Roadmap-to-document coverage

Create `coverage-matrix.md` **before or alongside** the detailed work specs. Cover every active task, dependency, acceptance requirement and Definition-of-Done obligation.

| Task ID | Concept | Source evidence | Primary work spec / external artifact | Verification evidence | Gap |
| --- | --- | --- | --- | --- | --- |
| `DATA-001` | Data model | `[GK §...](../global-knowledge.md#...)` | `[Data model](data-model.md#...)` | `<test/review/artifact>` | None / `[?]` |

Each row must have **one primary document or external artifact owner**. It may have additional references, but “covered somewhere in architecture” is insufficient. This matrix provides traceability, **not** additional task statuses.

## 8. Compact templates

Expand templates only where the source warrants more detail. Execution templates use explicit stages; supporting specifications use concepts/responsibilities. Stage/task names and counts come from the approved scope, not a fixed generic lifecycle.

### 8.1 `docs/roadmap.md` — global router

```markdown
# Global Roadmap — Router

This is the entry point to the staged roadmap and supporting concept specifications. Canonical subject knowledge:
[Global Knowledge](global-knowledge.md). Live task statuses and executable
ordering belong only to the [active roadmap](<active-objective>/roadmap.md).

## Current objective
<Named outcome; mark PROPOSED if not confirmed>
[Active objective package](<active-objective>/README.md)

## Stage navigation
Link to the stage overview in the active roadmap; do not maintain a second status table here.

## Supporting concept navigation
| Concept | Purpose | Global plan |
| --- | --- | --- |
| <concept-a> | <intended outcome> | [Concept A](roadmap/<concept-a>.md) |
| <concept-b> | <intended outcome> | [Concept B](roadmap/<concept-b>.md) |

## Dependencies and acceptance
[Concept map](roadmap/concept-map.md) defines concept ownership.
[Dependencies](roadmap/dependencies.md) explains prerequisite relationships.
[Quality and acceptance](roadmap/quality-and-acceptance.md) defines shared rules.

## Source-of-truth policy
No mutable task-status ledger lives here. Historical plans in
`roadmap/legacy/` are not operative.
```

### 8.2 `docs/roadmap/<concept>.md` — concept plan

```markdown
# <Concept> — Global Roadmap

**Source:** [Global Knowledge §...](../global-knowledge.md#...)
**Related concepts:** [<Other concept>](<other-concept>.md)

## Outcome and responsibility
## Scope and non-goals
## Dependencies and interfaces
## Intended deliverables
## Risks, unknowns and pending decisions
## Acceptance conditions and evidence
## Active-objective and ADR links
```

### 8.3 `docs/<active-objective>/README.md`

```markdown
# <Objective> — Delivery Package

## Outcome, scope and non-goals
<Bounded and source-linked objective>

## Reading route
1. [Global knowledge](../global-knowledge.md) — linked sections as needed.
2. [Bounded knowledge](knowledge-base.md).
3. [Active roadmap](roadmap.md) — only status and executable ordering.
4. [Coverage matrix](coverage-matrix.md).
5. Concept specifications relevant to the task.
6. Relevant [ADRs](adr/) and supporting references.

## Next action
See **Next eligible action** in [roadmap.md](roadmap.md), not another queue.

## Completion
<Verifiable objective acceptance and handoff>
```

### 8.4 `docs/<active-objective>/roadmap.md`

```markdown
# <Objective> — Execution Roadmap

<Source-linked outcome and boundaries. State the current stage and link to Next eligible action.>

| Stage | Outcome and section | Derived progress |
| --- | --- | --- |
| 1 | [<First bounded outcome>](#stage-1-first-bounded-outcome) | Derived from its task entries |
| 2 | [<Next bounded outcome>](#stage-2-next-bounded-outcome) | Derived from its task entries |

### Status and transition rules
<One canonical status per task. Complete required tasks and verification before closing a stage. Cross-stage exceptions need explicit owner approval.>

## Stage 1: First bounded outcome
**Outcome:** ...
**Entry conditions:** ...
### Task 1.1: Specific result
- [ ] DATA-001 — <result> | Depends on: ... | [Spec](data-model.md#...)
**Stage completion and transition:** <Observable acceptance and evidence needed before Stage 2.>

## Stage 2: Next bounded outcome
**Outcome:** ...
**Entry conditions:** Stage 1 accepted, plus any actual technical prerequisites.
### Task 2.1: Specific result
- [!] VALIDATION-001 — <result> | Blocked by: <named issue> | [Spec](validation.md#...)
**Stage completion and transition:** ...

## Objective acceptance
- AC-001: <observable result + pass condition + evidence location>

## Next eligible action
<One eligible task in the current stage, or the precise action that resolves its blocker. Do not skip to a later stage without recorded owner approval.>

## Update log
<Optional short changes; never a competing status ledger>
```

### 8.5 `docs/<active-objective>/<concept>.md` — active specification

```markdown
# <Concept> — Delivery Specification

**Roadmap coverage:** DATA-001, DATA-002
**Knowledge evidence:** [Global Knowledge §...](../global-knowledge.md#...)
**Decisions:** [ADR-...](adr/...)

## Scope and non-goals
## Inputs, outputs and boundaries
## Rules, invariants and contracts
## Normal flow and lifecycle
## Invalid, failure and recovery paths
## Security, privacy, safety and operations
## Verification and acceptance
## Open questions and related concepts
```

### 8.6 ADR template

```markdown
# ADR-0001 — <Decision>

Status: Proposed | Accepted | Superseded | Rejected
Date: <only if known>

## Context and source evidence
## Decision or proposal
## Alternatives considered
## Consequences and tradeoffs
## Affected concepts and specifications
## Supersedes / superseded by
```

Never call a decision “Accepted” without evidence of approval or an existing authoritative decision.

## 9. Workflow: create, update, audit

### Create from a global knowledge file

1. Locate the exact source file and documentation destination; do not invent an Obsidian vault, repository or path.
2. Inspect existing documentation conventions, source-of-truth rules, decisions, and verified progress.
3. Inventory every source section and build the extraction/provenance ledger.
4. Distinguish explicit facts from examples, options, historical information, assumptions and future ideas.
5. Discover coherent concepts and define responsibility-based file boundaries.
6. Create a **source-section-to-concept ownership map** and global `concept-map.md`.
7. Preserve or explicitly propose the numbered stage structure; build `dependencies.md` separating technical prerequisites from stage gates and authorized parallel work.
8. Select an owner-declared active objective or propose one and mark it provisional.
9. Draft the complete active task-to-document coverage matrix before expanding spec prose.
10. Generate root navigation, a router linking the stage overview, shared planning rules and source-linked concept specifications.
11. Generate active README, bounded knowledge, authoritative stage-first roadmap, coverage matrix, concept specifications, ADRs and references as appropriate. Verify every stage has a real main heading and every task belongs to one stage.
12. Resolve source conflicts when supported by evidence; otherwise record unresolved decisions and blockers.
13. Verify source coverage, links, unique ownership, factual accuracy, status evidence, file sizes and unrelated file changes.

### Update existing documentation

1. Diff source knowledge against the prior version or identify changed sections.
2. Trace changes through the concept map, dependencies, active tasks, specs, acceptance checks and ADRs.
3. Preserve verified execution statuses; regeneration is not grounds for resetting progress.
4. Reconcile canonical facts first (only with authorization), then derived concept plans and routing.
5. If a concept is split/merged, move content with a coverage map; preserve stable task references or make an explicit migration mapping.
6. Recompute affected dependencies, stage summaries/gates, coverage and the next eligible task in the current stage. Do not change accepted stage boundaries or skip incomplete stages without owner authorization.
7. Archive superseded material if necessary, marking it non-operative; run the full audit.

### Audit an existing architecture

1. Compare every original source section with its conceptual destination.
2. Check that the authoritative execution roadmap has numbered stage headings, nested bounded tasks, a first-screen overview, and explicit completion/transition gates; supporting specs remain organized by coherent responsibilities. A concept-only task list or compatibility table is not a substitute for visible stages.
3. Check all active tasks, prerequisites and acceptance items for primary spec/evidence ownership.
4. Flag duplicate statuses, contradictory authority, invented requirements and unsupported completion claims.
5. Check technical dependencies for cycles, invented constraints and missing prerequisites; separately check stage progression and approval for any cross-stage exception.
6. Check broken links, orphaned documents, stale routers, mixed-responsibility files and oversized duplicates.
7. Return risk-prioritized findings with exact file/section references; modify only what is authorized.

## 10. LLM routing for subsequent sessions

For a routine implementation or delivery task, read:

1. Repository agent instructions, if present.
2. Project `AGENTS.md` and root `README.md`.
3. `docs/roadmap.md` and any relevant concept/dependency router.
4. Active objective `README.md`, `knowledge-base.md`, and `roadmap.md`; identify the current stage and selected task before loading deeper specifications.
5. The **specific concept specification** mapped to the task in `coverage-matrix.md`.
6. Relevant linked global knowledge **sections** and ADRs.

Read the **whole** global knowledge file for initial roadmap creation, complete source coverage audits, or global semantic revisions. Do not routinely load it in full for every active task. If no execution task is requested, stop after the routers and active overview.

`AGENTS.md` must stay navigational; do not store product meaning, accepted technical contracts, or mutable task status inside it.

## 11. Quality checks and definition of done

- [ ] Exactly one canonical global knowledge file remains intact and identifiable.
- [ ] Every source section is mapped to a concept or explicitly classified as context/history/example/future reference.
- [ ] Every substantive planned requirement links to source evidence or is clearly marked proposed/assumed.
- [ ] The execution roadmap has explicit numbered main headings for every stage, with a first-screen overview and smaller tasks under their owning stage; supporting filenames/specifications remain coherent and descriptive.
- [ ] Every stage has entry conditions, an outcome, completion evidence and a transition rule; verified completion is distinct from implementation.
- [ ] No incomplete stage is bypassed, silently narrowed, or relabeled complete without an explicit authorized change; optional stages remain optional.
- [ ] The global concept map covers the complete known subject, not just the current objective.
- [ ] Every substantial concept has a bounded outcome, dependencies, intended deliverables and verifiable acceptance conditions.
- [ ] Technical prerequisites are explicit and distinct from staged workflow gates; independent work/cross-stage exceptions are identified without inventing authorization.
- [ ] Exactly one active roadmap owns mutable task status and executable ordering for its objective.
- [ ] Every active task and objective acceptance condition maps to a primary spec or external evidence artifact.
- [ ] The active package covers its entire objective, including prerequisites and non-happy-path behavior where relevant.
- [ ] Completion claims are supported by evidence, not by documentation alone.
- [ ] Optional or future ideas have not silently become near-term obligations.
- [ ] ADRs distinguish proposed from accepted decisions and preserve historical rationale.
- [ ] Derived knowledge does not compete with the original source; routers are concise.
- [ ] The active README and active roadmap agree on the outcome and authoritative next action.
- [ ] Relative Markdown links resolve from the containing files.
- [ ] No active file is needlessly longer than about 300–500 lines; split at real conceptual boundaries when useful.
- [ ] Historical materials are clearly separated from operative plans.
- [ ] No unrelated files were moved, deleted, or changed.

Reuse owner-approved stage names/numbers and project conventions while preserving source-of-truth authority and traceability. Never replace requested stage headings with a concept-only roadmap merely because the supporting specifications are concept-based. A flat concept list with hidden stage mapping fails this skill's readability contract.

## 12. Final response contract

After creating, updating, or auditing the documentation, report:

1. **Knowledge input:** exact canonical file and whether complete coverage was possible.
2. **Generated structure:** created/modified files, including the global router, concept map, and active roadmap.
3. **Stage overview:** stage count, current stage, accepted completion evidence, and smaller tasks; link supporting concepts and important dependencies without creating another status ledger.
4. **Traceability:** source-section coverage and active task-to-document coverage, with counts when feasible.
5. **Open issues:** assumptions, unresolved decisions, blockers, missing source information and evidence gaps.
6. **Next actionable step:** link to the one authoritative active roadmap; do not create a duplicate queue.

Never say that implementation, operations, research, or a launch is complete merely because its documentation has been generated.
