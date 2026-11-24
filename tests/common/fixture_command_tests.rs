//! Regression tests for the fixture command entry point.
//!
//! The lock and the deadline have their own tests; these cover the composition
//! and the operator override. The override is parsed by a pure function so the
//! tests never mutate the process environment, which the workspace's testing
//! policy forbids.

use std::ffi::OsStr;
use std::process::Command;
use std::time::Duration;

use super::{DEFAULT_FIXTURE_COMMAND_TIMEOUT, FixtureOutcome, deadline_from_override, run_locked};
use crate::integration::fixture_lock::FixtureLock;

#[test]
fn the_default_deadline_leaves_room_inside_the_nextest_group_allowance() {
    assert!(
        DEFAULT_FIXTURE_COMMAND_TIMEOUT < Duration::from_secs(600),
        "the nextest group allows 600 s, so the deadline must fire first; got {DEFAULT_FIXTURE_COMMAND_TIMEOUT:?}",
    );
    assert!(
        DEFAULT_FIXTURE_COMMAND_TIMEOUT >= Duration::from_secs(60),
        "a deadline below a minute would fail ordinary cold builds; got {DEFAULT_FIXTURE_COMMAND_TIMEOUT:?}",
    );
}

#[test]
fn an_absent_override_keeps_the_default() {
    assert_eq!(
        deadline_from_override(None),
        DEFAULT_FIXTURE_COMMAND_TIMEOUT,
        "no override means the default deadline",
    );
}

#[test]
fn a_positive_override_replaces_the_default() {
    let timeout = deadline_from_override(Some(OsStr::new("1200")));

    assert_eq!(timeout, Duration::from_secs(1200), "the override must win");
}

#[test]
fn surrounding_whitespace_is_tolerated_in_the_override() {
    let timeout = deadline_from_override(Some(OsStr::new(" 90 ")));

    assert_eq!(
        timeout,
        Duration::from_secs(90),
        "the value must be trimmed"
    );
}

#[test]
fn a_malformed_override_falls_back_to_the_default() {
    for value in ["", "  ", "soon", "0", "-5", "1.5"] {
        assert_eq!(
            deadline_from_override(Some(OsStr::new(value))),
            DEFAULT_FIXTURE_COMMAND_TIMEOUT,
            "{value:?} must not replace the default deadline",
        );
    }
}

#[cfg(unix)]
#[test]
fn a_shell_command_runs_through_the_entry_point() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let directory = camino::Utf8Path::from_path(temp.path())
        .ok_or_else(|| "the scratch path is not UTF-8".to_owned())?;
    let lock = FixtureLock::acquire_in(directory, Duration::from_secs(5))?;
    let mut command = Command::new("sh");
    command.args(["-c", "echo through-the-entry-point"]);

    let outcome = run_locked(&lock, command, super::fixture_command_timeout())?;
    let FixtureOutcome::Completed(output) = outcome else {
        return Err("a short shell command must complete".to_owned());
    };

    if !output.stdout.contains("through-the-entry-point") {
        return Err(format!(
            "the entry point must return the command's output, got: {:?}",
            output.stdout
        ));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_timed_out_command_is_reported_as_a_timeout_not_an_error() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let directory = camino::Utf8Path::from_path(temp.path())
        .ok_or_else(|| "the scratch path is not UTF-8".to_owned())?;
    let lock = FixtureLock::acquire_in(directory, Duration::from_secs(5))?;
    let mut command = Command::new("sh");
    command.args(["-c", "exec sleep 30"]);

    let outcome = run_locked(&lock, command, Duration::from_millis(300))?;

    match outcome {
        FixtureOutcome::TimedOut { deadline, .. } => {
            if deadline != Duration::from_millis(300) {
                return Err(format!(
                    "the reported deadline must be the one applied, got {deadline:?}"
                ));
            }
            Ok(())
        }
        FixtureOutcome::Completed(output) => Err(format!(
            "a command past its deadline must not complete, got status {}",
            output.status
        )),
    }
}
