//! Snapshot tests for structured parser and validator diagnostics.

mod common;

use common::load_fixture;
use theoremc::schema::load_theorem_docs_with_source;

fn render_diagnostic_for_fixture(fixture_name: &str) -> String {
    let source = format!("tests/fixtures/{fixture_name}");
    let yaml = load_fixture(fixture_name);
    let Err(error) = load_theorem_docs_with_source(&source, &yaml) else {
        panic!("fixture should fail");
    };
    let Some(diagnostic) = error.diagnostic() else {
        panic!("schema failures should carry structured diagnostics");
    };
    diagnostic.render()
}

#[test]
fn parser_unknown_key_diagnostic_snapshot() {
    let actual = render_diagnostic_for_fixture("invalid_unknown_key.theorem");
    let expected = include_str!("snapshots/diagnostics/parser_unknown_key.snap").trim_end();
    assert_eq!(actual, expected);
}

#[test]
fn validation_empty_about_diagnostic_snapshot() {
    let actual = render_diagnostic_for_fixture("invalid_empty_about.theorem");
    let expected = include_str!("snapshots/diagnostics/validation_empty_about.snap").trim_end();
    assert_eq!(actual, expected);
}

#[test]
fn validation_second_assert_diagnostic_snapshot() {
    let actual = render_diagnostic_for_fixture("invalid_second_empty_assert.theorem");
    let expected = include_str!("snapshots/diagnostics/validation_second_assert.snap").trim_end();
    assert_eq!(actual, expected);
}

#[test]
fn validation_missing_witness_diagnostic_snapshot() {
    let actual = render_diagnostic_for_fixture("invalid_missing_witness_default.theorem");
    let expected = include_str!("snapshots/diagnostics/validation_missing_witness.snap").trim_end();
    assert_eq!(actual, expected);
}
