//! Canonical action-name validation for theorem `ActionCall.action` fields.
//!
//! The canonical grammar is `Segment ("." Segment)+`, where each `Segment`
//! follows the restricted ASCII identifier pattern and is not a Rust reserved
//! keyword.

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
pub(crate) fn validate_canonical_action_name(name: &str) -> Result<(), String> {
    let segments: Vec<&str> = name.split('.').collect();
    if segments.len() < 2 {
        return Err(CANONICAL_ACTION_HINT.to_owned());
    }

    for (index, segment) in segments.iter().enumerate() {
        validate_segment(segment, index + 1)?;
    }

    Ok(())
}

fn validate_segment(segment: &str, position: usize) -> Result<(), String> {
    if segment.is_empty() {
        return Err(format!("action segment {position} must be non-empty"));
    }

    if !is_valid_ascii_identifier_pattern(segment) {
        return Err(format!(
            concat!(
                "action segment {position} ('{segment}') must match ",
                "identifier pattern ^[A-Za-z_][A-Za-z0-9_]*"
            ),
            position = position,
            segment = segment,
        ));
    }

    if is_rust_reserved_keyword(segment) {
        return Err(format!(
            "action segment {position} ('{segment}') must not be a Rust reserved keyword"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

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
        assert!(
            error.contains(expected),
            "expected '{expected}' in '{error}'"
        );
    }

    #[rstest]
    #[case::first_segment("self.deposit")]
    #[case::second_segment("account.fn")]
    #[case::third_segment("graph.path.type")]
    fn keyword_segment_fails(#[case] name: &str) {
        let error = validate_canonical_action_name(name).expect_err("should fail");
        assert!(
            error.contains("Rust reserved keyword"),
            "expected keyword error, got: {error}"
        );
    }
}
