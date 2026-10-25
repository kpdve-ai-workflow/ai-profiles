#!/bin/bash
# Lekkie regresje powłoki i transportu; NIE dowód działania VM/E2E.
set -euo pipefail
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp/opencode}/ai-profiles-vm-check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
expect() {
    local expected=$1; shift
    local status=0
    "$@" > "$scratch/output" 2>&1 || status=$?
    if [[ $status != "$expected" ]]; then
        cat "$scratch/output" >&2
        echo "Oczekiwano $expected, otrzymano $status: $*" >&2; exit 1
    fi
}
for script in host guest provision check report-export; do bash -n "$root/tests/vm/$script.sh"; done
expect 1 bash "$root/tests/vm/host.sh" unknown
grep -q 'Nieznany tryb' "$scratch/output"
expect 1 bash "$root/tests/vm/guest.sh" test invalid
grep -q 'tester 1000:1000' "$scratch/output"
expect 2 make -s -C "$root" _test-guest

checkout="$scratch/checkout space,comma'quote"
mkdir -p "$checkout/tests/vm" "$scratch/bin" "$scratch/minimal"
cp "$root"/tests/vm/{host.sh,excludes} "$checkout/tests/vm/"
ln -s /usr/bin/uname "$scratch/minimal/uname"
# Brak CLI: żadnego lokalnego fallbacku.
expect 1 env PATH="$scratch/minimal" /bin/bash "$checkout/tests/vm/host.sh" test
grep -q 'Brak incus' "$scratch/output"

export MOCK_ID MOCK_NAME MOCK_CALLS="$scratch/calls" MOCK_STATE=daemon MOCK_RESULT=0 MOCK_EXPORT=0
MOCK_ID=$(printf '%s\0%s\0%s' "$checkout" "$(id -u)" "$(cat /etc/machine-id)" | sha256sum | cut -d' ' -f1)
MOCK_NAME=ai-profiles-${MOCK_ID:0:24}
cat > "$scratch/bin/incus" <<'EOF'
#!/bin/bash
set -eu
printf '%s\n' "$*" >> "$MOCK_CALLS"
[[ $1 == --force-local ]] || exit 99
shift
if [[ $1 == query ]]; then
    # Rzeczywisty CLI nie dopuszcza --project dla query.
    [[ $# == 2 ]] || exit 99
else
    [[ $1 == --project && $2 == default && $3 != query ]] || exit 99
    shift 2
fi
[[ $MOCK_STATE != daemon ]] || exit 19
case "$1 $2" in
    'query /1.0') echo '{"environment":{"driver":"lxc | qemu"}}' ;;
    "query /1.0/instances/$MOCK_NAME?project=default")
        id=$MOCK_ID; [[ $MOCK_STATE != collision ]] || id=foreign
        printf '{"type":"virtual-machine","status":"Running","config":{"user.ai-profiles.identity":"%s"}}\n' "$id" ;;
    list*) printf '[{"name":"%s","status":"Running"}]\n' "$MOCK_NAME" ;;
    'exec '*)
        [[ $* != *'runuser -u tester'* ]] || exit "$MOCK_RESULT"
        ;;
    'file pull')
        [[ $MOCK_EXPORT == 0 ]] || exit "$MOCK_EXPORT"
        ;;
    'file push'|'config get') ;;
    *) echo "Nieoczekiwane wywołanie mocka: $*" >&2; exit 98 ;;
esac
EOF
chmod +x "$scratch/bin/incus"
export PATH="$scratch/bin:$PATH"
expect 1 bash "$checkout/tests/vm/host.sh" test
grep -q 'daemon Incus niedostępny' "$scratch/output"
MOCK_STATE=collision
expect 1 bash "$checkout/tests/vm/host.sh" sync
grep -q 'Kolizja nazwy' "$scratch/output"
! grep -Eq ' (start|stop|exec|file) ' "$MOCK_CALLS"
MOCK_STATE=owned
# Wspólny lock dla wszystkich operacji, bez czekania.
exec 8>"$checkout/.test-vm/lock"
flock -n 8
expect 1 bash "$checkout/tests/vm/host.sh" stop
grep -q 'Inna operacja' "$scratch/output"
flock -u 8

# Rzeczywisty rsync: prywatność stagingu, zmiany/delecje, symlinki i cache.
src=$scratch/source; stage=$scratch/stage; dest=$scratch/dest
mkdir -p "$src/core/rust/target" "$src/.git" "$src/.opencode" "$src/node_modules" "$stage" "$dest/core/rust/target"
echo secret > "$src/.env.local"
echo secret > "$src/base-auth.json"
echo secret > "$src/playwright-auth.json"
echo private > "$src/.git/config"
echo host-build > "$src/core/rust/target/host"
echo cache > "$dest/core/rust/target/cache"
echo stale > "$dest/removed"
echo modified > "$src/new file"
echo context > "$src/.dockerignore"
printf '#!/bin/sh\nexit 0\n' > "$src/executable"
chmod 755 "$src/executable"
echo unrelated > "$scratch/private"
ln -s "$scratch/private" "$src/link"
rsync -a --no-owner --no-group --exclude-from="$root/tests/vm/excludes" -- "$src/" "$stage/"
[[ ! -e $stage/.env.local && ! -e $stage/base-auth.json && ! -e $stage/playwright-auth.json && ! -e $stage/.git && ! -e $stage/core/rust/target ]]
[[ -L $stage/link && $(readlink "$stage/link") == "$scratch/private" ]]
mkdir "$scratch/unpacked"
tar -C "$stage" -cf "$scratch/source.tar" .
tar --no-same-owner -xf "$scratch/source.tar" -C "$scratch/unpacked"
[[ -L $scratch/unpacked/link && -x $scratch/unpacked/executable && -f $scratch/unpacked/.dockerignore ]]
rsync -a --delete --no-owner --no-group --exclude-from="$root/tests/vm/excludes" -- "$stage/" "$dest/"
[[ -f $dest/core/rust/target/cache && ! -e $dest/removed && -x $dest/executable && -f $dest/.dockerignore && -L $dest/link ]]
rm "$src/new file"
rsync -a --delete --exclude-from="$root/tests/vm/excludes" -- "$src/" "$stage/"
rsync -a --delete --exclude-from="$root/tests/vm/excludes" -- "$stage/" "$dest/"
[[ ! -e $dest/'new file' && -f $dest/core/rust/target/cache ]]
# Usunięcie również katalogów nadrzędnych źródeł nie usuwa guest target.
rm -rf -- "$src/core"
rsync -a --delete --exclude-from="$root/tests/vm/excludes" -- "$src/" "$stage/"
rsync -a --delete --exclude-from="$root/tests/vm/excludes" -- "$stage/" "$dest/"
[[ -f $dest/core/rust/target/cache ]]

# Exercise the exact production selector: nested and top-level reports are
# included, while private files, unsupported extensions and symlinks are not.
source "$root/tests/vm/report-export.sh"
report_fixture="$scratch/report fixture"
mkdir -p "$report_fixture/project/reports/wcag/run" "$report_fixture/project/.private"
printf 'report\n' > "$report_fixture/project/reports/summary.json"
printf 'image\n' > "$report_fixture/project/reports/wcag/run/screenshot.png"
printf 'html\n' > "$report_fixture/project/reports/wcag/run/report.html"
printf 'private\n' > "$report_fixture/project/.private/secret.json"
printf 'not exported\n' > "$report_fixture/project/reports/debug.txt"
ln -s "$report_fixture/project/.private/secret.json" "$report_fixture/project/reports/leak.json"
mapfile -d '' selected_reports < <(report_files "$report_fixture")
[[ ${#selected_reports[@]} == 3 ]]
for expected_report in \
    "$report_fixture/project/reports/summary.json" \
    "$report_fixture/project/reports/wcag/run/screenshot.png" \
    "$report_fixture/project/reports/wcag/run/report.html"; do
    [[ " ${selected_reports[*]} " == *" $expected_report "* ]]
done
for excluded_report in \
    "$report_fixture/project/.private/secret.json" \
    "$report_fixture/project/reports/debug.txt" \
    "$report_fixture/project/reports/leak.json"; do
    [[ " ${selected_reports[*]} " != *" $excluded_report "* ]]
done

if [[ -r /dev/kvm && -w /dev/kvm ]]; then
    MOCK_RESULT=37 MOCK_EXPORT=0 expect 37 bash "$checkout/tests/vm/host.sh" test
    grep -q 'runuser -u tester.*setsid --wait.*guest.sh test' "$MOCK_CALLS"
    grep -q -- '--force-local --project default file pull -r -P' "$MOCK_CALLS"
    MOCK_RESULT=37 MOCK_EXPORT=9 expect 37 bash "$checkout/tests/vm/host.sh" test
    MOCK_RESULT=0 MOCK_EXPORT=9 expect 1 bash "$checkout/tests/vm/host.sh" test
    grep -q 'eksport artefaktów' "$scratch/output"
    MOCK_RESULT=0 MOCK_EXPORT=0 expect 0 bash "$checkout/tests/vm/host.sh" test
else
    echo 'SKIP: mockowane pełne dispatch/exit status wymagają dostępnego /dev/kvm (brak obejścia kontroli hosta).'
fi
echo 'OK: granice orkiestracji (mock), guard gościa, rsync/symlinki/excludes/cache. To nie jest test VM ani Playwright E2E.'
