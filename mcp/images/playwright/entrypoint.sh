#!/bin/sh
set -eu
if [ "${TARGETARCH:-$(dpkg --print-architecture)}" != "amd64" ] && [ "${TARGETARCH:-$(dpkg --print-architecture)}" != "arm64" ]; then
  echo "playwright: unsupported architecture ${TARGETARCH:-unknown}; supported: amd64, arm64" >&2
  exit 2
fi
if [ -n "${PLAYWRIGHT_CA_CERT:-}" ]; then
  if [ ! -f "$PLAYWRIGHT_CA_CERT" ]; then echo "playwright: configured CA file is unavailable" >&2; exit 2; fi
  install -m 0644 "$PLAYWRIGHT_CA_CERT" /usr/local/share/ca-certificates/ai-profile-custom.crt
  update-ca-certificates >&2
fi
uid=${HOST_UID:-1000}
gid=${HOST_GID:-1000}
case "$uid:$gid" in *[!0-9:]*|:*|*:) echo "playwright: HOST_UID/HOST_GID must be numeric" >&2; exit 2;; esac
home=/tmp/playwright-home-$uid
mkdir -p "$home"
chown "$uid:$gid" "$home"
chmod 700 "$home"
exec setpriv --reuid="$uid" --regid="$gid" --clear-groups \
  env HOME="$home" XDG_CACHE_HOME="$home/.cache" node /opt/playwright/node_modules/@playwright/mcp/cli.js \
  --browser chromium --headless --isolated --init-script /opt/playwright/node_modules/axe-core/axe.min.js
