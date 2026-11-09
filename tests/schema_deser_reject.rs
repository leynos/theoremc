//! Integration tests for unhappy-path deserialization and edge cases.
//!
//! These tests verify that invalid `.theorem` documents produce appropriate
//! errors, cover `KaniExpectation` variants, and exercise inline edge cases
//! for unknown keys in subordinate structures.

use rstest::rstest;
use test_helpers::{FixtureName, StrNewtype, fixture_error_message};
use theoremc::schema::{KaniExpectation, LetBinding, Step, load_theorem_docs};

fn require_message_contains(message: &str, expected: &str) -> Result<(), String> {
    if message.contains(expected) {
        Ok(())
    } else {
        Err(format!("error should mention {expected:?}, got: {message}"))
    }
}

// ── Unhappy-path tests ──────────────────────────────────────────────

#[test]
fn rejects_unknown_top_level_key() -> Result<(), String> {
    let msg = fixture_error_message(FixtureName::new("invalid_unknown_key.theorem"))?;
    require_message_contains(&msg, "unknown field")
}

#[rstest]
#[case::wrong_scalar_type("invalid_wrong_type.theorem")]
#[case::missing_theorem("invalid_missing_theorem.theorem")]
#[case::missing_about("invalid_missing_about.theorem")]
#[case::missing_prove("invalid_missing_prove.theorem")]
#[case::missing_evidence("invalid_missing_evidence.theorem")]
#[case::invalid_kani_expect("invalid_bad_expect.theorem")]
fn rejects_invalid_fixture(#[case] fixture_name: &str) -> Result<(), String> {
    fixture_error_message(FixtureName::new(fixture_name))?;
    Ok(())
}

#[test]
fn rejects_rust_keyword_theorem_name() -> Result<(), String> {
    let msg = fixture_error_message(FixtureName::new("invalid_keyword_name.theorem"))?;
    require_message_contains(&msg, "Rust reserved keyword")
}

#[test]
fn rejects_invalid_identifier_starting_with_digit() -> Result<(), String> {
    let msg = fixture_error_message(FixtureName::new("invalid_bad_identifier.theorem"))?;
    require_message_contains(&msg, "must match the pattern")
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

// ── Single-key mappings in `Let` and `Do` ───────────────────────────

const DISPATCH_HEADER: &str = concat!(
    "Theorem: Dispatch\n",
    "About: Single-key mapping dispatch\n",
    "Actions:\n",
    "  account.deposit:\n",
    "    params: {}\n",
);

const DISPATCH_FOOTER: &str = concat!(
    "Prove:\n",
    "  - assert: 'true'\n",
    "    because: trivially true\n",
    "Evidence:\n",
    "  kani:\n",
    "    unwind: 1\n",
    "    expect: SUCCESS\n",
    "Witness:\n",
    "  - cover: 'true'\n",
    "    because: always reachable\n",
);

/// Renders a document with the supplied `Let` and `Do` sections.
fn document_with(let_section: &str, do_section: &str) -> String {
    format!("{DISPATCH_HEADER}{let_section}{do_section}{DISPATCH_FOOTER}")
}

/// The `let_section` for a single `value` binding whose body is `body`.
fn let_section(body: &str) -> String {
    format!("Let:\n  value:\n{body}")
}

/// The `do_section` holding one step whose body is `body`.
fn do_section(body: &str) -> String {
    format!("Do:\n  - {body}")
}

#[rstest]
#[case::two_supported_keys(concat!(
    "    call:\n      action: account.deposit\n      args: {}\n",
    "    must:\n      action: account.deposit\n      args: {}\n",
))]
#[case::extra_key(concat!(
    "    call:\n      action: account.deposit\n      args: {}\n",
    "    spurious: true\n",
))]
#[case::unknown_key("    invoke:\n      action: account.deposit\n      args: {}\n")]
#[case::empty_mapping("    {}\n")]
fn rejects_malformed_let_binding_mapping(#[case] body: &str) {
    let yaml = document_with(&let_section(body), "");

    let error = load_theorem_docs(&yaml).expect_err("malformed Let mapping should be rejected");
    let message = error.to_string();
    assert!(
        message.contains("exactly one") || message.contains("unknown variant"),
        "error should explain the single-key requirement, got: {message}"
    );
}

#[rstest]
#[case::two_supported_keys(concat!(
    "call:\n      action: account.deposit\n      args: {}\n",
    "    must:\n      action: account.deposit\n      args: {}\n",
))]
#[case::extra_key(concat!(
    "call:\n      action: account.deposit\n      args: {}\n",
    "    spurious: true\n",
))]
#[case::unknown_key("perform:\n      action: account.deposit\n      args: {}\n")]
#[case::empty_mapping("{}\n")]
fn rejects_malformed_do_step_mapping(#[case] body: &str) {
    let yaml = document_with("", &do_section(body));

    let error = load_theorem_docs(&yaml).expect_err("malformed Do step should be rejected");
    let message = error.to_string();
    assert!(
        message.contains("exactly one") || message.contains("unknown variant"),
        "error should explain the single-key requirement, got: {message}"
    );
}

#[rstest]
#[case::call("    call:\n      action: account.deposit\n      args: {}\n")]
#[case::must("    must:\n      action: account.deposit\n      args: {}\n")]
fn accepts_single_key_let_binding(#[case] body: &str) {
    let yaml = document_with(&let_section(body), "");

    let docs = load_theorem_docs(&yaml).expect("single-key Let mapping should decode");
    let binding = docs
        .first()
        .and_then(|doc| doc.let_bindings.get("value"))
        .expect("the value binding should decode");

    if body.trim_start().starts_with("call:") {
        assert!(matches!(binding, LetBinding::Call(_)));
    } else {
        assert!(matches!(binding, LetBinding::Must(_)));
    }
}

#[rstest]
#[case::call("call:\n      action: account.deposit\n      args: {}\n")]
#[case::must("must:\n      action: account.deposit\n      args: {}\n")]
fn accepts_single_key_do_step(#[case] body: &str) {
    let yaml = document_with("", &do_section(body));

    let docs = load_theorem_docs(&yaml).expect("single-key Do mapping should decode");
    let step = docs
        .first()
        .and_then(|doc| doc.do_steps.first())
        .expect("the step should decode");

    if body.starts_with("call:") {
        assert!(matches!(step, Step::Call(_)));
    } else {
        assert!(matches!(step, Step::Must(_)));
    }
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
