//! Action name collision detection across loaded theorem documents.
//!
//! This module detects two collision classes before code generation:
//!
//! 1. **Duplicate canonical action names** — the same dot-separated action
//!    name string collected from two or more distinct theorem documents.
//!    Within a single loaded file, the unique set is inherently deduplicated;
//!    this check supports future cross-file collision detection.
//! 2. **Duplicate mangled identifiers** — two different canonical names
//!    that produce the same mangled Rust identifier. This is a defensive
//!    safety net since mangling is injective by design.
//!
//! Collision detection is a cross-cutting concern that wires together
//! `crate::schema` (document traversal) and `crate::mangle` (identifier
//! generation). It is intentionally placed outside both modules to
//! preserve the architectural boundary defined in ADR-003.

use std::collections::{BTreeMap, BTreeSet};

use crate::mangle::mangle_action_name;
use crate::schema::{LetBinding, SchemaError, Step, TheoremDoc};

// ── Public entry point ──────────────────────────────────────────────

/// Checks for action name collisions across loaded theorem documents.
///
/// Detects two collision classes:
///
/// 1. Duplicate canonical action names: the same canonical dot-separated
///    action name referenced from two or more distinct theorem documents.
/// 2. Duplicate mangled identifiers: two different canonical action names
///    that produce the same mangled Rust identifier.
///
/// # Errors
///
/// Returns [`SchemaError::DuplicateActionName`] listing all colliding
/// names when any collision is detected.
///
/// # Examples
///
///     use theoremc::schema::load_theorem_docs;
///     use theoremc::collision::check_action_collisions;
///
///     let yaml = r#"
///     Theorem: A
///     About: first
///     Prove:
///       - assert: "true"
///         because: trivial
///     Evidence:
///       kani:
///         unwind: 1
///         expect: SUCCESS
///     Witness:
///       - cover: "true"
///         because: reachable
///     "#;
///     let docs = load_theorem_docs(yaml).unwrap();
///     assert!(check_action_collisions(&docs).is_ok());
pub fn check_action_collisions(docs: &[TheoremDoc]) -> Result<(), SchemaError> {
    let occurrences = collect_all_occurrences(docs);
    let by_canonical = group_by_canonical(&occurrences);
    let unique_names = unique_canonical_names(&by_canonical);
    let mangled_collisions = find_mangled_collisions(&unique_names);

    if mangled_collisions.is_empty() {
        return Ok(());
    }

    Err(SchemaError::DuplicateActionName {
        message: format_collision_message(&mangled_collisions),
    })
}

// ── Action-name collection ──────────────────────────────────────────

/// A single occurrence of a canonical action name within a theorem.
struct ActionOccurrence<'a> {
    /// The canonical dot-separated action name.
    canonical: &'a str,
    /// The theorem name where this action was referenced.
    theorem: &'a str,
}

/// Collects all canonical action name occurrences from all documents.
fn collect_all_occurrences(docs: &[TheoremDoc]) -> Vec<ActionOccurrence<'_>> {
    let mut out = Vec::new();
    for doc in docs {
        collect_doc_actions(doc, &mut out);
    }
    out
}

/// Collects all canonical action name occurrences from a single
/// theorem document, traversing both `Let` bindings and `Do` steps.
fn collect_doc_actions<'a>(doc: &'a TheoremDoc, out: &mut Vec<ActionOccurrence<'a>>) {
    let theorem = doc.theorem.as_str();

    for binding in doc.let_bindings.values() {
        let action_name = let_binding_action(binding);
        out.push(ActionOccurrence {
            canonical: action_name,
            theorem,
        });
    }

    collect_step_actions(&doc.do_steps, theorem, out);
}

/// Extracts the canonical action name from a `LetBinding`.
fn let_binding_action(binding: &LetBinding) -> &str {
    match binding {
        LetBinding::Call(c) => &c.call.action,
        LetBinding::Must(m) => &m.must.action,
    }
}

/// Recursively collects action names from a step list, including
/// nested `maybe` blocks.
fn collect_step_actions<'a>(
    steps: &'a [Step],
    theorem: &'a str,
    out: &mut Vec<ActionOccurrence<'a>>,
) {
    for step in steps {
        match step {
            Step::Call(c) => {
                out.push(ActionOccurrence {
                    canonical: &c.call.action,
                    theorem,
                });
            }
            Step::Must(m) => {
                out.push(ActionOccurrence {
                    canonical: &m.must.action,
                    theorem,
                });
            }
            Step::Maybe(s) => {
                collect_step_actions(&s.maybe.do_steps, theorem, out);
            }
        }
    }
}

// ── Grouping and collision detection ────────────────────────────────

/// Groups action occurrences by canonical name, mapping each to the
/// set of theorem names that reference it. Uses `BTreeMap` for
/// deterministic iteration order.
fn group_by_canonical<'a>(
    occurrences: &[ActionOccurrence<'a>],
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    let mut map: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for occ in occurrences {
        map.entry(occ.canonical).or_default().insert(occ.theorem);
    }
    map
}

/// Extracts the set of unique canonical names from the grouped map.
fn unique_canonical_names<'a>(
    by_canonical: &BTreeMap<&'a str, BTreeSet<&'a str>>,
) -> BTreeSet<&'a str> {
    by_canonical.keys().copied().collect()
}

/// Mangles each unique canonical name and groups by mangled identifier.
/// Returns only groups where two or more different canonical names
/// produce the same mangled identifier.
fn find_mangled_collisions(canonical_names: &BTreeSet<&str>) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_identifier: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for &name in canonical_names {
        let mangled = mangle_action_name(name);
        by_identifier
            .entry(mangled.identifier().to_owned())
            .or_default()
            .insert(name.to_owned());
    }
    by_identifier.retain(|_, names| names.len() > 1);
    by_identifier
}

/// Formats mangled-identifier collisions into a human-readable error
/// message listing all colliding canonical names per mangled identifier.
fn format_collision_message(mangled_collisions: &BTreeMap<String, BTreeSet<String>>) -> String {
    let mut parts = Vec::new();
    for (identifier, names) in mangled_collisions {
        let name_list: Vec<&str> = names.iter().map(String::as_str).collect();
        parts.push(format!(
            concat!(
                "mangled identifier '{identifier}' is produced by ",
                "multiple canonical names: {names}",
            ),
            identifier = identifier,
            names = name_list.join(", "),
        ));
    }
    parts.join("; ")
}

#[cfg(test)]
mod tests {
    //! Unit tests for action name collision detection.

    use super::*;
    use crate::schema::{
        ActionCall, Assertion, Evidence, KaniEvidence, KaniExpectation, LetCall, StepCall,
        StepMaybe, StepMust, TheoremDoc, TheoremName, WitnessCheck,
    };
    use indexmap::IndexMap;
    use rstest::rstest;

    /// Builds a minimal valid `TheoremDoc` with the given name and
    /// action names in `Let` bindings.
    fn doc_with_let_actions(name: &str, actions: &[&str]) -> TheoremDoc {
        let mut let_bindings = IndexMap::new();
        for (i, action) in actions.iter().enumerate() {
            let_bindings.insert(
                format!("binding_{i}"),
                LetBinding::Call(LetCall {
                    call: action_call(action),
                }),
            );
        }
        theorem_doc(name, let_bindings, Vec::new())
    }

    /// Builds a minimal valid `TheoremDoc` with the given name and
    /// action names in `Do` steps.
    fn doc_with_do_actions(name: &str, actions: &[&str]) -> TheoremDoc {
        let steps: Vec<Step> = actions
            .iter()
            .map(|a| {
                Step::Call(StepCall {
                    call: action_call(a),
                })
            })
            .collect();
        theorem_doc(name, IndexMap::new(), steps)
    }

    /// Builds an `ActionCall` with the given action name and empty args.
    fn action_call(name: &str) -> ActionCall {
        ActionCall {
            action: name.to_owned(),
            args: IndexMap::new(),
            as_binding: None,
        }
    }

    /// Builds a minimal valid `TheoremDoc` with custom bindings and
    /// steps.
    fn theorem_doc(
        name: &str,
        let_bindings: IndexMap<String, LetBinding>,
        do_steps: Vec<Step>,
    ) -> TheoremDoc {
        TheoremDoc {
            schema: None,
            theorem: TheoremName::new(name.to_owned()).expect("valid theorem name"),
            about: "test theorem".to_owned(),
            tags: Vec::new(),
            given: Vec::new(),
            forall: IndexMap::new(),
            assume: Vec::new(),
            witness: vec![WitnessCheck {
                cover: "true".to_owned(),
                because: "reachable".to_owned(),
            }],
            let_bindings,
            do_steps,
            prove: vec![Assertion {
                assert_expr: "true".to_owned(),
                because: "trivial".to_owned(),
            }],
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
        }
    }

    // ── Collection tests ─────────────────────────────────────────────

    #[test]
    fn collect_from_let_bindings() {
        let doc = doc_with_let_actions("T", &["account.deposit", "account.withdraw"]);
        let mut out = Vec::new();
        collect_doc_actions(&doc, &mut out);
        let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
        assert_eq!(names, vec!["account.deposit", "account.withdraw"]);
    }

    #[test]
    fn collect_from_do_steps() {
        let doc = doc_with_do_actions("T", &["hnsw.attach_node", "hnsw.detach_node"]);
        let mut out = Vec::new();
        collect_doc_actions(&doc, &mut out);
        let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
        assert_eq!(names, vec!["hnsw.attach_node", "hnsw.detach_node"]);
    }

    #[test]
    fn collect_from_nested_maybe() {
        let inner_step = Step::Must(StepMust {
            must: action_call("inner.action"),
        });
        let maybe = Step::Maybe(StepMaybe {
            maybe: crate::schema::MaybeBlock {
                because: "optional branch".to_owned(),
                do_steps: vec![inner_step],
            },
        });
        let doc = theorem_doc("T", IndexMap::new(), vec![maybe]);
        let mut out = Vec::new();
        collect_doc_actions(&doc, &mut out);
        let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
        assert_eq!(names, vec!["inner.action"]);
    }

    #[test]
    fn collect_from_let_and_do_combined() {
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
        let doc = theorem_doc("T", let_bindings, steps);
        let mut out = Vec::new();
        collect_doc_actions(&doc, &mut out);
        let names: Vec<&str> = out.iter().map(|o| o.canonical).collect();
        assert_eq!(names, vec!["account.deposit", "account.validate"]);
    }

    // ── Collision detection tests ────────────────────────────────────

    #[test]
    fn no_collisions_returns_ok() {
        let docs = vec![
            doc_with_let_actions("Alpha", &["account.deposit"]),
            doc_with_do_actions("Beta", &["hnsw.attach_node"]),
        ];
        assert!(check_action_collisions(&docs).is_ok());
    }

    #[test]
    fn same_action_across_theorems_is_accepted() {
        // Multiple theorems referencing the same action is normal usage.
        let docs = vec![
            doc_with_let_actions("Alpha", &["account.deposit"]),
            doc_with_do_actions("Beta", &["account.deposit"]),
        ];
        assert!(check_action_collisions(&docs).is_ok());
    }

    #[test]
    fn same_action_within_one_theorem_is_accepted() {
        // Calling the same action twice in one theorem is normal.
        let doc = doc_with_do_actions("T", &["account.deposit", "account.deposit"]);
        assert!(check_action_collisions(&[doc]).is_ok());
    }

    #[test]
    fn empty_docs_returns_ok() {
        assert!(check_action_collisions(&[]).is_ok());
    }

    #[rstest]
    fn doc_without_actions_returns_ok() {
        let doc = theorem_doc("T", IndexMap::new(), Vec::new());
        assert!(check_action_collisions(&[doc]).is_ok());
    }

    // ── Mangled collision detection ──────────────────────────────────

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

    // ── Grouping tests ──────────────────────────────────────────────

    #[test]
    fn group_by_canonical_deduplicates_within_theorem() {
        let occurrences = vec![
            ActionOccurrence {
                canonical: "account.deposit",
                theorem: "T",
            },
            ActionOccurrence {
                canonical: "account.deposit",
                theorem: "T",
            },
        ];
        let grouped = group_by_canonical(&occurrences);
        assert_eq!(grouped.len(), 1);
        let theorems = grouped.get("account.deposit").expect("key should exist");
        assert_eq!(theorems.len(), 1, "same theorem should deduplicate");
    }

    #[test]
    fn group_by_canonical_tracks_multiple_theorems() {
        let occurrences = vec![
            ActionOccurrence {
                canonical: "account.deposit",
                theorem: "Alpha",
            },
            ActionOccurrence {
                canonical: "account.deposit",
                theorem: "Beta",
            },
        ];
        let grouped = group_by_canonical(&occurrences);
        let theorems = grouped.get("account.deposit").expect("key should exist");
        assert_eq!(theorems.len(), 2);
    }
}
