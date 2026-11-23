//! Regression tests for the cross-process fixture lock.
//!
//! The lock exists because `nextest` runs every test in its own process, so an
//! in-process mutex serialises nothing. These tests therefore assert the
//! property that matters across processes: a second holder cannot take the lock
//! while the first holds it, and the second holder's failure names the lock and
//! how long it waited.

use std::time::Duration;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{FixtureLock, LOCK_FILE_NAME};

/// Creates a temporary directory to hold a lock file.
fn lock_directory() -> (tempfile::TempDir, camino::Utf8PathBuf) {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let path = camino::Utf8PathBuf::from_path_buf(temp.path().to_path_buf())
        .expect("the scratch directory path should be UTF-8");
    (temp, path)
}

#[test]
fn a_second_handle_cannot_take_a_held_lock() {
    let (_temp, directory) = lock_directory();
    let _held = FixtureLock::acquire_in(&directory, Duration::from_secs(5)).expect("first lock");

    let second = FixtureLock::acquire_in(&directory, Duration::from_millis(150));

    let error = second.expect_err("a held lock must not be taken twice");
    assert!(
        error.contains(LOCK_FILE_NAME),
        "the timeout must name the lock file, got: {error}",
    );
}

#[test]
fn the_timeout_error_reports_how_long_it_waited() {
    let (_temp, directory) = lock_directory();
    let _held = FixtureLock::acquire_in(&directory, Duration::from_secs(5)).expect("first lock");

    let started = std::time::Instant::now();
    let error = FixtureLock::acquire_in(&directory, Duration::from_millis(120))
        .expect_err("a held lock must time out");
    let waited = started.elapsed();

    assert!(
        waited >= Duration::from_millis(120),
        "the wait must last at least the requested bound, waited {waited:?}",
    );
    assert!(
        error.contains("timed out after"),
        "the error must report the elapsed wait, got: {error}",
    );
}

#[test]
fn dropping_the_guard_releases_the_lock() {
    let (_temp, directory) = lock_directory();
    let held = FixtureLock::acquire_in(&directory, Duration::from_secs(5)).expect("first lock");
    drop(held);

    let second = FixtureLock::acquire_in(&directory, Duration::from_millis(500));

    assert!(
        second.is_ok(),
        "a dropped guard must free the lock, got: {second:?}",
    );
}

#[test]
fn the_lock_is_created_inside_the_shared_target() {
    let (_temp, directory) = lock_directory();

    let lock = FixtureLock::acquire_in(&directory, Duration::from_secs(5)).expect("acquire");

    assert_eq!(
        lock.path(),
        directory.join(LOCK_FILE_NAME),
        "the lock must live in the shared target directory",
    );
    let dir = Dir::open_ambient_dir(&directory, ambient_authority()).expect("open the directory");
    assert!(
        dir.symlink_metadata(LOCK_FILE_NAME).is_ok(),
        "the lock file must exist on disk",
    );
}

#[test]
fn a_directory_that_cannot_be_opened_is_an_error_not_a_silent_success() {
    let (_temp, directory) = lock_directory();
    let missing = directory.join("absent");

    let outcome = FixtureLock::acquire_in(&missing, Duration::from_millis(50));

    assert!(
        outcome.is_err(),
        "a missing target directory must be reported, got: {outcome:?}",
    );
}

/// Two handles opened in one process conflict, which is what makes the guarantee
/// hold when each test is a separate process rather than a separate thread.
#[test]
fn two_handles_in_one_process_still_conflict() {
    let (_temp, directory) = lock_directory();
    let first = FixtureLock::acquire_in(&directory, Duration::from_secs(5)).expect("first lock");

    let second = FixtureLock::acquire_in(&directory, Duration::from_millis(100));

    assert!(
        second.is_err(),
        "the lock belongs to the file description, not the process",
    );
    drop(first);
    assert!(
        FixtureLock::acquire_in(&directory, Duration::from_millis(500)).is_ok(),
        "the lock must be free again once both handles are gone",
    );
}

/// A path that is a file rather than a directory cannot hold a lock file.
#[test]
fn a_lock_directory_that_is_a_file_is_reported() {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let path = Utf8Path::from_path(temp.path()).expect("temp path is UTF-8");
    let dir = Dir::open_ambient_dir(path, ambient_authority()).expect("open the directory");
    dir.write("not-a-directory", "contents")
        .expect("write the file");

    let outcome = FixtureLock::acquire_in(&path.join("not-a-directory"), Duration::from_millis(50));

    assert!(outcome.is_err(), "a file is not a lock directory");
}
