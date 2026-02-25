//! Canonical action-name validation for theorem `ActionCall.action` fields.
//!
//! The canonical grammar is `Segment ("." Segment)+`, where each `Segment`
//! follows the restricted ASCII identifier pattern and is not a Rust reserved
//! keyword.

use super::error::SchemaError;
use super::identifier::{is_rust_reserved_keyword, is_valid_ascii_identifier_pattern};

const CANONICAL_ACTION_HINT: &str =
    "action must be a dot-separated canonical name with at least two segments";

/// Validates a canonical action name.
///
/// A valid canonical action name:
///
/// - contains at least one `.` separator,
/// - has no empty segments,
/// - uses only segments matching `^[A-Za-z_][A-Za-z0-9_]*$`,
/// - and has no Rust reserved-keyword segment.
pub(crate) fn validate_canonical_action_name(name: &str) -> Result<(), SchemaError> {
    if !name.contains('.') {
        return Err(invalid_action_name_error(
            name,
            CANONICAL_ACTION_HINT.to_owned(),
        ));
    }

    for (index, segment) in name.split('.').enumerate() {
        validate_segment(name, segment, index + 1)?;
    }

    Ok(())
}

fn validate_segment(name: &str, segment: &str, position: usize) -> Result<(), SchemaError> {
    if segment.is_empty() {
        return Err(invalid_action_name_error(
            name,
            format!("action segment {position} must be non-empty"),
        ));
    }

    if !is_valid_ascii_identifier_pattern(segment) {
        return Err(invalid_action_name_error(
            name,
            format!(
                concat!(
                    "action segment {position} ('{segment}') must match ",
                    "identifier pattern ^[A-Za-z_][A-Za-z0-9_]*"
                ),
                position = position,
                segment = segment,
            ),
        ));
    }

    if is_rust_reserved_keyword(segment) {
        return Err(invalid_action_name_error(
            name,
            format!("action segment {position} ('{segment}') must not be a Rust reserved keyword"),
        ));
    }

    Ok(())
}

fn invalid_action_name_error(name: &str, reason: String) -> SchemaError {
    SchemaError::InvalidActionName {
        action: name.to_owned(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for canonical action-name parsing and validation behaviour.

    use rstest::rstest;

    use crate::schema::error::SchemaError;

    use super::validate_canonical_action_name;

    #[rstest]
    #[case::two_segments("account.deposit")]
    #[case::underscores("hnsw.graph_with_capacity")]
    #[case::underscore_prefixes("_a._b1")]
    #[case::many_segments("hnsw.graph.with_capacity")]
    fn valid_canonical_action_name_passes(#[case] name: &str) {
        assert!(validate_canonical_action_name(name).is_ok());
    }

    #[rstest]
    #[case::missing_dot("deposit", "dot-separated canonical name")]
    #[case::leading_dot(".deposit", "segment 1 must be non-empty")]
    #[case::trailing_dot("deposit.", "segment 2 must be non-empty")]
    #[case::double_dot("account..deposit", "segment 2 must be non-empty")]
    #[case::hyphen("account.deposit-now", "must match identifier pattern")]
    #[case::leading_whitespace(" account.deposit", "must match identifier pattern")]
    fn malformed_canonical_action_name_fails(#[case] name: &str, #[case] expected: &str) {
        let error = validate_canonical_action_name(name).expect_err("should fail");
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "expected '{expected}' in '{message}'"
        );
    }

    #[rstest]
    #[case::first_segment("self.deposit")]
    #[case::second_segment("account.fn")]
    #[case::third_segment("graph.path.type")]
    fn keyword_segment_fails(#[case] name: &str) {
        let error = validate_canonical_action_name(name).expect_err("should fail");
        let message = error.to_string();
        assert!(
            message.contains("Rust reserved keyword"),
            "expected keyword error, got: {message}"
        );
    }

    #[test]
    fn malformed_action_name_returns_invalid_action_name_error() {
        let error = validate_canonical_action_name("deposit").expect_err("should fail");
        match error {
            SchemaError::InvalidActionName { action, reason } => {
                assert_eq!(action, "deposit");
                assert!(reason.contains("dot-separated canonical name"));
            }
            other => panic!("expected InvalidActionName, got {other}"),
        }
    }
}
