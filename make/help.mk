# Topic-specific help. Operational Makefiles are intentionally not imported
# when `help` is one of the requested goals.
HELP_REQUEST := $(firstword $(filter-out help,$(MAKECMDGOALS)))

ifeq ($(HELP_REQUEST),)
HELP_FILE := make/help/index.txt
else ifneq ($(filter $(HELP_REQUEST),profile profiles),)
HELP_FILE := make/help/profile.txt
else ifeq ($(HELP_REQUEST),test)
HELP_FILE := make/help/test.txt
else ifeq ($(HELP_REQUEST),bootstrap)
HELP_FILE := make/help/bootstrap.txt
else ifneq ($(filter $(HELP_REQUEST),agent agent-start brain-researcher agent-status agent-stop),)
HELP_FILE := make/help/agent.txt
endif

.PHONY: help
help:
	@if test -n "$(HELP_FILE)"; then cat "$(HELP_FILE)"; else \
	  echo 'Nieznany temat pomocy: $(HELP_REQUEST)' >&2; \
	  echo 'Użyj: make help' >&2; exit 2; \
	fi

ifneq ($(filter help,$(MAKECMDGOALS)),)
.PHONY: test profile profiles bootstrap agent agent-start brain-researcher agent-status agent-stop
test profile profiles bootstrap agent agent-start brain-researcher agent-status agent-stop:
	@:
endif
