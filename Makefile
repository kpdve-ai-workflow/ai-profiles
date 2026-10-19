PYTHON ?= python3
PROJECT ?=
PROFILE ?= backend-go
PART ?= patch

.PHONY: check-project install update uninstall switch status validate test pull-update approve lock gate-status bump

check-project:
	@test -n "$(PROJECT)" || (echo "Podaj PROJECT=/sciezka/do/projektu" >&2; exit 2)

install: check-project
	$(PYTHON) profile.py install $(PROFILE) --project "$(PROJECT)"

update: check-project
	$(PYTHON) profile.py update --project "$(PROJECT)"

uninstall: check-project
	$(PYTHON) profile.py uninstall --project "$(PROJECT)"

switch: check-project
	$(PYTHON) profile.py switch $(PROFILE) --project "$(PROJECT)"

status: check-project
	$(PYTHON) profile.py status --project "$(PROJECT)"

validate:
	$(PYTHON) profile.py validate

bump:
	$(PYTHON) profile.py bump "$(PROFILE)" --part "$(PART)"

test:
	$(PYTHON) -m unittest discover -s tests -v

pull-update: check-project
	git pull --ff-only
	$(PYTHON) profile.py update --project "$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."
