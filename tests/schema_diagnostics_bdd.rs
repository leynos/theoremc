//! Behavioural tests for structured diagnostics using `rstest-bdd`.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::load_theorem_docs_with_source;

#[given("a parser-invalid theorem fixture")]
fn given_parser_invalid_theorem_fixture() {}

#[then("loading fails with source-located parser diagnostics")]
fn then_loading_fails_with_source_located_parser_diagnostics() {
    let source = "tests/fixtures/invalid_unknown_key.theorem";
    let yaml = load_fixture("invalid_unknown_key.theorem");
    let result = load_theorem_docs_with_source(source, &yaml);
    assert!(result.is_err(), "fixture should fail parsing");

    let Err(error) = result else {
        panic!("error should be present");
    };
    let Some(diagnostic) = error.diagnostic() else {
        panic!("diagnostic should be present");
    };
    assert_eq!(diagnostic.code.as_str(), "schema.parse_failure");
    assert_eq!(diagnostic.location.source, source);
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}

#[given("a validator-invalid theorem fixture")]
fn given_validator_invalid_theorem_fixture() {}

#[then("loading fails with source-located validator diagnostics")]
fn then_loading_fails_with_source_located_validator_diagnostics() {
    let source = "tests/fixtures/invalid_empty_about.theorem";
    let yaml = load_fixture("invalid_empty_about.theorem");
    let result = load_theorem_docs_with_source(source, &yaml);
    assert!(result.is_err(), "fixture should fail validation");

    let Err(error) = result else {
        panic!("error should be present");
    };
    let Some(diagnostic) = error.diagnostic() else {
        panic!("diagnostic should be present");
    };
    assert_eq!(diagnostic.code.as_str(), "schema.validation_failure");
    assert_eq!(diagnostic.location.source, source);
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}

#[given("a valid theorem fixture for diagnostics")]
fn given_valid_theorem_fixture_for_diagnostics() {}

#[then("loading succeeds with explicit source")]
fn then_loading_succeeds_with_explicit_source() {
    let source = "tests/fixtures/valid_aliases_and_must.theorem";
    let yaml = load_fixture("valid_aliases_and_must.theorem");
    let result = load_theorem_docs_with_source(source, &yaml);
    assert!(result.is_ok(), "fixture should parse successfully");
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
