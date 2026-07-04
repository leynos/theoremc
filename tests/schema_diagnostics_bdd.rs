//! Behavioural tests for structured diagnostics using `rstest-bdd`.

mod common {
    pub(crate) use test_helpers::{assert_diagnostic_failure, load_fixture_text};
}

use common::{assert_diagnostic_failure, load_fixture_text};
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::{SchemaDiagnosticCode, SourceId, load_theorem_docs_with_source};

#[given("a parser-invalid theorem fixture")]
fn given_parser_invalid_theorem_fixture() {}

#[then("loading fails with source-located parser diagnostics")]
fn then_loading_fails_with_source_located_parser_diagnostics() -> Result<(), String> {
    assert_diagnostic_failure(
        "invalid_unknown_key.theorem",
        SchemaDiagnosticCode::ParseFailure,
    )?;

    Ok(())
}

#[given("a validator-invalid theorem fixture")]
fn given_validator_invalid_theorem_fixture() {}

#[then("loading fails with source-located validator diagnostics")]
fn then_loading_fails_with_source_located_validator_diagnostics() -> Result<(), String> {
    assert_diagnostic_failure(
        "invalid_empty_about.theorem",
        SchemaDiagnosticCode::ValidationFailure,
    )?;

    Ok(())
}

#[given("a valid theorem fixture for diagnostics")]
fn given_valid_theorem_fixture_for_diagnostics() {}

#[then("loading succeeds with explicit source")]
fn then_loading_succeeds_with_explicit_source() -> Result<(), String> {
    let source = "tests/fixtures/valid_aliases_and_must.theorem";
    let yaml = load_fixture_text("valid_aliases_and_must.theorem")?;
    load_theorem_docs_with_source(&SourceId::new(source), &yaml)
        .map_err(|error| error.to_string())?;

    Ok(())
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
