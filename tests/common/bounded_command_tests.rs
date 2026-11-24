//! Regression tests for the deadline applied to a fixture command.
//!
//! The stubs run `sh`, so they exercise the real spawn, pipe-drain, wait, kill
//! and reap paths without starting a compiler. Each test states the defect it
//! would catch: a missing deadline shows up as a slow test, a missing drain
//! shows up as a deadlock, a missing process-group kill shows up as a surviving
//! grandchild.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{CommandFailure, run_within_deadline};

/// Builds a `sh` command that runs `script`.
#[cfg(unix)]
fn shell(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

#[cfg(unix)]
#[test]
fn both_streams_are_captured_and_the_status_is_reported() {
    let outcome = run_within_deadline(
        shell("echo to-stdout; echo to-stderr >&2; exit 7"),
        Duration::from_secs(20),
    );

    let output = outcome.expect("the command exits inside its deadline");
    assert!(!output.is_success(), "exit 7 must not read as success");
    assert_eq!(output.status.code(), Some(7), "the exit code must be kept");
    assert!(
        output.stdout.contains("to-stdout"),
        "stdout must be captured, got: {:?}",
        output.stdout,
    );
    assert!(
        output.stderr.contains("to-stderr"),
        "stderr must be captured, got: {:?}",
        output.stderr,
    );
    assert!(
        output.combined().contains("to-stdout") && output.combined().contains("to-stderr"),
        "the combined log must carry both streams",
    );
}

/// Output larger than a pipe buffer would deadlock a runner that waits for the
/// child without draining. 200 KiB is far beyond a 64 KiB pipe.
#[cfg(unix)]
#[test]
fn output_larger_than_a_pipe_buffer_is_drained_without_deadlocking() {
    let script = "i=0; while [ $i -lt 2000 ]; do \
                  printf '0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\\n'; \
                  i=$((i + 1)); done";
    let started = Instant::now();

    let outcome = run_within_deadline(shell(script), Duration::from_secs(30));

    let output = outcome.expect("a chatty child inside its deadline must complete");
    assert!(
        output.stdout.len() > 200_000,
        "the whole stream must be read, got {} bytes",
        output.stdout.len(),
    );
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "draining must not wait for the deadline",
    );
}

#[cfg(unix)]
#[test]
fn a_child_that_outlives_its_deadline_fails_with_a_timeout() {
    let started = Instant::now();

    let outcome = run_within_deadline(shell("exec sleep 30"), Duration::from_millis(300));

    assert!(
        matches!(&outcome, Err(CommandFailure::Timeout { .. })),
        "a child past its deadline must time out, got {outcome:?}",
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the wait must end at the deadline, took {:?}",
        started.elapsed(),
    );
}

#[cfg(unix)]
#[test]
fn a_timed_out_child_reports_what_it_printed_first() -> Result<(), String> {
    let outcome = run_within_deadline(
        shell("echo before-the-hang; exec sleep 30"),
        Duration::from_millis(300),
    );

    match outcome {
        Err(CommandFailure::Timeout { output, .. }) => {
            if output.contains("before-the-hang") {
                Ok(())
            } else {
                Err(format!("the partial output must be kept, got: {output}"))
            }
        }
        other => Err(format!(
            "a sleep that outlives the deadline must be a timeout, got {other:?}"
        )),
    }
}

#[cfg(unix)]
#[test]
fn a_timed_out_direct_child_is_killed_and_reaped() {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let pid_file = temp.path().join("pid");

    let mut command = shell("echo $$ > \"$1\"; exec sleep 30");
    command.arg("sh").arg(&pid_file);
    let outcome = run_within_deadline(command, Duration::from_millis(300));
    assert!(
        matches!(&outcome, Err(CommandFailure::Timeout { .. })),
        "the child must time out, got {outcome:?}",
    );

    let pid = read_pid_file(&pid_file);
    assert!(
        !process_is_alive(pid.trim()),
        "process {} must have been killed and reaped",
        pid.trim(),
    );
}

/// A descendant that keeps the pipe open after its parent exits must not make
/// the runner wait for ever. The child here exits at once and leaves a `sleep`
/// holding stdout; the run must still finish promptly and report the child's
/// success. The test kills the descendant itself, because the runner deliberately
/// leaves a successfully completed command's descendants alone.
#[cfg(unix)]
#[test]
fn a_descendant_holding_the_pipe_open_does_not_block_the_runner() {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let descendant_pid_file = temp.path().join("descendant.pid");
    let started = Instant::now();

    let mut command = shell("sleep 30 & echo $! > \"$1\"; echo parent-done; exit 0");
    command.arg("sh").arg(&descendant_pid_file);
    let outcome = run_within_deadline(command, Duration::from_secs(3));

    let output = outcome.expect("the direct child exits at once");
    assert!(
        output.is_success(),
        "the direct child's status must be reported",
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the runner must not wait for the descendant, took {:?}",
        started.elapsed(),
    );
    assert!(
        output.stdout.contains("parent-done"),
        "the output the child wrote before exiting must be kept, got: {:?}",
        output.stdout,
    );
    kill_recorded_process(&descendant_pid_file);
}

/// Reads the pid a child recorded in `path`.
#[cfg(unix)]
fn read_pid_file(path: &std::path::Path) -> String {
    let root = camino::Utf8Path::from_path(path.parent().expect("the pid file has a parent"))
        .expect("the scratch path is UTF-8");
    let name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .expect("the pid file name is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open the scratch directory");
    dir.read_to_string(name)
        .expect("the child recorded its pid")
}

/// Kills the process whose id `path` holds, so a test leaves nothing behind.
#[cfg(unix)]
fn kill_recorded_process(path: &std::path::Path) {
    let pid = read_pid_file(path);
    let _kill_result = Command::new("kill")
        .args(["-9", pid.trim()])
        .stderr(Stdio::null())
        .status();
}

#[test]
fn a_command_that_cannot_be_started_reports_the_spawn_failure() {
    let command = Command::new("/nonexistent/fixture-cargo-binary");

    let outcome = run_within_deadline(command, Duration::from_secs(5));

    assert!(
        matches!(&outcome, Err(CommandFailure::Spawn(_))),
        "a missing program must be a spawn failure, got {outcome:?}",
    );
}

/// Returns whether `pid` names a live, non-zombie process.
#[cfg(unix)]
fn process_is_alive(pid: &str) -> bool {
    let probe = Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    probe && !process_is_zombie(pid)
}

/// Returns whether `pid` is a terminated process awaiting its parent's reap.
///
/// A zombie has already exited; `kill -0` still succeeds for it, so a
/// termination check that ignored zombies could fail after a correct group
/// kill, for example when PID 1 in a container does not reap orphans.
#[cfg(unix)]
fn process_is_zombie(pid: &str) -> bool {
    Command::new("ps")
        .args(["-o", "stat=", "-p", pid])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .starts_with('Z')
        })
}

/// Polls `pid` until it has exited, for at most `budget`.
///
/// A signal returns as soon as it is delivered, so a single immediate probe can
/// see the descendant still alive even when the group kill worked.
#[cfg(unix)]
fn process_exits_within(pid: &str, budget: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < budget {
        if !process_is_alive(pid.trim()) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    !process_is_alive(pid.trim())
}

/// A timeout must terminate a descendant, not only the direct child.
///
/// The stub starts a descendant that records its own pid and outlives the
/// deadline. Killing only the direct child would leave it running, so this test
/// fails whenever the process-group kill is removed.
#[cfg(unix)]
#[test]
fn a_timed_out_command_terminates_its_descendants() {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let pid_file = temp.path().join("descendant.pid");

    // The direct child waits on the descendant, so the deadline fires while
    // both are alive, and the descendant is the only thing recording a pid.
    let mut command = shell("sh -c 'echo $$ > \"$1\"; exec sleep 30' sh \"$1\" & wait");
    command.arg("sh").arg(&pid_file);
    let outcome = run_within_deadline(command, Duration::from_millis(400));
    assert!(
        matches!(&outcome, Err(CommandFailure::Timeout { .. })),
        "the command must time out, got {outcome:?}",
    );

    let pid = read_pid_file(&pid_file);
    assert!(
        process_exits_within(&pid, Duration::from_secs(10)),
        "descendant {} must have been killed with its group",
        pid.trim(),
    );
}
