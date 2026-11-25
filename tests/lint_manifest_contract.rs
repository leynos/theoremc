//! Contract tests for the workspace lint baseline's manifest wiring.
//!
//! `clippy::missing_docs_in_private_items` is denied once, in the root
//! `[workspace.lints.clippy]` table, and reaches a package only when that
//! package's manifest opts in with `[lints] workspace = true`. These tests
//! read the root manifest and every member manifest that `cargo metadata`
//! reports, so a package that drops the opt-in, or a baseline entry that is
//! downgraded or removed, fails here instead of quietly leaving the gate.
//!
//! `tests/lint_workspace_execution.rs` proves the rule then reaches a non-root
//! member's ordinary library target, and `tests/lint_makefile_contract.rs`
//! proves the workspace-wide recipe that carries it still runs.

use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use serde_json::Value;
use std::process::Command;
use test_helpers::toml_section;

/// The root manifest, which declares the workspace baseline.
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// The baseline entry these tests hold in place.
const LINT: &str = "missing_docs_in_private_items";

/// The level the baseline must deny the entry at, as written in TOML.
const DENIED: &str = "\"deny\"";

/// The hybrid root package, which must appear in the discovered set.
///
/// A `cargo metadata` call that silently omitted the root would leave the one
/// package no member manifest can vouch for untested.
const ROOT_PACKAGE: &str = "theoremc";

/// Returns the body of a top-level TOML section, or an empty string.
///
/// An absent section yields an empty string so the value assertions report a
/// missing entry rather than panicking on `None`.
fn section(document: &str, name: &str) -> String {
    toml_section(document, name).unwrap_or_default()
}

/// Returns the configured level of `lint` in a Clippy lint table.
///
/// The value keeps its surrounding quotes, so an assertion compares the TOML
/// literal rather than a substring of it.
fn lint_level<'a>(table: &'a str, lint: &str) -> Option<&'a str> {
    table.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == lint).then_some(value.trim())
    })
}

/// One workspace package as `cargo metadata` reports it.
struct Package {
    /// The package name, used in failure messages.
    name: String,
    /// The absolute path of the package's manifest.
    manifest_path: String,
}

/// Returns every package in the workspace, including the hybrid root.
///
/// `cargo metadata --no-deps` resolves the member list from the repository
/// manifest, so the check follows the real workspace rather than a
/// hand-maintained copy. The call is offline and dependency-free, which keeps
/// it as cheap as the MSRV contract's equivalent.
///
/// # Errors
///
/// Returns the reason when `cargo metadata` cannot run, fails, reports no
/// packages, or emits an entry without a name or manifest path. The standard
/// error is included so a resolver or lockfile failure is visible.
fn workspace_packages() -> Result<Vec<Package>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| format!("running cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata emitted invalid JSON: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| "cargo metadata listed no packages".to_owned())?;
    packages.iter().map(package_entry).collect()
}

/// Returns one package's name and manifest path from metadata.
///
/// # Errors
///
/// Returns the reason when the entry has no name or manifest path.
fn package_entry(package: &Value) -> Result<Package, String> {
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "a package in cargo metadata has no name".to_owned())?
        .to_owned();
    let manifest_path = package
        .get("manifest_path")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("package {name} has no manifest path"))?
        .to_owned();
    Ok(Package {
        name,
        manifest_path,
    })
}

/// Reads a manifest through its parent directory.
///
/// Opening the parent as a capability rather than reading an absolute path
/// keeps the read confined to the directory the metadata call reported.
///
/// # Errors
///
/// Returns the reason when the path has no parent or file name, or when the
/// directory or file cannot be opened.
fn read_manifest(manifest_path: &str) -> Result<String, String> {
    let manifest = Utf8Path::new(manifest_path);
    let parent = manifest
        .parent()
        .ok_or_else(|| format!("{manifest} has no parent directory"))?;
    let file_name = manifest
        .file_name()
        .ok_or_else(|| format!("{manifest} has no file name"))?;
    let directory = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|error| format!("opening {parent}: {error}"))?;
    directory
        .read_to_string(file_name)
        .map_err(|error| format!("reading {manifest}: {error}"))
}

/// Verifies that the workspace baseline denies private-item documentation.
#[test]
fn the_workspace_baseline_denies_missing_docs_in_private_items() {
    let clippy = section(MANIFEST, "workspace.lints.clippy");
    assert_eq!(
        lint_level(&clippy, LINT),
        Some(DENIED),
        "`[workspace.lints.clippy]` must set `{LINT} = \"deny\"`, so a private item without a doc comment fails the lint gate"
    );
}

/// Verifies that every workspace package inherits the shared lint baseline.
///
/// Cargo applies `[workspace.lints]` only to a package whose manifest opts in,
/// so a missing `[lints] workspace = true` leaves that package outside the
/// private-documentation rule even though the root table still denies it. The
/// hybrid root is checked like every member.
#[test]
fn every_workspace_package_opts_into_the_workspace_lints() -> Result<(), String> {
    let packages = workspace_packages()?;
    if packages.is_empty() {
        return Err("cargo metadata must report at least one workspace package".to_owned());
    }
    if !packages.iter().any(|package| package.name == ROOT_PACKAGE) {
        return Err(format!(
            "cargo metadata must include the hybrid root package `{ROOT_PACKAGE}`"
        ));
    }
    for package in packages {
        let source = read_manifest(&package.manifest_path)?;
        let lints = section(&source, "lints");
        let opted_in = lints.lines().any(|line| line.trim() == "workspace = true");
        if !opted_in {
            return Err(format!(
                "{} must inherit the workspace lint baseline with `[lints] workspace = true`; found {lints:?}",
                package.name
            ));
        }
    }
    Ok(())
}
