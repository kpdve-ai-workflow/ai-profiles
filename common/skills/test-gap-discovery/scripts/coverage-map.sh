#!/usr/bin/env bash
# Read-only coverage tooling hints. Does not run tests or claim coverage percentages.
set -euo pipefail
ROOT="${1:-.}"
if [ ! -d "$ROOT" ]; then echo "Not a directory: $ROOT" >&2; exit 2; fi
cd "$ROOT"
echo 'Coverage tooling candidates (suggestions only; NOTHING executed):'
if [ -f go.mod ]; then echo '  Go: go test -coverprofile=<file> ./... ; go tool cover -func=<file>'; fi
if [ -f Cargo.toml ]; then echo '  Rust: cargo llvm-cov / cargo tarpaulin if separately installed'; fi
if [ -f pyproject.toml ] || [ -f pytest.ini ]; then echo '  Python: pytest --cov if the plugin is installed'; fi
if [ -f package.json ]; then echo '  JS/TS: framework-specific test --coverage option, if configured'; fi
if [ -f pom.xml ]; then echo '  Java/Maven: JaCoCo if configured'; fi
if [ -f build.gradle ] || [ -f build.gradle.kts ]; then echo '  JVM/Gradle: configured JaCoCo plugin'; fi
cat <<'MSG'
These are discovery hints, not proof of installed tools or validated commands.
Coverage is diagnostic; behavior coverage, independent assertions and valid counterexamples matter more.
Do not run testing or installation commands unless repository policy allows it.
MSG
