//! Regression tests for the Kani availability probe.
//!
//! The probe decides whether the Kani scenarios skip or fail, so the three
//! outcomes have to be distinguishable: a missing Kani is a skip, an unusable
//! Kani carries its reason, and a hung probe is a failure. The stubs are `sh`
//! scripts, so no compiler starts.

#![cfg(unix)]

use std::path::PathBuf;
use std::time::Duration;

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};

use super::{KaniAvailability, probe_kani_within};

/// Writes an executable `sh` script named `name` and returns its path.
fn stub_program(name: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("create a scratch directory");
    let dir = Dir::open_ambient_dir(temp.path(), ambient_authority()).expect("open the directory");
    dir.write(name, format!("#!/bin/sh\n{body}\n"))
        .expect("write the stub");
    dir.set_permissions(name, Permissions::from_mode(0o755))
        .expect("make the stub executable");
    let path = temp.path().join(name);
    (temp, path)
}

#[test]
fn a_probe_that_succeeds_reports_kani_as_available() {
    let (_temp, program) = stub_program("cargo-ok", "exit 0");

    let outcome = probe_kani_within(&program, Duration::from_secs(10));

    assert!(
        matches!(outcome, Ok(KaniAvailability::Available)),
        "a successful probe must report availability, got {outcome:?}",
    );
}

#[test]
fn a_probe_that_fails_carries_the_reason_from_stderr() -> Result<(), String> {
    let (_temp, program) = stub_program(
        "cargo-missing",
        "echo \"error: no such command: 'kani'\" >&2; exit 101",
    );

    let outcome = probe_kani_within(&program, Duration::from_secs(10));

    let Ok(KaniAvailability::Unavailable(reason)) = outcome else {
        return Err(format!(
            "a failing probe must report unavailability, got {outcome:?}"
        ));
    };
    if !reason.contains("no such command") {
        return Err(format!(
            "the reason must come from the probe's stderr, got: {reason}"
        ));
    }
    Ok(())
}

#[test]
fn a_probe_that_cannot_start_is_an_error_not_a_skip() {
    let missing = PathBuf::from("/nonexistent/cargo-for-the-kani-probe");

    let outcome = probe_kani_within(&missing, Duration::from_secs(10));

    assert!(
        outcome.is_err(),
        "a probe that cannot start must fail the test, got {outcome:?}",
    );
}

#[test]
fn a_probe_that_outlives_its_deadline_reports_a_timeout() {
    let (_temp, program) = stub_program("cargo-hung", "exec sleep 30");
    let started = std::time::Instant::now();

    let outcome = probe_kani_within(&program, Duration::from_millis(300));

    assert!(
        matches!(outcome, Ok(KaniAvailability::TimedOut(_))),
        "a hung probe must be distinguishable from a missing one, got {outcome:?}",
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the probe must return at its deadline",
    );
}
