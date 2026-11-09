//! Integration tests for unhappy-path deserialization and edge cases.
//!
//! These tests verify that invalid `.theorem` documents produce appropriate
//! errors, cover `KaniExpectation` variants, and exercise inline edge cases
//! for unknown keys in subordinate structures.

use rstest::rstest;
use test_helpers::{FixtureName, StrNewtype, fixture_error_message};
use theoremc::schema::{
    KaniExpectation, LetBinding, SchemaError, Step, TheoremDoc, load_theorem_docs,
};

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

/// One single-key dispatch shape under test, rendered as a full document.
///
/// A case carries the mapping shape rather than its YAML text so the tests
/// keep one spelling of each fixture. The `Let` and `Do` positions exercise
/// the same shapes, so both documents derive from one body.
#[derive(Debug, Clone, Copy)]
enum DispatchCase {
    TwoKeys,
    ExtraKey,
    UnknownKey,
    EmptyMapping,
    Call,
    Must,
}

impl DispatchCase {
    /// The mapping body, indented to sit under a `Let` binding key.
    fn body(self) -> &'static str {
        match self {
            Self::TwoKeys => concat!(
                "    call:\n      action: account.deposit\n      args: {}\n",
                "    must:\n      action: account.deposit\n      args: {}\n",
            ),
            Self::ExtraKey => concat!(
                "    call:\n      action: account.deposit\n      args: {}\n",
                "    spurious: true\n",
            ),
            Self::UnknownKey => "    invoke:\n      action: account.deposit\n      args: {}\n",
            Self::EmptyMapping => "    {}\n",
            Self::Call => "    call:\n      action: account.deposit\n      args: {}\n",
            Self::Must => "    must:\n      action: account.deposit\n      args: {}\n",
        }
    }

    /// The whole document holding this mapping as `Let.value`.
    fn let_document(self) -> String {
        format!(
            "{DISPATCH_HEADER}Let:\n  value:\n{}{DISPATCH_FOOTER}",
            self.body()
        )
    }

    /// The whole document holding this mapping as the only `Do` entry.
    ///
    /// The body is written for the `Let` position, where the binding key
    /// occupies column four. Removing that first-line indent lets the same
    /// body follow the sequence dash at the same column.
    fn do_document(self) -> String {
        format!(
            "{DISPATCH_HEADER}Do:\n  - {}{DISPATCH_FOOTER}",
            self.body().trim_start()
        )
    }

    /// Whether this case dispatches through `call` rather than `must`.
    fn is_call(self) -> bool {
        matches!(self, Self::Call)
    }
}

/// The position a decoded single-key mapping is read from.
#[derive(Debug, Clone, Copy)]
enum Position {
    Let,
    Do,
}

impl Position {
    /// The document holding `case` at this position.
    fn document(self, case: DispatchCase) -> String {
        match self {
            Self::Let => case.let_document(),
            Self::Do => case.do_document(),
        }
    }

    /// Whether the first entry decoded from `doc` used the `call` key.
    fn decoded_as_call(self, doc: &TheoremDoc) -> bool {
        match self {
            Self::Let => matches!(doc.let_bindings.get("value"), Some(LetBinding::Call(_))),
            Self::Do => matches!(doc.do_steps.first(), Some(Step::Call(_))),
        }
    }
}

/// Asserts that a rejection explains the single-key requirement.
fn assert_single_key_error(error: &SchemaError) {
    let message = error.to_string();
    assert!(
        message.contains("exactly one") || message.contains("unknown variant"),
        "error should explain the single-key requirement, got: {message}"
    );
}

#[rstest]
#[case::two_supported_keys(DispatchCase::TwoKeys)]
#[case::extra_key(DispatchCase::ExtraKey)]
#[case::unknown_key(DispatchCase::UnknownKey)]
#[case::empty_mapping(DispatchCase::EmptyMapping)]
fn rejects_malformed_let_binding_mapping(#[case] case: DispatchCase) {
    let error = load_theorem_docs(&case.let_document())
        .expect_err("malformed Let mapping should be rejected");

    assert_single_key_error(&error);
}

#[rstest]
#[case::two_supported_keys(DispatchCase::TwoKeys)]
#[case::extra_key(DispatchCase::ExtraKey)]
#[case::unknown_key(DispatchCase::UnknownKey)]
#[case::empty_mapping(DispatchCase::EmptyMapping)]
fn rejects_malformed_do_step_mapping(#[case] case: DispatchCase) {
    let error =
        load_theorem_docs(&case.do_document()).expect_err("malformed Do step should be rejected");

    assert_single_key_error(&error);
}

#[rstest]
fn accepts_single_key_mapping(
    #[values(DispatchCase::Call, DispatchCase::Must)] case: DispatchCase,
    #[values(Position::Let, Position::Do)] position: Position,
) {
    let docs =
        load_theorem_docs(&position.document(case)).expect("single-key mapping should decode");
    let doc = docs.first().expect("the document should decode");

    assert_eq!(
        position.decoded_as_call(doc),
        case.is_call(),
        "the decoded variant should follow the mapping key"
    );
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
