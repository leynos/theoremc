//! Cargo command serialisation helpers for theorem-file macro fixtures.

use std::process::Command;
use std::sync::{Mutex, PoisonError};

use camino::Utf8Path;

#[derive(Clone, Copy)]
pub(crate) enum CargoSubcommand {
    Build,
    Test,
}

impl CargoSubcommand {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
        }
    }
}

/// Proof that `FIXTURE_CARGO_LOCK` is held by the current thread.
///
/// Pass a `CargoGuard<'_>` to fixture Cargo helpers to enforce at compile time
/// that no caller bypasses the serialisation contract.
pub(crate) struct CargoGuard<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
}

impl CargoGuard<'_> {
    pub(crate) fn acquire() -> Self {
        Self {
            _guard: FIXTURE_CARGO_LOCK
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }
}

/// Serializes all fixture `cargo` invocations.
///
/// `cargo` writes lock files, registry caches, and incremental artefacts to
/// paths derived from the manifest directory. When multiple fixture crates run
/// `cargo build` concurrently they race on the shared `~/.cargo` registry
/// cache and on any workspace-level `Cargo.lock`, producing spurious build
/// failures even with isolated `CARGO_TARGET_DIR` values. A single global
/// mutex is the minimal correct serialization mechanism for in-process test
/// parallelism; the alternative--spawning each fixture in a fully isolated
/// toolchain environment--would require per-test network access and is
/// disproportionate for a test suite this size.
pub(crate) static FIXTURE_CARGO_LOCK: Mutex<()> = Mutex::new(());

/// Runs a Cargo subcommand in the fixture crate directory.
///
/// `CARGO_TARGET_DIR` is set to a per-fixture subdirectory to avoid artefact
/// collisions. The caller must supply a [`CargoGuard`] proving that
/// `FIXTURE_CARGO_LOCK` is held, because registry cache access and
/// workspace-level lock files are still shared across invocations.
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
    _guard: &CargoGuard<'_>,
) -> Result<(), String> {
    let target_dir = manifest_dir.join("target");
    let output = Command::new("cargo")
        .current_dir(manifest_dir)
        .env("CARGO_TARGET_DIR", target_dir.as_str())
        .args([subcommand.as_str(), "--color", "never"])
        .output()
        .map_err(|error| error.to_string())?;
    command_result(&output)
}

fn command_result(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ))
}
