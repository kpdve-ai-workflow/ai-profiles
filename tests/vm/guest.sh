#!/bin/bash
set -euo pipefail
[[ -f /var/lib/ai-profiles-vm/ready && $(id -u):$(id -g) == 1000:1000 ]] || {
    echo 'Wymagana przygotowana VM i tester 1000:1000' >&2; exit 1;
}
mode=${1:-}
case $mode in test|validate-rust-migration|test-playwright|test-all) ;; *) echo 'Nieznany tryb gościa' >&2; exit 2;; esac
run=${2:-}
[[ $run =~ ^[a-zA-Z0-9-]+$ ]] || exit 2
export HOME=/home/tester PATH=/home/tester/.cargo/bin:/usr/local/bin:/usr/bin:/bin
unset CARGO_TARGET_DIR RUSTUP_HOME CARGO_HOME MAKEFLAGS MFLAGS
cd /home/tester/ai-profiles
directory=/home/tester/test-runs/$run
mkdir -m 700 "$directory"
mkdir "$directory/tmp" "$directory/export"
export TMPDIR=$directory/tmp
# PID jest liderem grupy sesji utworzonej przez setsid na hoście.
echo "$$" > "$directory/pid"
exec 3>&1 4>&2
exec > >(tee "$directory/export/operation.log" >&3) 2>&1
logger=$!
collect() {
    result=$?
    trap - EXIT
    # Tylko raporty, nigdy .private, auth.json ani cały projekt.
    while IFS= read -r -d '' report; do
        relative=${report#"$directory/tmp/"}
        if ! { mkdir -p "$directory/export/$(dirname "$relative")" &&
            cp --no-dereference -- "$report" "$directory/export/$relative"; }; then
            echo "Nie udało się zebrać raportu: $relative" >&2
            if (( result == 0 )); then result=1; fi
        fi
    done < <(report_files "$directory/tmp")
    rm -f "$directory/pid"
    rm -f /home/tester/test-runs/.cancel-"$run"
    # Zamknij pipe i dokończ log przed powrotem exec/eksportem hosta.
    exec 1>&3 2>&4
    if ! wait "$logger"; then
        echo 'Nie udało się zapisać pełnego logu gościa.' >&2
        if (( result == 0 )); then result=1; fi
    fi
    exec 3>&- 4>&-
    exit "$result"
}
source /home/tester/ai-profiles/tests/vm/report-export.sh
trap collect EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
if [[ -f /home/tester/test-runs/.cancel-$run ]]; then exit 143; fi
playwright() {
    docker build --tag ai-profiles-playwright:dependency-smoke mcp/images/playwright
    sh mcp/images/playwright/tests/run.sh ai-profiles-playwright:dependency-smoke
}
case $mode in
    test) make _test-guest ;;
    validate-rust-migration) make _validate-rust-migration-guest ;;
    test-playwright) playwright ;;
    test-all) make _validate-rust-migration-guest; playwright ;;
esac
