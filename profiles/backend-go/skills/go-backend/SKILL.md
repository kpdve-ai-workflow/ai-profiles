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
- Treat the PostgreSQL MCP as **read-only** context; never perform database writes or schema changes from MCP.
- Use `make-plan` and `refine-plan` when planning is requested; routine implementation does not require a separate unlock.
