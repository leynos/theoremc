//! Behavioural tests for non-vacuity defaults using `rstest-bdd`.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::load_theorem_docs;

/// Asserts that loading a fixture succeeds.
fn assert_fixture_ok(fixture_name: &str) {
    let yaml = load_fixture(fixture_name);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_ok(),
        "expected fixture {fixture_name} to load successfully, got: {:?}",
        result.err()
    );
}

/// Asserts that loading a fixture fails with a message fragment.
fn assert_fixture_err_contains(fixture_name: &str, expected_fragment: &str) {
    let yaml = load_fixture(fixture_name);
    let result = load_theorem_docs(&yaml);
    assert!(result.is_err(), "expected fixture {fixture_name} to fail");

    let message = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    assert!(
        message.contains(expected_fragment),
        concat!(
            "expected fixture {fixture_name} to fail with fragment ",
            "'{expected_fragment}', got: {message}",
        ),
        fixture_name = fixture_name,
        expected_fragment = expected_fragment,
        message = message,
    );
}

#[given("a valid default theorem fixture")]
fn given_valid_default_theorem_fixture() {}

#[then("the default theorem fixture loads successfully")]
fn then_default_theorem_fixture_loads_successfully() {
    assert_fixture_ok("valid_minimal.theorem");
}

#[given("a valid vacuous theorem fixture")]
fn given_valid_vacuous_theorem_fixture() {}

#[then("the vacuous theorem fixture loads successfully")]
fn then_vacuous_theorem_fixture_loads_successfully() {
    assert_fixture_ok("valid_vacuous.theorem");
}

#[given("a default theorem fixture missing witness")]
fn given_default_theorem_fixture_missing_witness() {}

#[then("loading fails because witness is required by default")]
fn then_loading_fails_because_witness_is_required_by_default() {
    assert_fixture_err_contains(
        "invalid_missing_witness_default.theorem",
        "Witness section must contain at least one witness",
    );
}

#[given("an explicit non-vacuous theorem fixture missing witness")]
fn given_explicit_non_vacuous_theorem_fixture_missing_witness() {}

#[then("loading fails because witness is required when non-vacuous is explicit")]
fn then_loading_fails_because_witness_is_required_when_non_vacuous_is_explicit() {
    assert_fixture_err_contains(
        "invalid_missing_witness_explicit_false.theorem",
        "Witness section must contain at least one witness",
    );
}

#[given("a vacuous theorem fixture without vacuity reason")]
fn given_vacuous_theorem_fixture_without_vacuity_reason() {}

#[then("loading fails because vacuity reason is required")]
fn then_loading_fails_because_vacuity_reason_is_required() {
    assert_fixture_err_contains(
        "invalid_vacuous_missing_reason.theorem",
        "vacuity_because is required when allow_vacuous is true",
    );
}

#[given("a vacuous theorem fixture with blank vacuity reason")]
fn given_vacuous_theorem_fixture_with_blank_vacuity_reason() {}

#[then("loading fails because vacuity reason is blank")]
fn then_loading_fails_because_vacuity_reason_is_blank() {
    assert_fixture_err_contains(
        "invalid_empty_vacuity_because.theorem",
        "Evidence.kani.vacuity_because must be non-empty after trimming",
    );
}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Default policy accepts witness-backed theorem"
)]
fn default_policy_accepts_witness_backed_theorem() {}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Vacuous override accepts theorem with reason"
)]
fn vacuous_override_accepts_theorem_with_reason() {}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Default policy rejects missing witness"
)]
fn default_policy_rejects_missing_witness() {}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Explicit non-vacuous policy rejects missing witness"
)]
fn explicit_non_vacuous_policy_rejects_missing_witness() {}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Vacuous override rejects missing reason"
)]
fn vacuous_override_rejects_missing_reason() {}

#[scenario(
    path = "tests/features/schema_vacuity.feature",
    name = "Vacuous override rejects blank reason"
)]
fn vacuous_override_rejects_blank_reason() {}
