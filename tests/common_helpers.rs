//! Tests for shared integration fixture helpers.
//!
//! The helpers live in `tests/common/` and are re-exported through the
//! `test-helpers` crate so that workspace unit tests and integration binaries
//! share one implementation.

use googletest::prelude::{contains_substring, eq, fail, verify_that};
use rstest::rstest;
use test_helpers::{
    ExpectedFragment, FixtureName, StrNewtype, assert_diagnostic_failure,
    assert_fixture_error_contains, fixture_error_message, load_fixture_docs,
};
use theoremc::schema::SchemaDiagnosticCode;

#[rstest]
#[case::minimal("valid_minimal.theorem", "Minimal")]
#[case::multi("valid_multi.theorem", "FirstTheorem")]
fn fixture_docs_load_successfully(
    #[case] fixture_name: &str,
    #[case] first_theorem: &str,
) -> googletest::Result<()> {
    let docs = match load_fixture_docs(FixtureName::new(fixture_name)) {
        Ok(docs) => docs,
        Err(error) => return fail!("{error}"),
    };
    let Some(first) = docs.first() else {
        return fail!("fixture should contain at least one document");
    };
    verify_that!(first.theorem.as_str(), eq(first_theorem))
}

#[rstest]
#[case::unknown_key("invalid_unknown_key.theorem", "unknown field")]
#[case::bad_identifier("invalid_bad_identifier.theorem", "must match the pattern")]
fn assert_fixture_error_contains_matches_fragment(
    #[case] fixture_name: &str,
    #[case] expected_fragment: &str,
) -> googletest::Result<()> {
    if let Err(error) = assert_fixture_error_contains(
        FixtureName::new(fixture_name),
        ExpectedFragment::new(expected_fragment),
    ) {
        return fail!("{error}");
    }
    Ok(())
}

#[rstest]
#[case::unknown_key("invalid_unknown_key.theorem", "unknown field")]
#[case::bad_identifier("invalid_bad_identifier.theorem", "must match the pattern")]
fn fixture_error_message_contains_fragment(
    #[case] fixture_name: &str,
    #[case] expected_fragment: &str,
) -> googletest::Result<()> {
    let message = match fixture_error_message(FixtureName::new(fixture_name)) {
        Ok(message) => message,
        Err(error) => return fail!("{error}"),
    };
    verify_that!(message, contains_substring(expected_fragment))
}

#[rstest]
#[case::parser_failure("invalid_unknown_key.theorem", SchemaDiagnosticCode::ParseFailure)]
#[case::validation_failure(
    "invalid_empty_about.theorem",
    SchemaDiagnosticCode::ValidationFailure
)]
fn diagnostic_failure_checks_source_path(
    #[case] fixture_name: &str,
    #[case] expected_code: SchemaDiagnosticCode,
) -> googletest::Result<()> {
    // The helper derives `tests/fixtures/<name>` as the diagnostic source and
    // rejects a missing or zeroed location, so success proves the path and
    // position checks hold for a real fixture.
    if let Err(error) = assert_diagnostic_failure(FixtureName::new(fixture_name), expected_code) {
        return fail!("{error}");
    }
    Ok(())
}