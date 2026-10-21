RUST_MANIFEST := core/rust/Cargo.toml
RUST_TARGET := $(abspath core/rust/target)
PROFILE_CLI := "$(RUST_TARGET)/release/ai-profile"
PROJECT ?=
PROFILE ?=
NEW_NAME ?=
DESCRIPTION ?=
PART ?= patch
.DEFAULT_GOAL := help

.PHONY: help create rename check-profile check-new-name check-project install update uninstall switch status validate test pull-update approve lock gate-status bump rust-build validate-rust-migration migrate-legacy

help:
	@cat core/rust/help.txt

check-profile:
	@test -n "$(PROFILE)" || (echo "Podaj PROFILE=nazwa-profilu" >&2; exit 2)

create: check-profile
	$(PROFILE_CLI) create "$(PROFILE)" $(if $(DESCRIPTION),--description "$(DESCRIPTION)")

check-new-name:
	@test -n "$(NEW_NAME)" || (echo "Podaj NEW_NAME=nowa-nazwa-profilu" >&2; exit 2)

rename: check-profile check-new-name
	$(PROFILE_CLI) rename "$(PROFILE)" --new-name "$(NEW_NAME)"

check-project:
	@test -n "$(PROJECT)" || (echo "Podaj PROJECT=/sciezka/do/projektu" >&2; exit 2)

install: check-project check-profile
	$(PROFILE_CLI) install "$(PROFILE)" --project "$(PROJECT)"

update: check-project
	$(PROFILE_CLI) update --project "$(PROJECT)"

uninstall: check-project
	$(PROFILE_CLI) uninstall --project "$(PROJECT)"

switch: check-project check-profile
	$(PROFILE_CLI) switch "$(PROFILE)" --project "$(PROJECT)"

status: check-project
	$(PROFILE_CLI) status --project "$(PROJECT)"

validate:
	$(PROFILE_CLI) validate $(if $(strip $(PROFILE)),"$(PROFILE)")

bump: check-profile
	$(PROFILE_CLI) bump "$(PROFILE)" --part "$(PART)"

test:
	cargo test --manifest-path $(RUST_MANIFEST) --locked --all-targets

rust-build:
	cargo build --manifest-path $(RUST_MANIFEST) --target-dir "$(RUST_TARGET)" --locked --release --bins

# Build the manager and every native payload from the same sources before use.
create rename install update uninstall switch status validate bump migrate-legacy: rust-build

migrate-legacy: check-project
	$(PROFILE_CLI) migrate-legacy --accept-existing --project "$(PROJECT)"

validate-rust-migration:
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
	      for suite in foundation_tests profiles_tests mcp_config_tests mcp_runtime_tests lifecycle_tests cli_tests mariadb_tests wordpress_dump_tests wordpress_preflight_tests wordpress_localize_tests restore_tests; do \
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

pull-update: check-project
	git pull --ff-only
	$(MAKE) update PROJECT="$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."
