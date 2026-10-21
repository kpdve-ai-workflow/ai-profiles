# Techniques for discovering genuine gaps

Use the smallest technique that distinguishes correct from plausible incorrect behavior.

## Property-based testing

Generate many inputs while checking a **domain invariant** independent of the algorithm. Shrink counterexamples and persist seeds. Good for parsers, serializers, sorting, validation and transformations. Generated volume is not itself evidence of quality.

## Metamorphic testing

Where exact output is hard to predict, assert a justified relation: encoding then decoding preserves valid data; applying an idempotent operation twice has the same observable effect; changing irrelevant metadata does not change authorization. Verify that the relation genuinely follows from the contract.

## Model / state-machine testing

Represent meaningful states and allowed transitions; generate sequences rather than isolated calls. Useful for retries, lifecycles, persistence, cancellations and cache coherence. Keep the model independent from production control flow.

## Differential testing

Compare behavior against a trusted version or independent implementation where compatibility is a requirement. Compare public semantics, not incidental ordering or message formatting unless specified. Be mindful that the reference may also contain defects.

## Contract and integration testing

Exercise actual protocol boundaries, serialization, persistence semantics and side effects when those are the behavior at risk. Mocks at those same boundaries can hide the defect. Use isolated test services rather than production systems.

## Fuzzing and adversarial tests

Explore malformed/truncated/oversized data and unusual encodings; record a minimized reproducer. Treat a crash as a lead that still needs interpretation. Add resource limits and never fuzz production endpoints without explicit permission.

## Mutation / sensitivity check

A test can pass on the current correct implementation and still be valuable. When appropriate, ask whether it detects a plausible semantic fault. An isolated, reversible mutation or mutation tool can support this. A surviving mutation can mean a weak test, an irrelevant mutation, an equivalent mutant or unexecuted path; investigate rather than automatically calling it a bug.

Do not mutate shared worktrees, production code in-place, or untrusted services. This skill's helper only detects available tools; it **does not** execute mutation tests.

## Coverage

Line and branch coverage are maps of what executed, not proofs of correctness. Use missing coverage to inform exploration, never as a target that dictates low-value test generation.
