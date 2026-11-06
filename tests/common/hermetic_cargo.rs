//! `cargo` commands for fixture crates that own their build directories.
//!
//! A fixture build is read back from the target directory the test expects, so
//! the command must not inherit anything that moves it. Building the command in
//! one place keeps every fixture spawn on the same environment and lets a unit
//! test inspect it without running Cargo.

use std::process::Command;

use camino::Utf8Path;

/// Variables that would move a build's artefacts, or put its lock, somewhere
/// the fixture does not own.
const INHERITED_BUILD_LOCATIONS: [&str; 2] = ["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"];

/// Returns a `cargo` command that runs in `manifest_dir` and builds into
/// `target_dir`, whatever the caller exports.
///
/// `CARGO_TARGET_DIR` is set explicitly, so an exported value cannot win, and
/// the two variables that would split or share the build directory are removed.
#[must_use]
pub fn hermetic_cargo(manifest_dir: &Utf8Path, target_dir: &Utf8Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(manifest_dir)
        .env("CARGO_TARGET_DIR", target_dir.as_str());
    for variable in INHERITED_BUILD_LOCATIONS {
        command.env_remove(variable);
    }
    command
}

#[cfg(test)]
#[path = "hermetic_cargo_tests.rs"]
mod tests;
