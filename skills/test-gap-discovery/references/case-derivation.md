# Derive use cases from behavior, not from branches

The primary unit is a **checkable user/system scenario**, not a method or `if` statement. Start with the observable obligation, then vary the situation in which it must hold.

## Derivation loop

1. Identify an operation, workflow, domain rule or inter-component seam and a valid independent oracle.
2. Phrase the obligation: **Given** relevant state and permissions, **when** an action/sequence occurs, **then** an externally visible property holds.
3. Challenge its assumptions with relevant dimensions below. Consider *combinations* of dimensions, not just one example per branch.
4. Pick cases that distinguish plausible wrong behaviors from correct ones. Plan fixtures and timing after selecting the cases.
5. Inspect implementation/tests to locate seams, prepare safe execution and spot hypotheses; do not substitute current outputs for expected outputs.

## Scenario dimensions (use only those relevant to the scope)

| Dimension | Concrete contrasts to explore |
| --- | --- |
| Input classes | Valid/invalid; optional/missing/null; supported/unsupported types; whitespace/Unicode; malformed payload |
| Boundaries | Empty, one, many; min/max and just outside; off-by-one; overflow; zero length; partial input |
| State | New/existing/deleted; initialized/partially initialized; stale cache; duplicate identifier |
| Sequences | A→B versus B→A; repeat operation; undo; cancellation; partial order; replay |
| Idempotence | Same request twice; retry after lost response; duplicate events; at-least-once delivery |
| Failure | Timeout; downstream rejection; disconnect; partial write; malformed dependency response |
| Recovery | Resume after crash; rollback/compensation; consistency after interruption |
| Concurrency | Simultaneous updates; lost writes; read-after-write; duplicate creation; resource contention |
| Access | Anonymous/authorized/forbidden; ownership changes; cross-tenant leakage |
| Compatibility | Old data or API client; schema change; mixed versions; invalid persisted state |
| Resource/scale | One versus large input; limits; pagination edges; bounded memory/time |
| Adversarial | Untrusted paths, control characters, injection, huge input, traversal, hidden mutation |
| Observability | Durable effect, correct error, audit event or contract-visible response — not just absence of an exception |

### Use high-yield interactions

When combinations matter, use targeted pairwise or risk-driven multi-variable cases. Examples: *retry after timeout with stale state*, *concurrent update while permissions change*, *empty input during recovery*. Avoid Cartesian explosion unless systematic generation is justified.

### Prioritize

Consider impact if wrong × likelihood × defect-detection power against cost/flakiness. Security, data integrity and external-contract seams may outrank convenient units. Do not impose a fixed number of scenarios and never count a hypothetical case as a discovered defect.

### Example (generic)

For an operation that should create one unique resource per idempotency key, do not just check that a single valid request returns success. Test a repeated request, a timeout before acknowledgement followed by retry, two concurrent requests with the same key, and different keys. Assert the *durable* result, not how many times a mock was called. Only use these cases if the key's semantics are actually specified.
