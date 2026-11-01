//! Cargo command serialization helpers for theorem-file macro fixtures.
//!
//! This module is the process boundary for `theorem_file_macro_bdd.rs`.
//! `fixture_crate.rs` builds temporary crates, then delegates all Cargo
//! invocations here so fixture builds and `cargo kani list` runs share one
//! lock, target-directory policy, and error-reporting format.

use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

use camino::Utf8Path;
use test_helpers::hermetic_cargo;

#[derive(Clone, Copy)]
pub(crate) enum CargoSubcommand {
    Build,
    KaniList,
}

impl CargoSubcommand {
    const fn args(self) -> &'static [&'static str] {
        match self {
            Self::Build => &["build", "--color", "never"],
            Self::KaniList => &["kani", "list"],
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::KaniList => "kani list",
        }
    }
}

/// Proof that `FIXTURE_CARGO_LOCK` is held by the current thread.
///
/// Pass a `CargoGuard<'_>` to fixture Cargo helpers to enforce at compile time
/// that no caller bypasses the serialization contract.
pub(crate) struct CargoGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
}

impl CargoGuard<'_> {
    pub(crate) fn acquire() -> Self {
        Self {
            _guard: recover_mutex_guard(&FIXTURE_CARGO_LOCK),
        }
    }
}

/// Serializes fixture Cargo commands within a libtest process.
///
/// Nextest runs each test in its own process; its `nested-cargo` test group
/// supplies serialization across those processes. Cargo also protects the
/// shared persistent fixture target with its own target-directory lock.
pub(crate) static FIXTURE_CARGO_LOCK: Mutex<()> = Mutex::new(());

/// Runs a Cargo subcommand in the fixture crate directory.
///
/// Dependencies use a persistent workspace fixture target. Source trees remain
/// unique to each fixture, so Cargo rebuilds each fixture package and build
/// script while retaining reusable dependency artefacts. The [`CargoGuard`]
/// supplies serialization for callers running in the same libtest process.
///
/// # Timeout
///
/// `Command::output()` blocks until the subprocess exits. No explicit timeout
/// is imposed: `cargo` is a first-party trusted tool, and introducing
/// cross-platform `wait_timeout` polling would add significant complexity
/// disproportionate to this test suite's scope. If a runaway `cargo` process
/// causes a stall in CI, the job-level timeout enforced by the CI runtime
/// (e.g. GitHub Actions' `timeout-minutes`) provides the outer bound.
pub(crate) fn cargo_run(
    manifest_dir: &Utf8Path,
    subcommand: CargoSubcommand,
    guard: &CargoGuard<'_>,
) -> Result<(), String> {
    cargo_run_output(manifest_dir, subcommand, guard).map(|_| ())
}

pub(crate) fn cargo_run_output(
    manifest_dir: &Utf8Path,
    subcommand: CargoSubcommand,
    _guard: &CargoGuard<'_>,
) -> Result<String, String> {
    let target_dir = test_helpers::fixture_target_dir();
    let output = hermetic_cargo(manifest_dir, &target_dir)
        .args(subcommand.args())
        .output()
        .map_err(|error| error.to_string())?;
    command_result(subcommand, &output)
}

pub(crate) fn recover_mutex_guard(mutex: &Mutex<()>) -> MutexGuard<'_, ()> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Returns `true` if `cargo kani` is available in the current environment.
///
/// Probes by running `cargo kani --version`. When `cargo-kani` is not
/// installed, `cargo` exits with a non-zero status and emits
/// `error: no such command: 'kani'`; `Command::output()` itself still succeeds
/// because the `cargo` binary was found. The boolean is derived from the exit
/// status alone, so no stderr parsing is required.
pub(crate) fn kani_is_installed() -> bool {
    Command::new("cargo")
        .args(["kani", "--version"])
        .output()
        .is_ok_and(|o| o.status.success())
}

fn command_result(
    subcommand: CargoSubcommand,
    output: &std::process::Output,
) -> Result<String, String> {
    if output.status.success() {
        return Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    Err(format!(
        "cargo {} failed with status {}\nstdout:\n{}\nstderr:\n{}",
        subcommand.label(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ))
}

#[cfg(test)]
mod tests {
    //! Unit tests for fixture cargo runner helpers.

    use std::sync::Mutex;

    use super::recover_mutex_guard;

    #[test]
    fn recover_mutex_guard_recovers_from_poisoned_local_mutex() {
        let mutex = Mutex::new(());
        let result = std::panic::catch_unwind(|| {
            let _guard = mutex
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            panic!("deliberate panic to poison local mutex");
        });
        assert!(result.is_err(), "catch_unwind should have caught the panic");

        let guard = recover_mutex_guard(&mutex);
        drop(guard);
    }
}
