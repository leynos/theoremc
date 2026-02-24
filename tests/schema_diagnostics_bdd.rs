//! Behavioural tests for structured diagnostics using `rstest-bdd`.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::{SourceId, load_theorem_docs_with_source};

#[expect(
    clippy::manual_let_else,
    reason = "This helper intentionally uses explicit match arms for clarity."
)]
#[expect(
    clippy::option_if_let_else,
    reason = "This helper intentionally uses explicit match arms for clarity."
)]
fn assert_diagnostic_failure(fixture_name: &str, expected_code: &str, failure_type: &str) {
    let source = format!("tests/fixtures/{fixture_name}");
    let yaml = match load_fixture(fixture_name) {
        Ok(yaml) => yaml,
        Err(error) => panic!("failed to load fixture {fixture_name}: {error}"),
    };
    let error = match load_theorem_docs_with_source(&SourceId::new(&source), &yaml) {
        Err(error) => error,
        Ok(_) => panic!("fixture should fail {failure_type}: {fixture_name}"),
    };
    let diagnostic = match error.diagnostic() {
        Some(diagnostic) => diagnostic,
        None => panic!("diagnostic should be present"),
    };

    assert_eq!(diagnostic.code.as_str(), expected_code);
    assert_eq!(diagnostic.location.source, source);
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}

#[given("a parser-invalid theorem fixture")]
fn given_parser_invalid_theorem_fixture() {}

#[then("loading fails with source-located parser diagnostics")]
fn then_loading_fails_with_source_located_parser_diagnostics() {
    assert_diagnostic_failure(
        "invalid_unknown_key.theorem",
        "schema.parse_failure",
        "parsing",
    );
}

#[given("a validator-invalid theorem fixture")]
fn given_validator_invalid_theorem_fixture() {}

#[then("loading fails with source-located validator diagnostics")]
fn then_loading_fails_with_source_located_validator_diagnostics() {
    assert_diagnostic_failure(
        "invalid_empty_about.theorem",
        "schema.validation_failure",
        "validation",
    );
}

#[given("a valid theorem fixture for diagnostics")]
fn given_valid_theorem_fixture_for_diagnostics() {}

#[then("loading succeeds with explicit source")]
fn then_loading_succeeds_with_explicit_source() {
    let source = "tests/fixtures/valid_aliases_and_must.theorem";
    let yaml = match load_fixture("valid_aliases_and_must.theorem") {
        Ok(yaml) => yaml,
        Err(error) => panic!("failed to load fixture: {error}"),
    };
    match load_theorem_docs_with_source(&SourceId::new(source), &yaml) {
        Ok(_) => {}
        Err(error) => panic!("fixture should parse successfully: {error}"),
    }
}

#[scenario(
    path = "tests/features/schema_diagnostics.feature",
    name = "Parser failures include explicit source and location"
)]
fn parser_failures_include_explicit_source_and_location() {}

#[scenario(
    path = "tests/features/schema_diagnostics.feature",
    name = "Validator failures include explicit source and location"
)]
fn validator_failures_include_explicit_source_and_location() {}

#[scenario(
    path = "tests/features/schema_diagnostics.feature",
    name = "Valid fixtures still parse when source is supplied"
)]
fn valid_fixtures_still_parse_when_source_is_supplied() {}
