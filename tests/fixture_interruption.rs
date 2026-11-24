//! Verifies where a fixture lives and what an interrupted run leaves behind.
//!
//! Until this change a fixture crate was created in `$TMPDIR` and removed by its
//! `TempDir` destructor. A run that was killed, timed out or signalled skipped
//! that destructor and left the crate behind; one host accumulated about 2 GB of
//! them. This test proves the replacement placement and the release of the lock
//! across a process that dies without running any destructor.
//!
//! The test is split in two halves in one binary. The *owner* half creates a
//! fixture, takes the fixture lock, prints the fixture's path as a handshake and
//! waits to be killed. The *parent* half re-runs this binary with an empty
//! dedicated `TMPDIR` and an external `CARGO_TARGET_DIR`, waits for the
//! handshake, and asserts the fixture is under the target, `TMPDIR` is empty and
//! the lock is held. It then kills the owner and asserts that `TMPDIR` is still
//! empty and the lock is immediately available.

#![cfg(unix)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use test_helpers::{FixtureCrate, FixtureLock, build_support_fixture_manifest};

/// Set for the owner half only; its presence selects owner behaviour.
const OWNER_ENV: &str = "THEOREMC_FIXTURE_INTERRUPTION_OWNER";

/// The ignored test the parent asks the binary to run.
const OWNER_TEST: &str = "owner_creates_a_fixture_and_holds_the_lock";

/// How long the parent waits for the owner's ready handshake.
const HANDSHAKE_DEADLINE: Duration = Duration::from_secs(60);

/// How long the owner waits to be killed before giving up on its own.
const OWNER_LIFETIME: Duration = Duration::from_secs(120);

/// The prefix the owner prints once its fixture and lock exist.
const READY_PREFIX: &str = "READY ";

/// Creates a fixture and holds the fixture lock until the parent kills it.
///
/// This is ignored so an ordinary run never starts it; the parent selects it by
/// name and sets [`OWNER_ENV`]. It gives up after [`OWNER_LIFETIME`] so a parent
/// that dies cannot leave it running for ever.
#[test]
#[ignore = "run by the parent test in this binary, not by an ordinary test run"]
#[expect(
    clippy::print_stdout,
    reason = "the owner half of this test hands its fixture path to the parent over stdout"
)]
#[expect(
    clippy::disallowed_methods,
    reason = "the parent selects this half of the test by setting a variable in the child's environment, which is the only channel a libtest binary offers"
)]
fn owner_creates_a_fixture_and_holds_the_lock() -> Result<(), String> {
    if std::env::var_os(OWNER_ENV).is_none() {
        return Ok(());
    }
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest("interruption_fixture"),
        "//! Fixture crate for the interruption test.\n",
    )?;
    let lock = FixtureLock::acquire_for(Duration::from_secs(30))?;
    // Serial libtest prints its running-test prefix without a newline, so the
    // handshake starts on a line of its own and stays recognisable.
    println!("\n{READY_PREFIX}{}", fixture.manifest_dir());
    std::io::stdout()
        .flush()
        .map_err(|error| format!("the ready handshake could not be flushed: {error}"))?;

    let started = Instant::now();
    while started.elapsed() < OWNER_LIFETIME {
        std::thread::sleep(Duration::from_millis(100));
    }
    drop(lock);
    Err("the owner was not killed by its parent".to_owned())
}

#[test]
fn an_interrupted_run_leaves_no_fixture_in_the_temp_directory() -> Result<(), String> {
    let scratch = Scratch::create()?;
    let mut owner = Owner::spawn(&scratch)?;

    let source = owner.await_ready()?;
    scratch.require_empty_temp_dir()?;
    scratch.require_source_is_under_target(&source)?;
    scratch.require_lock_is_held()?;

    owner.kill_and_reap()?;

    scratch.require_empty_temp_dir()?;
    scratch.require_lock_is_free()?;
    scratch.remove()?;
    Ok(())
}

/// The isolated directories this test owns.
struct Scratch {
    /// The dedicated `TMPDIR` handed to the owner.
    temp_dir: tempfile::TempDir,
    /// The external `CARGO_TARGET_DIR` handed to the owner.
    ///
    /// It is kept alive for the test's duration and removed by `remove`.
    target_dir: tempfile::TempDir,
    /// The `TMPDIR` path, for diagnostics and assertions.
    temp_path: Utf8PathBuf,
    /// The external target path, for diagnostics and assertions.
    target_path: Utf8PathBuf,
}

impl Scratch {
    /// Creates both isolated directories.
    fn create() -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let target_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        Ok(Self {
            temp_path: utf8_path(temp_dir.path())?,
            target_path: utf8_path(target_dir.path())?,
            temp_dir,
            target_dir,
        })
    }

    /// The shared fixture target the owner resolves from `CARGO_TARGET_DIR`.
    fn shared_target(&self) -> Utf8PathBuf {
        self.target_path.join("theoremc-fixture-builds")
    }

    /// Asserts the dedicated `TMPDIR` holds no entries.
    fn require_empty_temp_dir(&self) -> Result<(), String> {
        let dir = Dir::open_ambient_dir(&self.temp_path, ambient_authority())
            .map_err(|error| error.to_string())?;
        let entries: Vec<String> = dir
            .entries()
            .map_err(|error| error.to_string())?
            .map(|entry| {
                entry
                    .and_then(|found| found.file_name())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<_, _>>()?;
        if entries.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "an interrupted run left {entries:?} in TMPDIR {}",
                self.temp_path
            ))
        }
    }

    /// Asserts the fixture's sources sit under the external target.
    fn require_source_is_under_target(&self, source: &Utf8Path) -> Result<(), String> {
        let sources = shared_target_sources_dir_for(&self.target_path)?;
        if source.starts_with(&sources) {
            Ok(())
        } else {
            Err(format!(
                "the fixture was created at {source}, outside the shared target {sources}"
            ))
        }
    }

    /// Asserts the owner holds the fixture lock.
    fn require_lock_is_held(&self) -> Result<(), String> {
        let directory = self.shared_target();
        match FixtureLock::acquire_in(&directory, Duration::from_millis(200)) {
            Ok(_) => Err(format!(
                "the owner should hold the fixture lock in {directory}, but it was free"
            )),
            Err(_) => Ok(()),
        }
    }

    /// Asserts the lock is free again once the owner is gone.
    fn require_lock_is_free(&self) -> Result<(), String> {
        let directory = self.shared_target();
        FixtureLock::acquire_in(&directory, Duration::from_secs(10))
            .map(|_| ())
            .map_err(|error| format!("the lock was not released when the owner died: {error}"))
    }

    /// Removes both isolated directories and everything under them.
    fn remove(self) -> Result<(), String> {
        for directory in [self.temp_dir, self.target_dir] {
            directory
                .close()
                .map_err(|error| format!("could not remove the isolated area: {error}"))?;
        }
        Ok(())
    }
}

/// The owner process the parent starts and kills.
struct Owner {
    /// The spawned test binary.
    child: std::process::Child,
    /// Lines the owner has printed.
    lines: Receiver<String>,
}

impl Owner {
    /// Starts the owner half of this binary.
    fn spawn(scratch: &Scratch) -> Result<Self, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut child = Command::new(executable)
            .args(["--ignored", "--exact", OWNER_TEST, "--nocapture"])
            .env(OWNER_ENV, "1")
            .env("TMPDIR", &scratch.temp_path)
            .env("CARGO_TARGET_DIR", &scratch.target_path)
            .env_remove("NEXTEST")
            .env_remove("NEXTEST_EXECUTION_MODE")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("could not start the owner test binary: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "the owner's stdout was not captured".to_owned())?;
        Ok(Self {
            child,
            lines: line_reader(stdout),
        })
    }

    /// Waits for the ready handshake and returns the fixture's source path.
    fn await_ready(&mut self) -> Result<Utf8PathBuf, String> {
        let started = Instant::now();
        loop {
            let left = HANDSHAKE_DEADLINE.saturating_sub(started.elapsed());
            let line = self.next_line(left)?;
            if let Some(path) = ready_path(&line) {
                return Ok(path);
            }
        }
    }

    /// Returns the owner's next line, or why the wait ended without one.
    fn next_line(&mut self, wait: Duration) -> Result<String, String> {
        match self.lines.recv_timeout(wait) {
            Ok(line) => Ok(line),
            Err(RecvTimeoutError::Timeout) => Err(timeout_error()),
            Err(RecvTimeoutError::Disconnected) => {
                Err("the owner exited before announcing its fixture".to_owned())
            }
        }
    }

    /// Kills the owner without letting it run a destructor, then reaps it.
    fn kill_and_reap(&mut self) -> Result<(), String> {
        self.child
            .kill()
            .map_err(|error| format!("could not kill the owner: {error}"))?;
        self.child
            .wait()
            .map(|_| ())
            .map_err(|error| format!("could not reap the owner: {error}"))
    }
}

/// The error reported when the owner never announces its fixture.
fn timeout_error() -> String {
    format!("the owner did not announce its fixture within {HANDSHAKE_DEADLINE:?}")
}

/// Returns the fixture path a ready line carries, if it is one.
fn ready_path(line: &str) -> Option<Utf8PathBuf> {
    line.strip_prefix(READY_PREFIX)
        .map(|path| Utf8PathBuf::from(path.trim()))
}

/// Reads `reader` line by line on its own thread and forwards each line.
fn line_reader(reader: impl std::io::Read + Send + 'static) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(text) = line else { break };
            if sender.send(text).is_err() {
                break;
            }
        }
    });
    receiver
}

/// Resolves the sources directory under an alternative `CARGO_TARGET_DIR`.
///
/// The helper in `test_helpers` reads the ambient variable; this test sets it
/// only for a child process, so it derives the same path from the path it gave
/// that child.
fn shared_target_sources_dir_for(target: &Utf8Path) -> Result<Utf8PathBuf, String> {
    let directory = target.join("theoremc-fixture-builds/sources");
    directory
        .canonicalize_utf8()
        .map_err(|error| format!("could not resolve {directory}: {error}"))
}

/// Converts a path from the platform form to UTF-8.
fn utf8_path(path: &std::path::Path) -> Result<Utf8PathBuf, String> {
    Utf8Path::from_path(path)
        .map(Utf8Path::to_path_buf)
        .ok_or_else(|| format!("{} is not valid UTF-8", path.display()))
}
