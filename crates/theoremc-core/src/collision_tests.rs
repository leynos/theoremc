//! Unit tests for mangled-identifier collision detection.

use super::*;
use crate::schema::{
    ActionCall, Assertion, Evidence, KaniEvidence, KaniExpectation, LetCall, StepCall, StepMaybe,
    StepMust, TheoremDoc, TheoremName, WitnessCheck,
};
use indexmap::IndexMap;
use rstest::rstest;

// ── rstest fixtures ─────────────────────────────────────────────────

/// Shared boilerplate fields required by every `TheoremDoc` in tests.
#[derive(Clone)]
struct DocBoilerplate {
    evidence: Evidence,
    assertions: Vec<Assertion>,
    witnesses: Vec<WitnessCheck>,
}

/// Minimal valid boilerplate for constructing test documents.
#[rstest::fixture]
fn boilerplate() -> DocBoilerplate {
    DocBoilerplate {
        evidence: Evidence {
            kani: Some(KaniEvidence {
                unwind: 1,
                expect: KaniExpectation::Success,
                allow_vacuous: false,
                vacuity_because: None,
            }),
            verus: None,
            stateright: None,
        },
        assertions: vec![Assertion {
            assert_expr: "true".to_owned(),
            because: "trivial".to_owned(),
        }],
        witnesses: vec![WitnessCheck {
            cover: "true".to_owned(),
            because: "reachable".to_owned(),
        }],
    }
}

// ── Test builders ───────────────────────────────────────────────────

/// Builds an `ActionCall` with the given action name and empty args.
fn action_call(name: &str) -> ActionCall {
    ActionCall {
        action: name.to_owned(),
        args: IndexMap::new(),
        as_binding: None,
    }
}

/// Builds a minimal valid `TheoremDoc` with custom bindings and steps.
fn theorem_doc(
    name: &str,
    let_bindings: IndexMap<String, LetBinding>,
    do_steps: Vec<Step>,
    bp: &DocBoilerplate,
) -> TheoremDoc {
    TheoremDoc {
        schema: None,
        theorem: TheoremName::new(name.to_owned()).expect("valid theorem name"),
        about: "test theorem".to_owned(),
        tags: Vec::new(),
        given: Vec::new(),
        forall: IndexMap::new(),
        assume: Vec::new(),
        witness: bp.witnesses.clone(),
        let_bindings,
        do_steps,
        prove: bp.assertions.clone(),
        evidence: bp.evidence.clone(),
    }
}

/// Builds a `TheoremDoc` with the given action names in `Let` bindings.
fn doc_with_let_actions(name: &str, actions: &[&str], bp: &DocBoilerplate) -> TheoremDoc {
    let mut let_bindings = IndexMap::new();
    for (i, action) in actions.iter().enumerate() {
        let_bindings.insert(
            format!("binding_{i}"),
            LetBinding::Call(LetCall {
                call: action_call(action),
            }),
        );
    }
    theorem_doc(name, let_bindings, Vec::new(), bp)
}

/// Builds a `TheoremDoc` with the given action names in `Do` steps.
fn doc_with_do_actions(name: &str, actions: &[&str], bp: &DocBoilerplate) -> TheoremDoc {
    let steps: Vec<Step> = actions
        .iter()
        .map(|a| {
            Step::Call(StepCall {
                call: action_call(a),
            })
        })
        .collect();
    theorem_doc(name, IndexMap::new(), steps, bp)
}

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
        LetBinding::Call(LetCall {
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
    let collisions = find_mangled_collisions(&names);
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

    let always_collide = |_: &str| "colliding__identifier__h000000000000".to_owned();
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

    use proptest::prelude::{prop_assert, proptest};

    use super::super::find_mangled_collisions;

    proptest! {
        /// The collision detector must report no collisions for a single
        /// distinct canonical name (trivially no collision possible).
        #[test]
        fn single_canonical_name_never_collides(
            name in "[a-z][a-z0-9]{0,8}\\.[a-z][a-z0-9]{0,8}",
        ) {
            let names: BTreeSet<&str> = std::iter::once(name.as_str()).collect();
            let collisions = find_mangled_collisions(&names);
            prop_assert!(
                collisions.is_empty(),
                "single name '{name}' should not self-collide",
            );
        }

        /// Two identical canonical names must not be reported as a collision
        /// (same name -> same identifier, but that is not a collision by
        /// definition).
        #[test]
        fn identical_names_do_not_collide(
            name in "[a-z][a-z0-9]{0,8}\\.[a-z][a-z0-9]{0,8}",
        ) {
            // `BTreeSet` deduplicates, so this is equivalent to a single-name set.
            let mut names: BTreeSet<&str> = BTreeSet::new();
            names.insert(name.as_str());
            let collisions = find_mangled_collisions(&names);
            prop_assert!(collisions.is_empty());
        }
    }
}
