//! Cargo command helpers for theorem-file macro fixtures.
//!
//! This module is the process boundary for `theorem_file_macro_bdd.rs`.
//! `fixture_crate.rs` builds temporary crates, then delegates all Cargo
//! invocations here so fixture builds and `cargo kani` runs share one lock, one
//! deadline and one error-reporting format.
//!
//! The lock is a lock *file*, not a mutex: `cargo nextest` runs every test in its
//! own process, so an in-process mutex would serialize nothing and every fixture
//! build would start at once. See `test_helpers::FixtureLock`.

use std::process::Command;
use std::time::Duration;

use camino::Utf8Path;
use test_helpers::{
    FixtureOutcome, hermetic_cargo, run_fixture_command, run_fixture_command_within,
};

/// How long one `cargo kani` build may take before the test fails.
///
/// Kani compiles the whole fixture graph with its own compiler into its own
/// target directory, so its first run is a second cold build and is much slower
/// than a fixture `cargo build`. Giving it the ordinary fixture deadline would
/// fail a legitimate verification run on a slow or busy host; the nextest
/// group's allowance still bounds it.
const KANI_COMMAND_DEADLINE: Duration = Duration::from_secs(1140);

/// How long the Kani probe may take before it is reported as a failure.
///
/// `cargo kani --version` starts a toolchain lookup, not a build, so a probe that
/// outlives a minute is a broken environment rather than a slow one. Bounding it
/// separately keeps a hung probe from spending the whole fixture deadline.
const KANI_PROBE_DEADLINE: Duration = Duration::from_secs(60);

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
            Self::Build => &["build", "--color", "never"],
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

/// Runs a Cargo subcommand in the fixture crate directory.
///
/// `CARGO_TARGET_DIR` is set to the target directory every fixture shares, so
/// third-party dependencies compile once per warm target rather than once per
/// fixture; a `cargo clean` or a different `CARGO_TARGET_DIR` starts a new one.
/// Fixtures keep their own artefacts apart through unique package names. The
/// command holds the fixture lock for its whole lifetime and is bounded by the
/// fixture deadline, so it cannot start beside another fixture build or outlive
/// its allowance.
pub(crate) fn cargo_run(
    manifest_dir: &Utf8Path,
    target_dir: &Utf8Path,
    subcommand: CargoSubcommand,
) -> Result<(), String> {
    cargo_run_output(manifest_dir, target_dir, subcommand).map(|_| ())
}

pub(crate) fn cargo_run_output(
    manifest_dir: &Utf8Path,
    target_dir: &Utf8Path,
    subcommand: CargoSubcommand,
) -> Result<String, String> {
    let mut command = hermetic_cargo(manifest_dir, target_dir);
    command.args(subcommand.args());
    let outcome = match subcommand {
        CargoSubcommand::Build => run_fixture_command(command)?,
        CargoSubcommand::KaniList | CargoSubcommand::Kani => {
            run_fixture_command_within(command, KANI_COMMAND_DEADLINE)?
        }
    };
    match outcome {
        FixtureOutcome::Completed(output) if output.is_success() => Ok(output.combined()),
        FixtureOutcome::Completed(output) => Err(format!(
            "cargo {} failed with status {}\nstdout:\n{}\nstderr:\n{}",
            subcommand.label(),
            output.status,
            output.stdout,
            output.stderr,
        )),
        FixtureOutcome::TimedOut {
            deadline,
            elapsed,
            output,
        } => Err(format!(
            "cargo {} did not finish within {deadline:?} (ran for {elapsed:?}); \
             it printed:\n{output}",
            subcommand.label(),
        )),
    }
}

/// Whether a usable `cargo kani` is available, and why not when it is not.
#[derive(Debug)]
pub(crate) enum KaniAvailability {
    /// `cargo kani --version` succeeded.
    Available,
    /// Kani is not installed, or its toolchain is unusable, for this reason.
    Unavailable(String),
    /// The probe itself did not finish in time.
    TimedOut(Duration),
}

/// Probes for a usable `cargo kani`.
///
/// The probe runs under the fixture lock and its own short deadline, so a broken
/// Kani cannot leave a `kani-compiler` build running while the rest of the suite
/// waits behind the lock. A timeout is distinguished from unavailability: the
/// first is a failure of the environment this run must report, the second is a
/// reason to skip.
pub(crate) fn kani_is_available() -> Result<KaniAvailability, String> {
    probe_kani("cargo")
}

/// Probes `program kani --version`, using the production deadline.
fn probe_kani(program: impl AsRef<std::ffi::OsStr>) -> Result<KaniAvailability, String> {
    probe_kani_within(program, KANI_PROBE_DEADLINE)
}

/// Probes `program kani --version` with an explicit deadline.
fn probe_kani_within(
    program: impl AsRef<std::ffi::OsStr>,
    deadline: Duration,
) -> Result<KaniAvailability, String> {
    let mut command = Command::new(program);
    command.args(["kani", "--version"]);
    match run_fixture_command_within(command, deadline)? {
        FixtureOutcome::Completed(output) if output.is_success() => Ok(KaniAvailability::Available),
        FixtureOutcome::Completed(output) => {
            let detail = output.stderr.trim();
            Ok(KaniAvailability::Unavailable(if detail.is_empty() {
                format!(
                    "`cargo kani --version` exited with status {}",
                    output.status
                )
            } else {
                detail.to_owned()
            }))
        }
        FixtureOutcome::TimedOut {
            deadline: overran, ..
        } => Ok(KaniAvailability::TimedOut(overran)),
    }
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

#[cfg(test)]
#[path = "cargo_runner_tests.rs"]
mod tests;
