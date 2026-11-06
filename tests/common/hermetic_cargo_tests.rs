//! Regression tests for the fixture `cargo` environment.

use std::ffi::OsStr;

use camino::Utf8Path;

use super::hermetic_cargo;

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
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    assert_eq!(
        environment(&command, "CARGO_TARGET_DIR"),
        EnvState::Exported("/fixture/target".to_owned()),
        "an exported CARGO_TARGET_DIR must not win",
    );
}

#[test]
fn removes_the_variables_that_would_move_or_share_the_build() {
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    for variable in ["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
        assert_eq!(
            environment(&command, variable),
            EnvState::Removed,
            "{variable} must be removed, not merely unset in the test"
        );
    }
}

#[test]
fn runs_in_the_fixture_directory() {
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    assert_eq!(
        command.get_current_dir(),
        Some(std::path::Path::new("/fixture"))
    );
}

#[test]
fn runs_offline() {
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    assert_eq!(
        environment(&command, "CARGO_NET_OFFLINE"),
        EnvState::Exported("true".to_owned()),
        "a fixture build must not wait on the registry index",
    );
}
