# Optional repo-specific testing vision (only on explicit request)

A testing vision normally belongs in the **conversation**, not automatically in a Markdown report. If the user explicitly asks to save a document, adapt these prompts to the actual repository; omit irrelevant sections.

1. **Observable obligations.** What are the critical actions, data invariants, public contracts and integration seams?
2. **Existing test quality.** Which concrete tests assert behavioral outcomes? Which mirror internals or hide dependency effects? Cite real files/examples, not invented numbers.
3. **High-yield scenarios.** Provide contrasting positive, negative, boundary, transition and failure cases appropriate for each critical workflow.
4. **Test levels.** What belongs in unit, integration, contract or end-to-end tests? What must stay real and what can safely be mocked?
5. **Fixtures and isolation.** Deterministic state, clocks, seeds, cleanup, safe dependencies and concurrency simulation.
6. **First useful improvements.** Prioritized, actionable tests that could expose meaningful errors; describe what each is meant to distinguish.

Only include actual observations and grounded examples. Never add a generic risk inventory, fake findings or a mandatory backlog to satisfy the template.
