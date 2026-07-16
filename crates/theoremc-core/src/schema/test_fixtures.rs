//! Hidden test-fixture helpers shared by unit and integration tests.

/// Multi-document theorem source with one duplicated theorem identifier.
#[must_use]
pub const fn duplicate_theorem_keys_yaml() -> &'static str {
    include_str!("../../tests/fixtures/duplicate_theorem_keys.theorem")
}

/// Multi-document theorem source with two duplicated theorem identifiers.
#[must_use]
pub const fn multi_duplicate_theorem_keys_yaml() -> &'static str {
    include_str!("../../tests/fixtures/multi_duplicate_theorem_keys.theorem")
}

/// Theorem source containing an invalid `Forall` Rust type.
#[must_use]
pub const fn invalid_forall_type_yaml() -> &'static str {
    include_str!("../../tests/fixtures/invalid_forall_type.theorem")
}

/// Theorem source containing a free named `Forall` lifetime.
#[must_use]
pub const fn free_lifetime_forall_yaml() -> &'static str {
    include_str!("../../tests/fixtures/free_lifetime_forall.theorem")
}

/// Theorem source containing a bound bare-function lifetime.
#[must_use]
pub const fn bound_lifetime_bare_fn_yaml() -> &'static str {
    include_str!("../../tests/fixtures/bound_lifetime_bare_fn.theorem")
}

/// Theorem source containing a bound trait-object lifetime.
#[must_use]
pub const fn bound_lifetime_trait_object_yaml() -> &'static str {
    include_str!("../../tests/fixtures/bound_lifetime_trait_object.theorem")
}
