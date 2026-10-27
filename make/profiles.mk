# Profile lifecycle and production commands.
PROJECT ?=
PROFILE ?=
CLIENT ?=
NEW_NAME ?=
DESCRIPTION ?=
PART ?= patch

.PHONY: create rename check-profile check-new-name check-project check-client check-no-client \
	install update uninstall switch status validate bump migrate-legacy \
	install-opencode install-claude install-codex approve lock gate-status

check-profile:
	@test -n "$(PROFILE)" || (echo "Podaj PROFILE=nazwa-profilu" >&2; exit 2)

create: check-profile check-no-client
	$(PROFILE_CLI) create "$(PROFILE)" $(if $(DESCRIPTION),--description "$(DESCRIPTION)")

check-new-name:
	@test -n "$(NEW_NAME)" || (echo "Podaj NEW_NAME=nowa-nazwa-profilu" >&2; exit 2)

rename: check-profile check-new-name check-no-client
	$(PROFILE_CLI) rename "$(PROFILE)" --new-name "$(NEW_NAME)"

check-project:
	@test -n "$(PROJECT)" || (echo "Podaj PROJECT=/sciezka/do/projektu" >&2; exit 2)

check-client:
	@case "$(CLIENT)" in opencode|claude|codex|all) ;; *) echo 'Podaj CLIENT=opencode|claude|codex|all' >&2; exit 2;; esac

check-no-client:
	@test "$(origin CLIENT)" = undefined -o "$(origin CLIENT)" = file || (echo 'Ta komenda nie przyjmuje CLIENT' >&2; exit 2)

install: check-client check-project check-profile
	$(PROFILE_CLI) install "$(PROFILE)" --client "$(CLIENT)" --project "$(PROJECT)"

update: check-client check-project
	$(PROFILE_CLI) update --client "$(CLIENT)" --project "$(PROJECT)"

uninstall: check-client check-project
	$(PROFILE_CLI) uninstall --client "$(CLIENT)" --project "$(PROJECT)"

switch: check-client check-project check-profile
	$(PROFILE_CLI) switch "$(PROFILE)" --client "$(CLIENT)" --project "$(PROJECT)"

status: check-project
	$(PROFILE_CLI) status $(if $(filter-out file undefined,$(origin CLIENT)),--client "$(CLIENT)") --project "$(PROJECT)"

validate: check-no-client
	$(PROFILE_CLI) validate $(if $(strip $(PROFILE)),"$(PROFILE)")

bump: check-profile check-no-client
	$(PROFILE_CLI) bump "$(PROFILE)" --part "$(PART)"

migrate-legacy: check-project
	$(PROFILE_CLI) migrate-legacy --accept-existing --project "$(PROJECT)"

# Compatibility aliases: the approval gate no longer exists.
approve lock gate-status: check-project
	@echo "Approval gate removed; no task unlock is required."

install-opencode install-claude install-codex:
	@selected="$(@:install-%=%)"; \
	if test "$(origin CLIENT)" != file && test "$(CLIENT)" != "$$selected"; then echo 'CLIENT jest sprzeczne z aliasem' >&2; exit 2; fi; \
	$(MAKE) install CLIENT="$$selected" PROFILE="$(PROFILE)" PROJECT="$(PROJECT)"
