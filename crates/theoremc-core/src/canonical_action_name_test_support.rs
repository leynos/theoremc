//! Shared property-test strategies for canonical action names.
//!
//! These generators are available only to `theoremc-core` unit tests. They
//! keep grammar coverage consistent without adding test helpers to the public
//! API.

use proptest::prelude::Strategy;

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
