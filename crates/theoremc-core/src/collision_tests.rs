//! Unit tests for mangled-identifier collision detection.

use super::test_helpers::{
    DocBoilerplate, action_call, boilerplate, doc_with_do_actions, doc_with_let_actions,
    theorem_doc,
};
use super::*;
use crate::schema::{StepCall, StepMaybe, StepMust};
use indexmap::IndexMap;
use proptest::prelude::{Just, prop, prop_assert, prop_assume, proptest};
use rstest::rstest;

// ── Collection tests ────────────────────────────────────────────────

#[rstest]
fn collect_from_let_bindings(boilerplate: DocBoilerplate) {
    let doc = doc_with_let_actions("T", &["account.deposit", "account.withdraw"], &boilerplate);
    let mut out = Vec::new();
    collect_doc_actions(&doc, &mut out);
    let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
    assert_eq!(names, vec!["account.deposit", "account.withdraw"]);
}

#[rstest]
fn collect_from_do_steps(boilerplate: DocBoilerplate) {
    let doc = doc_with_do_actions("T", &["hnsw.attach_node", "hnsw.detach_node"], &boilerplate);
    let mut out = Vec::new();
    collect_doc_actions(&doc, &mut out);
    let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
    assert_eq!(names, vec!["hnsw.attach_node", "hnsw.detach_node"]);
}

#[rstest]
fn collect_from_nested_maybe(boilerplate: DocBoilerplate) {
    let inner_step = Step::Must(StepMust {
        must: action_call("inner.action"),
    });
    let maybe = Step::Maybe(StepMaybe {
        maybe: crate::schema::MaybeBlock {
            because: "optional branch".to_owned(),
            do_steps: vec![inner_step],
        },
    });
    let doc = theorem_doc("T", IndexMap::new(), vec![maybe], &boilerplate);
    let mut out = Vec::new();
    collect_doc_actions(&doc, &mut out);
    let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
    assert_eq!(names, vec!["inner.action"]);
}

#[rstest]
fn collect_from_let_and_do_combined(boilerplate: DocBoilerplate) {
    let mut let_bindings = IndexMap::new();
    let_bindings.insert(
        "result".to_owned(),
        LetBinding::Call(crate::schema::LetCall {
            call: action_call("account.deposit"),
        }),
    );
    let steps = vec![Step::Call(StepCall {
        call: action_call("account.validate"),
    })];
    let doc = theorem_doc("T", let_bindings, steps, &boilerplate);
    let mut out = Vec::new();
    collect_doc_actions(&doc, &mut out);
    let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
    assert_eq!(names, vec!["account.deposit", "account.validate"]);
}

// ── Collision detection tests ───────────────────────────────────────

#[rstest]
fn no_collisions_returns_ok(boilerplate: DocBoilerplate) {
    let docs = vec![
        doc_with_let_actions("Alpha", &["account.deposit"], &boilerplate),
        doc_with_do_actions("Beta", &["hnsw.attach_node"], &boilerplate),
    ];
    assert!(check_action_collisions(&docs).is_ok());
}

#[rstest]
fn same_action_across_theorems_is_accepted(boilerplate: DocBoilerplate) {
    // Multiple theorems referencing the same action is normal usage.
    let docs = vec![
        doc_with_let_actions("Alpha", &["account.deposit"], &boilerplate),
        doc_with_do_actions("Beta", &["account.deposit"], &boilerplate),
    ];
    assert!(check_action_collisions(&docs).is_ok());
}

#[rstest]
fn same_action_within_one_theorem_is_accepted(boilerplate: DocBoilerplate) {
    // Calling the same action twice in one theorem is normal.
    let doc = doc_with_do_actions("T", &["account.deposit", "account.deposit"], &boilerplate);
    assert!(check_action_collisions(&[doc]).is_ok());
}

#[test]
fn empty_docs_returns_ok() {
    assert!(check_action_collisions(&[]).is_ok());
}

#[rstest]
fn doc_without_actions_returns_ok(boilerplate: DocBoilerplate) {
    let doc = theorem_doc("T", IndexMap::new(), Vec::new(), &boilerplate);
    assert!(check_action_collisions(&[doc]).is_ok());
}

// ── Mangled collision detection ─────────────────────────────────────

#[test]
fn find_mangled_collisions_with_distinct_names_returns_empty() {
    let names: BTreeSet<&str> = ["account.deposit", "hnsw.attach_node"]
        .into_iter()
        .collect();
    let collisions = find_mangled_collisions(&names).expect("valid canonical names");
    assert!(
        collisions.is_empty(),
        "distinct names should not collide: {collisions:?}",
    );
}

#[test]
fn format_message_includes_identifier_and_names() {
    let mut collisions = BTreeMap::new();
    let mut names = BTreeSet::new();
    names.insert("alpha.beta".to_owned());
    names.insert("gamma.delta".to_owned());
    collisions.insert("fake__identifier__h000000000000".to_owned(), names);

    let msg = format_collision_message(&collisions);
    assert!(
        msg.contains("fake__identifier__h000000000000"),
        "message should include the identifier: {msg}",
    );
    assert!(
        msg.contains("alpha.beta"),
        "message should include first name: {msg}",
    );
    assert!(
        msg.contains("gamma.delta"),
        "message should include second name: {msg}",
    );
}

#[rstest]
fn crafted_collision_returns_mangled_identifier_collision_error(boilerplate: DocBoilerplate) {
    // The mangling algorithm is collision-resistant, so a real collision
    // is unlikely through normal inputs. This test injects a deterministic
    // mangler that maps all names to the same identifier, forcing the
    // collision path through the public pipeline.
    let docs = vec![
        doc_with_do_actions("Alpha", &["alpha.beta"], &boilerplate),
        doc_with_do_actions("Gamma", &["gamma.delta"], &boilerplate),
    ];

    let always_collide = |_: &str| Ok("colliding__identifier__h000000000000".to_owned());
    let result = check_action_collisions_with(&docs, always_collide);

    let error = result.expect_err("should return MangledIdentifierCollision");
    assert!(
        matches!(error, SchemaError::MangledIdentifierCollision { .. }),
        "expected MangledIdentifierCollision variant, got: {error}",
    );
    let display = error.to_string();
    assert!(
        display.contains("mangled identifier collision"),
        "error display should include variant prefix: {display}",
    );
    assert!(
        display.contains("alpha.beta"),
        "error display should include first name: {display}",
    );
    assert!(
        display.contains("gamma.delta"),
        "error display should include second name: {display}",
    );
}

// ── Grouping tests ──────────────────────────────────────────────────

#[rstest]
#[case::deduplicates_within_theorem(
    &["T", "T"],
    1,
    "same theorem should deduplicate",
)]
#[case::tracks_multiple_theorems(
    &["Alpha", "Beta"],
    2,
    "different theorems should both appear",
)]
fn group_by_canonical_behaviour(
    #[case] theorem_names: &[&str],
    #[case] expected_theorem_count: usize,
    #[case] message: &str,
) {
    let occurrences: Vec<ActionOccurrence<'_>> = theorem_names
        .iter()
        .map(|&name| ActionOccurrence {
            canonical: "account.deposit",
            theorem: name,
        })
        .collect();
    let grouped = group_by_canonical(&occurrences);
    assert_eq!(grouped.len(), 1);
    let theorems = grouped.get("account.deposit").expect("key should exist");
    assert_eq!(theorems.len(), expected_theorem_count, "{message}");
}

#[cfg(test)]
mod prop_tests {
    //! Property-based tests for collision-detection invariants.

    use std::collections::BTreeSet;

    use crate::canonical_action_name::CanonicalActionName;

    use super::super::find_mangled_collisions;
    use super::{Just, prop, prop_assert, prop_assume, proptest};

    proptest! {
        /// The collision detector must report no collisions for a single
        /// distinct canonical name (trivially no collision possible).
        #[test]
        fn single_canonical_name_never_collides(
            name in "[a-z][a-z0-9]{0,8}\\.[a-z][a-z0-9]{0,8}",
        ) {
            prop_assume!(CanonicalActionName::new(&name).is_ok());
            let names: BTreeSet<&str> = std::iter::once(name.as_str()).collect();
            let collisions = find_mangled_collisions(&names)?;
            prop_assert!(
                collisions.is_empty(),
                "single name '{name}' should not self-collide; got: {collisions:?}",
            );
        }

        /// An empty name set produces no collisions.
        #[test]
        fn empty_name_set_produces_no_collisions(
            // No generator needed; always use an empty set.
            _unused in Just(()),
        ) {
            let names: BTreeSet<&str> = BTreeSet::new();
            prop_assert!(find_mangled_collisions(&names)?.is_empty());
        }

        /// Two identical canonical names must not be reported as a collision
        /// (same name -> same identifier, but that is not a collision by
        /// definition).
        #[test]
        fn identical_names_do_not_collide(
            name in "[a-z][a-z0-9]{0,8}\\.[a-z][a-z0-9]{0,8}",
        ) {
            prop_assume!(CanonicalActionName::new(&name).is_ok());
            // `BTreeSet` deduplicates, so this is equivalent to a single-name set.
            let mut names: BTreeSet<&str> = BTreeSet::new();
            names.insert(name.as_str());
            let collisions = find_mangled_collisions(&names)?;
            prop_assert!(collisions.is_empty());
        }

        /// Two names that are equal after deduplication (i.e. the same name
        /// provided twice) produce no collisions because a `BTreeSet`
        /// deduplicates them; only genuinely distinct names can collide.
        #[test]
        fn duplicate_insertion_of_same_name_is_not_a_collision(
            name in "[a-z][a-z0-9]{0,8}\\.[a-z][a-z0-9]{0,8}",
        ) {
            prop_assume!(CanonicalActionName::new(&name).is_ok());
            let mut names: BTreeSet<&str> = BTreeSet::new();
            names.insert(name.as_str());
            names.insert(name.as_str());
            prop_assert!(find_mangled_collisions(&names)?.is_empty());
        }

        /// The number of collision groups reported is never greater than
        /// floor(n/2) for n input names, because a collision requires at least
        /// two names to share an identifier.
        #[test]
        fn collision_group_count_bounded_by_input_size(
            names in prop::collection::btree_set(
                "[a-z][a-z0-9]{0,6}\\.[a-z][a-z0-9]{0,6}",
                0..=8_usize,
            ),
        ) {
            prop_assume!(
                names
                    .iter()
                    .all(|name| CanonicalActionName::new(name).is_ok())
            );
            let name_refs: BTreeSet<&str> = names.iter().map(String::as_str).collect();
            let n = name_refs.len();
            let collisions = find_mangled_collisions(&name_refs)?;
            prop_assert!(
                collisions.len() <= n / 2,
                "expected at most {} collision groups for {} names, got {}",
                n / 2,
                n,
                collisions.len(),
            );
        }

        /// Every name reported inside a collision group must have been present
        /// in the input set; the detector must not invent phantom names.
        #[test]
        fn collision_groups_contain_only_input_names(
            names in prop::collection::btree_set(
                "[a-z][a-z0-9]{0,6}\\.[a-z][a-z0-9]{0,6}",
                0..=8_usize,
            ),
        ) {
            prop_assume!(
                names
                    .iter()
                    .all(|name| CanonicalActionName::new(name).is_ok())
            );
            let name_refs: BTreeSet<&str> = names.iter().map(String::as_str).collect();
            let collisions = find_mangled_collisions(&name_refs)?;
            for (_identifier, colliding_names) in &collisions {
                for colliding_name in colliding_names {
                    prop_assert!(
                        names.contains(colliding_name.as_str()),
                        "collision group contains unexpected name '{}' not in input {:?}",
                        colliding_name,
                        names,
                    );
                }
            }
        }
    }
}
