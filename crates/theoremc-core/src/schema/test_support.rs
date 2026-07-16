//! Assertion helpers shared by schema unit tests.

use super::load_theorem_docs;

pub(super) fn assert_parse_error_contains(yaml: &str, expected_substring: &str) {
    let error = load_theorem_docs(yaml).expect_err("expected parser to reject fixture");
    let message = error.to_string();
    assert!(
        message.contains(expected_substring),
        "expected parse error to contain '{expected_substring}', got: {message}"
    );
}
