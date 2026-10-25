#!/bin/sh
set -eu

image=${1:-ai-profiles-playwright:dependency-smoke}
tests=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
run=$(mktemp -d "${TMPDIR:-/tmp}/playwright-smoke.XXXXXXXX")
name=playwright-smoke-${run##*.}
project="$run/project space,comma"
mkdir -p "$project/.ai-profiles/reports"
cp "$tests/fixture.html" "$project/fixture.html"
chmod 755 "$run" "$project" "$project/.ai-profiles" "$project/.ai-profiles/reports"
# Only this fresh, disposable reports tree is writable by container UID 1000.
chmod 777 "$project/.ai-profiles/reports"
cleanup() {
  docker rm -f "$name" "$name-fixtures" "$name-dns" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
echo "Smoke artifacts (retained on failure too): $project/.ai-profiles/reports"
docker image inspect --format 'Image architecture: {{.Os}}/{{.Architecture}}' "$image"
docker run --rm --pull=never --entrypoint node \
  -e BASIC_AUTH_HELPER=/opt/playwright/basic-auth.cjs \
  --mount "type=bind,src=$tests,dst=/smoke,readonly" \
  "$image" --test /smoke/basic-auth.test.cjs
# Empty adapter metadata must fail during interpolation, before Docker access.
if AI_PROFILE_PROJECT_DIR= docker compose -f "$tests/../compose.yaml" config > "$run/missing-project.log" 2>&1; then
  echo 'FAIL: Compose accepted an empty project path' >&2
  exit 1
fi
grep -q 'AI_PROFILE_PROJECT_DIR must be the absolute project path' "$run/missing-project.log"
timeout --signal=TERM --kill-after=10s 30s docker run --rm --pull=never --name "$name-fixtures" --entrypoint node \
  --mount "type=bind,src=$tests,dst=/smoke,readonly" \
  --volume "$project:/project" \
  "$image" /smoke/create-fixtures.cjs
cat > "$run/smoke.yaml" <<'YAML'
services:
  playwright:
    image: "${AI_PROFILE_SMOKE_IMAGE:?}"
    entrypoint: ["node", "/smoke/smoke.cjs"]
    environment:
      HOST_UID: ""
      HOST_GID: ""
      SMOKE_PROJECT_DIR: "${AI_PROFILE_PROJECT_DIR:?}"
      SMOKE_MOUNT_PROJECT: "${AI_PROFILE_SMOKE_MOUNT_PROJECT:-}"
    volumes:
      - type: bind
        source: "${AI_PROFILE_SMOKE_DIR:?}"
        target: /smoke
        read_only: true
        bind:
          create_host_path: false
YAML
# Test-only overrides can replay an old Compose layout with its /workspace mount
# while still advertising the real host project through MCP roots. This must fail
# at the host-visible relative screenshot assertion, not just at mount inspection.
AI_PROFILE_PROJECT_DIR="$project" AI_PROFILE_SMOKE_IMAGE="$image" AI_PROFILE_SMOKE_DIR="$tests" \
  AI_PROFILE_PLAYWRIGHT_CA_CERT=.ai-profiles/.private/ca.crt \
  AI_PROFILE_PLAYWRIGHT_STORAGE_STATE=.ai-profiles/.private/auth.json \
  timeout --signal=TERM --kill-after=10s 180s docker compose \
  --project-directory "$project" \
  -f "${AI_PROFILE_SMOKE_COMPOSE:-$tests/../compose.yaml}" -f "$tests/../compose.ca.yaml" \
  -f "$tests/../compose.auth.yaml" -f "$run/smoke.yaml" \
  run --rm --no-deps --pull never -T --name "$name" playwright

# Real installed adapter integration, not merely the image entrypoint. Host Node
# is used only as the test MCP client; browser/runtime dependencies stay in Docker.
repo=$(CDPATH= cd -- "$tests/../../../.." && pwd)
make -C "$repo" install CLIENT=opencode PROFILE=react PROJECT="$project"
cat > "$project/.ai-profiles/opencode/mcp/playwright/smoke-image.yaml" <<'YAML'
services:
  playwright:
    image: "${AI_PROFILE_SMOKE_IMAGE:?}"
YAML
cat > "$project/.ai-profiles/.env.smoke" <<'ENV'
AI_PROFILE_PROJECT_DIR=/wrong/dotenv
ENV
cat > "$project/.ai-profiles/opencode/local.json" <<'JSON'
{"mcp":{"playwright":{"env_file":".ai-profiles/.env.smoke","environment":{
  "AI_PROFILE_PROJECT_DIR":"/wrong/local-mapping",
  "AI_PROFILE_PLAYWRIGHT_NETWORK":"host",
  "AI_PROFILE_PLAYWRIGHT_CA_CERT":".ai-profiles/.private/ca.crt",
  "AI_PROFILE_PLAYWRIGHT_STORAGE_STATE":".ai-profiles/.private/auth.json",
  "COMPOSE_FILE":".ai-profiles/opencode/mcp/playwright/compose.yaml:.ai-profiles/opencode/mcp/playwright/compose.ca.yaml:.ai-profiles/opencode/mcp/playwright/compose.auth.yaml:.ai-profiles/opencode/mcp/playwright/smoke-image.yaml"
}}}}
JSON
# Preserve the first run's evidence, including automatic filename checks.
mkdir "$project/.ai-profiles/reports/direct"
find "$project/.ai-profiles/reports" -maxdepth 1 -type f -exec mv {} "$project/.ai-profiles/reports/direct/" \;
SMOKE_PROJECT_DIR="$project" SMOKE_ADAPTER="$project/.ai-profiles/opencode/ai-profile-mcp" \
  AI_PROFILE_SMOKE_IMAGE="$image" \
  timeout --signal=TERM --kill-after=10s 180s node "$tests/smoke.cjs"

# The production workflow must work without reports, local.json or an environment
# override, after only a manual image build. This fresh scratch project tests it.
auth_project="$run/clean project"
mkdir -p "$auth_project"
cp "$tests/fixture.html" "$auth_project/fixture.html"
repo=$(CDPATH= cd -- "$tests/../../../.." && pwd)
make -C "$repo" install CLIENT=opencode PROFILE=react PROJECT="$auth_project"
test ! -e "$auth_project/.ai-profiles/reports"
test ! -e "$auth_project/.ai-profiles/opencode/local.json"
git -C "$auth_project" init -q
docker image tag "$image" ai-profiles-playwright:0.2.0
SMOKE_AUTH_PROJECT="$auth_project" \
  SMOKE_AUTH_ADAPTER="$auth_project/.ai-profiles/opencode/ai-profile-mcp" \
  timeout --signal=TERM --kill-after=10s 240s node "$tests/auth-smoke.cjs"
git -C "$auth_project" check-ignore --no-index -q .ai-profiles/.private/playwright-basic-auth.json

network_project="$run/network project space,comma"
mkdir "$network_project"
make -C "$repo" install CLIENT=opencode PROFILE=react PROJECT="$network_project"
docker run -d --rm --pull=never --name "$name-dns" --network host --entrypoint node \
  --mount "type=bind,src=$tests,dst=/smoke,readonly" \
  "$image" /smoke/dns-fixture.cjs
attempt=0
until docker logs "$name-dns" 2>&1 | grep -q DNS_READY; do
  if [ "$(docker inspect --format '{{.State.Running}}' "$name-dns")" != true ] || [ "$attempt" -ge 50 ]; then
    docker logs "$name-dns" >&2
    echo 'FAIL: synthetic DNS fixture unavailable (infrastructure)' >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 0.1
done
SMOKE_NETWORK_PROJECT="$network_project" SMOKE_DNS_CONTAINER="$name-dns" \
  timeout --signal=TERM --kill-after=10s 180s node "$tests/network-smoke.cjs"

ca_project="$run/CA project space,comma"
ca_fixtures="$run/CA fixtures"
mkdir "$ca_project" "$ca_fixtures"
for ca in 'CA with spaces' 'other CA'; do
  timeout --signal=TERM --kill-after=10s 30s docker run --rm --pull=never --name "$name-fixtures" --entrypoint node \
    -e "FIXTURE_DIRECTORY=/fixtures/$ca" \
    --mount "type=bind,src=$tests,dst=/smoke,readonly" \
    --volume "$ca_fixtures:/fixtures" \
    "$image" /smoke/create-fixtures.cjs
done
make -C "$repo" install CLIENT=opencode PROFILE=react PROJECT="$ca_project"
SMOKE_CA_PROJECT="$ca_project" SMOKE_CA_FIXTURES="$ca_fixtures" \
  timeout --signal=TERM --kill-after=10s 180s node "$tests/ca-smoke.cjs"
