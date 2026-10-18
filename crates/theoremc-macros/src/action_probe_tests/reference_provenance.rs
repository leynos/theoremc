//! Diagnostic provenance assertions shared by reference-selection cases.

use super::{MacroExpansionError, TheoremFixture, expand_fixture};
use camino::Utf8Path;
use googletest::prelude::*;

/// The provenance a conflict diagnostic must report.
///
/// Mirrors the fields of
/// [`MacroExpansionError::ConflictingActionSignature`] so a test states its
/// expectation once and has both the typed error and the rendered message
/// checked against it.
pub(super) struct Provenance<'a> {
    pub(super) action: &'a str,
    pub(super) first_theorem: &'a str,
    pub(super) conflicting_theorem: &'a str,
    pub(super) first_signature: &'a str,
    pub(super) conflicting_signature: &'a str,
}

impl Provenance<'_> {
    /// Expands `fixture` and asserts the diagnostic carries exactly `self`.
    ///
    /// Returns the rendered message, so a caller can compare one arrangement
    /// against another.
    pub(super) fn assert_reported(
        &self,
        fixture_path: &str,
        fixture: &TheoremFixture,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let expansion = expand_fixture(Utf8Path::new(fixture_path), fixture)
            .expect_err("conflicting action signatures should fail expansion");
        let error = expansion
            .downcast_ref::<MacroExpansionError>()
            .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

        assert_that!(
            error,
            matches_pattern!(MacroExpansionError::ConflictingActionSignature {
                action: eq(self.action),
                first_theorem: eq(self.first_theorem),
                conflicting_theorem: eq(self.conflicting_theorem),
                first_signature: eq(self.first_signature),
                conflicting_signature: eq(self.conflicting_signature),
            })
        );

        // A user only ever sees the rendered message, so assert the provenance
        // reaches it rather than trusting the typed fields alone.
        let message = error.to_string();
        for fragment in [
            self.action,
            self.first_theorem,
            self.conflicting_theorem,
            self.first_signature,
            self.conflicting_signature,
        ] {
            assert_that!(message.as_str(), contains_substring(fragment));
        }
        Ok(message)
    }
}
