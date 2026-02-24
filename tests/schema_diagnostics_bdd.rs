//! Behavioural tests for structured diagnostics using `rstest-bdd`.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::{SourceId, load_theorem_docs_with_source};

fn assert_diagnostic_failure(fixture_name: &str, expected_code: &str) -> Result<(), String> {
    let source = format!("tests/fixtures/{fixture_name}");
    let yaml = load_fixture(fixture_name)
        .map_err(|error| format!("failed to load fixture {fixture_name}: {error}"))?;
    let error = load_theorem_docs_with_source(&SourceId::new(&source), &yaml)
        .err()
        .ok_or_else(|| format!("fixture should fail: {fixture_name}"))?;
    let diagnostic = error
        .diagnostic()
        .ok_or_else(|| String::from("diagnostic should be present"))?;

    if diagnostic.code.as_str() != expected_code {
        return Err(format!(
            "unexpected diagnostic code: expected {expected_code}, got {}",
            diagnostic.code.as_str()
        ));
    }
    if diagnostic.location.source != source {
        return Err(format!(
            "unexpected diagnostic source: expected {source}, got {}",
            diagnostic.location.source
        ));
    }
    if diagnostic.location.line == 0 {
        return Err(String::from("diagnostic line should be greater than 0"));
    }
    if diagnostic.location.column == 0 {
        return Err(String::from("diagnostic column should be greater than 0"));
    }

    Ok(())
}

#[given("a parser-invalid theorem fixture")]
fn given_parser_invalid_theorem_fixture() {}

#[then("loading fails with source-located parser diagnostics")]
fn then_loading_fails_with_source_located_parser_diagnostics() -> Result<(), String> {
    assert_diagnostic_failure("invalid_unknown_key.theorem", "schema.parse_failure")?;

    Ok(())
}

#[given("a validator-invalid theorem fixture")]
fn given_validator_invalid_theorem_fixture() {}

#[then("loading fails with source-located validator diagnostics")]
fn then_loading_fails_with_source_located_validator_diagnostics() -> Result<(), String> {
    assert_diagnostic_failure("invalid_empty_about.theorem", "schema.validation_failure")?;

    Ok(())
}

#[given("a valid theorem fixture for diagnostics")]
fn given_valid_theorem_fixture_for_diagnostics() {}

#[then("loading succeeds with explicit source")]
#[expect(
    clippy::expect_used,
    reason = "Test step uses expect to give direct failure context."
)]
fn then_loading_succeeds_with_explicit_source() {
    let source = "tests/fixtures/valid_aliases_and_must.theorem";
    let yaml = load_fixture("valid_aliases_and_must.theorem")
        .expect("failed to load fixture valid_aliases_and_must.theorem");
    load_theorem_docs_with_source(&SourceId::new(source), &yaml)
        .expect("failed to parse theorem docs with explicit source");
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
