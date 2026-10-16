//! Internal support for theoremc Cargo build scripts.
//!
//! This crate owns build-time theorem discovery and generated-suite writing.
//! It is an internal workspace boundary: `build.rs`, direct unit tests, and
//! Cargo fixture crates must depend on it rather than include its source files
//! by path. The root `theoremc` library deliberately does not re-export this
//! API for downstream consumers.

mod build_script;
mod discovery;
mod suite;

pub use build_script::{BuildScriptError, BuildScriptOutput, prepare_build_script};
pub use discovery::{BuildDiscovery, BuildDiscoveryError, discover_theorem_inputs};
pub use suite::{BuildSuiteError, write_theorem_suite};
