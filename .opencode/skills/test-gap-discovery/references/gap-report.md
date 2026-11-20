# Optional finding notes (NOT a required output)

Only use this reference if the user explicitly requests a formal artifact or the repository's existing task contract requires one. The default skill behavior is **not** to create reports, YAML registers, dashboards, backlog files, or a fixed list of findings.

A verified issue should minimally include: affected behavior; independent basis for the expectation; reproducible inputs/steps; actual vs expected result; and the smallest meaningful evidence (command/output, failing test or observed state). Prefer linking to the test itself over duplicating logs.

Useful distinctions *only when relevant*:

- **Confirmed defect:** reproducible contradiction of an independently justified requirement.
- **Ambiguous requirement:** desired behavior not established; needs owner decision, not a fake bug.
- **Test infrastructure limitation:** behavior cannot be safely exercised with available tools/permissions.
- **Known accepted behavior:** supported by a concrete design decision, not invented acceptance.
- **Untested hypothesis:** plausible concern not verified; label explicitly if communicating it at all.

Never invent a minimum number of issues or mark a task incomplete simply because no defects were found. No mandatory severity classifications, case counts, evidence identifiers or sections.
