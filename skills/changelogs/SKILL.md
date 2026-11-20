---
name: changelogs
description: Create a factual agent worklog after a task actually modifies, adds, or deletes repository files. Do not use for questions, investigation, diagnostics, review, or planning without repository changes.
---

# Changelogs — timestamped agent worklog and handoff

## Purpose

Create **one new worklog file per task/session handoff that changed repository files** in `changelogs/`. The agent writes the actual work summary; the helper supplies the real timestamp and creates an empty file. Worklog creation itself and pre-existing changes do not qualify as task changes.

The requested human-readable timestamp and normal filename format is:

```text
MM-DD-YYYY-HH:MM-agent-worklog.md
```

Example:

```text
10-07-2026-13:05-agent-worklog.md
```

Use the same timestamp in the worklog heading:

```markdown
# Agent Worklog — 10-07-2026-13:05
```

`/` cannot appear inside a filename because it separates directories, so the date uses `-`. `:` is retained between hour and minute as requested. If a filesystem rejects `:` (for example, Windows filesystems), the helper uses `MM-DD-YYYY-HH-MM-agent-worklog.md` and reports that fallback; the worklog heading still uses the canonical `MM-DD-YYYY-HH:MM` timestamp.

This is an internal session handoff journal, not a user-facing release changelog.

## Mandatory invocation point

Follow this skill before ending, yielding, or handing off a task only if that task actually modified, added, or deleted repository files other than the worklog itself. This applies to completed and unfinished tasks with such changes.

Questions, explanations, code review, investigation, diagnostics, planning, and decisions without repository changes must not create a worklog, regardless of useful findings or unfinished work. Pre-existing worktree changes do not trigger this skill.

Invoke it **after the work and relevant verification are finished**, so the log reflects the actual state. Create an entry only for a task/session handoff that meets the repository-change condition above.

## Procedure

1. **Read the prior handoff.** Find the most recently modified `changelogs/*-agent-worklog.md` and read it before resuming unfinished work. Filenames are not chronologically sortable across months/years; select by file modification time, not lexical name order. Verify the log against current Git state and files.
2. **Reconcile the repository.** Inspect `git status`, relevant diffs, and new files. Distinguish this task's changes from pre-existing worktree changes.
3. **Verify before recording.** Run relevant checks and record actual outcomes. If a test was not run, say so and why. Never infer a passing test from an agent's statement or a previous session.
4. **Create a fresh file with the existing helper.** From the repository, run:

   ```bash
   ./scripts/create-agent-worklog.sh
   ```

   It creates an empty file and prints its path and timestamp. Write the entry into that exact file. Do not append to an older worklog, invent a timestamp, or overwrite an existing entry. On a same-minute collision, the helper waits until it can create a unique timestamped file. If the filesystem rejects `:`, use the reported portable filename fallback and keep the canonical timestamp in the heading.
5. **Make the handoff actionable.** State current status, exact remaining work, and the first concrete action the next session should take. If there is no follow-up, say so explicitly.
6. **Protect sensitive data.** Never write tokens, passwords, API keys, cookies, private keys, `.env` contents, or other secrets. Redact sensitive command output and MCP arguments. Do not paste a full diff into the log.
7. **Report the file.** In the final response, mention the generated worklog path and task status. If it could not be written, say so; never claim that a handoff was logged when it was not.

## Entry format

Use this structure. Omit fields only when genuinely unavailable; mark them `unknown` or `not applicable` rather than inventing values.

```markdown
# Agent Worklog — MM-DD-YYYY-HH:MM

- **Task / objective:** ...
- **Status:** `COMPLETED`, `PARTIAL`, or `BLOCKED`.
- **Timestamp:** actual local date/time and timezone, if available.
- **Agent/session:** Pi session ID if available; otherwise `unknown`.
- **Branch / base:** branch and relevant base/head SHA if available.
- **Outcome:** concise statement of what is now true.

## Changes
- `path/to/file`: what changed and why; mention important behavior or contract.

## Decisions and context
- Decisions, assumptions, constraints, and findings needed to continue.

## Verification
- `command`: actual result and exit status.
- Not run: what was not verified and why.

## Current state
- Relevant uncommitted changes, worktree, running services, or preserved artifacts.
- Clearly identify pre-existing changes that this task did not make.

## Next session
1. First concrete action.
2. Following action or acceptance check.

## Blockers / risks
- Active blockers or `None`.
```

## Quality bar

A later agent should be able to answer from the latest worklog:

- What was the task, and what changed in each relevant file?
- What was tested, with what result, and what remains unverified?
- What decisions or constraints must not be rediscovered?
- Is there unfinished work, a blocker, or a dirty worktree?
- What exact first step should the next session take?

Prefer concrete paths, command names, identifiers, and observed results over vague phrases such as “continued setup” or “tests should pass.”
