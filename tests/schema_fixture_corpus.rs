//! Regression corpus tests for parser and validator fixtures.

mod common;

use common::load_fixture;
use rstest::rstest;
use theoremc::schema::load_theorem_docs_with_source;

fn fixture_source(fixture_name: &str) -> String {
    format!("tests/fixtures/{fixture_name}")
}

fn load_from_fixture(fixture_name: &str) -> Result<(), String> {
    let source = fixture_source(fixture_name);
    let yaml = load_fixture(fixture_name);
    load_theorem_docs_with_source(&source, &yaml)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[rstest]
#[case::minimal("valid_minimal.theorem")]
#[case::full("valid_full.theorem")]
#[case::lower_alias_and_must("valid_aliases_and_must.theorem")]
#[case::nested_maybe("valid_nested_maybe.theorem")]
#[case::vacuous_policy("valid_vacuous.theorem")]
fn valid_fixture_corpus_parses(#[case] fixture_name: &str) {
    let result = load_from_fixture(fixture_name);
    assert!(
        result.is_ok(),
        "expected {fixture_name} to parse, got: {:?}",
        result.err()
    );
}

#[rstest]
#[case::unknown_key("invalid_unknown_key.theorem")]
#[case::nested_maybe_blank_because("invalid_nested_maybe_empty_because.theorem")]
#[case::must_empty_action("invalid_must_empty_action.theorem")]
#[case::missing_witness_default("invalid_missing_witness_default.theorem")]
fn invalid_fixture_corpus_fails_with_diagnostic_source(#[case] fixture_name: &str) {
    let source = fixture_source(fixture_name);
    let yaml = load_fixture(fixture_name);
    let result = load_theorem_docs_with_source(&source, &yaml);
    assert!(result.is_err(), "expected {fixture_name} to fail");

    let Err(error) = result else {
        panic!("error should be present");
    };
    let Some(diagnostic) = error.diagnostic() else {
        panic!("diagnostic should be present");
    };
    assert_eq!(diagnostic.location.source, source);
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}
