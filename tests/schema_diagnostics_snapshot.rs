//! Snapshot tests for structured parser and validator diagnostics.

mod common;

use common::{fixture_source_id, load_fixture};
use rstest::rstest;
use theoremc::schema::load_theorem_docs_with_source;

fn render_diagnostic_for_fixture(fixture_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let yaml = load_fixture(fixture_name)?;
    let error = load_theorem_docs_with_source(&fixture_source_id(fixture_name), &yaml)
        .err()
        .ok_or_else(|| std::io::Error::other("fixture should fail"))?;
    let diagnostic = error.diagnostic().ok_or_else(|| {
        std::io::Error::other("schema failures should carry structured diagnostics")
    })?;
    Ok(diagnostic.render())
}

#[rstest]
#[case(
    "invalid_unknown_key.theorem",
    include_str!("snapshots/diagnostics/parser_unknown_key.snap")
)]
#[case(
    "invalid_empty_about.theorem",
    include_str!("snapshots/diagnostics/validation_empty_about.snap")
)]
#[case(
    "invalid_second_empty_assert.theorem",
    include_str!("snapshots/diagnostics/validation_second_assert.snap")
)]
#[case(
    "invalid_missing_witness_default.theorem",
    include_str!("snapshots/diagnostics/validation_missing_witness.snap")
)]
fn schema_diagnostic_snapshot_matches(#[case] fixture_name: &str, #[case] expected_snapshot: &str) {
    let actual = render_diagnostic_for_fixture(fixture_name)
        .expect("fixture should fail with a structured schema diagnostic");
    let expected = expected_snapshot.trim_end();
    assert_eq!(actual, expected);
}
