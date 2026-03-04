//! Behavioural tests for action name mangling.

use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::golden::ACTION_GOLDEN_TRIPLES;
use theoremc::mangle::{RESOLUTION_TARGET, hash12, mangle_action_name};

#[given("representative canonical action names")]
fn given_representative_canonical_action_names() {}

#[then("each name produces the expected mangled identifier")]
fn then_each_name_produces_the_expected_mangled_identifier() {
    for (canonical, expected_slug, expected_hash) in ACTION_GOLDEN_TRIPLES {
        let m = mangle_action_name(canonical);
        assert_eq!(m.slug(), *expected_slug, "slug mismatch for {canonical}",);
        assert_eq!(m.hash(), *expected_hash, "hash mismatch for {canonical}",);
        let expected_ident = format!("{expected_slug}__h{expected_hash}");
        assert_eq!(
            m.identifier(),
            expected_ident,
            "identifier mismatch for {canonical}",
        );
    }
}

#[given("action names that differ only in underscore placement")]
fn given_action_names_that_differ_only_in_underscore_placement() {}

#[then("their mangled identifiers are distinct")]
fn then_their_mangled_identifiers_are_distinct() {
    // "a.b_c" and "a_b.c" differ only in where the underscore sits
    // relative to the dot separator. The escaping rule must keep them
    // distinct.
    let m_a = mangle_action_name("a.b_c");
    let m_b = mangle_action_name("a_b.c");
    assert_ne!(
        m_a.slug(),
        m_b.slug(),
        "slugs must differ: {} vs {}",
        m_a.slug(),
        m_b.slug(),
    );
    assert_ne!(
        m_a.identifier(),
        m_b.identifier(),
        "identifiers must differ",
    );
}

#[given("a mangled canonical action name")]
fn given_a_mangled_canonical_action_name() {}

#[then("the resolution path begins with crate::theorem_actions")]
fn then_the_resolution_path_begins_with_resolution_target() {
    let names = [
        "account.deposit",
        "hnsw.attach_node",
        "hnsw.graph.with_capacity",
        "_a._b",
    ];

    let prefix = format!("{RESOLUTION_TARGET}::");
    for name in &names {
        let m = mangle_action_name(name);
        assert!(
            m.path().starts_with(&prefix),
            "path for {name} must start with resolution target: {}",
            m.path(),
        );
        assert!(
            m.path().ends_with(m.identifier()),
            "path for {name} must end with identifier: {}",
            m.path(),
        );
        // The hash suffix must match an independent hash12 call.
        assert_eq!(m.hash(), hash12(name), "hash mismatch for {name}",);
    }
}

#[scenario(
    path = "tests/features/action_mangle.feature",
    name = "Simple action names produce correct mangled identifiers"
)]
fn simple_action_names_produce_correct_mangled_identifiers() {}

#[scenario(
    path = "tests/features/action_mangle.feature",
    name = "Underscore escaping preserves injectivity"
)]
fn underscore_escaping_preserves_injectivity() {}

#[scenario(
    path = "tests/features/action_mangle.feature",
    name = "Mangled identifiers resolve to crate::theorem_actions"
)]
fn mangled_identifiers_resolve_to_resolution_target() {}
