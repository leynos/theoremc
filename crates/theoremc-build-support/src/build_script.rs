//! Cargo build-script orchestration for theorem discovery and suite generation.

use std::{env, io};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::fs_utf8::Dir as Utf8Dir;
use thiserror::Error;

use crate::{BuildDiscoveryError, BuildSuiteError, discover_theorem_inputs, write_theorem_suite};

/// Cargo metadata that the root build-script adapter must emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildScriptOutput {
    /// Whether the generated suite contains at least one theorem.
    has_theorems: bool,
    /// Paths emitted as Cargo rerun-if-changed directives.
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
    /// use theoremc_build_support::{BuildScriptContext, prepare_build_script};
    ///
    /// let context = BuildScriptContext::from_cargo_env()?;
    /// let output = prepare_build_script(&context)?;
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
    /// use theoremc_build_support::{BuildScriptContext, prepare_build_script};
    ///
    /// let context = BuildScriptContext::from_cargo_env()?;
    /// let output = prepare_build_script(&context)?;
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

/// The two Cargo-provided paths that configure build-script preparation.
///
/// Cargo supplies exactly two values a build script cannot otherwise learn:
/// the manifest directory of the crate being built, and the output directory
/// the script must write into. Capturing them once as a context lets discovery
/// and suite generation stay pure functions of explicit input, so every test
/// supplies its own temporary directories and never touches the process
/// environment.
///
/// # Examples
///
/// A test can build a context from temporary directories:
///
/// ```
/// use camino::Utf8PathBuf;
/// use theoremc_build_support::BuildScriptContext;
///
/// let context = BuildScriptContext::new(
///     Utf8PathBuf::from("/workspace/crate"),
///     Utf8PathBuf::from("/workspace/target/out"),
/// );
/// assert_eq!(context.manifest_dir().as_str(), "/workspace/crate");
/// assert_eq!(context.out_dir().as_str(), "/workspace/target/out");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildScriptContext {
    /// Manifest directory of the crate whose theorem tree is discovered.
    manifest_dir: Utf8PathBuf,
    /// Output directory Cargo expects the generated suite to be written into.
    out_dir: Utf8PathBuf,
}

impl BuildScriptContext {
    /// Creates a context from explicit manifest and output directories.
    ///
    /// # Examples
    ///
    /// ```
    /// use camino::Utf8PathBuf;
    /// use theoremc_build_support::BuildScriptContext;
    ///
    /// let context = BuildScriptContext::new(
    ///     Utf8PathBuf::from("crate"),
    ///     Utf8PathBuf::from("target/out"),
    /// );
    /// assert_eq!(context.manifest_dir().as_str(), "crate");
    /// ```
    #[must_use]
    pub const fn new(manifest_dir: Utf8PathBuf, out_dir: Utf8PathBuf) -> Self {
        Self {
            manifest_dir,
            out_dir,
        }
    }

    /// Returns the manifest directory used for theorem discovery.
    ///
    /// # Examples
    ///
    /// ```
    /// use camino::Utf8PathBuf;
    /// use theoremc_build_support::BuildScriptContext;
    ///
    /// let context = BuildScriptContext::new(
    ///     Utf8PathBuf::from("crate"),
    ///     Utf8PathBuf::from("target/out"),
    /// );
    /// assert_eq!(context.manifest_dir().as_str(), "crate");
    /// ```
    #[must_use]
    pub fn manifest_dir(&self) -> &Utf8Path {
        &self.manifest_dir
    }

    /// Returns the output directory the generated suite is written into.
    ///
    /// # Examples
    ///
    /// ```
    /// use camino::Utf8PathBuf;
    /// use theoremc_build_support::BuildScriptContext;
    ///
    /// let context = BuildScriptContext::new(
    ///     Utf8PathBuf::from("crate"),
    ///     Utf8PathBuf::from("target/out"),
    /// );
    /// assert_eq!(context.out_dir().as_str(), "target/out");
    /// ```
    #[must_use]
    pub fn out_dir(&self) -> &Utf8Path {
        &self.out_dir
    }

    /// Captures Cargo's manifest and output directories from the environment.
    ///
    /// This is the only place in the crate that reads the process environment,
    /// so it is the crate's composition root. Everything downstream receives
    /// the captured paths explicitly.
    ///
    /// # Examples
    ///
    /// A build script captures the context and hands it to preparation:
    ///
    /// ```no_run
    /// # fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
    /// use theoremc_build_support::{BuildScriptContext, prepare_build_script};
    ///
    /// let context = BuildScriptContext::from_cargo_env()?;
    /// let output = prepare_build_script(&context)?;
    /// # let _ = output;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`BuildScriptError::Environment`] when Cargo does not export
    /// `CARGO_MANIFEST_DIR` or `OUT_DIR`, including when either is not valid
    /// Unicode.
    #[expect(
        clippy::disallowed_methods,
        reason = "a Cargo build script can only learn its crate root and output directory from Cargo's environment, so this single reader is the boundary seam injected into the pure preparation step"
    )]
    pub fn from_cargo_env() -> Result<Self, BuildScriptError> {
        Self::from_lookup(env::var)
    }

    /// Builds a context through an injected variable reader.
    ///
    /// `CARGO_MANIFEST_DIR` is read before `OUT_DIR`, and the first failure
    /// stops the lookup so an error always names the variable that actually
    /// failed. A stub reader lets tests exercise this mapping without changing
    /// process state.
    fn from_lookup(
        mut read_cargo_variable: impl FnMut(&'static str) -> Result<String, env::VarError>,
    ) -> Result<Self, BuildScriptError> {
        let manifest_dir = required_cargo_path(MANIFEST_DIR_VARIABLE, &mut read_cargo_variable)?;
        let out_dir = required_cargo_path(OUT_DIR_VARIABLE, &mut read_cargo_variable)?;
        Ok(Self {
            manifest_dir,
            out_dir,
        })
    }
}

/// Cargo's variable holding the manifest directory of the crate being built.
const MANIFEST_DIR_VARIABLE: &str = "CARGO_MANIFEST_DIR";

/// Cargo's variable holding the build-script output directory.
const OUT_DIR_VARIABLE: &str = "OUT_DIR";

/// Prepares theorem discovery, generated-suite output, and Cargo rerun metadata.
///
/// # Examples
///
/// A root `build.rs` can capture Cargo's context and delegate preparation,
/// then emit the Cargo metadata it owns:
///
/// ```no_run
/// fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
///     let context = theoremc_build_support::BuildScriptContext::from_cargo_env()?;
///     let output = theoremc_build_support::prepare_build_script(&context)?;
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
/// Returns [`BuildScriptError`] when the theorem tree cannot be inspected or
/// the generated suite cannot be written.
pub fn prepare_build_script(
    context: &BuildScriptContext,
) -> Result<BuildScriptOutput, BuildScriptError> {
    let discovery = discover_theorem_inputs(context.manifest_dir())?;
    let out_dir = Utf8Dir::open_ambient_dir(context.out_dir(), cap_std::ambient_authority())
        .map_err(|source| BuildScriptError::OpenOutputDirectory {
            path: context.out_dir().to_path_buf(),
            source,
        })?;

    write_theorem_suite(&out_dir, &discovery)?;

    Ok(BuildScriptOutput {
        has_theorems: discovery.theorem_files().next().is_some(),
        rerun_paths: discovery.rerun_paths().map(ToOwned::to_owned).collect(),
    })
}

/// Converts one required Cargo environment variable into a UTF-8 path.
fn required_cargo_path(
    variable: &'static str,
    read_cargo_variable: &mut impl FnMut(&'static str) -> Result<String, env::VarError>,
) -> Result<Utf8PathBuf, BuildScriptError> {
    read_cargo_variable(variable)
        .map(Utf8PathBuf::from)
        .map_err(|source| BuildScriptError::Environment { variable, source })
}

#[cfg(test)]
#[path = "build_script_tests.rs"]
mod tests;
