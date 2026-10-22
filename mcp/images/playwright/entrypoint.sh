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
# Match the owner of the host-created report directory without a host launcher.
uid=${HOST_UID:-}
gid=${HOST_GID:-}
if [ -d "${PLAYWRIGHT_MCP_OUTPUT_DIR:-}" ]; then
  uid=${uid:-$(stat -c %u "$PLAYWRIGHT_MCP_OUTPUT_DIR")}
  gid=${gid:-$(stat -c %g "$PLAYWRIGHT_MCP_OUTPUT_DIR")}
fi
uid=${uid:-1000}
gid=${gid:-1000}
case "$uid:$gid" in *[!0-9:]*|:*|*:) echo "playwright: HOST_UID/HOST_GID must be numeric" >&2; exit 2;; esac
if [ -n "${PLAYWRIGHT_MCP_STORAGE_STATE:-}" ]; then
  if [ ! -f "$PLAYWRIGHT_MCP_STORAGE_STATE" ]; then echo "playwright: authentication storage state must be a file" >&2; exit 2; fi
  mode=$(stat -c %a "$PLAYWRIGHT_MCP_STORAGE_STATE")
  if [ "$((0$mode & 077))" -ne 0 ]; then echo "playwright: authentication storage state must have mode 0600 or stricter" >&2; exit 2; fi
fi
home=/tmp/playwright-home-$uid
mkdir -p "$home"
chown "$uid:$gid" "$home"
chmod 700 "$home"
set --
if [ -n "${PLAYWRIGHT_BASIC_AUTH_FILE:-}" ]; then
  if [ ! -f "$PLAYWRIGHT_BASIC_AUTH_FILE" ]; then echo "playwright: Basic Auth must be a private file" >&2; exit 2; fi
  mode=$(stat -c %a "$PLAYWRIGHT_BASIC_AUTH_FILE")
  if [ "$((0$mode & 077))" -ne 0 ]; then echo "playwright: Basic Auth file must have mode 0600 or stricter" >&2; exit 2; fi
  # Read as the browser user; keep the derived MCP config in its private home.
  config_dir=$(mktemp -d "$home/basic-auth-XXXXXX")
  chown "$uid:$gid" "$config_dir"
  config="$config_dir/config.json"
  setpriv --reuid="$uid" --regid="$gid" --clear-groups \
    node /opt/playwright/basic-auth.cjs "$PLAYWRIGHT_BASIC_AUTH_FILE" "$config"
  set -- --config "$config"
fi
if [ -n "${PLAYWRIGHT_CA_CERT:-}" ]; then
  # Chromium on Linux uses an NSS trust database, not just /etc/ssl/certs.
  mkdir -p "$home/.pki/nssdb"
  certutil -N --empty-password -d "sql:$home/.pki/nssdb" >&2
  certutil -A -d "sql:$home/.pki/nssdb" -n ai-profile-custom -t 'C,,' -i "$PLAYWRIGHT_CA_CERT" >&2
  chown -R "$uid:$gid" "$home/.pki"
  export NODE_EXTRA_CA_CERTS="$PLAYWRIGHT_CA_CERT"
fi
exec setpriv --reuid="$uid" --regid="$gid" --clear-groups \
  env HOME="$home" XDG_CACHE_HOME="$home/.cache" node /opt/playwright/node_modules/@playwright/mcp/cli.js \
  --browser chromium --headless --isolated --init-script /opt/playwright/node_modules/axe-core/axe.min.js "$@"
