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

/// The development build standard: the parallel `rustc` frontend and, on
/// Linux, the `mold` linker, matching `.cargo/config.toml` and the Makefile.
const STANDARD_RUSTFLAGS: &str = if cfg!(target_os = "linux") {
    "-Zthreads=8 -Clink-arg=-fuse-ld=mold"
} else {
    "-Zthreads=8"
};

/// Runs a Cargo subcommand in the fixture crate with warnings denied.
///
/// The child process owns its flags, so this does not mutate the test
/// process. Assigning `RUSTFLAGS` makes Cargo ignore the `rustflags` in
/// `.cargo/config.toml`, so the development build standard is composed in
/// explicitly. Failing rather than guessing is deliberate: an inherited flag
/// set that this helper cannot read safely would silently weaken the very
/// check the scenario exists to perform.
pub(crate) fn cargo_run_denying_warnings(
    manifest_dir: &Utf8Path,
    target_dir: &Utf8Path,
    subcommand: CargoSubcommand,
    _guard: &CargoGuard<'_>,
) -> Result<(), String> {
    let rustflags = denying_warnings_rustflags()?;
    let output = hermetic_cargo(manifest_dir, target_dir)
        .args(subcommand.args())
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("RUSTFLAGS", rustflags)
        .output()
        .map_err(|error| error.to_string())?;
    command_result(subcommand, &output).map(|_| ())
}

/// Composes the build standard, inherited arguments, and the denial.
///
/// Inherited arguments are preserved rather than replaced, mirroring
/// `${RUSTFLAGS:+$RUSTFLAGS }` in the Makefile, and blank entries add nothing.
/// Any inherited `--cap-lints` is dropped, because a cap of `warn` would turn
/// the denial back into a warning and let a fixture pass that should fail.
fn compose_rustflags(standard: &str, inherited: &[String]) -> String {
    let mut composed = vec![standard.to_owned()];
    composed.extend(
        inherited
            .iter()
            .map(|flag| flag.trim())
            .filter(|flag| !flag.is_empty())
            .filter(|flag| !flag.starts_with("--cap-lints"))
            .map(str::to_owned),
    );
    composed.push("-D warnings".to_owned());
    composed.join(" ")
}

/// Returns the child build's `RUSTFLAGS`.
///
/// Cargo prefers `CARGO_ENCODED_RUSTFLAGS` over `RUSTFLAGS` and splits it on
/// its own separator rather than on whitespace, so an encoded value is read as
/// distinct arguments; that keeps a linker path containing spaces intact.
/// Blank values in the legacy variable are dropped, which is what the Makefile
/// does when it appends its flags.
#[expect(
    clippy::disallowed_methods,
    reason = "the fixture build must inherit the host Cargo invocation's rustflags, so it composes them exactly as the Makefile recipes do"
)]
fn denying_warnings_rustflags() -> Result<String, String> {
    let encoded = std::env::var("CARGO_ENCODED_RUSTFLAGS").ok();
    let inherited = match encoded.as_deref() {
        Some(value) if !value.trim().is_empty() => value
            .split('\u{1f}')
            .filter(|flag| !flag.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        _ => {
            let legacy = std::env::var("RUSTFLAGS").map_err(|error| {
                format!("reading the inherited RUSTFLAGS for the strict build: {error}")
            })?;
            legacy
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        }
    };
    Ok(compose_rustflags(STANDARD_RUSTFLAGS, &inherited))
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

mod tests {
    //! Tests for the child-build flag composition.

    use super::compose_rustflags;

    /// Builds the argument list a caller would pass in.
    fn flags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn compose_rustflags_without_inherited_flags_keeps_the_standard() {
        let composed = compose_rustflags("-Zthreads=8", &[]);

        assert_eq!(
            composed, "-Zthreads=8 -D warnings",
            "the standard and the denial should both be present"
        );
    }

    #[test]
    fn compose_rustflags_places_inherited_flags_before_the_denial() {
        let composed = compose_rustflags("-Zthreads=8", &flags(&["-Cdebuginfo=1"]));

        assert_eq!(
            composed, "-Zthreads=8 -Cdebuginfo=1 -D warnings",
            "inherited flags should sit between the standard and the denial"
        );
    }

    #[test]
    fn compose_rustflags_drops_a_weakening_lint_cap() {
        let composed = compose_rustflags(
            "-Zthreads=8",
            &flags(&["--cap-lints=warn", "-Cdebuginfo=1"]),
        );

        assert_eq!(
            composed, "-Zthreads=8 -Cdebuginfo=1 -D warnings",
            "an inherited cap must not survive to soften the denial"
        );
    }

    #[test]
    fn compose_rustflags_ignores_a_blank_inherited_argument() {
        let composed = compose_rustflags("-Zthreads=8", &flags(&["   "]));

        assert_eq!(
            composed, "-Zthreads=8 -D warnings",
            "a blank argument should not leave a stray space"
        );
    }
}
