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
/// `cargo` writes lock files, registry caches, and incremental artefacts to
/// paths derived from the manifest directory. When multiple fixture crates run
/// `cargo build` concurrently they race on the shared `~/.cargo` registry
/// cache and on any workspace-level `Cargo.lock`, producing spurious build
/// failures even though every fixture builds into the shared warm target.
/// Nextest's `nested-cargo-builds` group serializes these tests across
/// processes; this mutex covers the libtest threads inside one process.
pub(crate) static FIXTURE_CARGO_LOCK: Mutex<()> = Mutex::new(());

/// Runs a Cargo subcommand in the fixture crate directory.
///
/// `CARGO_TARGET_DIR` is set to the target directory every fixture shares, so
/// third-party dependencies compile once per run; fixtures keep their own
/// artefacts apart through unique package names. The caller must supply a
/// [`CargoGuard`] proving that `FIXTURE_CARGO_LOCK` is held, because registry
/// cache access and workspace-level lock files are still shared across
/// invocations.
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
    let output = hermetic_cargo(manifest_dir, target_dir)
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

/// What the Kani scenarios should do with one probe-plus-run outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KaniOutcome {
    /// The probe and the run both succeeded; the run's output follows.
    Ran(String),
    /// Kani is unavailable, so the check may be skipped.
    Skipped(String),
}

/// Classifies one availability probe and the Kani run that follows it.
///
/// The probe and the run have different meanings, and so do their failures. A
/// failed availability probe — a missing `cargo-kani`, a launch failure, or a
/// non-zero `--version` exit — means Kani is unavailable: a skip when the suite
/// runs without `require_kani`, and a failure when it is required.
///
/// Everything after a successful probe is a failure, whatever the error says.
/// A toolchain that answers `cargo kani --version` is installed, so a run that
/// then cannot load a shared library or produce output is a broken installation
/// rather than an absent optional tool. Downgrading such a failure to a skip is
/// the silent pass that the required-Kani jobs exist to prevent, so
/// [`KaniOutcome::Skipped`] is only ever produced for a failed probe.
///
/// # Errors
///
/// Returns the run's error unchanged when the probe succeeded, and the
/// required-Kani refusal when the probe failed under `require_kani`.
pub(crate) fn classify_kani_run<F>(
    check: &str,
    is_installed: bool,
    require_kani: bool,
    run: F,
) -> Result<KaniOutcome, String>
where
    F: FnOnce() -> Result<String, String>,
{
    if !is_installed {
        let reason = "the cargo kani --version probe failed";
        return if require_kani {
            Err(format!(
                "THEOREMC_REQUIRE_KANI=1 forbids skipping the Kani {check} check: {reason}"
            ))
        } else {
            Ok(KaniOutcome::Skipped(reason.to_owned()))
        };
    }

    run().map(KaniOutcome::Ran)
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
    //! Unit tests for the Kani probe-and-run policy.

    use super::{KaniOutcome, classify_kani_run};

    /// The loader failure observed when Kani's toolchain cannot start.
    const LOADER_ERROR: &str = "error while loading shared libraries: libLLVM.so";

    /// Proves the probe gate by failing the test when the run closure executes.
    fn run_must_not_happen() -> Result<String, String> {
        Err("the Kani run must not start when the availability probe fails".to_owned())
    }

    #[test]
    fn a_missing_kani_toolchain_skips_the_check_when_kani_is_optional() {
        let outcome = classify_kani_run("listing", false, false, run_must_not_happen)
            .expect("an unavailable optional tool should skip rather than fail");

        assert_eq!(
            outcome,
            KaniOutcome::Skipped("the cargo kani --version probe failed".to_owned()),
            "a failed probe should report why the check was skipped"
        );
    }

    #[test]
    fn a_missing_kani_toolchain_fails_the_check_when_kani_is_required() {
        let error = classify_kani_run("listing", false, true, run_must_not_happen)
            .expect_err("a required Kani must not skip when the probe fails");

        assert!(
            error.contains("THEOREMC_REQUIRE_KANI=1") && error.contains("listing"),
            "the failure should name the requirement and the check, got: {error}"
        );
    }

    #[test]
    fn a_successful_probe_reports_the_run_output() {
        let outcome = classify_kani_run("listing", true, false, || Ok("harnesses".to_owned()))
            .expect("a successful run should report its output");

        assert_eq!(outcome, KaniOutcome::Ran("harnesses".to_owned()));
    }

    #[test]
    fn a_loader_failure_after_a_successful_probe_fails_even_when_kani_is_optional() {
        let error = classify_kani_run("listing", true, false, || Err(LOADER_ERROR.to_owned()))
            .expect_err("a broken installation must not pass as an optional skip");

        assert_eq!(
            error, LOADER_ERROR,
            "the listing error should propagate unchanged"
        );
    }

    #[test]
    fn another_run_failure_after_a_successful_probe_fails_the_check() {
        let error = classify_kani_run("verification", true, false, || {
            Err("cargo kani failed with status exit status: 1".to_owned())
        })
        .expect_err("a run failure after a successful probe must fail the check");

        assert!(
            error.contains("exit status: 1"),
            "the run failure should propagate, got: {error}"
        );
    }

    #[test]
    fn a_fixture_build_failure_after_a_successful_probe_fails_the_check() {
        let error = classify_kani_run("listing", true, true, || {
            Err("could not compile fixture crate".to_owned())
        })
        .expect_err("a fixture build failure must fail the check");

        assert!(
            error.contains("could not compile fixture crate"),
            "the build failure should propagate, got: {error}"
        );
    }
}
