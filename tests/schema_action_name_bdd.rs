//! Behavioural tests for canonical action-name validation.

use rstest_bdd_macros::{given, scenario, then};
use test_helpers::{
    FixtureName, StrNewtype, assert_fixture_loads, fixture_source_id, load_fixture_text,
};
use theoremc::schema::{SchemaDiagnosticCode, load_theorem_docs_with_source};

/// Assert a malformed-action fixture reports the full structured contract.
///
/// Checking only the message would let a diagnostic drift to the theorem
/// header, another document, or the wrong error class without failing. Each
/// case therefore pins the fixture path, the exact position of the rejected
/// action value, the diagnostic code, and the grammar reason.
fn assert_malformed_action_diagnostic(
    fixture_name: &str,
    expected_line: usize,
    expected_column: usize,
    expected_reason: &str,
) -> Result<(), String> {
    let name = FixtureName::new(fixture_name);
    let source = fixture_source_id(name);
    let yaml = load_fixture_text(name)?;
    let error = load_theorem_docs_with_source(&source, &yaml)
        .err()
        .ok_or_else(|| format!("fixture should fail: {fixture_name}"))?;
    let diagnostic = error
        .diagnostic()
        .ok_or_else(|| format!("diagnostic should be present: {fixture_name}"))?;

    if diagnostic.code != SchemaDiagnosticCode::ValidationFailure {
        return Err(format!(
            "{fixture_name}: expected validation diagnostic, got {}",
            diagnostic.code.as_str()
        ));
    }
    if diagnostic.location.source != source.as_str() {
        return Err(format!(
            "{fixture_name}: expected source {}, got {}",
            source.as_str(),
            diagnostic.location.source
        ));
    }
    if diagnostic.location.line != expected_line {
        return Err(format!(
            "{fixture_name}: expected line {expected_line}, got {}",
            diagnostic.location.line
        ));
    }
    if diagnostic.location.column != expected_column {
        return Err(format!(
            "{fixture_name}: expected column {expected_column}, got {}",
            diagnostic.location.column
        ));
    }
    if !diagnostic.message.contains(expected_reason) {
        return Err(format!(
            "{fixture_name}: expected {expected_reason:?} in {:?}",
            diagnostic.message
        ));
    }

    Ok(())
}

/// Helper to assert multiple fixtures fail with expected error messages.
fn assert_fixtures_fail_with_errors(test_cases: &[(&str, &str)]) -> Result<(), String> {
    for (fixture_name, expected) in test_cases {
        assert_malformed_action_diagnostic(fixture_name, 5, 15, expected)?;
    }
    Ok(())
}

#[given("a theorem fixture with canonical action names")]
fn given_theorem_fixture_with_canonical_action_names() {}

#[then("loading succeeds for canonical action names")]
fn then_loading_succeeds_for_canonical_action_names() -> Result<(), String> {
    assert_fixture_loads(FixtureName::new("valid_full.theorem"))?;
    assert_fixture_loads(FixtureName::new("valid_nested_maybe.theorem"))
}

#[given("a theorem fixture with malformed action names")]
fn given_theorem_fixture_with_malformed_action_names() {}

#[then("loading fails for malformed action names")]
fn then_loading_fails_for_malformed_action_names() -> Result<(), String> {
    assert_fixtures_fail_with_errors(&[
        (
            "invalid_action_missing_dot.theorem",
            "must contain at least two dot-separated segments",
        ),
        (
            "invalid_action_empty_segment.theorem",
            "segment 2 must be non-empty",
        ),
    ])
}

#[given("a theorem fixture with keyword action segments")]
fn given_theorem_fixture_with_keyword_action_segments() {}

#[then("loading fails for keyword action segments")]
fn then_loading_fails_for_keyword_action_segments() -> Result<(), String> {
    assert_malformed_action_diagnostic(
        "invalid_action_keyword_segment.theorem",
        5,
        15,
        "Rust reserved keyword",
    )?;
    assert_malformed_action_diagnostic(
        "invalid_let_action_keyword_segment.theorem",
        6,
        15,
        "Rust reserved keyword",
    )
}

#[scenario(
    path = "tests/features/schema_action_name.feature",
    name = "Canonical action names are accepted"
)]
fn canonical_action_names_are_accepted() {}

#[scenario(
    path = "tests/features/schema_action_name.feature",
    name = "Malformed canonical action names are rejected"
)]
fn malformed_canonical_action_names_are_rejected() {}

#[scenario(
    path = "tests/features/schema_action_name.feature",
    name = "Reserved keyword action segments are rejected"
)]
fn reserved_keyword_action_segments_are_rejected() {}
