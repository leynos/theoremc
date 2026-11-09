//! Property tests for canonical action-name rejection.
//!
//! Construction must reject any name that holds a malformed segment, and it
//! must report the first offending segment so a caller sees the actual
//! problem rather than a later consequence of it.

use proptest::prelude::{prop_assert_eq, proptest};

use super::CanonicalActionName;
use super::test_support::one_invalid_segment_strategy;

proptest! {
    #[test]
    fn construction_rejects_one_invalid_segment(
        (name, expected) in one_invalid_segment_strategy(),
    ) {
        let error = CanonicalActionName::new(&name)
            .expect_err("a name with a malformed segment must be rejected");

        prop_assert_eq!(error.name(), name.as_str());
        prop_assert_eq!(error.reason(), &expected);
    }
}
