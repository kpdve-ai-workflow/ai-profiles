---
ticket: none
scope: repo
summary: Add a continuous decision log
why: Preserve changes and their reasons alongside existing agent handoff worklogs
---

- Added a decision-log skill with the gon-stack entry format and allowed verbs
- Added create-agent-decision.sh to create timestamped decision files safely
  - Rejects invalid ticket or slug and duplicate ticket and slug combinations
- Kept changelogs for agent handoffs and decisions for the ongoing history of changes
- Removed the requirement for one decision per session or commit
- Content validation and Git hooks are not installed
