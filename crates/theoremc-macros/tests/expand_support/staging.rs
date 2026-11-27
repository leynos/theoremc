//! trybuild fixture staging for the `theorem_file!` proc-macro.
//!
//! trybuild generates its test project under
//! `<target_directory>/tests/trybuild/<crate-name>/`, and that generated Cargo
//! project is the manifest directory its child compiler runs in. A fixture that
//! calls `theorem_file!("tests/expand/name.theorem")` therefore reads the
//! theorem document from the staged project, not from this crate. These helpers
//! copy each `.theorem` document to where the expanded macro will look.
//!
//! The target directory comes from `cargo metadata`, which is the same source
//! trybuild itself uses, and is passed in explicitly. Staging never reads the
//! process environment and never mutates process state.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};

/// The package whose source tree drives the metadata lookup.
const PACKAGE: &str = "theoremc-macros";

/// The directory inside the generated trybuild project matching this crate.
const PROJECT_FIXTURE_DIR: &str = "tests/expand";

/// Resolves the target directory the trybuild child Cargo will build into.
///
/// Cargo reports one `target_directory` for the workspace. Running the same
/// `cargo metadata` invocation trybuild runs, from the same working directory
/// with the same environment, yields the value trybuild will use, so a staged
/// fixture and the compiler agree on one location.
///
/// # Errors
///
/// Returns a message naming the failed command, the exit status, or the JSON
/// problem when Cargo cannot run or reports no usable target directory.
pub fn target_directory(crate_root: &Utf8Path) -> Result<Utf8PathBuf, String> {
    // `CARGO` is baked in at compile time; reading it there keeps this function
    // free of a runtime process-environment read.
    let cargo = env!("CARGO");
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(crate_root)
        .output()
        .map_err(|error| format!("running cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed with {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    target_directory_from_metadata(&output.stdout)
}

/// Reads `target_directory` from a `cargo metadata` JSON document.
///
/// Split from the process boundary so the parser can be exercised against
/// supplied documents without running Cargo.
///
/// # Errors
///
/// Returns a message when the document is not JSON, is not an object, or has no
/// UTF-8 `target_directory` string.
pub fn target_directory_from_metadata(document: &[u8]) -> Result<Utf8PathBuf, String> {
    let metadata: serde_json::Value = serde_json::from_slice(document)
        .map_err(|error| format!("cargo metadata emitted invalid JSON: {error}"))?;
    metadata
        .get("target_directory")
        .and_then(serde_json::Value::as_str)
        .map(Utf8PathBuf::from)
        .ok_or_else(|| "cargo metadata reported no target_directory".to_owned())
}

/// Copies every named fixture document into the trybuild project directory.
///
/// # Errors
///
/// Returns a message naming the fixture when a source is missing, or naming the
/// destination when a directory or file cannot be written.
pub fn stage_fixtures(
    crate_root: &Utf8Path,
    target: &Utf8Path,
    fixtures: &[&str],
) -> Result<(), String> {
    let destination = project_fixture_dir();
    let source_root =
        Utf8Dir::open_ambient_dir(crate_root.join(PROJECT_FIXTURE_DIR), ambient_authority())
            .map_err(|error| format!("opening the fixture source directory: {error}"))?;
    // The target directory may not exist yet: a caller can pass an explicit
    // `--target-dir` the outer build has not created, and `cargo metadata`
    // then reports the workspace default instead. Creating it first keeps
    // staging independent of whether the outer build has written there, and
    // `create_dir_all` succeeds on a directory that already exists.
    Utf8Dir::create_ambient_dir_all(target, ambient_authority())
        .map_err(|error| format!("creating the target directory {target}: {error}"))?;
    let target_root = Utf8Dir::open_ambient_dir(target, ambient_authority())
        .map_err(|error| format!("opening the target directory {target}: {error}"))?;
    target_root
        .create_dir_all(&destination)
        .map_err(|error| format!("creating {destination} under {target}: {error}"))?;
    let destination_root = target_root
        .open_dir(&destination)
        .map_err(|error| format!("opening {destination} under {target}: {error}"))?;

    for name in fixtures {
        source_root
            .copy(name, &destination_root, name)
            .map_err(|error| format!("staging fixture {name}: {error}"))?;
    }
    Ok(())
}

/// Returns the trybuild project path for this package, relative to `target`.
///
/// trybuild builds `<target>/tests/trybuild/<crate-name>/`; the fixture
/// documents live under the package's `tests/expand` directory inside it.
fn project_fixture_dir() -> Utf8PathBuf {
    Utf8Path::new("tests")
        .join("trybuild")
        .join(PACKAGE)
        .join(PROJECT_FIXTURE_DIR)
}

#[cfg(test)]
#[path = "staging_tests.rs"]
mod tests;
