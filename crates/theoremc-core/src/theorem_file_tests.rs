//! Unit tests for theorem file parsing and helper behaviour.

use super::{
    TheoremDoc, TheoremFileLoadError, TheoremPathViolation, Utf8Dir, Utf8Path, Utf8PathBuf,
    ambient_authority, is_invalid_theorem_path, load_theorem_file_from_manifest_dir,
    theorem_path_violation,
};
use rstest::{fixture, rstest};
use tempfile::TempDir;

mod prop_tests {
    //! Property-based tests for theorem file path validation.

    use super::{Utf8Path, is_invalid_theorem_path};
    use proptest::prelude::{prop_assert, proptest};

    proptest! {
        /// Any path component containing `..` must be rejected.
        #[test]
        fn traversal_paths_are_always_rejected(
            prefix in "[a-z]{1,8}",
            suffix in "[a-z]{1,8}\\.theorem",
        ) {
            let path_str = format!("{prefix}/../{suffix}");
            let path = Utf8Path::new(&path_str);
            prop_assert!(is_invalid_theorem_path(path));
        }

        /// Absolute paths are always rejected regardless of content.
        #[test]
        #[cfg(not(windows))]
        fn absolute_paths_are_always_rejected(segment in "[a-z]{1,8}") {
            let path_str = format!("/{segment}.theorem");
            let path = Utf8Path::new(&path_str);
            prop_assert!(is_invalid_theorem_path(path));
        }

        /// Drive-prefixed paths (Windows style) are always rejected.
        #[test]
        fn drive_prefixed_paths_are_always_rejected(
            drive in "[A-Za-z]",
            name in "[a-z]{1,8}",
        ) {
            let path_str = format!("{drive}:{name}.theorem");
            let path = Utf8Path::new(&path_str);
            prop_assert!(is_invalid_theorem_path(path));
        }

        /// Simple relative paths with no `..` or drive prefix are accepted.
        #[test]
        fn clean_relative_paths_are_accepted(
            dir in "[a-z]{1,8}",
            name in "[a-z]{1,8}",
        ) {
            let path_str = format!("{dir}/{name}.theorem");
            let path = Utf8Path::new(&path_str);
            prop_assert!(!is_invalid_theorem_path(path));
        }
    }
}

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
fn temp_manifest_dir() -> Result<TempManifestDir, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let manifest_dir =
        Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).map_err(|path| {
            std::io::Error::other(format!("non-UTF-8 temp path: {}", path.display()))
        })?;
    Ok(TempManifestDir {
        _temp_dir: temp_dir,
        manifest_dir,
    })
}

fn write_fixture(
    manifest_dir: &Utf8Path,
    theorem_path: &Utf8Path,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_root = Utf8Dir::open_ambient_dir(manifest_dir, ambient_authority())?;
    if let Some(parent) = theorem_path.parent()
        && !parent.as_str().is_empty()
    {
        fixture_root.create_dir_all(parent)?;
    }
    fixture_root.write(theorem_path.as_str(), contents)?;
    Ok(())
}

fn assert_expected_error(
    result: &Result<Vec<TheoremDoc>, TheoremFileLoadError>,
    expected: ExpectedErrorKind,
) -> Result<(), Box<dyn std::error::Error>> {
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
    if matches_expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("expected {expected:?}, got {result:?}",)).into())
    }
}

#[rstest]
#[case::relative_accepted("theorems/accepted.theorem", None)]
#[cfg_attr(
    not(windows),
    case::root_anchored("/absolute.theorem", Some(TheoremPathViolation::RootAnchored))
)]
#[case::drive_prefixed("C:escape.theorem", Some(TheoremPathViolation::DrivePrefixed))]
#[case::drive_prefixed_rooted("C:/escape.theorem", Some(TheoremPathViolation::DrivePrefixed))]
#[case::parent_traversal(
    "theorems/../escape.theorem",
    Some(TheoremPathViolation::ParentTraversal)
)]
fn theorem_path_violation_names_rejected_path_class(
    #[case] theorem_path: &str,
    #[case] expected: Option<TheoremPathViolation>,
) {
    let theorem_path = Utf8Path::new(theorem_path);
    assert_eq!(theorem_path_violation(theorem_path), expected);
    assert_eq!(is_invalid_theorem_path(theorem_path), expected.is_some());
}

#[test]
fn open_manifest_dir_error_when_directory_is_absent() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let missing = temp_dir.path().join("nonexistent");
    let missing = Utf8PathBuf::from_path_buf(missing).map_err(|path| {
        std::io::Error::other(format!("non-UTF-8 temp path: {}", path.display()))
    })?;
    let result =
        load_theorem_file_from_manifest_dir(&missing, Utf8Path::new("theorems/any.theorem"));
    match &result {
        Err(TheoremFileLoadError::OpenManifestDir { .. }) => Ok(()),
        _ => {
            Err(std::io::Error::other(format!("expected OpenManifestDir, got {result:?}",)).into())
        }
    }
}

#[rstest]
#[cfg_attr(
    not(windows),
    case("/absolute.theorem", None, ExpectedErrorKind::InvalidTheoremPath)
)]
#[cfg_attr(
    windows,
    case("C:/absolute.theorem", None, ExpectedErrorKind::InvalidTheoremPath)
)]
#[case(
    "theorems/../escape.theorem",
    None,
    ExpectedErrorKind::InvalidTheoremPath
)]
#[case("C:foo.theorem", None, ExpectedErrorKind::InvalidTheoremPath)]
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
    temp_manifest_dir: Result<TempManifestDir, Box<dyn std::error::Error>>,
    #[case] theorem_path: &str,
    #[case] file_contents: Option<&str>,
    #[case] expected: ExpectedErrorKind,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_manifest_dir = temp_manifest_dir?;
    let theorem_path = Utf8Path::new(theorem_path);
    if let Some(file_contents) = file_contents {
        write_fixture(&temp_manifest_dir.manifest_dir, theorem_path, file_contents)?;
    }
    let result = load_theorem_file_from_manifest_dir(&temp_manifest_dir.manifest_dir, theorem_path);
    assert_expected_error(&result, expected)
}

#[test]
fn backslash_relative_paths_load_after_normalization() -> Result<(), Box<dyn std::error::Error>> {
    let temp_manifest_dir = temp_manifest_dir()?;
    let normalized_path = Utf8Path::new("theorems/nested/valid.theorem");
    write_fixture(
        &temp_manifest_dir.manifest_dir,
        normalized_path,
        concat!(
            "Theorem: BackslashPath\n",
            "About: Loads normalized paths\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: reachable\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        ),
    )?;

    let docs = load_theorem_file_from_manifest_dir(
        &temp_manifest_dir.manifest_dir,
        Utf8Path::new(r"theorems\nested\valid.theorem"),
    )?;

    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].theorem, "BackslashPath");
    Ok(())
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
