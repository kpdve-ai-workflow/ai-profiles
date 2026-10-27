# Public entry point. Keep the default goal here, before importing categories.
.DEFAULT_GOAL := help

include make/help.mk

# A help invocation loads no operational recipes, so `make test help` only
# prints test guidance and can never accidentally start the test suite.
ifeq ($(filter help,$(MAKECMDGOALS)),)
include make/bootstrap.mk
include make/profiles.mk
include make/agent.mk
include make/tests.mk
endif
