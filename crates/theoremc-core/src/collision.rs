//! Mangled-identifier collision detection across loaded theorem documents.
//!
//! This module detects **mangled-identifier collisions** before code
//! generation: two different canonical action names that produce the same
//! mangled Rust identifier. This is a defensive safety net: the mangling
//! algorithm is designed to be collision-resistant, but the fixed-width
//! hash suffix means it cannot be strictly one-to-one.
//!
//! Multiple theorems referencing the same canonical action name is
//! expected and accepted — only distinct canonical names that collide
//! after mangling trigger an error.
//!
//! Collision detection is a cross-cutting concern that wires together
//! `crate::schema` (document traversal) and `crate::mangle` (identifier
//! generation). It is intentionally placed outside both modules to
//! preserve the architectural boundary defined in ADR-003.

use std::collections::{BTreeMap, BTreeSet};

use crate::mangle::mangle_action_name;
use crate::schema::{LetBinding, SchemaError, Step, TheoremDoc};

/// Mangles a canonical action name string and returns the identifier.
fn mangle_to_identifier(name: &str) -> String {
    mangle_action_name(name).identifier().to_owned()
}

// ── Public entry point ──────────────────────────────────────────────

/// Checks for mangled-identifier collisions across loaded theorem
/// documents.
///
/// Collects all canonical action names, mangles each one, and reports
/// an error when two or more different canonical names produce the
/// same mangled Rust identifier. Multiple theorems referencing the
/// same canonical name is accepted and does not trigger a collision.
///
/// # Errors
///
/// Returns [`SchemaError::MangledIdentifierCollision`] listing all
/// colliding canonical names per mangled identifier.
///
/// # Examples
///
///     use theoremc_core::schema::load_theorem_docs;
///     use theoremc_core::collision::check_action_collisions;
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
///     let docs = load_theorem_docs(yaml).expect("failed to load theorem docs");
///     assert!(check_action_collisions(&docs).is_ok());
pub fn check_action_collisions(docs: &[TheoremDoc]) -> Result<(), SchemaError> {
    check_action_collisions_with(docs, mangle_to_identifier)
}

/// Checks for mangled-identifier collisions using a caller-supplied
/// mangler function, allowing tests to inject deterministic manglers
/// that force collisions.
fn check_action_collisions_with(
    docs: &[TheoremDoc],
    mangler: impl Fn(&str) -> String,
) -> Result<(), SchemaError> {
    let occurrences = collect_all_occurrences(docs);
    let by_canonical = group_by_canonical(&occurrences);
    let unique_names = unique_canonical_names(&by_canonical);
    let mangled_collisions = find_mangled_collisions_with(&unique_names, mangler);

    if mangled_collisions.is_empty() {
        return Ok(());
    }

    Err(SchemaError::MangledIdentifierCollision {
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

/// Iteratively collects action names from a step list, including
/// nested `maybe` blocks, using an explicit stack to avoid
/// unbounded recursion on deeply nested inputs.
fn collect_step_actions<'a>(
    steps: &'a [Step],
    theorem: &'a str,
    out: &mut Vec<ActionOccurrence<'a>>,
) {
    let mut stack: Vec<&'a Step> = steps.iter().rev().collect();
    while let Some(step) = stack.pop() {
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
                for nested in s.maybe.do_steps.iter().rev() {
                    stack.push(nested);
                }
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

/// Mangles each unique canonical name using the production mangler and
/// groups by mangled identifier. Returns only groups where two or more
/// different canonical names produce the same mangled identifier.
#[cfg(test)]
fn find_mangled_collisions(canonical_names: &BTreeSet<&str>) -> BTreeMap<String, BTreeSet<String>> {
    find_mangled_collisions_with(canonical_names, mangle_to_identifier)
}

/// Groups canonical names by the identifier produced by `mangler`,
/// returning only groups where two or more names collide.
fn find_mangled_collisions_with(
    canonical_names: &BTreeSet<&str>,
    mangler: impl Fn(&str) -> String,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_identifier: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for &name in canonical_names {
        by_identifier
            .entry(mangler(name))
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
#[path = "collision_tests.rs"]
mod tests;
