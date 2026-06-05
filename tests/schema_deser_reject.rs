//! Integration tests for unhappy-path deserialization and edge cases.
//!
//! These tests verify that invalid `.theorem` documents produce appropriate
//! errors, cover `KaniExpectation` variants, and exercise inline edge cases
//! for unknown keys in subordinate structures.

mod common;

use common::fixture_error_message;
use rstest::rstest;
use theoremc::schema::{KaniExpectation, load_theorem_docs};

// ── Unhappy-path tests ──────────────────────────────────────────────

#[test]
fn rejects_unknown_top_level_key() {
    let msg = fixture_error_message("invalid_unknown_key.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        msg.contains("unknown field"),
        "error should mention unknown field, got: {msg}"
    );
}

#[test]
fn rejects_wrong_scalar_type_for_tags() {
    fixture_error_message("invalid_wrong_type.theorem").unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn rejects_missing_theorem_field() {
    fixture_error_message("invalid_missing_theorem.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn rejects_missing_about_field() {
    fixture_error_message("invalid_missing_about.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn rejects_missing_prove_field() {
    fixture_error_message("invalid_missing_prove.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn rejects_missing_evidence_field() {
    fixture_error_message("invalid_missing_evidence.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn rejects_rust_keyword_theorem_name() {
    let msg = fixture_error_message("invalid_keyword_name.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        msg.contains("Rust reserved keyword"),
        "error should mention keyword, got: {msg}"
    );
}

#[test]
fn rejects_invalid_identifier_starting_with_digit() {
    let msg = fixture_error_message("invalid_bad_identifier.theorem")
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        msg.contains("must match the pattern"),
        "error should mention pattern, got: {msg}"
    );
}

#[test]
fn rejects_invalid_kani_expect_value() {
    fixture_error_message("invalid_bad_expect.theorem").unwrap_or_else(|error| panic!("{error}"));
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
