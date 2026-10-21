# Rust implementation

This directory contains the native Rust implementation of the AI Profiles
tooling. The original Python source is preserved as migration reference and is
not required to build, run Make commands, or run the Rust test suite.

## Build and binaries

Build the native executables from the repository root:

```sh
make rust-build
```

Equivalent Cargo command:

```sh
cargo build --manifest-path core/rust/Cargo.toml --locked --release --bins
```

This builds, but does not run, the executables in
`core/rust/target/release/`:

| Binary | Purpose | Explicit invocation |
| --- | --- | --- |
| `ai-profile` | Profile manager CLI | `core/rust/target/release/ai-profile help` |
| `ai-profile-mcp` | Standalone MCP adapter | Configured as the project's MCP executable |
| `mariadb-launcher` | MariaDB MCP container launcher | Configured as the MariaDB MCP executable |
| `restore-updraft` | WordPress database restore | `core/rust/target/release/restore-updraft --help` |
| `test-inventory` | Shared test-discovery skill inventory | Installed as the skill's `scripts/inventory-tests` |

These are examples of how to invoke the programs, not build or validation
steps. `make install PROFILE=<profile> PROJECT=<project>` builds all
executables and invokes the Rust manager. Other manager Make targets also
build before invoking the manager. There is no `install2` target.

### Locating the source checkout

The native profile manager discovers its source checkout by walking ancestors
of its own executable location and looking for the repository layout
(`core/rust/src/`, `profiles/`, `common/skills/`, and
`mcp/servers/`). For a build inside the checkout, the executable is located at
`core/rust/target/release/ai-profile`. If it is copied elsewhere, preserve a
layout where its ancestor contains those source directories. The manager does
not use the caller's working directory or the compile-time Cargo manifest path
as the source root.

The MCP adapter is standalone after it has been copied to a project's
`.opencode/` directory. It reads that project's installed state and local MCP
override at each startup. The MariaDB launcher likewise starts only when
explicitly configured and invoked.

## Native project payload

Existing profile-management Make targets (`install`, `update`, `switch`,
`status`, `validate`, `test`, and the other manager operations) use the Rust
manager through Cargo. `make rust-build` only builds the native executables.
Make fixes the build directory to `core/rust/target/`, including when
`CARGO_TARGET_DIR` is set. The manager reads native payloads from its checkout's
`core/rust/target/release/`. Missing binaries stop payload preparation before
project writes; direct Cargo users must build all release binaries there first.

| Release binary | Installed executable (mode 0755) |
| --- | --- |
| `ai-profile-mcp` | `.opencode/ai-profile-mcp` |
| `mariadb-launcher` | `.opencode/mcp/mariadb/mariadb-launcher` (selected MariaDB) |
| `restore-updraft` | `.opencode/ai-profile/scripts/restore-updraft` (WordPress) |
| `test-inventory` | `.opencode/skills/test-gap-discovery/scripts/inventory-tests` |

Installed files are copied, hashed and managed through the existing ownership
checks. Update removes previously managed Python payloads only after verifying
their stored hashes. Unmanaged files and local overrides remain protected.
PHP helpers and profile documentation are still delivered. Python sources stay
in this repository as reference; Python files are not distributed. Installed
executables need neither Cargo nor the source checkout.

## Production Python-to-Rust coverage map

The entries below map the production Python callables to their Rust
implementation. Rust module filenames are relative to `core/rust/src/`.

| Python production source and symbols | Rust implementation |
| --- | --- |
| `profile_management/context.py`: `RepositoryContext` | `repository.rs`, `model.rs` |
| `profile_management/errors.py`: `ProfileError` | `error.rs` and operation-specific errors |
| `profile_management/versioning.py`: `version_tuple` | `model.rs` |
| `profile_management/constants.py`: constants | `model.rs`, operation modules |
| `profile_management/infrastructure/filesystem.py`: `sha`, `json_bytes`, `read_json`, `file_path`, `check_files`, `write_bytes`, `cleanup_empty_directories` | `codec.rs`, `filesystem.rs` |
| `profile_management/infrastructure/repository.py`: `profile_names`, `profile_metadata`, `source_commit` | `repository.rs` |
| `profile_management/infrastructure/project.py`: `selected_project`, `state_for`, `config_for`, `remove_gitignore_line` | `project.rs` |
| `profile_management/infrastructure/mcp.py`: `mcp_definition`, `desired_mcp`, `configured_mcp`, `check_mcp` | `mcp/config.rs` |
| `profile_management/infrastructure/payload.py`: `desired_files` | `profile_management/payload.rs` |
| `profile_management/actions/profiles.py`: `profile_names`, `version_tuple`, `profile_metadata`, `create_profile`, `bump_profile` | `repository.rs`, `model.rs`, `profile_management/profiles.rs` |
| `profile_management/actions/installation.py`: `apply_profile`, `uninstall` | `profile_management/installation.rs`, `profile_management/uninstall.rs` |
| `profile_management/actions/status.py`: `status` | `profile_management/status.rs` |
| `profile_management/actions/validation.py`: `validate_sources` | `profile_management/validation.rs` |
| `profile_management/actions/migration.py`: `migrate_legacy` | `profile_management/migration.rs` |
| `profile_management/cli.py`: `ProfileArgumentParser`, `main` | `profile_management/cli.rs`, `bin/ai-profile.rs` |
| `profile_management/mcp_runtime.py`: `ConfigError`, `read_json`, `project_path`, `load_local`, `merge`, `split_access_mode`, `apply_access_mode`, `validate_servers`, `resolve`, `render`, `changed_override_paths` | `mcp/config.rs` |
| `profile_management/mcp_runtime.py`: `dotenv`, `environment_for` | `mcp/environment.rs` |
| `profile_management/mcp_runtime.py`: `launch`, `main` | `mcp/runtime.rs`, `bin/ai-profile-mcp.rs` |
| `mcp/launchers/mariadb/launch.py`: `StartupError`, `docker_environment`, `image_available`, `fetch_source` | `mariadb.rs`, `mariadb/archive.rs` |
| `mcp/launchers/mariadb/launch.py`: `ensure_image`, `launch`, `main` | `mariadb/runtime.rs`, `bin/mariadb-launcher.rs` |
| `mcp/launchers/mariadb/launch.py`: pinned image/source/platform constants | `mariadb.rs`, `mariadb/archive.rs` |
| `profiles/wordpress/scripts/restore-updraft.py`: `RestoreError`, `inside`, `origin`, `mappings`, `mapped_url`, `inspect_dump`, `file_hash` | `wordpress.rs`, `wordpress/preflight.rs`, `wordpress/mapping.rs`, `wordpress/dump.rs` |
| `profiles/wordpress/scripts/restore-updraft.py`: `Ddev`, `literal`, `topology` | `wordpress/ddev.rs` |
| `profiles/wordpress/scripts/restore-updraft.py`: `preflight` | `wordpress/preflight.rs` |
| `profiles/wordpress/scripts/restore-updraft.py`: `localize` | `wordpress/localize.rs` |
| `profiles/wordpress/scripts/restore-updraft.py`: `restore`, `main` | `wordpress/restore.rs`, `bin/restore-updraft.rs` |
| Python embedded in `common/skills/test-gap-discovery/scripts/inventory-tests.sh` | `bin/test-inventory.rs` |

The Rust manager does not invoke a host Python interpreter to implement any
operation. The MariaDB launcher retains the upstream container's documented
entrypoint (`python src/server.py`) inside the pinned Docker image; that
upstream server is outside this migration's scope. Third-party MCP runtimes
(Docker/Node) and PHP helpers keep their own dependencies.
