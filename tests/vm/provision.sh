#!/bin/bash
set -euo pipefail
[[ $(id -u) == 0 ]] || exit 1
marker=/var/lib/ai-profiles-vm/ready
rm -f "$marker"
. /etc/os-release
[[ $ID == debian && $VERSION_ID == 13 && $(dpkg --print-architecture) == amd64 ]] || {
    echo 'Wymagany Debian 13 amd64' >&2; exit 1;
}
export DEBIAN_FRONTEND=noninteractive
install_missing() {
    local missing=() package
    for package in "$@"; do
        [[ $(dpkg-query -W -f='${Status}' "$package" 2>/dev/null || true) == 'install ok installed' ]] || missing+=("$package")
    done
    if ((${#missing[@]})); then apt-get install -y "${missing[@]}"; fi
}
apt-get update
install_missing ca-certificates curl gnupg rsync make git build-essential pkg-config libssl-dev xz-utils unzip util-linux coreutils jq
if getent passwd tester >/dev/null; then
    [[ $(id -u tester) == 1000 && $(id -g tester) == 1000 && $(getent passwd tester | cut -d: -f6) == /home/tester ]] || exit 1
else
    if getent passwd 1000 || getent group 1000 || getent group tester; then
        echo 'Konflikt UID/GID 1000; nie zmieniam kont obrazu.' >&2; exit 1
    fi
    groupadd -g 1000 tester
    useradd -m -u 1000 -g 1000 -s /bin/bash tester
fi
install -d -m 0755 /etc/apt/keyrings
if [[ ! -f /etc/apt/sources.list.d/docker.sources ]]; then
    curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
    cat > /etc/apt/sources.list.d/docker.sources <<'EOF'
Types: deb
URIs: https://download.docker.com/linux/debian
Suites: trixie
Components: stable
Architectures: amd64
Signed-By: /etc/apt/keyrings/docker.asc
EOF
    apt-get update
fi
install_missing docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
systemctl enable --now docker
usermod -aG docker tester
# NodeSource publikuje podpisane pakiety Node 24 dla nodistro (także Trixie).
if ! command -v node >/dev/null || [[ $(node -p 'process.versions.node.split(".")[0]') != 24 ]]; then
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor --yes -o /etc/apt/keyrings/nodesource.gpg
    echo 'deb [arch=amd64 signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_24.x nodistro main' > /etc/apt/sources.list.d/nodesource.list
    apt-get update
    apt-get install -y nodejs
fi
as_tester() { runuser -u tester -- env -i HOME=/home/tester USER=tester PATH=/home/tester/.cargo/bin:/usr/local/bin:/usr/bin:/bin "$@"; }
if [[ ! -x /home/tester/.cargo/bin/rustup ]]; then
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /var/tmp/ai-profiles-rustup.sh
    as_tester sh /var/tmp/ai-profiles-rustup.sh -y --profile minimal --default-toolchain stable --component rustfmt --component clippy
    rm -f /var/tmp/ai-profiles-rustup.sh
else
    as_tester rustup component add rustfmt clippy --toolchain stable
fi
install -d -o tester -g tester /home/tester/ai-profiles /home/tester/test-runs
as_tester bash -eu -c '
    test "$(id -u):$(id -g)" = 1000:1000
    test "$(node -p '\''process.versions.node.split(".")[0]'\'')" = 24
    version=$(rustc --version | cut -d" " -f2)
    test "$(printf "%s\n" 1.85.0 "$version" | sort -V | head -n1)" = 1.85.0
    cargo --version; docker info >/dev/null; docker compose version; docker buildx version
'
# Sprawdzenie rzeczywistego filesystemu, nie wyłącznie deklaracji dysku Incus.
df -h / /home/tester
bytes=$(df -B1 --output=size /home/tester | tail -n1 | tr -d ' ')
available=$(df -B1 --output=avail /home/tester | tail -n1 | tr -d ' ')
minimum=$(cat /var/lib/ai-profiles-vm/minimum-bytes)
[[ $bytes -ge $minimum && $available -ge 8589934592 ]] || {
    echo 'Filesystem nie urósł do 90% zamówionego dysku lub mniej niż 8 GiB wolnego.' >&2; exit 1;
}
install -d -m 0755 /var/lib/ai-profiles-vm
printf 'Debian 13 amd64; tester 1000:1000\n' > "$marker"
chmod 0644 "$marker"
