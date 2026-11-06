//! Runs a child process to completion within a deadline.
//!
//! The compile-fail tests start `rustc` themselves. Without a deadline a hung
//! compiler would wait for the runner to terminate the whole test, so each spawn
//! is bounded here and fails with a message when it overruns.

use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

/// How long one snippet may take to compile before the test fails.
///
/// A cold `rustc` start on a loaded host takes seconds; this is far above that
/// and far below the nextest group's allowance, so a hung compiler fails the
/// test with a message instead of waiting for the runner to terminate it.
pub(super) const RUSTC_DEADLINE: Duration = Duration::from_secs(120);

/// How often a running child is checked against its deadline.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Runs `command`, killing it when it outlives `deadline`.
///
/// Returns `(success, stderr)`. The child's stderr is drained on its own thread
/// so a chatty compiler cannot fill the pipe and stall while this waits.
pub(super) fn run_within_deadline(
    mut command: Command,
    deadline: Duration,
) -> io::Result<(bool, String)> {
    let mut child = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut pipe = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("child stderr was not captured"))?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        pipe.read_to_string(&mut text).map(|_| text)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= deadline {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("the compiler did not finish within {deadline:?}"),
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    };
    let stderr = reader
        .join()
        .map_err(|_| io::Error::other("the stderr reader panicked"))??;
    Ok((status.success(), stderr))
}

#[test]
fn a_child_that_outlives_its_deadline_is_killed_and_reported() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pid_file = temp.path().join("pid");
    let mut command = Command::new("sh");
    command
        .args(["-c", "echo $$ > \"$1\"; exec sleep 30", "sh"])
        .arg(&pid_file);

    let started = Instant::now();
    let outcome = run_within_deadline(command, Duration::from_millis(400));

    assert!(
        matches!(&outcome, Err(error) if error.kind() == io::ErrorKind::TimedOut),
        "a child past its deadline must fail with TimedOut, got {outcome:?}",
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the wait must end at the deadline",
    );
    let pid = Dir::open_ambient_dir(
        Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8"),
        ambient_authority(),
    )
    .and_then(|dir| dir.read_to_string("pid"))
    .expect("the child recorded its pid");
    let alive = Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(Stdio::null())
        .status()
        .expect("kill is available")
        .success();
    assert!(
        !alive,
        "process {} must have been killed and reaped",
        pid.trim()
    );
}

#[test]
fn a_child_whose_stderr_is_not_text_reports_the_read_error() {
    let mut command = Command::new("sh");
    command.args(["-c", "printf '\\377\\376' >&2"]);

    let outcome = run_within_deadline(command, Duration::from_secs(20));

    assert!(
        matches!(&outcome, Err(error) if error.kind() == io::ErrorKind::InvalidData),
        "a read error must be reported, not dropped, got {outcome:?}",
    );
}

#[test]
fn a_child_inside_its_deadline_reports_its_status_and_stderr() {
    let mut command = Command::new("sh");
    command.args(["-c", "echo diagnostic >&2; exit 3"]);

    let (success, stderr) = run_within_deadline(command, Duration::from_secs(20))
        .expect("a quick child finishes inside its deadline");

    assert!(!success, "exit status 3 is a failure");
    assert_eq!(stderr.trim(), "diagnostic");
}
