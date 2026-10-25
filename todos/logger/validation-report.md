# Documentation migration validation

## Input and scope

- Complete input: the 216-line `todos/logger.md`, moved intact to [logger.md](logger.md).
- Additional authority: the owner's request to archive that file and maintain the roadmap; repository testing policy; inspected current source boundaries; existing foundation verification artifacts.
- Applied skill: [make-roadmap](../../skills/make-roadmap/SKILL.md), read directly from the repository because the skill-loading tool did not expose it.
- The initial migration used the skill's former English/concept-only defaults. The owner's later correction supersedes that execution layout: the active roadmap now uses Polish numbered stage headings and smaller numbered task subsections; stable IDs and concept specifications remain intact.

## Coverage and ownership audit

- [Concept map](concept-map.md) accounts for the full source: requirements, accepted contracts, examples, history, superseded statements, current evidence, limitations, unknowns and optional export.
- [Coverage matrix](coverage-matrix.md) maps all 11 task IDs to a primary concept specification and verification responsibility.
- [Roadmap](roadmap.md) is the sole mutable execution/status owner. The archive remains frozen; reading instructions route normal work to current derived contracts instead.
- Eight concept specifications cover the whole known topic. README serves as the compact global router and active-objective entry point, avoiding duplicated global/active file trees for this single objective.
- Stable task IDs and supporting filenames remain responsibility-based. Stages 1–5 are now the actual primary execution headings, not merely a compatibility/provenance table.

## Evidence and limits

The existing foundation logs were inspected: required gate 109/109 plus `GATE_PASS`; Rust 1.85.0 109/109 plus `MSRV_PASS` and `SOURCE_AND_ISOLATION_PASS`. Those artifacts support the imported foundation completion, not future producer tasks. No application tests were rerun or claimed by this migration.

The roadmap does not claim every future task is implementation-ready. STREAM-BOUNDARY, NOVAMIRA-CHANNEL, HANDOFF-CONTRACT and EXPORT-APPROVAL remain explicit gaps; affected tasks must not be started by guessing. Resolve them through the authority/evidence specified in [decisions-and-gaps.md](decisions-and-gaps.md).

Validation for this documentation-only change checks unchanged archive bytes, relative file/heading links, task-to-spec coverage, one status owner, absence of active links to the old location, and whitespace. The original pathname above is migration provenance, not an operative link. This report is not evidence of application behavior beyond the existing artifacts.

## Migration check record

- The first link check found one incorrectly formed archival heading anchor; it was corrected without editing the archive.
- Archive SHA-256: `44c4734c04633d09bd38f2f1bce51c3ece8e6a0d0bd222127e8946576f18665a`; compared byte-for-byte with the original Git version before the move.
- Final structural check: **PASS** — 19 Markdown files, 192 local links/heading anchors, 11 unique task IDs with matching coverage rows, foundation as the only completed task, and the absence of the old file.
- Whitespace and single-status-owner checks: **PASS**. All moved/new Markdown files were checked, including files not yet tracked by Git; scoped `git diff --check` also passed. No application code, tests, dependencies or runtime configuration were modified.

## Stage-readability correction

The owner explicitly requested stages as main headings and corrected the skill target to `make-roadmap`, not `make-plan`. The roadmap now contains real stage headings, a linked first-screen overview, six visible tasks under stage 2, acceptance/transition criteria, and a single next task. Supporting links and the source-of-truth instructions were reconciled; the archived input and all stable task IDs/statuses are preserved.

The source `skills/make-roadmap/SKILL.md` now requires that structure in its description, rules, execution template, update/audit workflow and final checks. It no longer prohibits stages or silently substitutes concept-only grouping. `make-plan` remains unchanged. This is an instruction/content correction, not a claim of running a new agent-skill evaluation or application tests.

Correction checks: **PASS** — 20 Markdown files including the skill, 198 non-template local links/anchors, five real stage headings, six task subsections within stage 2, completion/entry gates, and all 11 original task IDs/statuses preserved. Whitespace, unchanged archive SHA-256, and an empty `git diff` for `skills/make-plan` were also confirmed. No new application tests were run for this documentation/instruction-only correction.
