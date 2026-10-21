#!/bin/sh
set -eu

image=${1:-ai-profiles-playwright:dependency-smoke}
tests=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
run=$(mktemp -d "${TMPDIR:-/tmp}/playwright-smoke.XXXXXXXX")
name=playwright-smoke-${run##*.}
mkdir -p "$run/project/reports/wcag"
cp "$tests/fixture.html" "$run/project/fixture.html"
chmod 755 "$run" "$run/project" "$run/project/reports"
# Only this fresh, disposable report directory is writable by container UID 1000.
chmod 777 "$run/project/reports/wcag"
cleanup() {
  docker rm -f "$name" "$name-fixtures" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
echo "Smoke artifacts (retained on failure too): $run/project/reports/wcag"
docker image inspect --format 'Image architecture: {{.Os}}/{{.Architecture}}' "$image"
timeout --signal=TERM --kill-after=10s 30s docker run --rm --pull=never --name "$name-fixtures" --entrypoint node \
  --mount "type=bind,src=$tests,dst=/smoke,readonly" \
  --mount "type=bind,src=$run/project,dst=/project" \
  "$image" /smoke/create-fixtures.cjs
cat > "$run/smoke.yaml" <<'YAML'
services:
  playwright:
    image: "${AI_PROFILE_SMOKE_IMAGE:?}"
    entrypoint: ["node", "/smoke/smoke.cjs"]
    environment:
      HOST_UID: ""
      HOST_GID: ""
      SMOKE_PROJECT_DIR: /workspace
    volumes:
      - type: bind
        source: "${AI_PROFILE_SMOKE_DIR:?}"
        target: /smoke
        read_only: true
        bind:
          create_host_path: false
YAML
AI_PROFILE_SMOKE_IMAGE="$image" AI_PROFILE_SMOKE_DIR="$tests" \
  AI_PROFILE_PLAYWRIGHT_CA_CERT=.private/ca.crt \
  AI_PROFILE_PLAYWRIGHT_STORAGE_STATE=.private/auth.json \
  timeout --signal=TERM --kill-after=10s 180s docker compose \
  --project-directory "$run/project" \
  -f "$tests/../compose.yaml" -f "$tests/../compose.ca.yaml" \
  -f "$tests/../compose.auth.yaml" -f "$run/smoke.yaml" \
  run --rm --no-deps --pull never -T --name "$name" playwright
