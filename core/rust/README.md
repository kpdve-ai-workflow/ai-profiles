# Rust implementation

This directory contains the native Rust implementation of the AI Profiles
tooling. The Python implementation remains supported and remains the default
used by the repository's existing Make targets.

The crate is intentionally a single package. Native binaries and operations are
added in their assigned migration stages; this foundation does not provide
placeholder commands or change project payloads.

Rust dependencies are declared in `Cargo.toml` and pinned by `Cargo.lock`.
Python runtime and test dependencies remain limited to the Python standard
library.
