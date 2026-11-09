//! Shared property-test strategies for canonical action names.
//!
//! These generators are available only to `theoremc-core` unit tests. They
//! keep grammar coverage consistent without adding test helpers to the public
//! API.

use proptest::prelude::{Just, Strategy, prop_oneof};

use crate::canonical_action_name::CanonicalActionNameInvalidReason;

pub(crate) fn valid_segment_strategy() -> impl Strategy<Value = String> {
    ("[A-Za-z_]", "[A-Za-z0-9_]{0,8}")
        .prop_map(|(first, rest)| format!("{first}{rest}"))
        .prop_filter("segment must not be a Rust keyword", |segment| {
            !crate::canonical_action_name::is_rust_reserved_keyword(segment)
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

/// A malformed segment kind a generated name can contain.
#[derive(Debug, Clone, Copy)]
enum InvalidSegmentKind {
    Empty,
    InvalidPattern,
    ReservedKeyword,
}

/// Generates a name holding exactly one malformed segment, together with the
/// typed reason validation must report.
///
/// The malformed segment sits at a generated position, so the property covers
/// the first-segment, interior, and final-segment cases. Valid filler segments
/// surround it, and a valid tail is added when the name would otherwise hold
/// the malformed segment alone and fail the two-segment rule instead.
pub(crate) fn one_invalid_segment_strategy()
-> impl Strategy<Value = (String, CanonicalActionNameInvalidReason)> {
    (
        proptest::collection::vec(valid_segment_strategy(), 0..3),
        proptest::collection::vec(valid_segment_strategy(), 0..3),
        prop_oneof![
            Just(InvalidSegmentKind::Empty),
            Just(InvalidSegmentKind::InvalidPattern),
            Just(InvalidSegmentKind::ReservedKeyword),
        ],
    )
        .prop_map(|(mut prefix, mut suffix, kind)| {
            if prefix.is_empty() && suffix.is_empty() {
                suffix.push("tail".to_owned());
            }
            let position = prefix.len() + 1;
            let (segment, reason) = match kind {
                InvalidSegmentKind::Empty => (
                    String::new(),
                    CanonicalActionNameInvalidReason::EmptySegment { position },
                ),
                InvalidSegmentKind::InvalidPattern => {
                    let segment = "1bad".to_owned();
                    (
                        segment.clone(),
                        CanonicalActionNameInvalidReason::InvalidIdentifierPattern {
                            position,
                            segment,
                        },
                    )
                }
                InvalidSegmentKind::ReservedKeyword => {
                    let segment = "fn".to_owned();
                    (
                        segment.clone(),
                        CanonicalActionNameInvalidReason::ReservedKeyword { position, segment },
                    )
                }
            };
            prefix.push(segment);
            prefix.extend(suffix);
            (prefix.join("."), reason)
        })
}
