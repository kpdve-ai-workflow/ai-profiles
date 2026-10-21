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
  docker rm -f "$name" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
echo "Smoke artifacts (retained on failure too): $run/project/reports/wcag"
docker image inspect --format 'Image architecture: {{.Os}}/{{.Architecture}}' "$image"
timeout --signal=TERM --kill-after=10s 180s docker run --rm --init --name "$name" \
  --workdir /project --entrypoint node \
  --mount "type=bind,src=$tests,dst=/smoke,readonly" \
  --mount "type=bind,src=$run/project,dst=/project,readonly" \
  --mount "type=bind,src=$run/project/reports/wcag,dst=/project/reports/wcag" \
  -e HOST_UID=1000 -e HOST_GID=1000 \
  -e PLAYWRIGHT_MCP_OUTPUT_DIR=/project/reports/wcag \
  "$image" /smoke/smoke.cjs
