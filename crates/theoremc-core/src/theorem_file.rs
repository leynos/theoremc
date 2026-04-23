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

    /// The theorem file could not be read from the manifest directory.
    #[error(
        "invalid theorem path '{path}': theorem paths must be crate-relative and must not contain '..'"
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
    if theorem_path.is_absolute()
        || theorem_path.components().any(|component| {
            matches!(
                component,
                Utf8Component::ParentDir | Utf8Component::Prefix(_)
            )
        })
    {
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
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use tempfile::TempDir;

    #[derive(Debug)]
    struct TempManifestDir {
        _temp_dir: TempDir,
        manifest_dir: Utf8PathBuf,
    }

    #[derive(Debug, Clone, Copy)]
    enum ExpectedErrorKind {
        InvalidTheoremPath,
        ReadTheoremFile,
        EmptyTheoremFile,
        InvalidTheoremFile,
    }

    #[fixture]
    fn temp_manifest_dir() -> TempManifestDir {
        let temp_dir = TempDir::new().expect("should create temp dir");
        let manifest_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .expect("temp dir path is valid UTF-8");
        TempManifestDir {
            _temp_dir: temp_dir,
            manifest_dir,
        }
    }

    fn write_fixture(manifest_dir: &Utf8Path, theorem_path: &Utf8Path, contents: &str) {
        let fixture_root = Utf8Dir::open_ambient_dir(manifest_dir, ambient_authority())
            .expect("should open temp manifest dir");
        if let Some(parent) = theorem_path
            .parent()
            .filter(|parent| !parent.as_str().is_empty())
        {
            fixture_root
                .create_dir_all(parent)
                .expect("should create theorem fixture parent directory");
        }
        fixture_root
            .write(theorem_path.as_str(), contents)
            .expect("should write theorem fixture");
    }

    fn assert_expected_error(
        result: &Result<Vec<TheoremDoc>, TheoremFileLoadError>,
        expected: ExpectedErrorKind,
    ) {
        let matches_expected = match expected {
            ExpectedErrorKind::InvalidTheoremPath => {
                matches!(result, Err(TheoremFileLoadError::InvalidTheoremPath { .. }))
            }
            ExpectedErrorKind::ReadTheoremFile => {
                matches!(result, Err(TheoremFileLoadError::ReadTheoremFile { .. }))
            }
            ExpectedErrorKind::EmptyTheoremFile => {
                matches!(result, Err(TheoremFileLoadError::EmptyTheoremFile { .. }))
            }
            ExpectedErrorKind::InvalidTheoremFile => {
                matches!(result, Err(TheoremFileLoadError::InvalidTheoremFile { .. }))
            }
        };
        assert!(matches_expected, "expected {expected:?}, got {result:?}");
    }

    #[test]
    fn open_manifest_dir_error_when_directory_is_absent() {
        let result = load_theorem_file_from_manifest_dir(
            Utf8Path::new("/this/path/must/not/exist/at/all"),
            Utf8Path::new("theorems/any.theorem"),
        );
        assert!(
            matches!(result, Err(TheoremFileLoadError::OpenManifestDir { .. })),
            "expected OpenManifestDir, got {result:?}",
        );
    }

    #[cfg(windows)]
    #[test]
    fn drive_prefixed_theorem_paths_are_rejected() {
        let temp_manifest_dir = temp_manifest_dir();
        let result = load_theorem_file_from_manifest_dir(
            &temp_manifest_dir.manifest_dir,
            Utf8Path::new("C:foo.theorem"),
        );
        assert!(
            matches!(result, Err(TheoremFileLoadError::InvalidTheoremPath { .. })),
            "expected InvalidTheoremPath, got {result:?}",
        );
    }

    #[rstest]
    #[case("/absolute.theorem", None, ExpectedErrorKind::InvalidTheoremPath)]
    #[case(
        "theorems/../escape.theorem",
        None,
        ExpectedErrorKind::InvalidTheoremPath
    )]
    #[case("no_such_file.theorem", None, ExpectedErrorKind::ReadTheoremFile)]
    #[case("empty.theorem", Some(""), ExpectedErrorKind::EmptyTheoremFile)]
    #[case(
        "invalid.theorem",
        Some(
            "Schema: 1\nTheorem: InvalidAbout\nAbout: \"\"\nProve:\n  - assert: \"true\"\n    because: trivial\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n",
        ),
        ExpectedErrorKind::InvalidTheoremFile
    )]
    fn theorem_file_load_errors_are_reported_consistently(
        temp_manifest_dir: TempManifestDir,
        #[case] theorem_path: &str,
        #[case] file_contents: Option<&str>,
        #[case] expected: ExpectedErrorKind,
    ) {
        let theorem_path = Utf8Path::new(theorem_path);
        if let Some(file_contents) = file_contents {
            write_fixture(&temp_manifest_dir.manifest_dir, theorem_path, file_contents);
        }
        let result =
            load_theorem_file_from_manifest_dir(&temp_manifest_dir.manifest_dir, theorem_path);
        assert_expected_error(&result, expected);
    }

    #[test]
    fn io_error_display_uses_stable_error_codes() {
        let open_error = TheoremFileLoadError::OpenManifestDir {
            path: Utf8PathBuf::from("/missing"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };
        let read_error = TheoremFileLoadError::ReadTheoremFile {
            path: Utf8PathBuf::from("missing.theorem"),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        };

        assert_eq!(
            open_error.to_string(),
            "failed to open manifest directory '/missing': io:PermissionDenied",
        );
        assert_eq!(
            read_error.to_string(),
            "failed to read theorem file 'missing.theorem': io:NotFound",
        );
    }
}
