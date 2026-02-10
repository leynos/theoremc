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
    "unsized", "virtual", "yield", // Edition 2024 reserved keyword
    "gen",   // Weak keywords used in specific contexts
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
    use super::*;

    // ── Valid identifiers ───────────────────────────────────────────

    #[test]
    fn valid_simple_name() {
        assert!(validate_identifier("Foo").is_ok());
    }

    #[test]
    fn valid_with_underscore_prefix() {
        assert!(validate_identifier("_bar").is_ok());
    }

    #[test]
    fn valid_with_digits() {
        assert!(validate_identifier("Baz123").is_ok());
    }

    #[test]
    fn valid_mixed_case_and_underscores() {
        assert!(validate_identifier("My_Theorem_42").is_ok());
    }

    #[test]
    fn valid_single_letter() {
        assert!(validate_identifier("x").is_ok());
    }

    #[test]
    fn valid_single_underscore() {
        assert!(validate_identifier("_").is_ok());
    }

    // ── Invalid identifier patterns ─────────────────────────────────

    #[test]
    fn invalid_starts_with_digit() {
        let err = validate_identifier("123abc");
        assert!(err.is_err());
        let msg = err.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("must match the pattern"));
    }

    #[test]
    fn invalid_empty_string() {
        assert!(validate_identifier("").is_err());
    }

    #[test]
    fn invalid_contains_hyphen() {
        assert!(validate_identifier("foo-bar").is_err());
    }

    #[test]
    fn invalid_contains_space() {
        assert!(validate_identifier("foo bar").is_err());
    }

    #[test]
    fn invalid_contains_dot() {
        assert!(validate_identifier("foo.bar").is_err());
    }

    // ── Rust keyword rejection ──────────────────────────────────────

    #[test]
    fn keyword_fn_rejected() {
        let err = validate_identifier("fn");
        assert!(err.is_err());
        let msg = err.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(msg.contains("Rust reserved keyword"));
    }

    #[test]
    fn keyword_let_rejected() {
        assert!(validate_identifier("let").is_err());
    }

    #[test]
    fn keyword_match_rejected() {
        assert!(validate_identifier("match").is_err());
    }

    #[test]
    fn keyword_type_rejected() {
        assert!(validate_identifier("type").is_err());
    }

    #[test]
    fn keyword_self_lowercase_rejected() {
        assert!(validate_identifier("self").is_err());
    }

    #[test]
    fn keyword_self_uppercase_rejected() {
        assert!(validate_identifier("Self").is_err());
    }

    #[test]
    fn keyword_async_rejected() {
        assert!(validate_identifier("async").is_err());
    }

    #[test]
    fn keyword_yield_rejected() {
        assert!(validate_identifier("yield").is_err());
    }

    // ── Non-keywords that look close ────────────────────────────────

    #[test]
    fn non_keyword_lets_accepted() {
        assert!(validate_identifier("lets").is_ok());
    }

    #[test]
    fn non_keyword_types_accepted() {
        assert!(validate_identifier("types").is_ok());
    }

    #[test]
    fn non_keyword_matching_accepted() {
        assert!(validate_identifier("matching").is_ok());
    }
}
