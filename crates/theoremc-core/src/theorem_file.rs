//! Filesystem-backed loading for crate-relative `.theorem` files.
//!
//! This module centralizes capability-oriented file access and schema loading
//! for theorem files so proc-macro expansion and any future compile-time
//! tooling share one IO and diagnostic contract.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};

use crate::schema::{SchemaError, SourceId, TheoremDoc, load_theorem_docs_with_source};

/// Errors raised while loading a crate-relative `.theorem` file.
#[derive(Debug, thiserror::Error)]
pub enum TheoremFileLoadError {
    /// The consumer crate's manifest directory could not be opened.
    #[error("failed to open manifest directory '{path}': {source}")]
    OpenManifestDir {
        /// Manifest-directory path that failed to open.
        path: Utf8PathBuf,
        /// Underlying IO failure.
        #[source]
        source: std::io::Error,
    },

    /// The theorem file could not be read from the manifest directory.
    #[error("failed to read theorem file '{path}': {source}")]
    ReadTheoremFile {
        /// Crate-relative theorem path that could not be read.
        path: Utf8PathBuf,
        /// Underlying IO failure.
        #[source]
        source: std::io::Error,
    },

    /// The theorem file parsed successfully but did not contain any YAML
    /// documents.
    #[error("theorem file '{path}' does not contain any theorem documents")]
    EmptyTheoremFile {
        /// Crate-relative theorem path that loaded zero documents.
        path: Utf8PathBuf,
    },

    /// The theorem file failed schema parsing or validation.
    #[error("failed to load theorem file '{path}': {source}")]
    InvalidTheoremFile {
        /// Crate-relative theorem path whose contents were invalid.
        path: Utf8PathBuf,
        /// Underlying schema-loading failure.
        #[source]
        source: Box<SchemaError>,
    },
}

/// Loads one or more theorem documents from a crate-relative theorem file.
///
/// The theorem path is resolved relative to `manifest_dir`, read through
/// `cap_std`, and then validated with the shared schema loader. Successful
/// loads must contain at least one theorem document.
///
/// # Errors
///
/// Returns [`TheoremFileLoadError::OpenManifestDir`] if the manifest directory
/// cannot be opened, [`TheoremFileLoadError::ReadTheoremFile`] if the theorem
/// file cannot be read, [`TheoremFileLoadError::InvalidTheoremFile`] if schema
/// parsing or validation fails, and [`TheoremFileLoadError::EmptyTheoremFile`]
/// if the file contains zero theorem documents.
///
/// # Examples
///
/// ```no_run
/// use camino::Utf8Path;
/// use theoremc_core::load_theorem_file_from_manifest_dir;
///
/// let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
/// let theorem_path = Utf8Path::new("tests/fixtures/valid_full.theorem");
/// let docs = load_theorem_file_from_manifest_dir(manifest_dir, theorem_path).unwrap();
///
/// assert_eq!(docs.len(), 1);
/// ```
pub fn load_theorem_file_from_manifest_dir(
    manifest_dir: &Utf8Path,
    theorem_path: &Utf8Path,
) -> Result<Vec<TheoremDoc>, TheoremFileLoadError> {
    let manifest_root =
        Utf8Dir::open_ambient_dir(manifest_dir, ambient_authority()).map_err(|source| {
            TheoremFileLoadError::OpenManifestDir {
                path: manifest_dir.to_path_buf(),
                source,
            }
        })?;
    let theorem_source = manifest_root
        .read_to_string(theorem_path)
        .map_err(|source| TheoremFileLoadError::ReadTheoremFile {
            path: theorem_path.to_path_buf(),
            source,
        })?;
    let theorem_docs =
        load_theorem_docs_with_source(&SourceId::new(theorem_path.as_str()), &theorem_source)
            .map_err(|source| TheoremFileLoadError::InvalidTheoremFile {
                path: theorem_path.to_path_buf(),
                source: Box::new(source),
            })?;

    if theorem_docs.is_empty() {
        return Err(TheoremFileLoadError::EmptyTheoremFile {
            path: theorem_path.to_path_buf(),
        });
    }

    Ok(theorem_docs)
}
