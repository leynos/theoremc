//! Executable contract test for the aggregate `lint` Makefile target.
//!
//! `tests/lint_contract.rs` reads the recipes as text, but a text copy cannot
//! show that `make lint` really runs each stage, in the documented order, or
//! that a failing stage stops the aggregate. This test puts stub `cargo` and
//! `whitaker` executables at the recipes' variables and records what each
//! invocation received.

#![cfg(unix)]

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

/// A stub `cargo`/`whitaker` that records its stage, environment, and arguments.
///
/// The stage is the executable name, except for `cargo`, where it is the first
/// argument (`doc` or `clippy`), so one script can stand in for every stage.
/// `STUB_FAIL_ON` names the stage that must exit non-zero.
const STUB: &str = concat!(
    "#!/bin/sh\n",
    "stage=$(basename \"$0\")\n",
    "if [ \"$stage\" = cargo ]; then stage=${1-}; fi\n",
    "{\n",
    "  printf 'STAGE=%s\\n' \"$stage\"\n",
    "  printf 'RUSTFLAGS=%s\\n' \"${RUSTFLAGS-<unset>}\"\n",
    "  printf 'RUSTDOCFLAGS=%s\\n' \"${RUSTDOCFLAGS-<unset>}\"\n",
    "  printf 'DYLINT_RUSTFLAGS=%s\\n' \"${DYLINT_RUSTFLAGS-<unset>}\"\n",
    "  for argument in \"$@\"; do printf 'ARG=%s\\n' \"$argument\"; done\n",
    "} >> \"$STUB_LOG\"\n",
    "if [ \"$stage\" = \"${STUB_FAIL_ON-}\" ]; then exit 5; fi\n",
    "exit 0\n",
);

/// The arguments the rustdoc stage must pass to Cargo.
const DOC_ARGUMENTS: &[&str] = &["doc", "--workspace", "--all-features", "--no-deps"];

/// The arguments the Clippy stage must pass to Cargo.
const CLIPPY_ARGUMENTS: &[&str] = &[
    "clippy",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings",
];

/// The arguments the Whitaker stage must pass to the wrapper.
/// `--workspace` is Whitaker's own flag and is forwarded to `cargo check`, so
/// it must appear once, before `--`.
const WHITAKER_ARGUMENTS: &[&str] = &[
    "--all",
    "--workspace",
    "--",
    "--all-targets",
    "--all-features",
];

/// One recorded stub invocation.
#[derive(Default)]
struct Invocation {
    stage: String,
    rustflags: String,
    rustdocflags: String,
    dylint_rustflags: String,
    arguments: Vec<String>,
}

/// Writes the stub under both executable names and makes them executable.
///
/// # Errors
///
/// Returns the reason when a script cannot be written or made executable.
fn write_stubs(directory: &Dir) -> Result<(), String> {
    for name in ["cargo", "whitaker"] {
        directory
            .write(name, STUB)
            .map_err(|error| format!("writing the stub {name}: {error}"))?;
        directory
            .set_permissions(name, Permissions::from_mode(0o755))
            .map_err(|error| format!("making the stub {name} executable: {error}"))?;
    }
    Ok(())
}

/// Runs the aggregate target against the stubs in `root`.
///
/// `fail_on` names the stage that must exit non-zero; an empty value lets every
/// stage succeed. `RUSTFLAGS` is deliberately set, so the test can prove the
/// Whitaker stage clears it.
///
/// # Errors
///
/// Returns the reason when the stubs cannot be written or `make` cannot start.
fn run_lint_target(root: &Path, fail_on: &str) -> Result<Output, String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    write_stubs(&directory)?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    Command::new("make")
        .arg("--no-print-directory")
        .arg("-f")
        .arg(manifest_dir.join("Makefile"))
        .arg("lint")
        .arg(format!("CARGO={}", root.join("cargo").display()))
        .arg(format!("WHITAKER={}", root.join("whitaker").display()))
        .current_dir(manifest_dir)
        .env("STUB_LOG", root.join("stub.log"))
        .env("STUB_FAIL_ON", fail_on)
        .env("RUSTFLAGS", "-Csentinel")
        .output()
        .map_err(|error| format!("running make lint: {error}"))
}

/// Reads the recorded invocations from the stub log.
///
/// Each invocation is one contiguous block that opens with its `STAGE=` line.
///
/// # Errors
///
/// Returns the reason when the log cannot be read.
fn read_invocations(root: &Path) -> Result<Vec<Invocation>, String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    let recorded = directory
        .read_to_string("stub.log")
        .map_err(|error| format!("reading the stub log: {error}"))?;
    Ok(recorded.split("STAGE=").skip(1).map(parse_block).collect())
}

/// Parses one `STAGE=`-delimited block of the stub log.
fn parse_block(block: &str) -> Invocation {
    let mut lines = block.lines();
    let mut invocation = Invocation {
        stage: lines.next().unwrap_or_default().to_owned(),
        ..Invocation::default()
    };
    for line in lines {
        if let Some(value) = line.strip_prefix("RUSTFLAGS=") {
            invocation.rustflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("RUSTDOCFLAGS=") {
            invocation.rustdocflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("DYLINT_RUSTFLAGS=") {
            invocation.dylint_rustflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("ARG=") {
            invocation.arguments.push(value.to_owned());
        }
    }
    invocation
}

/// Returns the recorded invocation for the named stage.
///
/// # Errors
///
/// Returns the reason when the stage never ran.
fn stage<'a>(invocations: &'a [Invocation], name: &str) -> Result<&'a Invocation, String> {
    invocations
        .iter()
        .find(|invocation| invocation.stage == name)
        .ok_or_else(|| format!("make lint never ran the {name} stage"))
}

/// Returns the names of the stages that ran, in order.
fn stage_names(invocations: &[Invocation]) -> Vec<&str> {
    invocations
        .iter()
        .map(|invocation| invocation.stage.as_str())
        .collect()
}

#[test]
fn aggregate_runs_rustdoc_then_clippy_then_whitaker() -> Result<(), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = run_lint_target(directory.path(), "")?;
    if !output.status.success() {
        return Err(format!(
            "make lint must succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let invocations = read_invocations(directory.path())?;
    if stage_names(&invocations) != ["doc", "clippy", "whitaker"] {
        return Err(format!(
            "make lint must run rustdoc, then Clippy, then Whitaker, costing one stage per failure; ran {:?}",
            stage_names(&invocations)
        ));
    }

    let doc = stage(&invocations, "doc")?;
    if doc.arguments.iter().map(String::as_str).collect::<Vec<_>>() != DOC_ARGUMENTS {
        return Err(format!(
            "the rustdoc stage passed {:?}, expected {DOC_ARGUMENTS:?}",
            doc.arguments
        ));
    }
    if doc.rustdocflags != "-D warnings" {
        return Err(format!(
            "the rustdoc stage must deny warnings; the stub saw `{}`",
            doc.rustdocflags
        ));
    }

    let clippy = stage(&invocations, "clippy")?;
    if clippy
        .arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != CLIPPY_ARGUMENTS
    {
        return Err(format!(
            "the Clippy stage passed {:?}, expected {CLIPPY_ARGUMENTS:?}",
            clippy.arguments
        ));
    }

    let whitaker = stage(&invocations, "whitaker")?;
    if whitaker
        .arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != WHITAKER_ARGUMENTS
    {
        return Err(format!(
            "the Whitaker stage passed {:?}, expected {WHITAKER_ARGUMENTS:?}",
            whitaker.arguments
        ));
    }
    if whitaker.dylint_rustflags != "-D warnings" {
        return Err(format!(
            "the Whitaker stage must deny lint warnings; the stub saw `{}`",
            whitaker.dylint_rustflags
        ));
    }
    if !whitaker.rustflags.is_empty() {
        return Err(format!(
            "the Whitaker stage must clear RUSTFLAGS; the stub saw `{}`",
            whitaker.rustflags
        ));
    }
    Ok(())
}

#[test]
fn a_failing_stage_stops_the_aggregate() -> Result<(), String> {
    for (failing, expected) in [
        ("doc", vec!["doc"]),
        ("clippy", vec!["doc", "clippy"]),
        ("whitaker", vec!["doc", "clippy", "whitaker"]),
    ] {
        let directory =
            TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
        let output = run_lint_target(directory.path(), failing)?;
        if output.status.success() {
            return Err(format!(
                "make lint must fail when the {failing} stage fails"
            ));
        }
        let invocations = read_invocations(directory.path())?;
        if stage_names(&invocations) != expected {
            return Err(format!(
                "a failing {failing} stage must stop the aggregate after {:?}; ran {:?}",
                expected,
                stage_names(&invocations)
            ));
        }
    }
    Ok(())
}
