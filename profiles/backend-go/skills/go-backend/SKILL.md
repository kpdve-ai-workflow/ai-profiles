---
name: go-backend
description: Go backend development including API design, Go idioms, tests, SQL and PostgreSQL integration.
metadata:
  opencode/slash: "true"
---

# Go backend

- Prefer simple, idiomatic Go, explicit dependencies, clear package boundaries and context propagation.
- Define API contracts, validation, error behavior, logging and tests before implementation.
- Propose DB migrations and SQL separately, and obtain explicit approval before any migration.
- Respect the project's PostgreSQL MCP access mode: `unrestricted` permits reads and writes (subject to database grants), while `restricted` is read-only. Do not assume read-only access unconditionally.
- Read/write access is not blanket authorization: perform only requested changes and obtain explicit approval for destructive operations or migrations.
- Use `make-plan` and `refine-plan` when planning is requested; routine implementation does not require a separate unlock.
