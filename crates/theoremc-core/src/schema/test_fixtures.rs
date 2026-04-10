//! Hidden test-fixture helpers shared by unit and integration tests.

/// Multi-document theorem source with one duplicated theorem identifier.
#[must_use]
pub const fn duplicate_theorem_keys_yaml() -> &'static str {
    include_str!("../../../../tests/fixtures/duplicate_theorem_keys.theorem")
}

/// Multi-document theorem source with two duplicated theorem identifiers.
#[must_use]
pub const fn multi_duplicate_theorem_keys_yaml() -> &'static str {
    include_str!("../../../../tests/fixtures/multi_duplicate_theorem_keys.theorem")
}
