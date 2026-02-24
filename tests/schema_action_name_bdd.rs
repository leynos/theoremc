//! Behavioural tests for canonical action-name validation.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::load_theorem_docs;

fn assert_fixture_ok(fixture_name: &str) -> Result<(), String> {
    let yaml =
        load_fixture(fixture_name).map_err(|error| format!("failed to load fixture: {error}"))?;
    load_theorem_docs(&yaml)
        .map(|_| ())
        .map_err(|error| format!("fixture should load: {error}"))
}

fn assert_fixture_err_contains(fixture_name: &str, expected: &str) -> Result<(), String> {
    let yaml =
        load_fixture(fixture_name).map_err(|error| format!("failed to load fixture: {error}"))?;
    let error = load_theorem_docs(&yaml)
        .err()
        .ok_or_else(|| format!("fixture should fail: {fixture_name}"))?;
    let message = error.to_string();

    if message.contains(expected) {
        return Ok(());
    }

    Err(format!(
        "expected '{expected}' in error for {fixture_name}, got: {message}"
    ))
}

#[given("a theorem fixture with canonical action names")]
fn given_theorem_fixture_with_canonical_action_names() {}

#[then("loading succeeds for canonical action names")]
fn then_loading_succeeds_for_canonical_action_names() -> Result<(), String> {
    assert_fixture_ok("valid_full.theorem")?;
    assert_fixture_ok("valid_nested_maybe.theorem")
}

#[given("a theorem fixture with malformed action names")]
fn given_theorem_fixture_with_malformed_action_names() {}

#[then("loading fails for malformed action names")]
fn then_loading_fails_for_malformed_action_names() -> Result<(), String> {
    assert_fixture_err_contains(
        "invalid_action_missing_dot.theorem",
        "dot-separated canonical",
    )?;
    assert_fixture_err_contains(
        "invalid_action_empty_segment.theorem",
        "segment 2 must be non-empty",
    )
}

#[given("a theorem fixture with keyword action segments")]
fn given_theorem_fixture_with_keyword_action_segments() {}

#[then("loading fails for keyword action segments")]
fn then_loading_fails_for_keyword_action_segments() -> Result<(), String> {
    assert_fixture_err_contains(
        "invalid_action_keyword_segment.theorem",
        "Rust reserved keyword",
    )?;
    assert_fixture_err_contains(
        "invalid_let_action_keyword_segment.theorem",
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
