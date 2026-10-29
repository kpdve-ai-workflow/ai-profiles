#!/bin/bash
# Public Make contract: asking for help must never build or launch an agent.
set -euo pipefail
cd "$(dirname "$0")/../.."
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
for program in cargo herdr codex; do
    printf '#!/bin/sh\necho "unexpected operational command" >&2\nexit 99\n' > "$scratch/$program"
    chmod +x "$scratch/$program"
done
export PATH="$scratch:$PATH"
output=$(make --no-print-directory agent help)
[[ $output == *'make agent CLIENT=codex brain-researcher'* ]]
[[ $output != *'cargo build'* && $output != *'unexpected operational command'* ]]
output=$(make --no-print-directory help)
[[ $output == *'make agent help'* ]]
plan=$(make --no-print-directory --dry-run agent CLIENT=codex brain-researcher)
[[ $(grep -c 'release/ai-profile-agent" start' <<< "$plan") -eq 1 ]]
if make --no-print-directory agent CLIENT=claude brain-researcher >"$scratch/invalid" 2>&1; then
    echo 'Unsupported client was accepted' >&2
    exit 1
fi
[[ $(cat "$scratch/invalid") == *'CLIENT=codex'* ]]
[[ $(cat "$scratch/invalid") != *'unexpected operational command'* ]]
printf 'PASSED: agent Make help, brain-researcher dispatch, unsupported client\n'
