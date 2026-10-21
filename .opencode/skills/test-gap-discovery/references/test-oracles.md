# Test Oracles: how to know what is correct

A test oracle is an independently justified expectation about **observable behavior**. The current implementation is the subject under test, not the authority for its own correctness.

## Useful sources

| Source | Examples | How to use |
| --- | --- | --- |
| Public contract | CLI help, API schema, SDK docs, type/validation rules | Assert inputs, outputs, error semantics, compatibility |
| Product requirement | Acceptance criteria, issue, user flow, PRD | Derive externally visible cases and counterexamples |
| Domain invariant | Ownership, uniqueness, accounting balance, no lost data | Assert properties across varied inputs and states |
| Protocol or standard | RFC, format spec, database guarantee | Exercise conforming and malformed cases |
| Human-validated example | Hand-reviewed sample or golden fixture | Assert exact values only if independently validated |
| Prior release or reference | Published behavior, independent parser/client | Differential test when compatibility is required |
| Security model | Tenant isolation, permissions, least privilege | Try valid and invalid actors and scopes |
| Incident or verified bug | Reproduction and root cause | Preserve regression with explicit failure signal |
| Metamorphic relation | Sort twice = sort once; round trip; monotonicity | Assert relation instead of a guessed exact output |

## Independence test

Before trusting an expectation, ask:

- Who chose it, and why should it be true if the production function were wrong?
- Would it remain valid after a correct refactor?
- Is the assertion about a user-observable result or only an internal implementation detail?
- Could the expected value share the same defect as the actual result?

Do not compute expected output by calling the function under test or duplicating the same algorithm line-for-line. A current snapshot is a change detector unless a separate oracle validated it.

## Missing or conflicting oracles

Not every behavior has a fully documented contract. First consider general safety/consistency invariants or metamorphic relations that do not need inventing business rules. If the intended behavior genuinely cannot be determined, state the ambiguity briefly when relevant and avoid marking it as a defect. Conflicting sources require a decision, not a test that arbitrarily blesses one side.

A test file does not need bureaucratic oracle IDs or citations for every assertion. Preserve enough context (clear name, nearby source link/comment when non-obvious) to let another developer understand why the expectation is correct.
