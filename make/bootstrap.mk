# Shared build configuration and commands for working with this repository.
RUST_MANIFEST := core/Cargo.toml
RUST_TARGET := $(abspath core/target)
PROFILE_CLI := "$(RUST_TARGET)/release/ai-profile"

.PHONY: rust-build pull-update

rust-build:
	cargo build --manifest-path $(RUST_MANIFEST) --target-dir "$(RUST_TARGET)" --locked --release --bins

# Build the manager and every native payload from the same sources before use.
create rename install update uninstall switch status validate bump migrate-legacy: rust-build

pull-update: check-client check-project
	git pull --ff-only
	$(MAKE) update CLIENT="$(CLIENT)" PROJECT="$(PROJECT)"
