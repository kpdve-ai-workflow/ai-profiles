PYTHON ?= python3
PROJECT ?=
PROFILE ?= backend-go

.PHONY: check-project install update uninstall switch status validate test pull-update approve lock gate-status

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

test:
	$(PYTHON) -m unittest discover -s tests -v

pull-update: check-project
	git pull --ff-only
	$(PYTHON) profile.py update --project "$(PROJECT)"

TASK ?= TASK-001
approve: check-project
	$(PYTHON) gate.py "$(PROJECT)" approve "$(TASK)"

lock: check-project
	$(PYTHON) gate.py "$(PROJECT)" lock

gate-status: check-project
	$(PYTHON) gate.py "$(PROJECT)" status
