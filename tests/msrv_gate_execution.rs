//! Executable contract test for the `msrv` Makefile target.
//!
//! `tests/msrv_contract.rs` reads the recipe as text, but a text copy cannot
//! prove that `make` actually runs it. This test puts a stub `cargo` at the
//! recipe's `CARGO` variable, then inspects the arguments and environment the
//! recipe passes. The real `make msrv` check stays in CI as its own step:
//! duplicating the full Rust 1.89 workspace check here would double the gate
//! cost without adding coverage.

#![cfg(unix)]

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// The exact argument vector the `msrv` recipe must pass to Cargo.
const EXPECTED_ARGUMENTS: &[&str] = &[
    "+1.89",
    "check",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--locked",
];

/// A stub `cargo` that logs its environment and arguments, then exits with the
/// code in `STUB_CARGO_EXIT`.
///
/// The first log line records `RUSTFLAGS`, so the test can prove the recipe
/// clears it; each following line holds one argument.
const STUB_CARGO: &str = concat!(
    "#!/bin/sh\n",
    "printf 'RUSTFLAGS=%s\\n' \"${RUSTFLAGS-<unset>}\" > \"$STUB_CARGO_LOG\"\n",
    "for argument in \"$@\"; do printf '%s\\n' \"$argument\"; done >> \"$STUB_CARGO_LOG\"\n",
    "exit \"${STUB_CARGO_EXIT:-0}\"\n",
);

/// Writes an executable stub `cargo` into `directory`.
///
/// # Errors
///
/// Returns the reason when the script cannot be written or made executable.
fn write_stub_cargo(directory: &Dir) -> Result<(), String> {
    directory
        .write("cargo", STUB_CARGO)
        .map_err(|error| format!("writing the stub cargo: {error}"))?;
    directory
        .set_permissions("cargo", Permissions::from_mode(0o755))
        .map_err(|error| format!("making the stub cargo executable: {error}"))
}

/// Runs the `msrv` target with the stub Cargo and returns its output.
///
/// The stub writes its log to `cargo.log` in `root`.
///
/// # Errors
///
/// Returns the reason when `make` cannot start.
fn run_msrv_target(root: &Path, exit_code: u32) -> Result<Output, String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    write_stub_cargo(&directory)?;
    let stub = root.join("cargo");
    let log = root.join("cargo.log");
    let makefile = Path::new(env!("CARGO_MANIFEST_DIR")).join("Makefile");
    let output = Command::new("make")
        .arg("--no-print-directory")
        .arg("-f")
        .arg(makefile)
        .arg("msrv")
        .arg(format!("CARGO={}", stub.display()))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("STUB_CARGO_LOG", &log)
        .env("STUB_CARGO_EXIT", exit_code.to_string())
        .output()
        .map_err(|error| format!("running make msrv: {error}"))?;
    Ok(output)
}

/// Splits the stub's log into its environment line and recorded arguments.
///
/// # Errors
///
/// Returns the reason when the log cannot be read or holds no environment
/// line.
fn read_log(root: &Path) -> Result<(String, Vec<String>), String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    let recorded = directory
        .read_to_string("cargo.log")
        .map_err(|error| format!("reading the stub log: {error}"))?;
    let mut lines = recorded.lines().map(str::to_owned);
    let environment = lines
        .next()
        .ok_or_else(|| "the stub log must start with the environment line".to_owned())?;
    Ok((environment, lines.collect()))
}

#[test]
fn msrv_target_passes_the_locked_rust_1_89_check_to_cargo() -> Result<(), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = run_msrv_target(directory.path(), 0)?;
    if !output.status.success() {
        return Err(format!(
            "make msrv must succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let (environment, arguments) = read_log(directory.path())?;
    if environment != "RUSTFLAGS=" {
        return Err(format!(
            "the recipe must clear RUSTFLAGS; the stub saw `{environment}`"
        ));
    }
    if arguments.as_slice() != EXPECTED_ARGUMENTS {
        return Err(format!(
            "the recipe passed {arguments:?}, expected {EXPECTED_ARGUMENTS:?}"
        ));
    }
    Ok(())
}

#[test]
fn msrv_target_fails_when_cargo_fails() -> Result<(), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = run_msrv_target(directory.path(), 5)?;
    if output.status.success() {
        return Err("make must fail when the recipe's cargo invocation fails".to_owned());
    }
    Ok(())
}
