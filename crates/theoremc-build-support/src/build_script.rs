//! Cargo build-script orchestration for theorem discovery and suite generation.

use std::{env, io};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::fs_utf8::Dir as Utf8Dir;
use thiserror::Error;

use crate::{BuildDiscoveryError, BuildSuiteError, discover_theorem_inputs, write_theorem_suite};

/// Cargo metadata that the root build-script adapter must emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildScriptOutput {
    has_theorems: bool,
    rerun_paths: Vec<Utf8PathBuf>,
}

impl BuildScriptOutput {
    /// Returns whether the generated suite contains at least one theorem.
    ///
    /// # Examples
    ///
    /// A build script can enable the generated-suite lint configuration only
    /// when theorem files are present:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
    /// let output = theoremc_build_support::prepare_build_script()?;
    /// if output.has_theorems() {
    ///     println!("cargo::rustc-cfg=theoremc_has_theorems");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn has_theorems(&self) -> bool {
        self.has_theorems
    }

    /// Returns the crate-relative paths that Cargo must watch for changes.
    ///
    /// # Examples
    ///
    /// A build script emits one Cargo rerun directive for each returned path:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
    /// let output = theoremc_build_support::prepare_build_script()?;
    /// for path in output.rerun_paths() {
    ///     println!("cargo::rerun-if-changed={path}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn rerun_paths(&self) -> impl Iterator<Item = &Utf8PathBuf> {
        self.rerun_paths.iter()
    }
}

/// Errors returned while configuring theoremc's Cargo build script.
#[derive(Debug, Error)]
pub enum BuildScriptError {
    /// Cargo did not provide a required build-script environment variable.
    #[error("{variable} is not set: {source}")]
    Environment {
        /// The required Cargo environment variable.
        variable: &'static str,
        /// The environment lookup failure.
        #[source]
        source: env::VarError,
    },
    /// Cargo's generated output directory could not be opened.
    #[error("failed to open OUT_DIR '{path}': {source}")]
    OpenOutputDirectory {
        /// The output-directory path supplied by Cargo.
        path: Utf8PathBuf,
        /// The filesystem failure.
        #[source]
        source: io::Error,
    },
    /// The theorem tree could not be discovered.
    #[error(transparent)]
    Discovery(#[from] BuildDiscoveryError),
    /// The generated theorem suite could not be written.
    #[error(transparent)]
    Suite(#[from] BuildSuiteError),
}

/// Prepares theorem discovery, generated-suite output, and Cargo rerun metadata.
///
/// # Examples
///
/// A root `build.rs` can delegate preparation and emit the Cargo metadata it
/// owns:
///
/// ```no_run
/// fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
///     let output = theoremc_build_support::prepare_build_script()?;
///     if output.has_theorems() {
///         println!("cargo::rustc-cfg=theoremc_has_theorems");
///     }
///     for path in output.rerun_paths() {
///         println!("cargo::rerun-if-changed={path}");
///     }
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns [`BuildScriptError`] when Cargo's environment is incomplete, the
/// theorem tree cannot be inspected, or the generated suite cannot be written.
pub fn prepare_build_script() -> Result<BuildScriptOutput, BuildScriptError> {
    prepare_build_script_with_env(env::var)
}

fn prepare_build_script_with_env(
    read_cargo_variable: impl Fn(&'static str) -> Result<String, env::VarError>,
) -> Result<BuildScriptOutput, BuildScriptError> {
    let manifest_dir = required_cargo_path("CARGO_MANIFEST_DIR", &read_cargo_variable)?;
    let out_dir_path = required_cargo_path("OUT_DIR", &read_cargo_variable)?;
    prepare_build_script_at(&manifest_dir, &out_dir_path)
}

fn prepare_build_script_at(
    manifest_dir: &Utf8Path,
    out_dir_path: &Utf8Path,
) -> Result<BuildScriptOutput, BuildScriptError> {
    let discovery = discover_theorem_inputs(manifest_dir)?;
    let out_dir = Utf8Dir::open_ambient_dir(out_dir_path, cap_std::ambient_authority()).map_err(
        |source| BuildScriptError::OpenOutputDirectory {
            path: out_dir_path.to_path_buf(),
            source,
        },
    )?;

    write_theorem_suite(&out_dir, &discovery)?;

    Ok(BuildScriptOutput {
        has_theorems: discovery.theorem_files().next().is_some(),
        rerun_paths: discovery.rerun_paths().map(ToOwned::to_owned).collect(),
    })
}

fn required_cargo_path(
    variable: &'static str,
    read_cargo_variable: &impl Fn(&'static str) -> Result<String, env::VarError>,
) -> Result<Utf8PathBuf, BuildScriptError> {
    read_cargo_variable(variable)
        .map(Utf8PathBuf::from)
        .map_err(|source| BuildScriptError::Environment { variable, source })
}

#[cfg(test)]
#[path = "build_script_tests.rs"]
mod tests;
