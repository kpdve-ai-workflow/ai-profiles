# Checkout-only workflow. Parameters travel through the environment, not shell code.
BRAIN_MODEL ?= gpt-6.1-sol
BRAIN_THINKING ?= xhigh
RESEARCHER_MODEL ?= gpt-6-luna
RESEARCHER_THINKING ?= medium
RESEARCH_TIMEOUT ?= 180
RESEARCHER_MAX_TASKS ?= 8
SESSION ?=
export AI_AGENT_CLIENT = $(CLIENT)
export AI_AGENT_BRAIN_MODEL = $(BRAIN_MODEL)
export AI_AGENT_BRAIN_THINKING = $(BRAIN_THINKING)
export AI_AGENT_RESEARCHER_MODEL = $(RESEARCHER_MODEL)
export AI_AGENT_RESEARCHER_THINKING = $(RESEARCHER_THINKING)
export AI_AGENT_RESEARCH_TIMEOUT = $(RESEARCH_TIMEOUT)
export AI_AGENT_RESEARCHER_MAX_TASKS = $(RESEARCHER_MAX_TASKS)
export AI_AGENT_SESSION = $(SESSION)
AGENT_CLI := "$(RUST_TARGET)/release/ai-profile-agent"

.PHONY: agent agent-start agent-status agent-stop
agent agent-start:
	@test "$$AI_AGENT_CLIENT" = codex || (echo 'Ten tryb wymaga CLIENT=codex' >&2; exit 2)
	$(MAKE) rust-build
	@$(AGENT_CLI) start

agent-status agent-stop:
	@test -n "$$AI_AGENT_SESSION" || (echo 'Podaj SESSION z wyniku make agent' >&2; exit 2)
	$(MAKE) rust-build
	@$(AGENT_CLI) $(@:agent-%=%)
