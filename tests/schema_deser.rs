//! Integration tests for theorem document deserialization.
//!
//! These tests load `.theorem` fixture files and verify that valid
//! documents deserialize correctly. Unhappy-path tests live in
//! `schema_deser_reject.rs`.

mod common;

use common::load_fixture;
use theoremc::schema::{LetBinding, Step, load_theorem_docs};

// ── Happy-path tests ────────────────────────────────────────────────

#[test]
fn valid_minimal_document_deserializes() {
    let yaml = load_fixture("valid_minimal.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse valid_minimal");
    assert_eq!(docs.len(), 1);
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.theorem.as_str(), "Minimal");
    assert_eq!(doc.about, "The simplest valid theorem");
    assert!(doc.tags.is_empty());
    assert!(doc.given.is_empty());
    assert!(doc.forall.is_empty());
    assert!(doc.assume.is_empty());
}

#[test]
fn valid_minimal_has_required_prove() {
    let yaml = load_fixture("valid_minimal.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.prove.len(), 1);
    assert_eq!(
        doc.prove.first().map(|p| p.assert_expr.as_str()),
        Some("true")
    );
    assert_eq!(
        doc.prove.first().map(|p| p.because.as_str()),
        Some("trivially true")
    );
}

#[test]
fn valid_minimal_has_kani_evidence() {
    use theoremc::schema::KaniExpectation;

    let yaml = load_fixture("valid_minimal.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    let kani = doc
        .evidence
        .kani
        .as_ref()
        .expect("should have kani evidence");
    assert_eq!(kani.unwind, 1);
    assert_eq!(kani.expect, KaniExpectation::Success);
    assert!(!kani.allow_vacuous);
    assert!(kani.vacuity_because.is_none());
}

#[test]
fn valid_full_populates_all_sections() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse valid_full");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.theorem.as_str(), "FullExample");
    assert_eq!(doc.schema, Some(1));
    assert_eq!(doc.tags, vec!["integration", "example"]);
    assert_eq!(doc.given.len(), 2);
    assert!(doc.forall.contains_key("amount"));
}

#[test]
fn valid_full_has_let_bindings() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.let_bindings.len(), 2);
    assert!(doc.let_bindings.contains_key("params"));
    assert!(doc.let_bindings.contains_key("result"));

    assert!(matches!(
        doc.let_bindings.get("params"),
        Some(LetBinding::Must(..))
    ));
    assert!(matches!(
        doc.let_bindings.get("result"),
        Some(LetBinding::Call(..))
    ));
}

#[test]
fn valid_full_has_maybe_step() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.do_steps.len(), 2);
    assert!(matches!(doc.do_steps.first(), Some(Step::Must(..))));
    assert!(matches!(doc.do_steps.get(1), Some(Step::Maybe(..))));
}

#[test]
fn valid_full_has_multiple_prove_assertions() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.prove.len(), 2);
}

// ── Multi-document tests ────────────────────────────────────────────

#[test]
fn multi_document_loads_all_theorems() {
    let yaml = load_fixture("valid_multi.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse valid_multi");
    assert_eq!(docs.len(), 3);
}

#[test]
fn multi_document_preserves_order() {
    let yaml = load_fixture("valid_multi.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let names: Vec<&str> = docs.iter().map(|d| d.theorem.as_str()).collect();
    assert_eq!(names, vec!["FirstTheorem", "SecondTheorem", "ThirdTheorem"]);
}

#[test]
fn multi_document_has_independent_sections() {
    let yaml = load_fixture("valid_multi.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    // Only the second document has tags.
    assert!(docs.first().is_some_and(|d| d.tags.is_empty()));
    let second_tags: Vec<&str> = docs
        .get(1)
        .map(|d| d.tags.iter().map(String::as_str).collect())
        .unwrap_or_default();
    assert_eq!(second_tags, vec!["smoke"]);
}

// ── Lowercase alias tests ───────────────────────────────────────────

#[test]
fn lowercase_aliases_deserialize_identically() {
    let yaml = load_fixture("valid_lowercase.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse lowercase");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.theorem.as_str(), "LowercaseAliases");
    assert_eq!(doc.tags, vec!["test", "alias"]);
    assert_eq!(doc.forall.len(), 1);
    assert_eq!(doc.assume.len(), 1);
    assert_eq!(doc.witness.len(), 1);
}

// ── Vacuous configuration test ──────────────────────────────────────

#[test]
fn vacuous_allowed_with_reason() {
    let yaml = load_fixture("valid_vacuous.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse vacuous");
    let doc = docs.first().expect("should have one document");
    let kani = doc.evidence.kani.as_ref().expect("should have kani");
    assert!(kani.allow_vacuous);
    assert!(kani.vacuity_because.is_some());
}

// ── Schema version test ─────────────────────────────────────────────

#[test]
fn schema_version_defaults_to_none() {
    let yaml = load_fixture("valid_minimal.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.schema, None);
}

#[test]
fn schema_version_can_be_set() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.schema, Some(1));
}

// ── Edge case: empty optional fields ────────────────────────────────

#[test]
fn empty_optional_fields_default_correctly() {
    let yaml = "
Theorem: EmptyOptionals
About: Only required fields
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
";
    let docs = load_theorem_docs(yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert!(doc.tags.is_empty());
    assert!(doc.given.is_empty());
    assert!(doc.forall.is_empty());
    assert!(doc.assume.is_empty());
    assert!(doc.let_bindings.is_empty());
    assert!(doc.do_steps.is_empty());
}
