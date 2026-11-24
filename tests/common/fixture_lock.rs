//! Cross-process serialization for fixture `cargo` commands.
//!
//! The fixture BDD suites start real `cargo` builds. Under `cargo nextest` every
//! test runs in its own process, so an in-process mutex serializes nothing: all
//! the fixture builds start together, contend on Cargo's package-cache lock, and
//! queue every other Cargo process on the host behind them (issue #197).
//!
//! This module supplies an advisory whole-file lock that every process takes
//! before running a fixture command. The lock file lives beside the shared
//! fixture target, so the lock covers exactly the directory whose contents the
//! commands race over.

use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use cap_std::fs::OpenOptions;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::fixture_isolation::shared_target_dir;

/// The name of the lock file inside the shared fixture target directory.
const LOCK_FILE_NAME: &str = "fixture-cargo.lock";

/// How often a held lock is retried while waiting for it.
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// How long a caller waits for the lock by default.
///
/// A fixture command is meant to be the only holder, so the wait is normally
/// instantaneous. The bound exists for the case where a second `nextest` run
/// shares the same target: the second run should fail with a clear message
/// rather than queue behind the first for the length of a build.
pub const DEFAULT_LOCK_WAIT: Duration = Duration::from_secs(300);

/// Proof that the current process holds the fixture `cargo` lock.
///
/// The lock is released when the guard is dropped, because dropping it closes
/// the file descriptor that carries the advisory lock. The operating system
/// releases the lock when the process exits for any reason, including a signal
/// that skips destructors, so a killed test cannot wedge every later run.
///
/// Two handles opened separately, even inside one process, conflict with each
/// other: the lock belongs to the open file description, not to the process.
/// That is what makes the guarantee hold under `nextest`, which runs each test in
/// its own process, without a shared in-process mutex.
#[derive(Debug)]
pub struct FixtureLock {
    /// The open lock file; the advisory lock lives as long as this descriptor.
    _file: LockFile,
    /// The absolute path of the lock file, for diagnostics.
    path: Utf8PathBuf,
}

/// The platform's lock file handle.
///
/// On Unix the file is opened through a capability and locked with `flock`. Off
/// Unix the standard library's advisory lock is the only portable choice, so the
/// handle is a standard file; the repository's filesystem policy is enforced on
/// the Unix builds, where the capability path is the one compiled.
#[cfg(unix)]
type LockFile = cap_std::fs_utf8::File;

/// The platform's lock file handle.
#[cfg(not(unix))]
type LockFile = std::fs::File;

impl FixtureLock {
    /// Takes the fixture lock for the default wait.
    ///
    /// # Errors
    ///
    /// Returns an error when the shared target directory or the lock file
    /// cannot be opened, or when the lock is still held after
    /// [`DEFAULT_LOCK_WAIT`].
    pub fn acquire() -> Result<Self, String> {
        Self::acquire_for(DEFAULT_LOCK_WAIT)
    }

    /// Takes the fixture lock, waiting at most `wait` for it.
    ///
    /// # Errors
    ///
    /// Returns an error when the shared target directory or the lock file
    /// cannot be opened, or when the lock is still held after `wait`. The
    /// timeout error names the lock path and how long the caller waited.
    pub fn acquire_for(wait: Duration) -> Result<Self, String> {
        let directory = shared_target_dir()?;
        Self::acquire_in(&directory, wait)
    }

    /// Takes the lock file in `directory`, waiting at most `wait` for it.
    ///
    /// This is the injectable seam: callers normally use [`Self::acquire`],
    /// which resolves the shared target, while tests can point it at a temporary
    /// directory.
    ///
    /// # Errors
    ///
    /// Returns an error when `directory` or the lock file cannot be opened, or
    /// when the lock is still held after `wait`.
    pub fn acquire_in(directory: &Utf8Path, wait: Duration) -> Result<Self, String> {
        let dir = Dir::open_ambient_dir(directory, ambient_authority())
            .map_err(|error| format!("could not open the fixture target {directory}: {error}"))?;
        let path = directory.join(LOCK_FILE_NAME);
        let file = open_lock_file(&dir, &path)?;
        lock_within(&file, &path, wait)?;
        Ok(Self { _file: file, path })
    }

    /// Returns the absolute path of the lock file.
    #[must_use]
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }
}

/// Opens the lock file through the capability, creating it if absent.
#[cfg(unix)]
fn open_lock_file(dir: &Dir, path: &Utf8Path) -> Result<LockFile, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    dir.open_with(LOCK_FILE_NAME, &options)
        .map_err(|error| format!("could not open the fixture lock {path}: {error}"))
}

/// Opens the lock file with the standard library, creating it if absent.
///
/// The capability handle is not used here because the standard library's
/// advisory lock lives on its own handle type, and its lock is what this path
/// exists to reach.
#[cfg(not(unix))]
fn open_lock_file(_dir: &Dir, path: &Utf8Path) -> Result<LockFile, String> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .map_err(|error| format!("could not open the fixture lock {path}: {error}"))
}

/// Polls the non-blocking lock until it is taken or `wait` elapses.
#[cfg(unix)]
fn lock_within(file: &LockFile, path: &Utf8Path, wait: Duration) -> Result<(), String> {
    use rustix::fs::{FlockOperation, flock};

    let started = Instant::now();
    loop {
        match flock(file, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => return Ok(()),
            Err(error) if error == rustix::io::Errno::WOULDBLOCK => {
                if started.elapsed() >= wait {
                    return Err(lock_timeout(path, started.elapsed()));
                }
                std::thread::sleep(LOCK_POLL_INTERVAL);
            }
            Err(error) => return Err(format!("could not lock {path}: {error}")),
        }
    }
}

/// Polls the exclusive lock until it is taken or `wait` elapses.
///
/// This path exists for hosts without `flock`; it uses the standard library's
/// advisory lock, which is also released when the handle closes. The repository's
/// gates run on Unix, where the capability-based path above is used.
#[cfg(not(unix))]
fn lock_within(file: &LockFile, path: &Utf8Path, wait: Duration) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if started.elapsed() >= wait {
                    return Err(lock_timeout(path, started.elapsed()));
                }
                std::thread::sleep(LOCK_POLL_INTERVAL);
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(format!("could not lock {path}: {error}"));
            }
        }
    }
}

/// Builds the error a caller sees when the lock was held for too long.
fn lock_timeout(path: &Utf8Path, elapsed: Duration) -> String {
    format!(
        "timed out after {elapsed:?} waiting for the fixture lock {path}; \
         another fixture cargo command holds it"
    )
}

#[cfg(test)]
#[path = "fixture_lock_tests.rs"]
mod tests;
