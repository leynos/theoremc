//! Behavioural tests for action name mangling.

use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::golden::ACTION_GOLDEN_TRIPLES;
use theoremc::mangle::{CanonicalActionName, RESOLUTION_TARGET, hash12, mangle_action_name};

fn canonical_action_name(name: &str) -> Result<CanonicalActionName, String> {
    CanonicalActionName::new(name)
        .map_err(|error| format!("static test input must be canonical: {error}"))
}

#[given("representative canonical action names")]
fn given_representative_canonical_action_names() {}

#[then("each name produces the expected mangled identifier")]
fn then_each_name_produces_the_expected_mangled_identifier() -> Result<(), String> {
    for (canonical, expected_slug, expected_hash) in ACTION_GOLDEN_TRIPLES {
        let name = canonical_action_name(canonical)?;
        let m = mangle_action_name(&name);
        if m.slug() != *expected_slug {
            return Err(format!(
                "slug mismatch for {canonical}: expected {expected_slug}, got {}",
                m.slug()
            ));
        }
        if m.hash() != *expected_hash {
            return Err(format!(
                "hash mismatch for {canonical}: expected {expected_hash}, got {}",
                m.hash()
            ));
        }
        let expected_ident = format!("{expected_slug}__h{expected_hash}");
        if m.identifier() != expected_ident {
            return Err(format!(
                "identifier mismatch for {canonical}: expected {expected_ident}, got {}",
                m.identifier()
            ));
        }
    }
    Ok(())
}

#[given("action names that differ only in underscore placement")]
fn given_action_names_that_differ_only_in_underscore_placement() {}

#[then("their mangled identifiers are distinct")]
fn then_their_mangled_identifiers_are_distinct() -> Result<(), String> {
    // "a.b_c" and "a_b.c" differ only in where the underscore sits
    // relative to the dot separator. The escaping rule must keep them
    // distinct.
    let a = canonical_action_name("a.b_c")?;
    let b = canonical_action_name("a_b.c")?;
    let m_a = mangle_action_name(&a);
    let m_b = mangle_action_name(&b);
    if m_a.slug() == m_b.slug() {
        return Err(format!(
            "slugs must differ: {} vs {}",
            m_a.slug(),
            m_b.slug()
        ));
    }
    if m_a.identifier() == m_b.identifier() {
        return Err("identifiers must differ".to_owned());
    }
    Ok(())
}

#[given("a mangled canonical action name")]
fn given_a_mangled_canonical_action_name() {}

#[then("the resolution path begins with crate::theorem_actions")]
fn then_the_resolution_path_begins_with_resolution_target() -> Result<(), String> {
    let names = [
        "account.deposit",
        "hnsw.attach_node",
        "hnsw.graph.with_capacity",
        "_a._b",
    ];

    let prefix = format!("{RESOLUTION_TARGET}::");
    for name in &names {
        let canonical = canonical_action_name(name)?;
        let m = mangle_action_name(&canonical);
        if !m.path().starts_with(&prefix) {
            return Err(format!(
                "path for {name} must start with resolution target: {}",
                m.path()
            ));
        }
        if !m.path().ends_with(m.identifier()) {
            return Err(format!(
                "path for {name} must end with identifier: {}",
                m.path()
            ));
        }
        // The hash suffix must match an independent hash12 call.
        if m.hash() != hash12(name) {
            return Err(format!(
                "hash mismatch for {name}: expected {}, got {}",
                hash12(name),
                m.hash()
            ));
        }
    }
    Ok(())
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
