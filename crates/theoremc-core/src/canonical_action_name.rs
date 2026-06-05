//! Canonical action-name grammar and domain newtype.
//!
//! Canonical action names identify theorem actions using a dot-separated
//! namespace. The grammar is shared by schema validation and code-generation
//! name mangling so invalid action names cannot cross domain boundaries
//! unnoticed.

/// Error returned when a string is not a valid canonical action name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid canonical action name '{name}': {reason}")]
pub struct InvalidCanonicalActionName {
    name: String,
    reason: CanonicalActionNameInvalidReason,
}

impl InvalidCanonicalActionName {
    /// Returns the rejected action name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the typed validation reason.
    #[must_use]
    pub const fn reason(&self) -> &CanonicalActionNameInvalidReason {
        &self.reason
    }
}

/// Typed reason data for canonical action-name validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalActionNameInvalidReason {
    /// The name does not contain the required namespace separator.
    MissingSeparator,
    /// A dot-separated segment was empty.
    EmptySegment {
        /// One-based segment position.
        position: usize,
    },
    /// A segment did not match the ASCII Rust identifier pattern.
    InvalidIdentifierPattern {
        /// One-based segment position.
        position: usize,
        /// The invalid segment value.
        segment: String,
    },
    /// A segment is a Rust reserved keyword.
    ReservedKeyword {
        /// One-based segment position.
        position: usize,
        /// The reserved keyword segment value.
        segment: String,
    },
}

impl std::fmt::Display for CanonicalActionNameInvalidReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSeparator => {
                f.write_str("must contain at least two dot-separated segments")
            }
            Self::EmptySegment { position } => {
                write!(f, "segment {position} must be non-empty")
            }
            Self::InvalidIdentifierPattern { position, segment } => write!(
                f,
                concat!(
                    "segment {position} ('{segment}') must match identifier ",
                    "pattern ^[A-Za-z_][A-Za-z0-9_]*"
                ),
                position = position,
                segment = segment,
            ),
            Self::ReservedKeyword { position, segment } => write!(
                f,
                "segment {position} ('{segment}') must not be a Rust reserved keyword",
            ),
        }
    }
}

/// A validated canonical action name, for example `"account.deposit"`.
///
/// ```rust
/// use theoremc_core::canonical_action_name::CanonicalActionName;
///
/// let name = CanonicalActionName::new("account.deposit")
///     .expect("valid canonical name");
/// assert_eq!(name.as_str(), "account.deposit");
/// assert!(CanonicalActionName::new("deposit").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalActionName(String);

impl CanonicalActionName {
    /// Validates and wraps a canonical action name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCanonicalActionName`] when the input does not satisfy
    /// the canonical grammar.
    pub fn new(value: &str) -> Result<Self, InvalidCanonicalActionName> {
        validate_canonical_action_name(value)?;
        Ok(Self(value.to_owned()))
    }

    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CanonicalActionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CanonicalActionName {
    type Error = InvalidCanonicalActionName;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for CanonicalActionName {
    type Error = InvalidCanonicalActionName;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_canonical_action_name(&value)?;
        Ok(Self(value))
    }
}

/// Validates the canonical grammar: `Segment ("." Segment)+`.
///
/// # Errors
///
/// Returns [`InvalidCanonicalActionName`] with typed reason data when the
/// supplied string is not canonical.
pub fn validate_canonical_action_name(name: &str) -> Result<(), InvalidCanonicalActionName> {
    if !name.contains('.') {
        return Err(invalid_name(
            name,
            CanonicalActionNameInvalidReason::MissingSeparator,
        ));
    }

    for (index, segment) in name.split('.').enumerate() {
        validate_segment(name, segment, index + 1)?;
    }

    Ok(())
}

fn validate_segment(
    name: &str,
    segment: &str,
    position: usize,
) -> Result<(), InvalidCanonicalActionName> {
    if segment.is_empty() {
        return Err(invalid_name(
            name,
            CanonicalActionNameInvalidReason::EmptySegment { position },
        ));
    }

    if !is_valid_ascii_identifier_pattern(segment) {
        return Err(invalid_name(
            name,
            CanonicalActionNameInvalidReason::InvalidIdentifierPattern {
                position,
                segment: segment.to_owned(),
            },
        ));
    }

    if is_rust_reserved_keyword(segment) {
        return Err(invalid_name(
            name,
            CanonicalActionNameInvalidReason::ReservedKeyword {
                position,
                segment: segment.to_owned(),
            },
        ));
    }

    Ok(())
}

fn invalid_name(
    name: &str,
    reason: CanonicalActionNameInvalidReason,
) -> InvalidCanonicalActionName {
    InvalidCanonicalActionName {
        name: name.to_owned(),
        reason,
    }
}

/// Returns `true` if the string matches `^[A-Za-z_][A-Za-z0-9_]*$`.
#[must_use]
pub(crate) fn is_valid_ascii_identifier_pattern(s: &str) -> bool {
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
pub(crate) fn is_rust_reserved_keyword(s: &str) -> bool {
    RUST_KEYWORDS.contains(&s)
}

/// Rust reserved keywords from the language reference.
///
/// Includes strict keywords, reserved keywords, and weak keywords that cannot
/// serve as raw identifiers. The list covers all keywords defined in the Rust
/// Reference (2024 edition and later).
#[rustfmt::skip]
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate",
    "dyn", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual",
    "yield", "gen", "union",
];

#[cfg(test)]
mod tests {
    //! Unit tests for canonical action-name parsing and validation behaviour.

    use googletest::prelude::*;
    use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};
    use rstest::rstest;

    use super::{
        CanonicalActionName, CanonicalActionNameInvalidReason, validate_canonical_action_name,
    };

    #[rstest]
    #[case::two_segments("account.deposit")]
    #[case::underscores("hnsw.graph_with_capacity")]
    #[case::underscore_prefixes("_a._b1")]
    #[case::many_segments("hnsw.graph.with_capacity")]
    fn valid_canonical_action_name_passes(#[case] name: &str) -> Result<()> {
        verify_that!(validate_canonical_action_name(name), ok(anything()))
    }

    #[rstest]
    #[case::missing_dot("deposit", CanonicalActionNameInvalidReason::MissingSeparator)]
    #[case::leading_dot(
        ".deposit",
        CanonicalActionNameInvalidReason::EmptySegment { position: 1 }
    )]
    #[case::trailing_dot(
        "deposit.",
        CanonicalActionNameInvalidReason::EmptySegment { position: 2 }
    )]
    #[case::double_dot(
        "account..deposit",
        CanonicalActionNameInvalidReason::EmptySegment { position: 2 }
    )]
    #[case::hyphen(
        "account.deposit-now",
        CanonicalActionNameInvalidReason::InvalidIdentifierPattern {
            position: 2,
            segment: "deposit-now".to_owned(),
        }
    )]
    #[case::keyword(
        "account.fn",
        CanonicalActionNameInvalidReason::ReservedKeyword {
            position: 2,
            segment: "fn".to_owned(),
        }
    )]
    fn malformed_canonical_action_name_reports_typed_reason(
        #[case] name: &str,
        #[case] expected: CanonicalActionNameInvalidReason,
    ) -> Result<()> {
        let error = validate_canonical_action_name(name).expect_err("should fail");

        verify_that!(error.name(), eq(name))?;
        verify_that!(error.reason(), eq(&expected))
    }

    #[test]
    fn canonical_action_name_newtype_preserves_input() -> Result<()> {
        let name =
            CanonicalActionName::new("account.deposit").expect("valid canonical action name");

        verify_that!(name.as_str(), eq("account.deposit"))
    }

    fn valid_segment_strategy() -> impl Strategy<Value = String> {
        ("[A-Za-z_]", "[A-Za-z0-9_]{0,8}")
            .prop_map(|(first, rest)| format!("{first}{rest}"))
            .prop_filter("segment must not be a Rust keyword", |segment| {
                !super::is_rust_reserved_keyword(segment)
            })
    }

    pub(crate) fn valid_canonical_action_name_strategy() -> impl Strategy<Value = String> {
        (
            valid_segment_strategy(),
            valid_segment_strategy(),
            proptest::collection::vec(valid_segment_strategy(), 0..4),
        )
            .prop_map(|(first, second, rest)| {
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join(".")
            })
    }

    proptest! {
        #[test]
        fn valid_generated_names_round_trip_through_newtype(
            name in valid_canonical_action_name_strategy(),
        ) {
            let canonical = CanonicalActionName::new(&name)?;

            prop_assert_eq!(canonical.as_str(), name);
            prop_assert!(validate_canonical_action_name(canonical.as_str()).is_ok());
        }

        #[test]
        fn single_generated_segments_are_rejected(
            segment in valid_segment_strategy(),
        ) {
            let error = validate_canonical_action_name(&segment).expect_err("single segment should fail");

            prop_assert_eq!(
                error.reason(),
                &CanonicalActionNameInvalidReason::MissingSeparator,
            );
        }
    }
}
