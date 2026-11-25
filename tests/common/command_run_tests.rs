//! Tests for command diagnostics, without running a real fixture build.

use std::io;
use std::process::Command;

use super::{completed_failure, run, spawn_failure};

#[test]
fn a_missing_program_names_the_command_that_could_not_start() {
    let mut command = Command::new("theoremc-absent-program-for-tests");

    let error = run(&mut command).expect_err("a missing program must fail to start");

    assert!(
        error.contains("running `theoremc-absent-program-for-tests`"),
        "spawn failure must name the command: {error}",
    );
    let system_error = io::Error::from_raw_os_error(2).to_string();
    assert!(
        error.contains(&system_error),
        "spawn failure must keep the system's own error text ({system_error}): {error}",
    );
}

/// A spawn failure embeds whatever text the operating system produced, so the
/// assertion follows `Display` rather than hard-coding a Unix-only string; on
/// Windows the same failure reads "The system cannot find the file specified.".
#[test]
fn a_spawn_failure_embeds_the_operating_systems_own_text() {
    let command = Command::new("theoremc-absent-program-for-tests");
    let system_error = io::Error::from_raw_os_error(2);

    let message = spawn_failure(&command, &system_error);

    assert!(
        message.contains(&system_error.to_string()),
        "the message must carry the system's own text: {message}",
    );
    assert!(
        message.contains("`theoremc-absent-program-for-tests`"),
        "the message must still name the command: {message}",
    );
}

#[test]
fn a_missing_working_directory_names_the_directory_and_the_command() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let absent = scratch.path().join("absent-fixture-root");
    let mut command = Command::new("cargo");
    command.current_dir(&absent);

    let error = run(&mut command).expect_err("a missing working directory must fail to start");

    assert!(
        error.contains(&absent.display().to_string()),
        "spawn failure must name the working directory that vanished: {error}",
    );
    assert!(
        error.contains("running `cargo` in"),
        "spawn failure must name the command: {error}",
    );
}

#[cfg(unix)]
#[test]
fn a_nonzero_exit_reports_the_status_and_each_stream_separately() {
    let mut command = Command::new("sh");
    command.args(["-c", "printf 'the stdout'; printf 'the stderr' >&2; exit 7"]);

    let output = run(&mut command).expect("the stub shell should run");

    assert!(
        !output.status.success(),
        "the stub is expected to exit nonzero",
    );
    let message = completed_failure(&command, &output);
    assert!(
        message.contains("failed with status"),
        "completed failure must report the status: {message}",
    );
    assert!(
        message.contains("stdout:\nthe stdout"),
        "completed failure must label stdout: {message}",
    );
    assert!(
        message.contains("stderr:\nthe stderr"),
        "completed failure must label stderr: {message}",
    );
}
