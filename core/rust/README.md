# Rust implementation

This directory contains the native Rust implementation of the AI Profiles
tooling. The original Python source is preserved as migration reference and is
not required to build, run Make commands, or run the Rust test suite.

## Build and binaries

Build all four native executables from the repository root:

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

These are examples of how to invoke the programs, not build or validation
steps. The binaries should be invoked explicitly; no Make install/update target
selects Rust automatically.

### Locating the source checkout

The native profile manager discovers its source checkout by walking ancestors
of its own executable location and looking for the repository layout
(`core/python/profile_management/`, `profiles/`, `common/skills/`, and
`mcp/servers/`). For a build inside the checkout, the executable is located at
`core/rust/target/release/ai-profile`. If it is copied elsewhere, preserve a
layout where its ancestor contains those source directories. The manager does
not use the caller's working directory or the compile-time Cargo manifest path
as the source root.

The MCP adapter is standalone after it has been copied to a project's
`.opencode/` directory. It reads that project's installed state and local MCP
override at each startup. The MariaDB launcher likewise starts only when
explicitly configured and invoked.

## Python coexistence

Existing profile-management Make targets (`install`, `update`, `switch`,
`status`, `validate`, `test`, and the other manager operations) use the Rust
manager through Cargo. `make rust-build` only builds the native executables.
The Rust payload still preserves existing Python files and commands where
profile compatibility requires them; those payload files do not make Python a
dependency of the manager build or its Rust test harness. Rust dependencies
are managed by Cargo in this directory.

**Current migration gap:** `mcp/config.rs` still renders local MCP entries with
`python3` and the preserved `.py` adapter, as required by the original K-03
compatibility contract. Thus Make/build/tests are Rust-only, but the stricter
goal of having no Python-based runtime in installed projects is not yet met;
the payload/adapter contract must be revised before that can be claimed.

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

The Rust manager does not invoke a host Python interpreter to implement any
operation. The MariaDB launcher retains the upstream container's documented
entrypoint (`python src/server.py`) inside the pinned Docker image; that
upstream server is outside this migration's scope. Existing project payloads
and local MCP renderings intentionally retain their original Python filenames
and commands for compatibility. PHP helpers are also outside the Python
migration scope.
