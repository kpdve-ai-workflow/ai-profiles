#!/bin/bash
# Podgląd wyłącznie do odczytu: bez locka, startowania VM i uruchamiania testów.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
state=$root/.test-vm
[[ ! -L $state ]] || { echo '.test-vm nie może być symlinkiem.' >&2; exit 1; }
shopt -s nullglob
logs=("$state"/artifacts/*/host.log)
if ((${#logs[@]} == 0)); then
    echo 'Brak logów tego checkoutu. Najpierw uruchom make test-vm-up lub make test.' >&2
    exit 1
fi
for tool in incus jq sha256sum stat tail wc mktemp tr sed; do
    command -v "$tool" >/dev/null || { echo "Brak narzędzia: $tool" >&2; exit 1; }
done
if [[ ! ${INCUS_SOCKET+x} && -S /run/incus/unix.socket ]]; then
    export INCUS_SOCKET=/run/incus/unix.socket
fi
identity=$(printf '%s\0%s\0%s' "$root" "$(id -u)" "$(cat /etc/machine-id)" | sha256sum | cut -d' ' -f1)
name=ai-profiles-${identity:0:24}
scratch=$(mktemp -d "${TMPDIR:-/tmp}/ai-profiles-logs.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
trap 'exit 0' INT TERM HUP
host_offset=0; cloud_offset=0; current=
show_new() {
    local file=$1 label=$3 size count
    local -n offset=$2
    size=$(stat -c %s -- "$file")
    if (( size < offset - 1 )); then offset=0; fi
    tail -c "+$((offset > 0 ? offset : 1))" -- "$file" > "$scratch/delta"
    count=$(wc -c < "$scratch/delta")
    if (( count > 0 )); then
        echo "== $label =="
        if (( offset == 0 )); then
            tr '\r' '\n' < "$scratch/delta" | sed '/^[[:space:]]*$/d' | tail -n 60
        else
            tr '\r' '\n' < "$scratch/delta"
        fi
    fi
    if (( offset == 0 )); then offset=1; fi
    offset=$((offset + count))
}
echo 'Podgląd na żywo. Ctrl+C zamyka tylko podgląd, nie VM ani testy.'
while :; do
    logs=("$state"/artifacts/*/host.log)
    if ((${#logs[@]})); then
        latest=${logs[${#logs[@]}-1]}
        if [[ $latest != "$current" ]]; then
            current=$latest; host_offset=0
            echo "Log operacji: $current"
        fi
        show_new "$current" host_offset host
    fi
    if incus --force-local query "/1.0/instances/$name?project=default" > "$scratch/instance.json" 2>/dev/null; then
        jq -e --arg id "$identity" '.type == "virtual-machine" and .config["user.ai-profiles.identity"] == $id' "$scratch/instance.json" >/dev/null || {
            echo 'Kolizja nazwy/obca VM: odmawiam odczytu logów gościa.' >&2; exit 1;
        }
        if [[ $(jq -r .status "$scratch/instance.json") == Running ]] &&
            incus --force-local --project default file pull -P "$name/var/log/cloud-init-output.log" "$scratch/cloud-init.log" >/dev/null 2>&1; then
            show_new "$scratch/cloud-init.log" cloud_offset cloud-init
        fi
    fi
    sleep 2
done
