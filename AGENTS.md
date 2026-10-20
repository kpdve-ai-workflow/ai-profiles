# Repository guide

- Python 3.10+; runtime and tests use only the standard library.
- Verify changes with `make validate` and `make test` (`python3 -m unittest discover -s tests -v`). Tests use temporary repo/project copies; never test against a real project or database. Mock Docker/downloads and DDEV/WP-CLI where needed.
- Preserve project-owned data and safety behavior. `.opencode/ai-profile.local.json` is user-owned; never replace it with generated configuration. Avoid hand-editing generated entries in project `opencode.json`.
- Installing/updating profiles must not execute user tools in `profiles/<name>/scripts/`. WordPress Updraft restore defaults to a read-only plan; database replacement requires `--apply`, project-name confirmation and a verified local backup. See [WordPress documentation](profiles/wordpress/README.md) before working on its restore tooling.
- Follow [profile management](docs/profile-management.md) and [MCP documentation](mcp/README.md) for repository conventions and component details. Bump affected profiles deliberately when changing delivered profile content; versions are independent.
- Documentation changes belong in the README of the relevant area. Update the root `README.md` only when repository-wide introductory information changes, and `AGENTS.md` only when agent rules change. Do not add task reports or change summaries to either file.
