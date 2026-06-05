//! Behavioural tests for canonical action-name validation.

mod common;

use common::{assert_fixture_error_contains, assert_fixture_loads};
use rstest_bdd_macros::{given, scenario, then};

/// Helper to assert multiple fixtures fail with expected error messages.
fn assert_fixtures_fail_with_errors(test_cases: &[(&str, &str)]) -> Result<(), String> {
    for (fixture_name, expected) in test_cases {
        assert_fixture_error_contains(fixture_name, expected)?;
    }
    Ok(())
}

#[given("a theorem fixture with canonical action names")]
fn given_theorem_fixture_with_canonical_action_names() {}

#[then("loading succeeds for canonical action names")]
fn then_loading_succeeds_for_canonical_action_names() -> Result<(), String> {
    assert_fixture_loads("valid_full.theorem")?;
    assert_fixture_loads("valid_nested_maybe.theorem")
}

#[given("a theorem fixture with malformed action names")]
fn given_theorem_fixture_with_malformed_action_names() {}

#[then("loading fails for malformed action names")]
fn then_loading_fails_for_malformed_action_names() -> Result<(), String> {
    assert_fixtures_fail_with_errors(&[
        (
            "invalid_action_missing_dot.theorem",
            "dot-separated canonical",
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
    assert_fixtures_fail_with_errors(&[
        (
            "invalid_action_keyword_segment.theorem",
            "Rust reserved keyword",
        ),
        (
            "invalid_let_action_keyword_segment.theorem",
            "Rust reserved keyword",
        ),
    ])
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
