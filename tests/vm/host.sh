#!/bin/bash
set -euo pipefail
fail() { echo "VM: $*" >&2; exit 1; }
mode=${1:-}
case $mode in up|stop|sync|reprovision|test|test-playwright|validate-rust-migration|test-all) ;; *) fail 'Nieznany tryb (up, stop, sync, reprovision, test, test-playwright, validate-rust-migration, test-all).';; esac
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
state=$root/.test-vm
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail 'Obsługiwany host: Linux amd64 z KVM.'
for tool in incus rsync tar jq flock sha256sum base64 realpath; do
    command -v "$tool" >/dev/null || fail "Brak $tool. Zainstaluj świadomie wymagania hosta (docs/testing.md)."
done
[[ ! -L $state ]] || fail '.test-vm nie może być symlinkiem.'
mkdir -p "$state"
chmod 700 "$state"
exec 9>"$state/lock"
flock -n 9 || fail 'Inna operacja tego checkoutu jest aktywna.'
run=$(date -u +%Y%m%dT%H%M%SZ)-$(cat /proc/sys/kernel/random/uuid)
artifact=$state/artifacts/$run
mkdir -p "$artifact"
echo "Artefakty: $artifact"
exec > >(tee "$artifact/host.log") 2>&1
identity=$(printf '%s\0%s\0%s' "$root" "$(id -u)" "$(cat /etc/machine-id)" | sha256sum | cut -d' ' -f1)
name=ai-profiles-${identity:0:24}
# --force-local i jawny projekt: nigdy aktualnie wybrany zdalny daemon/projekt.
ic() { incus --force-local --project default "$@"; }
# query odrzuca --project; projekt wskazujemy w URL zasobów projektowych.
ic_query() { incus --force-local query "$@"; }
ic_query /1.0 > "$state/server.json" || fail 'Lokalny daemon Incus niedostępny lub brak uprawnień do socketu. Użytkownik musi przygotować Incus; patrz docs/testing.md.'
exists=false
if ic list --format json | jq -e --arg n "$name" 'any(.[]; .name == $n)' >/dev/null; then
    exists=true
    ic_query "/1.0/instances/$name?project=default" > "$state/instance.json"
    jq -e --arg id "$identity" '.type == "virtual-machine" and .config["user.ai-profiles.identity"] == $id' "$state/instance.json" >/dev/null || fail 'Kolizja nazwy/obca VM: odmawiam modyfikacji.'
fi
if [[ $mode == stop ]]; then
    if $exists; then
        if [[ $(jq -r .status "$state/instance.json") != Stopped ]]; then ic stop "$name"; fi
    else echo 'VM jeszcze nie istnieje.'; fi
    exit
fi
[[ -r /dev/kvm && -w /dev/kvm ]] || fail 'Brak dostępnego /dev/kvm. Sprawdź wirtualizację sprzętową i świadomie skonfiguruj uprawnienia hosta.'
jq -e '.environment.driver | contains("qemu")' "$state/server.json" >/dev/null || fail 'Daemon nie zgłasza obsługi QEMU VM. Sprawdź instalację Incus/QEMU/KVM.'
if ! $exists; then
    cpu=${TEST_VM_CPUS:-4}; memory=${TEST_VM_MEMORY_GIB:-8}; disk=${TEST_VM_DISK_GIB:-64}
    for value in "$cpu" "$memory" "$disk"; do [[ $value =~ ^[1-9][0-9]{0,3}$ ]] || fail 'Zasoby muszą być dodatnimi liczbami całkowitymi.'; done
    (( cpu <= 256 && memory >= 4 && memory <= 1024 && disk >= 32 && disk <= 4096 )) || fail 'Zakres: CPU 1–256, RAM 4–1024 GiB, dysk 32–4096 GiB.'
    pools=$(ic storage list --format json)
    networks=$(ic network list --format json)
    pool=${TEST_VM_POOL:-$(jq -r 'if length == 1 then .[0].name else empty end' <<< "$pools")}
    network=${TEST_VM_NETWORK:-$(jq -r '[.[] | select(.managed and .type == "bridge" and .config["ipv4.address"] != "none" and .config["ipv4.nat"] == "true")] | if length == 1 then .[0].name else empty end' <<< "$networks")}
    [[ -n $pool && -n $network ]] || fail 'Wskaż istniejące TEST_VM_POOL i TEST_VM_NETWORK (zarządzany bridge z IPv4 NAT). Nie tworzę storage ani sieci hosta.'
    jq -e --arg n "$pool" 'any(.[]; .name == $n)' <<< "$pools" >/dev/null || fail 'Nieznany pool.'
    jq -e --arg n "$network" 'any(.[]; .name == $n and .managed and .type == "bridge" and .config["ipv4.nat"] == "true" and .config["ipv4.address"] != "none")' <<< "$networks" >/dev/null || fail 'Sieć wymaga zarządzanego bridge IPv4 z NAT/outbound.'
    incus remote list --format json | jq -e '.images.Addr == "https://images.linuxcontainers.org" and .images.Protocol == "simplestreams" and .images.Public == true' >/dev/null || fail 'Remote images musi wskazywać zaufane https://images.linuxcontainers.org (bez automatycznej zmiany konfiguracji).'
    ic image list images: debian/13/cloud --format json > "$state/images.json" || fail 'Nie można sprawdzić obrazu Debian 13 cloud VM.'
    fingerprint=$(jq -r '[.[] | select(.type == "virtual-machine" and .architecture == "x86_64" and .properties.os == "Debian" and .properties.release == "trixie" and .properties.variant == "cloud") | select(any(.aliases[]; .name == "debian/13/cloud" or .name == "debian/13/cloud/amd64"))] | sort_by(.created_at) | last | .fingerprint // empty' "$state/images.json")
    [[ $fingerprint =~ ^[0-9a-f]{64}$ ]] || fail 'Brak zweryfikowanego Debian 13 Trixie amd64 cloud virtual-machine; bez fallbacku.'
    printf '%s\n' "images:$fingerprint" > "$state/image-source"
    provision=$(base64 -w0 "$root/tests/vm/provision.sh")
    sed -e "s|@PROVISION_BASE64@|$provision|" -e "s|@MINIMUM_BYTES@|$((disk * 1024 * 1024 * 1024 * 90 / 100))|" "$root/tests/vm/cloud-init.yaml.in" > "$state/cloud-init.yaml"
    ic init "images:$fingerprint" "$name" --vm --no-profiles --storage "$pool" --network "$network" \
        -d "root,size=${disk}GiB" -c "limits.cpu=$cpu" -c "limits.memory=${memory}GiB" \
        -c "user.ai-profiles.identity=$identity" -c "user.ai-profiles.image=$fingerprint" \
        -c "cloud-init.user-data=$(cat "$state/cloud-init.yaml")"
fi
# Odczyt pojedynczego zasobu zamiast listy instancji; bez automatycznych restartów.
vm_status() {
    ic_query "/1.0/instances/$name/state?project=default" |
        jq -er '.status | select(type == "string" and length > 0)'
}
status=$(vm_status) || fail 'Nie można odczytać stanu VM.'
echo "VM: $name, stan przed startem: $status"
case $status in
    Running) ;;
    Stopped)
        start_result=0
        ic start "$name" || start_result=$?
        if (( start_result != 0 )); then
            # Stan mógł zmienić się po odczycie. Zachowaj błąd w logu i sprawdź
            # rzeczywisty stan, bez dopasowywania tekstu błędu ani ponawiania startu.
            status=$(vm_status) || { echo 'VM: nie można potwierdzić stanu po błędzie startu.' >&2; exit "$start_result"; }
            [[ $status == Running ]] || { echo "VM: start nie powiódł się; stan: $status" >&2; exit "$start_result"; }
            echo 'VM: po błędzie startu potwierdzono Running; sprawdzam gotowość.'
        fi
        ;;
    *) fail "Nieobsługiwany stan VM: $status; nie uruchamiam ani nie restartuję maszyny." ;;
esac
ready=false
for ((i=0; i<180; i++)); do
    if ic exec "$name" -- true >/dev/null 2>&1; then ready=true; break; fi
    sleep 2
done
$ready || fail "incus-agent niedostępny w $name; zachowano VM. Sprawdź incus console $name --show-log."
if ! ic exec "$name" -- timeout 1800 cloud-init status --wait; then
    ic file pull -P "$name/var/log/cloud-init-output.log" "$artifact/provision.log" || true
    if [[ $mode != reprovision ]]; then
        ic exec "$name" -- test -f /var/lib/ai-profiles-vm/repaired || fail "cloud-init nie zakończył się poprawnie. Diagnostyka: incus exec $name -- cat /var/log/cloud-init-output.log; po poprawkach użyj reprovision."
        echo 'INFO: cloud-init zachował historyczny błąd pierwszego startu; jawny reprovision zakończył się sukcesem. Sprawdzam gotowość VM.'
    fi
    # Jawna naprawa jest dozwolona po zakończonym błędem pierwszym provisioningu.
    ic exec "$name" -- cloud-init status --format json > "$state/cloud-init-status.json" || true
    jq -e '.status == "error" or .status == "done"' "$state/cloud-init-status.json" >/dev/null || fail 'cloud-init nadal działa lub brak stanu; nie uruchamiam provisioningu równolegle.'
fi
if [[ $mode == reprovision ]]; then
    ic file push "$root/tests/vm/provision.sh" "$name/usr/local/sbin/ai-profiles-provision" --uid 0 --gid 0 --mode 0700
    ic exec "$name" -- bash /usr/local/sbin/ai-profiles-provision > >(tee "$artifact/reprovision.log") 2>&1
    ic exec "$name" -- touch /var/lib/ai-profiles-vm/repaired
fi
ic exec "$name" -- test -f /var/lib/ai-profiles-vm/ready || fail 'Brak pełnego provisioningu. Wykonaj tests/vm/host.sh reprovision po sprawdzeniu logów.'
ic exec "$name" -- df -h /home/tester
[[ $mode != up && $mode != reprovision ]] || { echo "VM gotowa: $name"; exit; }
staging=$state/staging-$run
guest_stage=/var/tmp/ai-profiles-source-$run
cleanup() {
    rm -rf -- "$staging"
    rm -f -- "$state/source.tar"
    ic exec "$name" -- rm -rf -- "$guest_stage" >/dev/null 2>&1 || true
}
trap cleanup EXIT
mkdir -m 700 "$staging"
# Bez dereferencji symlinków; prywatne pliki i buildy nie trafiają nawet do staging.
rsync -a --no-owner --no-group --exclude-from="$root/tests/vm/excludes" -- "$root/" "$staging/"
tar -C "$staging" -cf "$state/source.tar" .
ic exec "$name" -- mkdir -m 700 "$guest_stage"
ic file push "$state/source.tar" "$name$guest_stage/source.tar" --uid 0 --gid 0 --mode 0600
rm -f "$state/source.tar"
ic exec "$name" -- mkdir "$guest_stage/source"
ic exec "$name" -- tar --no-same-owner -xf "$guest_stage/source.tar" -C "$guest_stage/source"
# Wykluczenia chronią cache także przed --delete; bez --delete-excluded.
ic exec "$name" -- rsync -a --delete --no-owner --no-group --chown=1000:1000 \
    --exclude-from="$guest_stage/source/tests/vm/excludes" -- "$guest_stage/source/" /home/tester/ai-profiles/
cleanup
[[ $mode != sync ]] || exit 0
printf '%s\n' "$name" > "$artifact/instance"
ic config get "$name" user.ai-profiles.image > "$artifact/image-fingerprint"
echo "Artefakty: $artifact"
interrupted=0
terminate() {
    interrupted=143
    # PID należy do tego unikalnego przebiegu; nie dotykamy VM ani innych procesów.
    ic exec "$name" -- bash -c 'touch "/home/tester/test-runs/.cancel-$1"; p=/home/tester/test-runs/$1/pid; if test -f "$p"; then read -r pid < "$p"; [[ $pid =~ ^[0-9]+$ ]] && kill -TERM -- "-$pid"; fi' bash "$run" || true
}
trap terminate INT TERM HUP
set +e
ic exec "$name" -- runuser -u tester -- env -i HOME=/home/tester USER=tester \
    PATH=/home/tester/.cargo/bin:/usr/local/bin:/usr/bin:/bin \
    setsid --wait bash /home/tester/ai-profiles/tests/vm/guest.sh "$mode" "$run" &
execution=$!
wait "$execution"
result=$?
if (( interrupted )); then wait "$execution"; result=$interrupted; fi
set -e
trap - INT TERM HUP
export_result=0
ic file pull -r -P "$name/home/tester/test-runs/$run/export" "$artifact/" || export_result=$?
echo "Artefakty: $artifact"
if (( result != 0 )); then exit "$result"; fi
(( export_result == 0 )) || fail 'Test zakończony sukcesem, ale eksport artefaktów nie powiódł się.'
