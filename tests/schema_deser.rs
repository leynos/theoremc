//! Integration tests for theorem document deserialization.
//!
//! These tests load `.theorem` fixture files and verify that valid
//! documents deserialize correctly and invalid documents produce
//! appropriate errors.

use theoremc::schema::{KaniExpectation, LetBinding, Step, load_theorem_docs};

/// Loads a fixture file from the `tests/fixtures/` directory.
fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}"))
        .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

// ── Happy-path tests ────────────────────────────────────────────────

#[test]
fn valid_minimal_document_deserializes() {
    let yaml = load_fixture("valid_minimal.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse valid_minimal");
    assert_eq!(docs.len(), 1);
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.theorem, "Minimal");
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
    assert_eq!(doc.theorem, "FullExample");
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
        Some(LetBinding::Must { .. })
    ));
    assert!(matches!(
        doc.let_bindings.get("result"),
        Some(LetBinding::Call { .. })
    ));
}

#[test]
fn valid_full_has_maybe_step() {
    let yaml = load_fixture("valid_full.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let doc = docs.first().expect("should have one document");
    assert_eq!(doc.do_steps.len(), 2);
    assert!(matches!(doc.do_steps.first(), Some(Step::Must { .. })));
    assert!(matches!(doc.do_steps.get(1), Some(Step::Maybe { .. })));
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
    assert_eq!(doc.theorem, "LowercaseAliases");
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

/// Helper to assert that loading a fixture fails.
fn assert_fixture_fails(fixture_name: &str) -> String {
    let yaml = load_fixture(fixture_name);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_err(),
        "expected {fixture_name} to fail deserialization"
    );
    result.err().map(|e| e.to_string()).unwrap_or_default()
}

// ── Unhappy-path tests ──────────────────────────────────────────────

#[test]
fn rejects_unknown_top_level_key() {
    let msg = assert_fixture_fails("invalid_unknown_key.theorem");
    assert!(
        msg.contains("unknown field"),
        "error should mention unknown field, got: {msg}"
    );
}

#[test]
fn rejects_wrong_scalar_type_for_tags() {
    assert_fixture_fails("invalid_wrong_type.theorem");
}

#[test]
fn rejects_missing_theorem_field() {
    assert_fixture_fails("invalid_missing_theorem.theorem");
}

#[test]
fn rejects_missing_about_field() {
    assert_fixture_fails("invalid_missing_about.theorem");
}

#[test]
fn rejects_missing_prove_field() {
    assert_fixture_fails("invalid_missing_prove.theorem");
}

#[test]
fn rejects_missing_evidence_field() {
    assert_fixture_fails("invalid_missing_evidence.theorem");
}

#[test]
fn rejects_rust_keyword_theorem_name() {
    let msg = assert_fixture_fails("invalid_keyword_name.theorem");
    assert!(
        msg.contains("Rust reserved keyword"),
        "error should mention keyword, got: {msg}"
    );
}

#[test]
fn rejects_invalid_identifier_starting_with_digit() {
    let msg = assert_fixture_fails("invalid_bad_identifier.theorem");
    assert!(
        msg.contains("must match the pattern"),
        "error should mention pattern, got: {msg}"
    );
}

#[test]
fn rejects_invalid_kani_expect_value() {
    assert_fixture_fails("invalid_bad_expect.theorem");
}

// ── KaniExpectation enum coverage ───────────────────────────────────

fn make_doc_with_expect(expect: &str) -> String {
    format!(
        "
Theorem: Test
About: Testing expect value
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: {expect}
Witness:
  - cover: 'true'
    because: always reachable
"
    )
}

#[test]
fn kani_expect_success_variant() {
    let yaml = make_doc_with_expect("SUCCESS");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let kani = docs
        .first()
        .and_then(|d| d.evidence.kani.as_ref())
        .expect("should have kani evidence");
    assert_eq!(kani.expect, KaniExpectation::Success);
}

#[test]
fn kani_expect_failure_variant() {
    let yaml = make_doc_with_expect("FAILURE");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let kani = docs
        .first()
        .and_then(|d| d.evidence.kani.as_ref())
        .expect("should have kani evidence");
    assert_eq!(kani.expect, KaniExpectation::Failure);
}

#[test]
fn kani_expect_unreachable_variant() {
    let yaml = make_doc_with_expect("UNREACHABLE");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let kani = docs
        .first()
        .and_then(|d| d.evidence.kani.as_ref())
        .expect("should have kani evidence");
    assert_eq!(kani.expect, KaniExpectation::Unreachable);
}

#[test]
fn kani_expect_undetermined_variant() {
    let yaml = make_doc_with_expect("UNDETERMINED");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let kani = docs
        .first()
        .and_then(|d| d.evidence.kani.as_ref())
        .expect("should have kani evidence");
    assert_eq!(kani.expect, KaniExpectation::Undetermined);
}

// ── Inline unhappy path edge cases ──────────────────────────────────

#[test]
fn rejects_unknown_key_in_action_call() {
    let yaml = "
Theorem: Bad
About: Unknown key inside an action call
Do:
  - call:
      action: foo.bar
      args: {}
      spurious: oops
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

#[test]
fn rejects_unknown_key_in_kani_evidence() {
    let yaml = "
Theorem: Bad
About: Unknown key inside kani evidence
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    unknown_field: true
";
    let result = load_theorem_docs(yaml);
    assert!(result.is_err());
}

#[test]
fn rejects_unknown_key_in_assumption() {
    let yaml = "
Theorem: Bad
About: Unknown key inside assumption
Assume:
  - expr: 'true'
    because: test
    extra: should not be here
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

#[test]
fn rejects_forall_key_that_is_rust_keyword() {
    let yaml = "
Theorem: Bad
About: Forall key is a Rust keyword
Forall:
  let: u64
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

// ── Guard: identifiers from the doc that should work ────────────────

#[test]
fn doc_example_identifiers_accepted() {
    // Identifiers from the design document examples
    for name in &[
        "BidirectionalLinksCommitPath3Nodes",
        "DepositWithdrawInverse",
        "hnsw_smoke",
        "_internal",
    ] {
        let yaml = format!(
            "
Theorem: {name}
About: testing identifier
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
"
        );
        let result = load_theorem_docs(&yaml);
        assert!(result.is_ok(), "identifier '{name}' should be accepted");
    }
}
