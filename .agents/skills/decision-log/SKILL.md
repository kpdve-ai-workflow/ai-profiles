---
name: decision-log
description: Record substantive ai-profiles changes and decisions as factual entries in decisions/. Use when documenting repository work; read-only investigation alone does not require an entry.
---

# Decision log

Maintain a continuous record of substantive work and decisions in root `decisions/`, independently of the existing `changelogs/` handoff worklogs. Related work may be grouped; there is no required number of entries per session or commit. Record actual changes and reasons, never secrets or invented facts.

Read [the format and template](references/template.md) and [allowed verbs](references/verbs.txt) before creating an entry. Choose the ticket (`none` if absent), scope, and a descriptive slug. From the repository, run:

```bash
./scripts/create-agent-decision.sh <ticket> <scope-prefix>-<verb>-<object>
```

The helper prints the path of an exclusively created empty file. Write the frontmatter and bullet list to that exact file. Set `scope` to the affected ai-profiles area and ensure its first segment matches the slug prefix. Check existing entries for context, duplicate ticket and slug, and valid `supersedes` references. The helper does not validate the content; review it against the template before finishing. Report the created path.
