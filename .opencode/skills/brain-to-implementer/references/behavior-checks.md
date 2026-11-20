# Persistent handoff regression contract

## Source and method

These expectations come from the user's request to replace a monolithic prompt with a persistent, incremental handoff and the selected contracts A–E in that request. They are a documentation regression contract, not an automated behavioral harness. No model execution was performed. Static review can establish instruction/template consistency, not prove that an agent will comply.

The baseline below was recorded before editing `SKILL.md`. Baseline references identify its original headings and wording, not observed agent responses. “Absent” means no explicit supporting instruction, not evidence of a behavioral failure.

| Scenario | Expected outcome and source | Baseline static review |
| --- | --- | --- |
| 1. Multi-part feature | Shared context in `plan.md`, several bounded task files; no enormous “implement solution” task (B, C). | Contradiction: role, workflow step 6 and required output demand one complete prompt; no persistent package. |
| 2. First execution | Select T-01, mark IN PROGRESS, record results, stop with T-02 unstarted (D). | Contradiction: output procedure says continue until fully integrated; no one-task stop rule. |
| 3. Fresh-session continuation | Read persisted queue, evidence and next action without chat history (B, D). | Absent: no persisted state or continuation procedure. |
| 4. Interrupted task | Resume that task from recorded state and actual working-tree changes; discard nothing (D). | Absent: no interruption recovery rule. |
| 5. Required test unavailable | Unchecked task, exact blocker recorded, no advance (C, D). | Absent: verification requested, but no blocked-state/queue gate. |
| 6. Code implemented, manual acceptance pending | IMPLEMENTED / MANUAL PENDING, unchecked, preserve progress separately from acceptance (D). | Absent: no separate status dimensions or acceptance gate. |
| 7. Directory already exists | Preserve it; new request uses first available numeric suffix; revise only explicitly selected handoff (B, E). | Contradiction: blanket prohibition on file edits prevents persistence; collision handling absent. |
| 8. Essential planning evidence missing | Affected work blocked; Brain resolves choices instead of delegating guesses (A, C). | Contradiction: workflow step 2 and quality rules allow discovery instructions for unknown context instead of blocking essential gaps. |
| 9. Small documentation request | Proportionate bounded task(s), content/consistency checks; no artificial tests or forced RED (C). | Partial support: adapt detail and avoid padding; documentation-specific verification absent. |
| 10. All tasks complete | Report completion; no repetition or invented work (D). | Absent: no persistent completion/next-action rule. |
| 11. Analyze and wait | No handoff writes or implementation (A). | Partial support: “unless explicitly asks otherwise”; activation still frames the next message as a change request. Needs explicit analyze/wait gate. |

## Review procedure after an edit

1. Read `../SKILL.md` and `handoff-layout.md` in full. Trace each scenario to an explicit instruction and, where relevant, its template counterpart.
2. Check status vocabulary, checkbox gating, single next action, stop conditions, authority boundaries, collision handling, continuation, and publication/fallback consistency.
3. Inspect new files separately: ordinary Git diffs omit untracked contents. Run `git diff --check -- .opencode/skills/brain-to-implementer` from the project root.
4. Report actual static findings and limitations separately from behavioral observations. Do not label these scenarios as executed model tests. Do not add a testing framework solely for this change.

## Updated-instruction static review

All eleven scenarios have explicit instruction coverage in the updated documents, as mapped below. This is a static consistency finding only; no Implementer session, sample feature, or live `todos/` package was executed/generated to test compliance.

| Scenario | Updated instruction / template coverage |
| --- | --- |
| 1 | Skill “Workflow” decomposition; layout “Placement and ownership,” shared contracts and bounded task template. |
| 2 | Skill “Persistence and execution rules”; reusable prompt steps 1–8 select, mark, record, advance pointer, and stop before another task. |
| 3 | Plan current state/queue/evidence plus reusable prompt step 1 explicitly rejecting reliance on chat history. |
| 4 | Skill interruption rule and prompt step 3 require persisted state and actual working-tree inspection. |
| 5 | Layout status contract and prompt stop rules keep unavailable required verification blocked, unchecked, and unadvanced. |
| 6 | Skill and layout status contracts plus prompt acceptance rule explicitly preserve IMPLEMENTED / MANUAL PENDING unchecked. |
| 7 | Skill persistence workflow and layout placement rules require suffixes and explicit selection for revision; preserve evidence/IDs. |
| 8 | Skill design gate and layout readiness/blocker rules require exact missing evidence, affected tasks, and resolution instead of delegated guessing. |
| 9 | Skill decomposition/verification rules and layout documentation adaptation permit one task and content/link/consistency checks without forced RED. |
| 10 | Skill next-action rule, layout next-action guidance, and prompt step 1 stop at reporting completion. |
| 11 | Skill “Role and authority” and final self-check explicitly honor analyze/discuss/wait without file creation or implementation. |

Publication/fallback review also found explicit rules for actual project-relative paths, resolving generated links/references, replacing template fields, language, narrow Brain write authority, and truthful reporting of denied or partial writes. Template placeholders are intentional in this reference; generated packages must replace them. Static review does not verify that a model will perform those substitutions or honor the stop rules.

## Testing-policy extension — separate static review (2026-10-22)

The baseline and eleven-scenario review above are historical records and remain unchanged. The scenarios below come from the subsequent request for a mandatory `test-gap-discovery` testing policy. They check written instructions and templates only, not model behavior. No Brain/Implementer session or application test was executed for this review.

| Scenario | Required static contract | Reviewed instruction / template coverage |
| --- | --- | --- |
| Planning and execution design or review tests | Both apply the same independent-oracle, risk-based methodology and read the target project's testing policy. | Skill “Workflow” and “Task and verification quality”; layout testing-policy guidance, shared contract, task verification fields and reusable prompt step 3/methodology. |
| Companion skill unavailable | Report absence; methodology and project rules still apply, without requiring an installed skill. | Skill authority/workflow and prompt requirements; layout pre-scenario guidance, shared contract and reusable prompt step 3. |
| Test of existing correct behavior is GREEN immediately | No artificial RED or broken production code; distinguish this from test-first for new/repaired behavior. | Skill verification sequence; layout task-sequence adaptation and reusable prompt GREEN exception. |
| Required verification unavailable after edits | Task remains unaccepted and unchecked; record blocker and actual results while preserving implementation progress, with no queue advance. | Skill persistence rules; layout status contract and reusable prompt unavailable-verification gate. |
| Handoff targets another project | Discover its own policy, environment, commands and isolation; no mandatory ai-profiles VM, Make commands or documentation path. | Skill workflow/project-policy rule; layout placement/testing guidance and project-specific policy fields. |

Static review found explicit coverage for these five scenarios in the locations above, while retaining the one-task-per-invocation rule and separate implementation/verification statuses. This establishes textual consistency only; it does not demonstrate agent compliance or application correctness.
