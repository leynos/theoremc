//! Tests for shared integration fixture helpers.

mod common;

use common::{
    assert_fixture_error_contains, fixture_error_message, fixture_source, load_fixture_docs,
};
use googletest::prelude::*;
use rstest::rstest;

#[rstest]
#[case::minimal("valid_minimal.theorem", "Minimal")]
#[case::multi("valid_multi.theorem", "FirstTheorem")]
fn fixture_docs_load_successfully(
    #[case] fixture_name: &str,
    #[case] first_theorem: &str,
) -> googletest::Result<()> {
    let docs = match load_fixture_docs(fixture_name) {
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
    if let Err(error) = assert_fixture_error_contains(fixture_name, expected_fragment) {
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
    let message = match fixture_error_message(fixture_name) {
        Ok(message) => message,
        Err(error) => return fail!("{error}"),
    };
    verify_that!(message, contains_substring(expected_fragment))
}

#[rstest]
#[case::minimal("valid_minimal.theorem")]
#[case::invalid("invalid_empty_about.theorem")]
fn fixture_source_matches_diagnostic_path(#[case] fixture_name: &str) -> googletest::Result<()> {
    let expected = format!("tests/fixtures/{fixture_name}");
    verify_that!(fixture_source(fixture_name).as_str(), eq(expected.as_str()))
}
