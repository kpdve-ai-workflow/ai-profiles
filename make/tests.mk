# Test suites and isolated VM orchestration.
.PHONY: test validate-rust-migration test-playwright test-all test-vm-up test-vm-stop \
	test-vm-sync test-vm-reprovision test-vm-logs _test-guest \
	_validate-rust-migration-guest _check-test-guest

test validate-rust-migration test-playwright test-all:
	@if test -f /var/lib/ai-profiles-vm/ready; then \
	  case "$@" in \
	    test) $(MAKE) _test-guest ;; \
	    validate-rust-migration) $(MAKE) _validate-rust-migration-guest ;; \
	    *) bash tests/vm/guest.sh "$@" "$$(cat /proc/sys/kernel/random/uuid)" ;; \
	  esac; \
	else bash tests/vm/host.sh "$@"; fi

test-vm-up:
	@bash tests/vm/host.sh up
test-vm-stop:
	@bash tests/vm/host.sh stop
test-vm-sync:
	@bash tests/vm/host.sh sync
test-vm-reprovision:
	@bash tests/vm/host.sh reprovision

test-vm-logs:
	@bash tests/vm/logs.sh

_check-test-guest:
	@test -f /var/lib/ai-profiles-vm/ready && test "$$(id -u):$$(id -g)" = 1000:1000 || \
	  (echo 'Wymagana przygotowana VM i tester 1000:1000' >&2; exit 1)

_test-guest: _check-test-guest
	cargo test --manifest-path $(RUST_MANIFEST) --locked --all-targets

_validate-rust-migration-guest: _check-test-guest
	@set -eu; \
	log=$$(mktemp); active=initializing; \
	trap 'rm -f "$$log"' EXIT; \
	remaining='format release-build clippy make-validate make-test'; \
	interrupted() { printf 'FAILED: %s (interrupted)\n' "$$active"; for pending in $$remaining; do printf 'NOT RUN: %s\n' "$$pending"; done; exit 130; }; \
	trap interrupted HUP INT TERM; \
	run_group() { \
	  group=$$1; shift; \
	  remaining=$$(printf '%s' "$$remaining" | sed "s/^$$group //; s/ $$group / /; s/ $$group$$//; s/^$$group$$//"); \
	  active=$$group; \
	  printf '== %s ==\n' "$$group"; \
	  if "$$@" >"$$log" 2>&1; then \
	    cat "$$log"; \
	    if [ "$$group" = make-test ]; then \
	      count=$$(grep -Eo '[0-9]+ passed' "$$log" | awk '{ total += $$1 } END { print total+0 }'); \
	      if [ "$$count" -eq 0 ]; then printf 'FAILED: %s (zero tests)\n' "$$group"; exit 1; fi; \
	      for suite in foundation_tests profiles_tests mcp_config_tests mcp_runtime_tests lifecycle_tests cli_tests wordpress_dump_tests wordpress_preflight_tests wordpress_localize_tests restore_tests; do \
	        suite_count=$$(awk -v pattern="^test .*$$suite::" '$$0 ~ pattern { count++ } END { print count+0 }' "$$log"); \
	        if [ "$$suite_count" -eq 0 ]; then printf 'FAILED: %s (required suite %s missing)\n' "$$group" "$$suite"; exit 1; fi; \
	        printf '  %s: %s tests\n' "$$suite" "$$suite_count"; \
	      done; \
	      printf 'PASSED: %s (%s tests)\n' "$$group" "$$count"; \
	    else printf 'PASSED: %s\n' "$$group"; fi; active=complete; \
	  else \
	    result=$$?; cat "$$log"; printf 'FAILED: %s (exit %s)\n' "$$group" "$$result"; \
	    for pending in $$remaining; do printf 'NOT RUN: %s\n' "$$pending"; done; exit "$$result"; \
	  fi; \
	}; \
	run_group format cargo fmt --manifest-path $(RUST_MANIFEST) -- --check; \
	run_group release-build cargo build --manifest-path $(RUST_MANIFEST) --target-dir "$(RUST_TARGET)" --locked --release --bins; \
	run_group clippy cargo clippy --manifest-path $(RUST_MANIFEST) --locked --all-targets -- -D warnings; \
	remaining='make-validate make-test'; active=make-validate; \
	run_group make-validate make validate; \
	run_group make-test make test
