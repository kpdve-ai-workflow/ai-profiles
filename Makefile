RUST_MANIFEST := core/rust/Cargo.toml
PROFILE_CLI := cargo run --quiet --manifest-path $(RUST_MANIFEST) --locked --bin ai-profile --
PROJECT ?=
PROFILE ?=
DESCRIPTION ?=
PART ?= patch
.DEFAULT_GOAL := help

.PHONY: help create check-profile check-project install install2 update uninstall switch status validate test pull-update approve lock gate-status bump rust-build validate-rust-migration

help:
	@$(PROFILE_CLI) help

check-profile:
	@test -n "$(PROFILE)" || (echo "Podaj PROFILE=nazwa-profilu" >&2; exit 2)

create: check-profile
	$(PROFILE_CLI) create "$(PROFILE)" $(if $(DESCRIPTION),--description "$(DESCRIPTION)")

check-project:
	@test -n "$(PROJECT)" || (echo "Podaj PROJECT=/sciezka/do/projektu" >&2; exit 2)

install: check-project check-profile
	$(PROFILE_CLI) install "$(PROFILE)" --project "$(PROJECT)"

install2: install

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
	cargo build --manifest-path $(RUST_MANIFEST) --locked --release --bins

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
	run_group release-build cargo build --manifest-path $(RUST_MANIFEST) --locked --release --bins; \
	run_group clippy cargo clippy --manifest-path $(RUST_MANIFEST) --locked --all-targets -- -D warnings; \
	remaining='make-validate make-test'; active=make-validate; \
	run_group make-validate make validate; \
	run_group make-test make test

pull-update: check-project
	git pull --ff-only
	$(PROFILE_CLI) update --project "$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."
