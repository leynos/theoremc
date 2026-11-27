//! End-to-end proof that the lint stages cover a non-root workspace member.
//!
//! `tests/lint_gate_execution.rs` proves the recipes pass the right arguments,
//! but a stub cannot show that Cargo acts on them. This test writes a throwaway
//! workspace with a root package and one member, runs the same `cargo doc` and
//! `cargo clippy` invocations the recipes use, and checks that a clean
//! workspace passes while a finding in the member fails the stage.
//!
//! The workspace has no dependencies, so it builds offline in a few seconds and
//! does not depend on the state of the shared registry cache.

#![cfg(unix)]

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use rstest::rstest;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

/// The root manifest, which mirrors this repository's hybrid shape: `[workspace]`
/// and `[package]` in one file, so a bare `cargo` invocation would select the
/// root package alone.
const ROOT_MANIFEST: &str = concat!(
    "[workspace]\n",
    "members = [\"member\"]\n",
    "resolver = \"2\"\n",
    "\n",
    "[package]\n",
    "name = \"lint-root\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2021\"\n",
);

/// A root package with nothing to report.
const ROOT_SOURCE: &str = "pub fn root() -> u8 { 1 }\n";

/// The member manifest.
const MEMBER_MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"lint-member\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2021\"\n",
);

/// A member with nothing to report.
const MEMBER_SOURCE: &str = "pub fn member() -> u8 { 2 }\n";

/// A member with a private function no caller reaches, which `dead_code` reports
/// and `-D warnings` turns into a failure.
const MEMBER_WITH_DEAD_CODE: &str = concat!(
    "pub fn member() -> u8 { 2 }\n",
    "\n",
    "fn unreachable_helper() -> u8 { 3 }\n",
);

/// A member whose documentation links to an item that does not exist, which
/// `broken_intra_doc_links` reports.
const MEMBER_WITH_BROKEN_LINK: &str = concat!(
    "/// See [`absent_item`].\n",
    "pub fn member() -> u8 { 2 }\n",
);

/// One lint stage, as the Makefile invokes it.
struct Stage {
    /// The name used in failure messages.
    name: &'static str,
    /// The Cargo subcommand and its arguments.
    arguments: &'static [&'static str],
    /// The environment variable that denies warnings, when the stage uses one
    /// instead of a trailing `-D warnings`.
    warning_variable: Option<(&'static str, &'static str)>,
}

/// The rustdoc stage: `cargo doc --workspace --all-features --no-deps`.
static RUSTDOC_STAGE: Stage = Stage {
    name: "rustdoc",
    arguments: &["doc", "--workspace", "--all-features", "--no-deps"],
    warning_variable: Some(("RUSTDOCFLAGS", "-D warnings")),
};

/// The Clippy stage: `cargo clippy --workspace --all-targets --all-features -- -D
/// warnings`.
static CLIPPY_STAGE: Stage = Stage {
    name: "Clippy",
    arguments: &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ],
    warning_variable: None,
};

/// Writes a two-package workspace whose member holds `member_source`.
///
/// # Errors
///
/// Returns the reason when the temporary directory or a file cannot be created.
fn write_workspace(member_source: &str) -> Result<TempDir, String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let fs = Dir::open_ambient_dir(directory.path(), ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    for (relative_path, contents) in [
        ("Cargo.toml", ROOT_MANIFEST),
        ("src/lib.rs", ROOT_SOURCE),
        ("member/Cargo.toml", MEMBER_MANIFEST),
        ("member/src/lib.rs", member_source),
    ] {
        write_file(&fs, relative_path, contents)?;
    }
    Ok(directory)
}

/// Writes `contents` to `relative_path`, creating parent directories first.
///
/// # Errors
///
/// Returns the reason when a directory or the file cannot be created.
fn write_file(fs: &Dir, relative_path: &str, contents: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(relative_path).parent() {
        fs.create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }
    fs.write(relative_path, contents)
        .map_err(|error| format!("writing {relative_path}: {error}"))
}

/// Runs `stage` in `workspace`, with the workspace's own target directory and a
/// cleared `RUSTFLAGS` so the outer suite's flags cannot decide the result.
///
/// # Errors
///
/// Returns the reason when Cargo cannot be started.
fn run_stage(workspace: &Path, stage: &Stage) -> Result<Output, String> {
    let mut command = Command::new("cargo");
    command
        .args(stage.arguments)
        .current_dir(workspace)
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true");
    if let Some((name, value)) = stage.warning_variable {
        command.env(name, value);
    }
    command
        .output()
        .map_err(|error| format!("running the {} stage: {error}", stage.name))
}

/// Verifies that a workspace with no findings passes each stage.
#[rstest]
#[case::rustdoc(&RUSTDOC_STAGE)]
#[case::clippy(&CLIPPY_STAGE)]
fn a_clean_workspace_passes(#[case] stage: &Stage) -> Result<(), String> {
    let workspace = write_workspace(MEMBER_SOURCE)?;
    let output = run_stage(workspace.path(), stage)?;
    if !output.status.success() {
        return Err(format!(
            "the {} stage must pass a clean workspace; stderr was:\n{}",
            stage.name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Verifies that a finding in the non-root member fails the stage.
///
/// This is the behaviour issue #99 was about: before `--workspace` reached the
/// stages, a finding in a member crate could not fail any gate.
#[rstest]
#[case::rustdoc(&RUSTDOC_STAGE, MEMBER_WITH_BROKEN_LINK)]
#[case::clippy(&CLIPPY_STAGE, MEMBER_WITH_DEAD_CODE)]
fn a_finding_in_a_member_crate_fails_the_stage(
    #[case] stage: &Stage,
    #[case] member_source: &str,
) -> Result<(), String> {
    let workspace = write_workspace(member_source)?;
    let output = run_stage(workspace.path(), stage)?;
    if output.status.success() {
        return Err(format!(
            "the {} stage must fail on a finding in the member crate, but it passed",
            stage.name
        ));
    }
    Ok(())
}

/// This repository's `clippy.toml`, which the policy workspace copies.
const CLIPPY_CONFIGURATION: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml"));

/// A manifest declaring the two workspace lint levels the policy depends on.
const POLICY_MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"lint-policy\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2021\"\n",
    "\n",
    "[lints.clippy]\n",
    "disallowed_methods = \"deny\"\n",
    "\n",
    "[lints.rust]\n",
    "unsafe_code = \"forbid\"\n",
);

/// A direct environment read, which the policy must refuse.
const AMBIENT_READ: &str = concat!(
    "pub fn read() -> Option<String> {\n",
    "    std::env::var(\"THEOREMC_EXAMPLE\").ok()\n",
    "}\n",
);

/// The same read behind a reasoned expectation, which the policy must accept.
const EXPECTED_READ: &str = concat!(
    "#[expect(\n",
    "    clippy::disallowed_methods,\n",
    "    reason = \"the policy workspace proves a reasoned expectation is accepted\"\n",
    ")]\n",
    "pub fn read() -> Option<String> {\n",
    "    std::env::var(\"THEOREMC_EXAMPLE\").ok()\n",
    "}\n",
);

/// Writes a single-package workspace carrying the environment policy.
///
/// # Errors
///
/// Returns the reason when a directory or file cannot be created.
fn write_policy_workspace(source: &str) -> Result<TempDir, String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let fs = Dir::open_ambient_dir(directory.path(), ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    for (relative_path, contents) in [
        ("Cargo.toml", POLICY_MANIFEST),
        ("clippy.toml", CLIPPY_CONFIGURATION),
        ("src/lib.rs", source),
    ] {
        write_file(&fs, relative_path, contents)?;
    }
    Ok(directory)
}

/// Runs Clippy over the policy workspace with the host's configuration removed.
///
/// `CLIPPY_CONF_DIR` can redirect Clippy to a different configuration
/// directory, so it is removed; otherwise the copied `clippy.toml` might not be
/// the one Clippy read, and the test would prove nothing.
///
/// # Errors
///
/// Returns the reason when Cargo cannot be started.
fn run_policy_clippy(workspace: &Path) -> Result<Output, String> {
    Command::new("cargo")
        .args(CLIPPY_STAGE.arguments)
        .current_dir(workspace)
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CLIPPY_CONF_DIR")
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| format!("running Clippy over the policy workspace: {error}"))
}

/// Verifies that the configured prohibition really fails a build.
///
/// A `clippy.toml` entry is inert without `disallowed_methods = "deny"`. This
/// case proves the two halves meet: the diagnostic must name the lint and the
/// advice the contributor is meant to follow.
#[test]
fn a_direct_environment_read_fails_the_configured_lint() -> Result<(), String> {
    let workspace = write_policy_workspace(AMBIENT_READ)?;
    let output = run_policy_clippy(workspace.path())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return Err(format!(
            "a direct `std::env::var` must fail the configured lint, but Clippy passed; \
             stderr was:\n{stderr}"
        ));
    }
    for fragment in ["disallowed_methods", "inject an environment reader"] {
        if !stderr.contains(fragment) {
            return Err(format!(
                "the diagnostic must contain {fragment:?}; stderr was:\n{stderr}"
            ));
        }
    }
    Ok(())
}

/// Verifies that a reasoned expectation is the sanctioned escape hatch.
///
/// Clippy reports an unfulfilled `#[expect]`, so this case also proves the lint
/// fires at all: if the configuration were inert, the expectation would be
/// unfulfilled and the build would fail instead of passing.
#[test]
fn a_reasoned_expectation_passes_the_configured_lint() -> Result<(), String> {
    let workspace = write_policy_workspace(EXPECTED_READ)?;
    let output = run_policy_clippy(workspace.path())?;
    if !output.status.success() {
        return Err(format!(
            "a reasoned `#[expect(clippy::disallowed_methods)]` must pass, but Clippy failed; \
             stderr was:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
