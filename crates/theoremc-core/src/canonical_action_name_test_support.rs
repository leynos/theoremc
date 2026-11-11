//! Shared property-test strategies for canonical action names.
//!
//! These generators are available only to `theoremc-core` unit tests. They
//! keep grammar coverage consistent without adding test helpers to the public
//! API.

use proptest::prelude::{Just, Strategy, prop_oneof};

use crate::canonical_action_name::CanonicalActionNameInvalidReason;

/// Generates one syntactically valid, non-keyword identifier segment.
pub(crate) fn valid_segment_strategy() -> impl Strategy<Value = String> {
    ("[A-Za-z_]", "[A-Za-z0-9_]{0,8}")
        .prop_map(|(first, rest)| format!("{first}{rest}"))
        .prop_filter("segment must not be a Rust keyword", |segment| {
            !crate::canonical_action_name::is_rust_reserved_keyword(segment)
        })
}

/// Generates a valid canonical action name of two or more segments.
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

/// Generates one of the malformed segment kinds.
fn invalid_segment_kind_strategy() -> impl Strategy<Value = InvalidSegmentKind> {
    prop_oneof![
        Just(InvalidSegmentKind::Empty),
        Just(InvalidSegmentKind::InvalidPattern),
        Just(InvalidSegmentKind::ReservedKeyword),
    ]
}

/// Builds the malformed segment text and the typed reason it must produce.
///
/// `position` is the one-based segment position the reason must name.
fn invalid_segment(
    kind: InvalidSegmentKind,
    position: usize,
) -> (String, CanonicalActionNameInvalidReason) {
    match kind {
        InvalidSegmentKind::Empty => (
            String::new(),
            CanonicalActionNameInvalidReason::EmptySegment { position },
        ),
        InvalidSegmentKind::InvalidPattern => {
            let segment = "1bad".to_owned();
            (
                segment.clone(),
                CanonicalActionNameInvalidReason::InvalidIdentifierPattern { position, segment },
            )
        }
        InvalidSegmentKind::ReservedKeyword => {
            let segment = "fn".to_owned();
            (
                segment.clone(),
                CanonicalActionNameInvalidReason::ReservedKeyword { position, segment },
            )
        }
    }
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
        invalid_segment_kind_strategy(),
    )
        .prop_map(|(mut prefix, mut suffix, kind)| {
            if prefix.is_empty() && suffix.is_empty() {
                suffix.push("tail".to_owned());
            }
            let position = prefix.len() + 1;
            let (segment, reason) = invalid_segment(kind, position);
            prefix.push(segment);
            prefix.extend(suffix);
            (prefix.join("."), reason)
        })
}

/// Generates a name holding two malformed segments, together with the typed
/// reason validation must report for the first of them.
///
/// Valid filler segments separate and surround the two malformed ones, so the
/// pair can appear at any first position and any later position. The expected
/// reason is always the first malformed segment's, which is the error a
/// caller must see rather than a later consequence of the first one.
pub(crate) fn multiple_invalid_segments_strategy()
-> impl Strategy<Value = (String, CanonicalActionNameInvalidReason)> {
    (
        proptest::collection::vec(valid_segment_strategy(), 0..2),
        invalid_segment_kind_strategy(),
        proptest::collection::vec(valid_segment_strategy(), 0..2),
        invalid_segment_kind_strategy(),
        proptest::collection::vec(valid_segment_strategy(), 0..2),
    )
        .prop_map(|(prefix, first_kind, middle, second_kind, suffix)| {
            let first_position = prefix.len() + 1;
            let second_position = first_position + 1 + middle.len();
            let (first_segment, first_reason) = invalid_segment(first_kind, first_position);
            let (second_segment, _) = invalid_segment(second_kind, second_position);

            let mut segments = prefix;
            segments.push(first_segment);
            segments.extend(middle);
            segments.push(second_segment);
            segments.extend(suffix);
            (segments.join("."), first_reason)
        })
}
