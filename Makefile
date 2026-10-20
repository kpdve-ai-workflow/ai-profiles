PYTHON ?= python3
PROJECT ?=
PROFILE ?=
DESCRIPTION ?=
PART ?= patch
.DEFAULT_GOAL := help

.PHONY: help create check-profile check-project install update uninstall switch status validate test pull-update approve lock gate-status bump

help:
	@$(PYTHON) profile.py help

check-profile:
	@test -n "$(PROFILE)" || (echo "Podaj PROFILE=nazwa-profilu" >&2; exit 2)

create: check-profile
	$(PYTHON) profile.py create "$(PROFILE)" $(if $(DESCRIPTION),--description "$(DESCRIPTION)")

check-project:
	@test -n "$(PROJECT)" || (echo "Podaj PROJECT=/sciezka/do/projektu" >&2; exit 2)

install: check-project check-profile
	$(PYTHON) profile.py install "$(PROFILE)" --project "$(PROJECT)"

update: check-project
	$(PYTHON) profile.py update --project "$(PROJECT)"

uninstall: check-project
	$(PYTHON) profile.py uninstall --project "$(PROJECT)"

switch: check-project check-profile
	$(PYTHON) profile.py switch "$(PROFILE)" --project "$(PROJECT)"

status: check-project
	$(PYTHON) profile.py status --project "$(PROJECT)"

validate:
	$(PYTHON) profile.py validate $(if $(strip $(PROFILE)),"$(PROFILE)")

bump: check-profile
	$(PYTHON) profile.py bump "$(PROFILE)" --part "$(PART)"

test:
	$(PYTHON) -m unittest discover -s tests -v

pull-update: check-project
	git pull --ff-only
	$(PYTHON) profile.py update --project "$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."
