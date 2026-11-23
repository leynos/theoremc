//! Direct tests for trybuild fixture staging.
//!
//! The parser is exercised against supplied JSON documents, and staging is
//! exercised against temporary directories, so nothing here runs Cargo or reads
//! the process environment.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use rstest::rstest;

use super::{stage_fixtures, target_directory_from_metadata};

/// Reports mismatched values without panicking inside a fallible test.
fn require_equal<T: std::fmt::Debug + PartialEq + ?Sized>(
    actual: &T,
    expected: &T,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(
            std::io::Error::other(format!("{context}: expected {expected:?}, got {actual:?}"))
                .into(),
        )
    }
}

/// Extracts an error message, retaining unexpected success.
fn require_error<T: std::fmt::Debug>(
    result: Result<T, String>,
    context: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match result {
        Err(message) => Ok(message),
        Ok(value) => Err(std::io::Error::other(format!("{context}: got {value:?}")).into()),
    }
}

#[test]
fn target_directory_is_read_from_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let document = br#"{"target_directory":"/workspace/target","packages":[]}"#;

    let target = target_directory_from_metadata(document)?;

    require_equal(
        &target,
        &Utf8PathBuf::from("/workspace/target"),
        "metadata target directory",
    )?;
    Ok(())
}

#[rstest]
#[case::missing_key(br#"{"packages":[]}"#, "no target_directory")]
#[case::not_a_string(br#"{"target_directory":7}"#, "no target_directory")]
#[case::invalid_json(b"not json", "invalid JSON")]
#[case::not_an_object(b"[]", "no target_directory")]
fn unusable_metadata_is_reported_with_its_cause(
    #[case] document: &[u8],
    #[case] expected_fragment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = require_error(
        target_directory_from_metadata(document),
        "unusable metadata should fail",
    )?;

    if message.contains(expected_fragment) {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "message should mention {expected_fragment:?}, got {message:?}"
        ))
        .into())
    }
}

/// A temporary crate root with a `tests/expand` fixture directory.
struct StagingFixture {
    _temp_dir: tempfile::TempDir,
    crate_root: Utf8PathBuf,
    target: Utf8PathBuf,
    source: Dir,
}

impl StagingFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let root = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| std::io::Error::other("temp dir path is not valid UTF-8"))?
            .to_path_buf();
        let crate_root = root.join("crate");
        let target = root.join("target");
        let source_dir = crate_root.join("tests/expand");
        let source = Dir::create_ambient_dir_all(&source_dir, ambient_authority())
            .and_then(|()| Dir::create_ambient_dir_all(&target, ambient_authority()))
            .and_then(|()| Dir::open_ambient_dir(&source_dir, ambient_authority()))?;
        source.write("valid_theorem.theorem", "theorem: valid\n")?;
        source.write("invalid_theorem.theorem", "theorem: invalid\n")?;
        Ok(Self {
            _temp_dir: temp_dir,
            crate_root,
            target,
            source,
        })
    }
}

#[test]
fn staging_copies_every_named_fixture_into_the_project_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = StagingFixture::new()?;

    stage_fixtures(
        &fixture.crate_root,
        &fixture.target,
        &["valid_theorem.theorem"],
    )?;

    let staged = Dir::open_ambient_dir(
        fixture
            .target
            .join("tests/trybuild/theoremc-macros/tests/expand"),
        ambient_authority(),
    )?;
    require_equal(
        &staged.read_to_string("valid_theorem.theorem")?,
        &"theorem: valid\n".to_owned(),
        "staged fixture contents",
    )?;
    if staged.read_to_string("invalid_theorem.theorem").is_ok() {
        return Err(
            std::io::Error::other("a fixture that was not named must not be staged").into(),
        );
    }
    Ok(())
}

#[test]
fn staging_reports_a_missing_source_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = StagingFixture::new()?;

    let message = require_error(
        stage_fixtures(&fixture.crate_root, &fixture.target, &["absent.theorem"]),
        "an absent source fixture should fail",
    )?;

    if message.contains("absent.theorem") {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("error should name the fixture, got {message:?}")).into())
    }
}

#[test]
fn staging_creates_the_project_directory_when_absent() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = StagingFixture::new()?;
    fixture
        .source
        .write("second.theorem", "theorem: second\n")?;

    stage_fixtures(&fixture.crate_root, &fixture.target, &["second.theorem"])?;

    let staged = fixture
        .target
        .join("tests/trybuild/theoremc-macros/tests/expand/second.theorem");
    if staged.is_file() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("staging should create {staged}")).into())
    }
}
