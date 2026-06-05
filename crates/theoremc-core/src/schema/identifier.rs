//! Identifier validation for theorem names and quantified variable keys.
//!
//! Identifiers must match the ASCII pattern `^[A-Za-z_][A-Za-z0-9_]*$`
//! and must not be a Rust reserved keyword. This keeps code generation
//! deterministic and avoids symbol collisions.

pub(crate) use crate::canonical_action_name::{
    is_rust_reserved_keyword, is_valid_ascii_identifier_pattern,
};

use super::error::SchemaError;

/// Validates that a string is a legal theorem identifier.
///
/// An identifier must:
/// - Match the pattern `^[A-Za-z_][A-Za-z0-9_]*$`.
/// - Not be a Rust reserved keyword.
///
/// # Errors
///
/// Returns `SchemaError::InvalidIdentifier` if the string fails either
/// check.
///
/// # Examples
///
///     use theoremc_core::schema::validate_identifier;
///
///     assert!(validate_identifier("MyTheorem").is_ok());
///     assert!(validate_identifier("_private").is_ok());
///     assert!(validate_identifier("fn").is_err());
///     assert!(validate_identifier("123bad").is_err());
pub fn validate_identifier(s: &str) -> Result<(), SchemaError> {
    if s.is_empty() {
        return Err(SchemaError::InvalidIdentifier {
            identifier: s.to_owned(),
            reason: "identifier must not be empty".to_owned(),
        });
    }

    if !is_valid_ascii_identifier_pattern(s) {
        return Err(SchemaError::InvalidIdentifier {
            identifier: s.to_owned(),
            reason: concat!(
                "must match the pattern ",
                "^[A-Za-z_][A-Za-z0-9_]*$ ",
                "(ASCII letters, digits, and underscores; ",
                "must not start with a digit)"
            )
            .to_owned(),
        });
    }

    if is_rust_reserved_keyword(s) {
        return Err(SchemaError::InvalidIdentifier {
            identifier: s.to_owned(),
            reason: concat!(
                "this is a Rust reserved keyword and cannot ",
                "be used as a theorem identifier",
            )
            .to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // ── Valid identifiers ───────────────────────────────────────────

    #[rstest]
    #[case::simple_name("Foo")]
    #[case::underscore_prefix("_bar")]
    #[case::with_digits("Baz123")]
    #[case::mixed_case_and_underscores("My_Theorem_42")]
    #[case::single_letter("x")]
    #[case::single_underscore("_")]
    fn valid_identifier_accepted(#[case] input: &str) {
        assert!(validate_identifier(input).is_ok());
    }

    // ── Invalid identifier patterns ─────────────────────────────────

    #[rstest]
    #[case::starts_with_digit("123abc", "must match the pattern")]
    #[case::empty_string("", "must not be empty")]
    #[case::contains_hyphen("foo-bar", "must match the pattern")]
    #[case::contains_space("foo bar", "must match the pattern")]
    #[case::contains_dot("foo.bar", "must match the pattern")]
    fn invalid_pattern_rejected(#[case] input: &str, #[case] expected_msg: &str) {
        let err = validate_identifier(input).expect_err("should be invalid");
        let msg = err.to_string();
        assert!(
            msg.contains(expected_msg),
            "expected message containing {expected_msg:?}, got {msg:?}",
        );
    }

    // ── Rust keyword rejection ──────────────────────────────────────

    #[rstest]
    #[case::keyword_fn("fn")]
    #[case::keyword_let("let")]
    #[case::keyword_match("match")]
    #[case::keyword_type("type")]
    #[case::keyword_self("self")]
    #[case::keyword_self_upper("Self")]
    #[case::keyword_async("async")]
    #[case::keyword_yield("yield")]
    fn keyword_rejected(#[case] input: &str) {
        let err = validate_identifier(input).expect_err("should be invalid");
        let msg = err.to_string();
        assert!(
            msg.contains("Rust reserved keyword"),
            "expected keyword message, got {msg:?}",
        );
    }

    // ── Non-keywords that look close ────────────────────────────────

    #[rstest]
    #[case::lets("lets")]
    #[case::types("types")]
    #[case::matching("matching")]
    fn non_keyword_near_miss_accepted(#[case] input: &str) {
        assert!(validate_identifier(input).is_ok());
    }
}
