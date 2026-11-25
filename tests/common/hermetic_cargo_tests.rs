//! Regression tests for the fixture `cargo` environment.
//!
//! The fixtures below name literal POSIX paths such as `/fixture/target`. On
//! Windows that path has a root but no drive prefix, so it is not absolute and
//! `hermetic_cargo` rejects it; the module is therefore Unix-only, as the other
//! path-shape suites already are. They also run Cargo through `sh` in a stub
//! case in `command_run_tests`.

#![cfg(unix)]

use std::ffi::OsStr;

use camino::Utf8Path;

use super::{hermetic_cargo, hermetic_cargo_build};

/// A prepared command for a fixture rooted at `/fixture`.
fn fixture_command() -> std::process::Command {
    hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"))
        .expect("an absolute target directory is accepted")
}

/// What a prepared command records for one environment variable.
///
/// `std::process::Command` distinguishes three cases and the tests must too:
/// leaving a variable untouched lets the parent's value leak into the child,
/// which is exactly what the fixture exists to prevent.
#[derive(Debug, PartialEq, Eq)]
enum EnvState {
    /// The command neither sets nor removes the variable.
    Untouched,
    /// The command removes the variable from the child environment.
    Removed,
    /// The command exports the variable with this value.
    Exported(String),
}

fn environment(command: &std::process::Command, name: &str) -> EnvState {
    let Some((_, recorded)) = command.get_envs().find(|(key, _)| *key == OsStr::new(name)) else {
        return EnvState::Untouched;
    };
    recorded.map_or(EnvState::Removed, |exported| {
        EnvState::Exported(exported.to_string_lossy().into_owned())
    })
}

#[test]
fn builds_into_the_target_directory_it_was_given() {
    let command = fixture_command();

    assert_eq!(
        environment(&command, "CARGO_TARGET_DIR"),
        EnvState::Exported("/fixture/target".to_owned()),
        "an exported CARGO_TARGET_DIR must not win",
    );
}

#[test]
fn pins_the_build_directory_only_for_a_plain_build() {
    let build = hermetic_cargo_build(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"))
        .expect("an absolute target directory is accepted");

    assert_eq!(
        environment(&build, "CARGO_BUILD_BUILD_DIR"),
        EnvState::Exported("/fixture/target".to_owned()),
        "a plain build must pin build-script output where the lookup expects it",
    );
}

#[test]
fn leaves_the_build_directory_alone_for_other_subcommands() {
    let command = fixture_command();

    assert_eq!(
        environment(&command, "CARGO_BUILD_BUILD_DIR"),
        EnvState::Untouched,
        "cargo kani keeps its own build directory, so imposing one breaks it",
    );
}

#[test]
fn removes_the_variables_that_would_move_or_share_the_build() {
    let command = fixture_command();

    for variable in ["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_TARGET"] {
        assert_eq!(
            environment(&command, variable),
            EnvState::Removed,
            "{variable} must be removed, not merely unset in the test"
        );
    }
}

#[test]
fn runs_in_the_fixture_directory() {
    let command = fixture_command();

    assert_eq!(
        command.get_current_dir(),
        Some(std::path::Path::new("/fixture"))
    );
}

#[test]
fn runs_offline() {
    let command = fixture_command();

    assert_eq!(
        environment(&command, "CARGO_NET_OFFLINE"),
        EnvState::Exported("true".to_owned()),
        "a fixture build must not wait on the registry index",
    );
}

#[test]
fn rejects_a_relative_target_directory() {
    let error = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("target"))
        .expect_err("a relative target is ambiguous in the child directory");

    assert!(
        error.contains("must be absolute"),
        "the error must say why the target was rejected: {error}",
    );
    assert!(
        error.contains("target"),
        "the error must name the rejected value: {error}",
    );
}
