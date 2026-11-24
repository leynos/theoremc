//! Cargo command serialization helpers for theorem-file macro fixtures.
//!
//! This module is the process boundary for `theorem_file_macro_bdd.rs`.
//! `fixture_crate.rs` builds temporary crates, then delegates all Cargo
//! invocations here so fixture builds and `cargo kani list` runs share one
//! lock, target-directory policy, and error-reporting format.

use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

use camino::Utf8Path;
use test_helpers::{fixture_cargo_home, hermetic_cargo};

#[derive(Clone, Copy)]
pub(crate) enum CargoSubcommand {
    Build,
    KaniList,
    /// Verifies every generated harness rather than only listing them.
    Kani,
}

impl CargoSubcommand {
    const fn args(self) -> &'static [&'static str] {
        match self {
            Self::Build => &["build", "--color", "never", "--frozen"],
            Self::KaniList => &["kani", "list"],
            Self::Kani => &["kani"],
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::KaniList => "kani list",
            Self::Kani => "kani",
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

/// Serializes all fixture `cargo` invocations.
///
/// Every fixture build resolves from the dedicated fixture Cargo home, so it
/// takes that home's package-cache lock rather than the caller's, and it runs
/// `--frozen`, so it never needs the lock for an index update. Two fixture
/// builds still share one warm target directory and one Cargo home, so running
/// them at once would have them contend for the same lock and interleave their
/// artefacts. A single global mutex keeps them apart; the alternative, a
/// private home and target per fixture, would lose the warm cache that keeps
/// these builds to seconds.
pub(crate) static FIXTURE_CARGO_LOCK: Mutex<()> = Mutex::new(());

/// Runs a Cargo subcommand in the fixture crate directory.
///
/// `CARGO_TARGET_DIR` is set to the target directory every fixture shares, so
/// third-party dependencies compile once per run; fixtures keep their own
/// artefacts apart through unique package names. `CARGO_HOME` is the dedicated
/// fixture home, so a build never waits on the caller's package-cache lock. The
/// caller must supply a [`CargoGuard`] proving that `FIXTURE_CARGO_LOCK` is
/// held, because the target and the home are still shared across invocations.
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
    target_dir: &Utf8Path,
    subcommand: CargoSubcommand,
    guard: &CargoGuard<'_>,
) -> Result<(), String> {
    cargo_run_output(manifest_dir, target_dir, subcommand, guard).map(|_| ())
}

pub(crate) fn cargo_run_output(
    manifest_dir: &Utf8Path,
    target_dir: &Utf8Path,
    subcommand: CargoSubcommand,
    _guard: &CargoGuard<'_>,
) -> Result<String, String> {
    let output = hermetic_cargo(manifest_dir, target_dir, &fixture_cargo_home()?)
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
///
/// The probe deliberately inherits the caller's environment and does not use
/// the fixture Cargo home. Two measurements settle that. A version probe never
/// opens the package cache — with the caller's lock held exclusively,
/// `cargo kani --version` still returned in 165 ms — so it cannot contend for
/// the lock the fixture builds avoid. And Cargo searches `$CARGO_HOME/bin`
/// before `PATH` for an external subcommand, so pointing `CARGO_HOME` at the
/// fixture home would hide a `cargo-kani` installed only in the caller's
/// `$CARGO_HOME/bin` and make both Kani scenarios skip.
pub(crate) fn kani_is_installed() -> bool {
    Command::new("cargo")
        .args(["kani", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Returns `true` when the run must fail rather than skip without Kani.
///
/// `make kani` and the proofs workflow set `THEOREMC_REQUIRE_KANI=1`, so a
/// runner without a usable Kani is a red job instead of a silent pass. A
/// developer without Kani leaves it unset and the Kani scenarios skip.
#[expect(
    clippy::disallowed_methods,
    reason = "the CI proofs workflow and `make kani` export THEOREMC_REQUIRE_KANI to the test process, so the harness reads the host's value at this boundary"
)]
pub(crate) fn kani_is_required() -> bool {
    std::env::var_os("THEOREMC_REQUIRE_KANI").is_some_and(|value| value == "1")
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
