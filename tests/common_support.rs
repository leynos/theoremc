//! Unit tests for shared integration-test support helpers.

pub mod common;

use common::toml_section;
use rstest::rstest;

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
