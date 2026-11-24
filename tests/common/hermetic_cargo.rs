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

/// Returns a `cargo` command that runs in `manifest_dir`, builds into
/// `target_dir` and resolves from `cargo_home`, whatever the caller exports.
///
/// `CARGO_TARGET_DIR` and `CARGO_HOME` are set explicitly, so an exported value
/// cannot win, and the two variables that would split or share the build
/// directory are removed. The command runs offline: a fixture's lock is seeded
/// from a committed template and its dependencies are fetched into the fixture
/// Cargo home before the tests run, so it needs no network and must not wait on
/// the index.
///
/// The home is the fixture's rather than the caller's because offline Cargo
/// still takes `$CARGO_HOME/.package-cache` exclusively. Sharing the caller's
/// home is what makes three nested builds block behind another repository's
/// `cargo`; a dedicated home has its own lock.
#[must_use]
pub fn hermetic_cargo(
    manifest_dir: &Utf8Path,
    target_dir: &Utf8Path,
    cargo_home: &Utf8Path,
) -> Command {
    let mut command = Command::new("cargo");
    for variable in INHERITED_BUILD_LOCATIONS {
        command.env_remove(variable);
    }
    // Offline is set after the removals and the home, so it is the last
    // operation on the command's environment.
    command
        .current_dir(manifest_dir)
        .env("CARGO_TARGET_DIR", target_dir.as_str())
        .env("CARGO_HOME", cargo_home.as_str())
        .env("CARGO_NET_OFFLINE", "true");
    command
}

#[cfg(test)]
#[path = "hermetic_cargo_tests.rs"]
mod tests;
