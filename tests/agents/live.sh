#!/bin/bash
# Live host acceptance of the explicitly requested Herdr production workflow.
# This is separate from Rust tests, which run exclusively in the VM.
set -euo pipefail
cd "$(dirname "$0")/../.."
[[ ${HERDR_ENV:-} == 1 ]] || { echo 'Run inside Herdr.' >&2; exit 1; }
for program in herdr codex jq make sha256sum; do command -v "$program" >/dev/null; done
scratch=$(mktemp -d)
session=
success=false
cleanup() {
    if $success && [[ -n $session ]]; then
        AI_AGENT_SESSION="$session" core/target/release/ai-profile-agent stop > "$scratch/stop.json"
        echo "PASSED: two delegated reports; researcher reused; stopped SESSION=$session"
    elif [[ -n $session ]]; then
        echo "FAILED: session retained for inspection: $session" >&2
    fi
    rm -rf "$scratch"
}
trap cleanup EXIT
# Include untracked source files and symlink identities without following directory links.
snapshot() {
    git ls-files --cached --others --exclude-standard -z |
    while IFS= read -r -d '' file; do
        if [[ -L $file ]]; then
            printf 'symlink %s -> ' "$file"
            readlink -- "$file"
        else
            sha256sum -- "$file"
        fi
    done
}
snapshot > "$scratch/before"
if ! make --no-print-directory agent CLIENT=codex > "$scratch/start.out" 2> "$scratch/start.err"; then
    session=$(sed -n 's/^SESSION=//p' "$scratch/start.err" | tail -1)
    cat "$scratch/start.err" >&2
    exit 1
fi
session=$(sed -n 's/^SESSION=//p' "$scratch/start.err" | tail -1)
[[ -n $session ]]
brain=$(jq -er '.brain.name' "$session/state.json")
[[ $(jq -r '.researcher' "$session/state.json") == null ]]
# Ask ordinary repository questions; do not tell Brain to call a particular tool.
herdr agent prompt "$brain" 'Znajdź główny punkt wejścia narzędzia ai-profiles, opisz inicjalizację i wskaż najważniejsze pliki z numerami linii. Tylko analiza, bez zmian plików.' --wait --timeout 300000 > "$scratch/first.json"
jq -e '.task.id == "task-1" and .sequence == 1 and .worker_tasks == 1' "$session/state.json" >/dev/null
jq -e '.task_id == "task-1" and (.report | contains("RELEVANT FILES"))' "$session/task-1.report.json" >/dev/null
researcher=$(jq -er '.researcher.name' "$session/state.json")
terminal=$(jq -er '.researcher.terminal' "$session/state.json")
herdr agent prompt "$brain" 'Prześledź teraz instalację profilu: jak wybierane są skille i przygotowywane pliki dla wskazanego klienta? Podaj najważniejsze zależności i odnośniki do kodu. Tylko analiza, bez zmian.' --wait --timeout 300000 > "$scratch/second.json"
jq -e --arg name "$researcher" --arg terminal "$terminal" '.task.id == "task-2" and .sequence == 2 and .worker_tasks == 2 and .researcher.name == $name and .researcher.terminal == $terminal' "$session/state.json" >/dev/null
jq -e '.task_id == "task-2" and (.report | contains("UNCERTAINTIES"))' "$session/task-2.report.json" >/dev/null
AI_AGENT_SESSION="$session" core/target/release/ai-profile-agent status > "$scratch/status.json"
jq -e '.config.brain.model == "gpt-6.1-sol" and .config.brain.thinking == "xhigh" and .config.researcher.model == "gpt-6-luna" and .config.researcher.thinking == "medium" and .task.report_ready' "$scratch/status.json" >/dev/null
snapshot > "$scratch/after"
cmp "$scratch/before" "$scratch/after"
success=true
