//! Application boundary for the `cargo theorem` Cargo subcommand.
//!
//! Configuration, command parsing, and process output live here so the
//! reusable theorem libraries remain independent of application policy.

mod agent_context;
mod cli;
mod config;
mod error;

pub use cli::run;
