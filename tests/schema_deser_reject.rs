//! Integration tests for unhappy-path deserialization and edge cases.
//!
//! These tests verify that invalid `.theorem` documents produce appropriate
//! errors, cover `KaniExpectation` variants, and exercise inline edge cases
//! for unknown keys in subordinate structures.

mod common;

use common::load_fixture;
use rstest::rstest;
use theoremc::schema::{KaniExpectation, load_theorem_docs};

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

#[rstest]
#[case::success("SUCCESS", KaniExpectation::Success)]
#[case::failure("FAILURE", KaniExpectation::Failure)]
#[case::unreachable("UNREACHABLE", KaniExpectation::Unreachable)]
#[case::undetermined("UNDETERMINED", KaniExpectation::Undetermined)]
fn kani_expect_variant_roundtrips(#[case] yaml_value: &str, #[case] expected: KaniExpectation) {
    let yaml = make_doc_with_expect(yaml_value);
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let kani = docs
        .first()
        .and_then(|d| d.evidence.kani.as_ref())
        .expect("should have kani evidence");
    assert_eq!(kani.expect, expected);
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
Witness:
  - cover: 'true'
    because: always reachable
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
Witness:
  - cover: 'true'
    because: always reachable
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
Witness:
  - cover: 'true'
    because: always reachable
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

#[rstest]
#[case::long_camel("BidirectionalLinksCommitPath3Nodes")]
#[case::inverse("DepositWithdrawInverse")]
#[case::snake("hnsw_smoke")]
#[case::underscore_prefix("_internal")]
fn doc_example_identifiers_accepted(#[case] name: &str) {
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
