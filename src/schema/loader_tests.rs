//! Unit tests for schema document loading.

use rstest::*;

use super::*;

/// Minimal valid YAML for a theorem document.
const MINIMAL_YAML: &str = r"
Theorem: Minimal
About: The simplest valid theorem
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

/// Parsed `valid_full.theorem` fixture document.
#[fixture]
fn full_doc() -> TheoremDoc {
    let yaml =
        std::fs::read_to_string("tests/fixtures/valid_full.theorem").expect("should read fixture");
    let docs = load_theorem_docs(&yaml).expect("should parse fixture");
    docs.into_iter()
        .next()
        .expect("fixture should have one doc")
}

#[rstest]
fn load_single_minimal_document() {
    let docs = load_theorem_docs(MINIMAL_YAML).expect("should parse");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("Minimal"));
}

#[rstest]
fn load_multi_document_file() {
    let yaml = r"
Theorem: First
About: First theorem
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
---
Theorem: Second
About: Second theorem
Prove:
  - assert: 'false'
    because: expected to fail
Evidence:
  kani:
    unwind: 5
    expect: FAILURE
Witness:
  - cover: 'true'
    because: always reachable
";
    let docs = load_theorem_docs(yaml).expect("should parse");
    assert_eq!(docs.len(), 2);
    assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("First"));
    assert_eq!(docs.get(1).map(|d| d.theorem.as_str()), Some("Second"));
}

#[rstest]
fn reject_unknown_top_level_key() {
    let yaml = r"
Theorem: Bad
About: Has an unknown key
UnknownKey: oops
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("unknown field"));
}

#[rstest]
fn reject_wrong_scalar_type_for_tags() {
    let yaml = r"
Theorem: Bad
About: Tags should be a list
Tags: not_a_list
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
}

#[rstest]
fn reject_missing_required_field_theorem() {
    let yaml = r"
About: Missing Theorem field
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
}

#[rstest]
fn reject_rust_keyword_theorem_name() {
    let yaml = r"
Theorem: fn
About: Theorem named after a keyword
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
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("Rust reserved keyword"));
}

#[rstest]
fn accept_lowercase_aliases() {
    let yaml = r"
theorem: LowercaseKeys
about: All keys use lowercase aliases
tags: [test]
given:
  - some context
prove:
  - assert: 'true'
    because: trivially true
evidence:
  kani:
    unwind: 1
    expect: SUCCESS
witness:
  - cover: 'true'
    because: always reachable
";
    let docs = load_theorem_docs(yaml).expect("should parse");
    assert_eq!(docs.len(), 1);
    assert_eq!(
        docs.first().map(|d| d.theorem.as_str()),
        Some("LowercaseKeys")
    );
}

#[rstest]
fn reject_invalid_identifier_in_forall() {
    let yaml = r"
Theorem: Bad
About: Forall key is invalid
Forall:
  123bad: u64
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
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
}

#[rstest]
fn reject_missing_witness_when_kani_not_vacuous() {
    let yaml = r"
Theorem: NoWitness
About: Missing witness with kani default
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("Witness section must contain at least one witness"));
}

#[rstest]
fn reject_missing_witness_when_kani_explicitly_not_vacuous() {
    let yaml = r"
Theorem: NoWitnessExplicit
About: Missing witness with allow_vacuous set to false
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    allow_vacuous: false
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("Witness section must contain at least one witness"));
}

#[rstest]
fn accept_missing_witness_when_kani_vacuous() {
    let yaml = r"
Theorem: VacuousOk
About: No witness needed when vacuous
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    allow_vacuous: true
    vacuity_because: intentionally vacuous
";
    let docs = load_theorem_docs(yaml).expect("should parse");
    assert_eq!(docs.len(), 1);
}

#[rstest]
fn reject_vacuous_without_reason() {
    let yaml = r"
Theorem: BadVacuous
About: Vacuous without reason
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    allow_vacuous: true
Witness:
  - cover: 'true'
    because: always reachable
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(msg.contains("vacuity_because is required"));
}

#[rstest]
fn load_full_example_populates_all_sections(full_doc: TheoremDoc) {
    assert_eq!(full_doc.theorem.as_str(), "FullExample");
    assert_eq!(full_doc.tags.len(), 2);
    assert_eq!(full_doc.given.len(), 2);
    assert_eq!(full_doc.forall.len(), 1);
    assert_eq!(full_doc.assume.len(), 1);
    assert_eq!(full_doc.witness.len(), 1);
    assert_eq!(full_doc.let_bindings.len(), 2);
    assert_eq!(full_doc.do_steps.len(), 2);
    assert_eq!(full_doc.prove.len(), 2);
}

#[rstest]
fn parse_diagnostics_include_explicit_source() {
    let yaml = "Theorem: T\nAbout: bad\nUnknown: key\n";
    let result = load_theorem_docs_with_source("tests/fixtures/invalid_unknown_key.theorem", yaml);
    assert!(result.is_err(), "fixture should fail parsing");

    let error = result.expect_err("error expected");
    let diagnostic = error.diagnostic().expect("diagnostic expected");
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_unknown_key.theorem"
    );
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}

#[rstest]
fn validation_diagnostics_include_source_and_location() {
    let yaml = r"
Theorem: InvalidAbout
About: ''
Prove:
  - assert: 'true'
    because: trivial
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: reachable
";
    let result = load_theorem_docs_with_source("tests/fixtures/invalid_empty_about.theorem", yaml);
    assert!(result.is_err(), "fixture should fail validation");

    let error = result.expect_err("error expected");
    let diagnostic = error.diagnostic().expect("diagnostic expected");
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_empty_about.theorem"
    );
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}
