//! Regression tests for the fixture `cargo` environment.

use std::ffi::OsStr;

use camino::Utf8Path;

use super::hermetic_cargo;

fn environment(command: &std::process::Command, name: &str) -> Option<Option<String>> {
    command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new(name))
        .map(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

#[test]
fn builds_into_the_target_directory_it_was_given() {
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    assert_eq!(
        environment(&command, "CARGO_TARGET_DIR"),
        Some(Some("/fixture/target".to_owned())),
        "an exported CARGO_TARGET_DIR must not win",
    );
}

#[test]
fn removes_the_variables_that_would_move_or_share_the_build() {
    let command = hermetic_cargo(Utf8Path::new("/fixture"), Utf8Path::new("/fixture/target"));

    for variable in ["CARGO_BUILD_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
        assert_eq!(
            environment(&command, variable),
            Some(None),
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
