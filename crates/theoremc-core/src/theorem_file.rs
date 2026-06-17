//! Filesystem-backed loading for crate-relative `.theorem` files.
//!
//! This module centralizes capability-oriented file access and schema loading
//! for theorem files so proc-macro expansion and any future compile-time
//! tooling share one IO and diagnostic contract.

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};

use crate::schema::{SchemaError, SourceId, TheoremDoc, load_theorem_docs_with_source};

/// Errors raised while loading a crate-relative `.theorem` file.
#[derive(Debug, thiserror::Error)]
pub enum TheoremFileLoadError {
    /// The consumer crate's manifest directory could not be opened.
    #[error(
        "failed to open manifest directory '{path}': {code}",
        code = io_error_code(source.kind())
    )]
    OpenManifestDir {
        /// Manifest-directory path that failed to open.
        path: Utf8PathBuf,
        /// Underlying IO failure.
        #[source]
        source: std::io::Error,
    },

    /// The provided theorem path failed validation because absolute paths,
    /// Windows drive-prefixed paths, and path traversal components (`..`) are
    /// not allowed.
    #[error(
        "invalid theorem path '{path}': absolute, drive-prefixed, and traversal ('..') paths are not allowed"
    )]
    InvalidTheoremPath {
        /// The invalid theorem path.
        path: Utf8PathBuf,
    },

    /// The theorem file could not be read from the manifest directory.
    #[error(
        "failed to read theorem file '{path}': {code}",
        code = io_error_code(source.kind())
    )]
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
/// cannot be opened, [`TheoremFileLoadError::InvalidTheoremPath`] if the
/// theorem path is absolute, drive-prefixed, or attempts to traverse upward,
/// [`TheoremFileLoadError::ReadTheoremFile`] if the theorem file cannot be
/// read, [`TheoremFileLoadError::InvalidTheoremFile`] if schema parsing or
/// validation fails, and [`TheoremFileLoadError::EmptyTheoremFile`] if the
/// file contains zero theorem documents.
///
/// # Examples
///
/// ```no_run
/// use camino::Utf8Path;
/// use theoremc_core::{load_theorem_file_from_manifest_dir, TheoremFileLoadError};
///
/// fn main() -> Result<(), TheoremFileLoadError> {
///     let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
///     let theorem_path = Utf8Path::new("tests/fixtures/valid_full.theorem");
///     let docs = load_theorem_file_from_manifest_dir(manifest_dir, theorem_path)?;
///
///     assert_eq!(docs.len(), 1);
///     Ok(())
/// }
/// ```
pub fn load_theorem_file_from_manifest_dir(
    manifest_dir: &Utf8Path,
    theorem_path: &Utf8Path,
) -> Result<Vec<TheoremDoc>, TheoremFileLoadError> {
    if is_invalid_theorem_path(theorem_path) {
        return Err(TheoremFileLoadError::InvalidTheoremPath {
            path: theorem_path.to_path_buf(),
        });
    }

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

fn has_windows_drive_prefix(path: &Utf8Path) -> bool {
    matches!(
        path.as_str().as_bytes(),
        [drive, b':', ..] if drive.is_ascii_alphabetic()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TheoremPathViolation {
    RootAnchored,
    DrivePrefixed,
    ParentTraversal,
}

fn theorem_path_violation(path: &Utf8Path) -> Option<TheoremPathViolation> {
    if has_windows_drive_prefix(path) {
        return Some(TheoremPathViolation::DrivePrefixed);
    }

    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Utf8Component::Prefix(_) | Utf8Component::RootDir))
    {
        return Some(TheoremPathViolation::RootAnchored);
    }

    if path
        .components()
        .any(|component| matches!(component, Utf8Component::ParentDir))
    {
        return Some(TheoremPathViolation::ParentTraversal);
    }

    None
}

fn is_invalid_theorem_path(path: &Utf8Path) -> bool {
    theorem_path_violation(path).is_some()
}

const fn io_error_code(kind: std::io::ErrorKind) -> &'static str {
    match kind {
        std::io::ErrorKind::NotFound => "io:NotFound",
        std::io::ErrorKind::PermissionDenied => "io:PermissionDenied",
        std::io::ErrorKind::AlreadyExists => "io:AlreadyExists",
        std::io::ErrorKind::WouldBlock => "io:WouldBlock",
        std::io::ErrorKind::InvalidInput => "io:InvalidInput",
        std::io::ErrorKind::InvalidData => "io:InvalidData",
        std::io::ErrorKind::TimedOut => "io:TimedOut",
        std::io::ErrorKind::WriteZero => "io:WriteZero",
        std::io::ErrorKind::Interrupted => "io:Interrupted",
        std::io::ErrorKind::Unsupported => "io:Unsupported",
        std::io::ErrorKind::UnexpectedEof => "io:UnexpectedEof",
        std::io::ErrorKind::OutOfMemory => "io:OutOfMemory",
        _ => "io:Other",
    }
}

#[cfg(test)]
#[path = "theorem_file_tests.rs"]
mod tests;
