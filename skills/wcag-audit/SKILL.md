---
name: wcag-audit
description: Run a scoped, evidence-based WCAG accessibility audit of a website with the packaged Playwright MCP and axe-core.
---

# WCAG audit

Use the installed `playwright` MCP server and its packaged axe-core runtime. Do
not install npm packages, download axe, build an image, or change application
source while auditing. The audit provides evidence for the tested scope; it
does not establish complete WCAG conformance. Automated checks miss barriers
that require human judgment. See the [W3C evaluation guidance](https://www.w3.org/WAI/test-evaluate/tools/).

## Start safely

1. Discover the available Playwright tool names and inspect their input schemas
   before calling them. Use that current browser session throughout the audit.
   If the local Docker image or report directory is missing, report a setup
   blocker and refer to `.opencode/mcp/playwright/README.md`. Do not build the
   image or install dependencies as part of the audit.
2. Default to WCAG 2.2 AA. If the requester names another target, use it and
   record it in the report.
3. Confirm `window.axe` is available after navigation and record
   `window.axe.version`. This image currently packages axe-core **4.14.0**;
   compare the browser value to this and `/opt/playwright/versions.txt`. Stop
   automated scanning and report a runtime issue if it differs.
4. Establish the requested origin. Keep navigation and interactions within
   that origin unless the requester explicitly expands scope. Do not submit
   forms that create persistent data unless authorized.
5. Choose a bounded, representative sample: normally the landing page, one
   representative page per distinct template, and the key requested flow.
   Record the page selection, reasons, and omitted routes or states.

## Test each selected page

- Capture and scan the initial state, then navigate and scan relevant
  interactive states in the same browser session. Open menus, dialogs, tabs,
  accordions, and other controls before scanning the resulting state.
- Run applicable axe A and AA rules from earlier WCAG versions as well as the
  selected version. For the default target, use tags
  `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`, `wcag22a`, and `wcag22aa` in the
  browser's axe run. Use the active browser page after each interaction, for
  example by invoking `window.axe.run(document, {runOnly:{type:"tag",
  values:[...]}})` through the discovered page evaluation tool. Save the
  complete raw axe output, including incomplete results. Treat `incomplete` as
  requiring further assessment, not as a pass.
- Check keyboard-only operation, visible and logical focus, dialog focus and
  dismissal, form labels and errors, and responsive layouts at representative
  narrow and wide viewports. Distinguish viewport resizing from browser zoom;
  test both when relevant.
- Record blocked, inaccessible, and untested routes and interactions. Never
  describe an unsuccessful or partial audit as successful.
- Group repeated defects in the summary while retaining every affected URL and
  element location.

## Artifacts

Create a fresh directory `reports/wcag/<timestamp>/` in the project for each
run. Keep authentication storage separate from this directory. Save:

- `report.md`: scope, target, scanner version, selected sample, omissions,
  coverage, summary, and limitations;
- `findings.json`: grouped machine-readable findings;
- `raw/`: unmodified axe results for each scanned state;
- `screenshots/`: relevant page and defect evidence.

Host and container project paths are identical. For explicit MCP `filename`
values prefer `reports/wcag/<timestamp>/...`; the corresponding absolute host
path `<project>/reports/wcag/<timestamp>/...` also works. Named relative files
resolve against the project root advertised through MCP roots, **not** the
configured output directory. Omitting `filename` uses the configured
`reports/wcag` output directory. A bare `file.png` targets the read-only project
root and is unsupported. Do not use `/workspace/...` or assume host
`/tmp/opencode` is available to Playwright. Only the report subtree is writable;
do not relax filesystem restrictions. The shared adapter supplies the project
path automatically; users do not configure `AI_PROFILE_PROJECT_DIR`.

Each finding records URL, interaction state, element or selector, axe rule and
WCAG criterion, evidence, severity, reproduction steps, suggested correction,
and all affected locations. Include keyboard, focus, form, responsive, and
manual observations alongside automated findings. Do not include credentials,
cookies, or authentication storage in artifacts.

## Completion

Report completed and omitted pages/states, blocked checks, unresolved
incomplete axe results, and any runtime problems. State clearly that automated
and agent checks are evidence within the tested scope, not proof of full WCAG
conformance.
