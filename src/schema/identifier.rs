//! Identifier validation for theorem names and quantified variable keys.
//!
//! Identifiers must match the ASCII pattern `^[A-Za-z_][A-Za-z0-9_]*$`
//! and must not be a Rust reserved keyword. This keeps code generation
//! deterministic and avoids symbol collisions.

use super::error::SchemaError;

/// Rust reserved keywords from the language reference.
///
/// Includes strict keywords, reserved keywords, and weak keywords that
/// cannot serve as raw identifiers. The list covers all keywords defined
/// in the Rust Reference (2024 edition and later).
const RUST_KEYWORDS: &[&str] = &[
    // Strict keywords
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
    // Reserved keywords (no current syntax but reserved for future use)
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try", "typeof",
    "unsized", "virtual", "yield", "gen", // Edition 2024 reserved keyword
    // Weak keywords used in specific contexts
    "union",
];

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
///     use theoremc::schema::validate_identifier;
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

    if !is_valid_identifier_pattern(s) {
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

    if is_rust_keyword(s) {
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

/// Returns `true` if the string matches `^[A-Za-z_][A-Za-z0-9_]*$`.
#[must_use]
fn is_valid_identifier_pattern(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Returns `true` if the string is a Rust reserved keyword.
#[must_use]
fn is_rust_keyword(s: &str) -> bool {
    RUST_KEYWORDS.contains(&s)
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
