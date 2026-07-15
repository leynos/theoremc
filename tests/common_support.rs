//! Unit tests for shared integration-test support helpers.

use std::time::SystemTime;

use camino::Utf8Path;
use rstest::rstest;
use test_helpers::{FixtureCrate, toml_section};

const MINIMAL_CARGO_TOML: &str = concat!(
    "[package]\n",
    "name = \"common_support_fixture\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2024\"\n",
);
const MINIMAL_LIB_RS: &str = "//! Fixture crate for shared support tests.\n";

#[rstest]
#[case::preserves_comments(
    concat!(
        "[package]\n",
        "name = \"demo\"\n",
        "\n",
        "[build-dependencies]\n",
        "# copied comment\n",
        "camino = \"1.2.2\"\n",
        "\n",
        "[dependencies]\n",
        "ignored = \"1.0.0\"\n",
    ),
    "build-dependencies",
    Some("# copied comment\ncamino = \"1.2.2\"\n\n")
)]
#[case::stops_before_neighbouring_section(
    concat!(
        "[build-dependencies]\n",
        "camino = \"1.2.2\"\n",
        "cap-std = \"4.0.2\"\n",
        "[dependencies]\n",
        "ignored = \"1.0.0\"\n",
    ),
    "build-dependencies",
    Some("camino = \"1.2.2\"\ncap-std = \"4.0.2\"\n")
)]
#[case::returns_none_for_missing_section(
    concat!(
        "[package]\n",
        "name = \"demo\"\n",
        "\n",
        "[dependencies]\n",
        "ignored = \"1.0.0\"\n",
    ),
    "build-dependencies",
    None
)]
fn toml_section_extracts_expected_section_body(
    #[case] document: &str,
    #[case] section_name: &str,
    #[case] expected: Option<&str>,
) {
    assert_eq!(toml_section(document, section_name).as_deref(), expected);
}

#[test]
fn write_with_advanced_mtime_marks_created_file_and_parent_newer() -> Result<(), String> {
    let fixture = FixtureCrate::new(MINIMAL_CARGO_TOML, MINIMAL_LIB_RS)?;
    let write_started = SystemTime::now();

    fixture.write_with_advanced_mtime(Utf8Path::new("theorems/created.theorem"), "created")?;

    let file_mtime = fixture.modified_time(Utf8Path::new("theorems/created.theorem"))?;
    let parent_mtime = fixture.modified_time(Utf8Path::new("theorems"))?;
    ensure_later(file_mtime, write_started, "created theorem file")?;
    ensure_later(
        parent_mtime,
        write_started,
        "created theorem parent directory",
    )?;
    Ok(())
}

#[test]
fn overwrite_in_place_with_advanced_mtime_marks_file_newer() -> Result<(), String> {
    let fixture = FixtureCrate::new(MINIMAL_CARGO_TOML, MINIMAL_LIB_RS)?;
    let theorem_path = Utf8Path::new("theorems/existing.theorem");
    fixture.write(theorem_path, "before")?;
    let previous_mtime = fixture.modified_time(theorem_path)?;
    let previous_parent_mtime = fixture.modified_time(Utf8Path::new("theorems"))?;

    fixture.overwrite_in_place_with_advanced_mtime(theorem_path, "after")?;

    let advanced_mtime = fixture.modified_time(theorem_path)?;
    let advanced_parent_mtime = fixture.modified_time(Utf8Path::new("theorems"))?;
    ensure_later(advanced_mtime, previous_mtime, "overwritten theorem file")?;
    ensure_later(
        advanced_parent_mtime,
        previous_parent_mtime,
        "overwritten theorem parent directory",
    )?;
    Ok(())
}

#[test]
fn overwrite_in_place_rejects_missing_files() -> Result<(), String> {
    let fixture = FixtureCrate::new(MINIMAL_CARGO_TOML, "//! fixture\n")?;
    fixture.create_dir(Utf8Path::new("theorems"))?;

    let result = fixture.overwrite_in_place(Utf8Path::new("theorems/missing.theorem"), "after");

    match result {
        Ok(()) => Err("missing fixture file was created".to_owned()),
        Err(_) => Ok(()),
    }
}

fn ensure_later(actual: SystemTime, previous: SystemTime, label: &str) -> Result<(), String> {
    if actual > previous {
        Ok(())
    } else {
        Err(format!(
            "{label} mtime should advance; previous: {previous:?}, actual: {actual:?}",
        ))
    }
}
