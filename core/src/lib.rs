//! Native Rust implementation of the AI Profiles tooling.
//!
//! Make and installed project tooling use this implementation. Python sources
//! remain in the repository as migration reference only.

mod codec;
mod error;
mod filesystem;
mod layout;
pub mod logging;
pub mod mcp;
mod model;
pub mod playwright;
mod process;
pub mod profile_management;
mod project;
mod repository;
pub mod wordpress;

#[cfg(test)]
#[path = "../tests/foundation.rs"]
mod foundation_tests;
#[cfg(test)]
#[path = "../tests/lifecycle.rs"]
mod lifecycle_tests;
#[cfg(test)]
#[path = "../tests/logging_session.rs"]
mod logging_session_tests;
#[cfg(test)]
#[path = "../tests/logging.rs"]
mod logging_tests;
#[cfg(test)]
#[path = "../tests/manager_fixture.rs"]
mod manager_fixture;
#[cfg(test)]
#[path = "../tests/manager_logging.rs"]
mod manager_logging_tests;
#[cfg(test)]
#[path = "../tests/mcp_config.rs"]
mod mcp_config_tests;
#[cfg(test)]
#[path = "../tests/mcp_runtime.rs"]
mod mcp_runtime_tests;
#[cfg(test)]
#[path = "../tests/novamira_bundle.rs"]
mod novamira_bundle_tests;
#[cfg(test)]
#[path = "../tests/novamira.rs"]
mod novamira_tests;
#[cfg(test)]
#[path = "../tests/playwright.rs"]
mod playwright_tests;
#[cfg(test)]
#[path = "../tests/process_logging.rs"]
mod process_logging_tests;
#[cfg(test)]
#[path = "../tests/profiles.rs"]
mod profiles_tests;
#[cfg(test)]
#[path = "../tests/runtime_logging.rs"]
mod runtime_logging_tests;
#[cfg(test)]
#[path = "../tests/wordpress_dump.rs"]
mod wordpress_dump_tests;
#[cfg(test)]
#[path = "../tests/wordpress_localize.rs"]
mod wordpress_localize_tests;
#[cfg(test)]
#[path = "../tests/wordpress_preflight.rs"]
mod wordpress_preflight_tests;
