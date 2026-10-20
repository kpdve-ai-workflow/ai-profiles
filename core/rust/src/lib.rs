//! Native Rust implementation of the AI Profiles tooling.
//!
//! The Python implementation remains supported; this crate is an additional,
//! explicitly selected implementation.

mod codec;
mod error;
mod filesystem;
mod model;
mod process;
pub(crate) mod profile_management;
mod project;
mod repository;
