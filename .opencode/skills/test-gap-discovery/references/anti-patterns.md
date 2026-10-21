# Anti-patterns: tests fitted to the implementation

| Pattern | Warning sign | Better approach |
| --- | --- | --- |
| Branch mirroring | One case copied from each `if` or `match` arm | Derive cases from contract/input/state space first |
| Self-oracle | `expected = implementation(x)` or duplicate algorithm | Use independently known example, invariant or reference |
| Mock-everything | Test only asserts calls to preprogrammed mocks | Assert real, observable effects at the seam in question |
| Over-specified internals | Private fields, incidental call sequence, noncontract logs | Assert semantics stable under correct refactoring |
| Happy path only | One valid request with one ideal dependency | Probe boundaries, error paths, retries and state transitions |
| No substantive assertion | Only `not throw`, `defined`, or successful mock invocation | Assert value, durable state, negative side effect or error contract |
| Snapshot-as-spec | Captures current behavior without independent review | Validate externally or use a stronger invariant |
| Green-at-any-cost | Weaken asserts, skip failures, over-stub | Diagnose expectation, setup and product behavior independently |
| Forced red-first | Treats all green-on-first-run tests as useless | Verify sensitivity when justified; passing regression guards are valid |
| Coverage theater | Maximize percentage/test count | Explore overlooked behavior and plausible faults |
| Flake camouflage | Arbitrary sleeps/retries, shared fixtures | Control clock/scheduling and isolate state |
| Artificial findings | Report an unsupported “gap” to fill a template | Zero confirmed findings is legitimate |

Treat these as diagnostic smells, not automatic convictions. Point to a real test and the behavior it misses before recommending changes.
