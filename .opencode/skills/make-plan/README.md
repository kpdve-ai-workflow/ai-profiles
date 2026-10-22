# Make Plan Skill v2

A replacement for the existing `.agents/skills/make-plan/` skill. The skill instructions are in English; plans are written in the user's language.

## Installation

Copy the `make-plan` directory to `.agents/skills/make-plan/` in your repository (back up your existing skill if needed).

## How to use

This skill now produces test-first implementation plans. For behavior changes it uses `test-gap-discovery` to derive independent scenarios and expected outcomes, and places tests before implementation in the plan. Planning itself does not authorize running tests or changing code.

Ask the agent to use `make-plan` when drafting or refactoring an implementation plan. In Pi, use `/skill:make-plan` if your setup exposes installed skills via that command.

Example request:

> Use make-plan. Restructure the existing F1-3 plan without changing approved scope. Separate user decisions, verified technical findings, engineering decisions, tasks, and risks. For behavior changes, derive independent scenarios/oracles with test-gap-discovery and plan focused tests before production implementation. Preserve current task IDs, execution status and pending manual acceptance. Do not modify code or run tests.

The core rules live in `SKILL.md`. The agent loads `references/plan-layout.md` when producing a plan; `examples/mcp-reorganized-excerpt.md` is an optional example, not mandatory context.

## Scope

This skill generates **implementation plans**. It is not a replacement for `make-docs`, which can remain dedicated to permanent, as-built harness documentation.
