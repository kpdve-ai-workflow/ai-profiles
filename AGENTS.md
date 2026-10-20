Instalacja odbywa się poprzez `make install2`.

- Python 3.10+; Python runtime and tests use only the standard library. Rust dependencies are managed separately by Cargo under `core/rust/`.
- During the Python-to-Rust migration, preserve the working Python implementation. Load and follow the `rust` skill for Rust implementation and review. Execute only the single `T-xx` task explicitly assigned per request; do not proceed to the next task independently. Do not run tests or behavioral/smoke checks before T-25. T-25 is the sole full validation, run once with `make validate-rust-migration` after T-24.
