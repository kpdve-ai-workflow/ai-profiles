#!/usr/bin/env bash
# Safety-first detection only. No installs, mutations or test execution.
set -euo pipefail
ROOT="${1:-.}"
if [ ! -d "$ROOT" ]; then echo "Not a directory: $ROOT" >&2; exit 2; fi
cd "$ROOT"
echo 'Available mutation tooling (detection only):'
found=0
for bin in cargo-mutants mutmut pitest stryker; do
  if command -v "$bin" >/dev/null 2>&1; then
    printf '  - %s: found on PATH\n' "$bin"
    found=1
  fi
done
if [ -f package.json ]; then
  if grep -Eq '"(@stryker-mutator/core|stryker)"' package.json; then
    echo '  - Stryker dependency declared in package.json (not verified installed)'
    found=1
  fi
fi
if [ -f pyproject.toml ] && grep -Eq 'mutmut|cosmic-ray' pyproject.toml; then
  echo '  - Python mutation dependency referenced in pyproject.toml (not verified installed)'
  found=1
fi
if [ "$found" -eq 0 ]; then echo '  - No known mutation tool detected. This does NOT block testing.'; fi
cat <<'MSG'

Mutation/sensitivity checks are OPTIONAL, never a mandatory quota.
Run them only with project permission in a disposable worktree/copy.
1. Choose a plausible, non-equivalent semantic fault related to an assertion.
2. Introduce it safely in the isolated copy (never in a shared checkout).
3. Run only authorized, targeted tests; confirm failure has the expected cause.
4. Revert your own mutation and verify no diff remains in the disposable copy.
5. Do not call a survived mutation a product defect without independent evidence.
This script intentionally does not modify or execute anything.
MSG
