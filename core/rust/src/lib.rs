//! Native Rust implementation of the AI Profiles tooling.
//!
//! The Python implementation remains supported; this crate is an additional,
//! explicitly selected implementation.

mod codec;
mod error;
mod filesystem;
pub mod mariadb;
pub mod mcp;
mod model;
mod process;
pub mod profile_management;
mod project;
mod repository;
pub(crate) mod wordpress;
