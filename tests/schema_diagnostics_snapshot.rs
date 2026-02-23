//! Snapshot tests for structured parser and validator diagnostics.

mod common;

use common::load_fixture;
use rstest::rstest;
use theoremc::schema::{SourceId, load_theorem_docs_with_source};

fn render_diagnostic_for_fixture(fixture_name: &str) -> String {
    let source = format!("tests/fixtures/{fixture_name}");
    let yaml = load_fixture(fixture_name)
        .unwrap_or_else(|error| panic!("failed to load fixture: {error}"));
    let Err(error) = load_theorem_docs_with_source(&SourceId::new(&source), &yaml) else {
        panic!("fixture should fail");
    };
    let Some(diagnostic) = error.diagnostic() else {
        panic!("schema failures should carry structured diagnostics");
    };
    diagnostic.render()
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
    let actual = render_diagnostic_for_fixture(fixture_name);
    let expected = expected_snapshot.trim_end();
    assert_eq!(actual, expected);
}
