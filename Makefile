PYTHON ?= python3
PROFILE_CLI := $(PYTHON) core/profile_management/implementations/python/ai-profile.py
PROJECT ?=
PROFILE ?=
DESCRIPTION ?=
PART ?= patch
.DEFAULT_GOAL := help

.PHONY: help create check-profile check-project install update uninstall switch status validate test pull-update approve lock gate-status bump

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
	$(PYTHON) -m unittest discover -s tests -v

pull-update: check-project
	git pull --ff-only
	$(PROFILE_CLI) update --project "$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."
