//! Unit tests for schema document loading.

use std::error::Error;

use cap_std::{ambient_authority, fs_utf8::Dir};
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
    let fixtures_dir = Dir::open_ambient_dir(
        camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        ambient_authority(),
    )
    .expect("should open fixtures");
    let yaml = fixtures_dir
        .read_to_string("valid_full.theorem")
        .expect("should read fixture");
    let docs = load_theorem_docs(&yaml).expect("should parse fixture");
    docs.into_iter()
        .next()
        .expect("fixture should have one doc")
}

fn assert_parse_error(yaml: &str) {
    let result = load_theorem_docs(yaml);
    assert!(result.is_err(), "expected parser to reject fixture");
}

fn assert_parse_error_contains(yaml: &str, expected_substring: &str) {
    let error = load_theorem_docs(yaml).expect_err("expected parser to reject fixture");
    let message = error.to_string();
    assert!(
        message.contains(expected_substring),
        "expected parse error to contain '{expected_substring}', got: {message}"
    );
}

#[rstest]
fn load_single_minimal_document() {
    let docs = load_theorem_docs(MINIMAL_YAML).expect("should parse");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs.first().map(|d| d.theorem.as_str()), Some("Minimal"));
}

const INVALID_FORALL_TYPE_YAML: &str = r"
Theorem: InvalidForallType
About: Declares an invalid Forall type
Forall:
  account: 'not a type %'
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

const FREE_LIFETIME_FORALL_YAML: &str = r#"
Theorem: InvalidForallLifetime
About: Declares an unbound Forall lifetime
Forall:
  account: "&'a crate::Account"
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
"#;

#[rstest]
#[case(
    INVALID_FORALL_TYPE_YAML,
    "Forall entry 'account': type is not a valid Rust type"
)]
#[case(
    FREE_LIFETIME_FORALL_YAML,
    "Forall entry 'account': type contains a free named lifetime parameter 'a'"
)]
fn invalid_rust_type_or_free_lifetime_is_rejected(
    #[case] yaml: &str,
    #[case] expected_fragment: &str,
) {
    assert_parse_error_contains(yaml, expected_fragment);
}

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
    assert_parse_error_contains(yaml, "unknown field");
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
    assert_parse_error(yaml);
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
    assert_parse_error(yaml);
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
    assert_parse_error_contains(yaml, "Rust reserved keyword");
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
    assert_parse_error(yaml);
}

#[rstest]
fn accept_assume_field_alias() {
    let yaml = r"
Theorem: AssumeAlias
About: Assumption alias key should parse
Assume:
  - assume: 'x > 0'
    because: positive input domain
Prove:
  - assert: 'x > 0'
    because: assumption carries through
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'x == 1'
    because: concrete witness
";
    let docs = load_theorem_docs(yaml).expect("should parse");
    let assume_expr = docs
        .first()
        .and_then(|doc| doc.assume.first())
        .map(|assumption| assumption.expr.as_str());
    assert_eq!(assume_expr, Some("x > 0"));
}

#[rstest]
#[case("reject_missing_witness_when_kani_not_vacuous", "")]
#[case(
    "reject_missing_witness_when_kani_explicitly_not_vacuous",
    "\n    allow_vacuous: false"
)]
fn reject_missing_witness(#[case] case_name: &str, #[case] allow_vacuous_config: &str) {
    let yaml = format!(
        r"
Theorem: NoWitness
About: Missing witness validation ({case_name})
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS{allow_vacuous_config}
",
    );
    assert_parse_error_contains(&yaml, "Witness section must contain at least one witness");
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
#[case(
    "reject_null_allow_vacuous",
    "    allow_vacuous: null",
    "allow_vacuous must be a boolean when provided"
)]
#[case(
    "reject_vacuous_without_reason",
    "    allow_vacuous: true",
    "vacuity_because is required"
)]
fn reject_invalid_vacuity_configuration(
    #[case] case_name: &str,
    #[case] allow_vacuous_config: &str,
    #[case] expected_message: &str,
) {
    let yaml = format!(
        r"
Theorem: NullAllowVacuous
About: Invalid vacuity configuration ({case_name})
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
{allow_vacuous_config}
Witness:
  - cover: 'true'
    because: always reachable
",
    );
    assert_parse_error_contains(&yaml, expected_message);
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
    let result = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_unknown_key.theorem"),
        yaml,
    );
    assert!(result.is_err(), "fixture should fail parsing");

    let error = result.expect_err("error expected");
    let diagnostic = error.diagnostic().expect("diagnostic expected");
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_unknown_key.theorem"
    );
    assert_eq!(diagnostic.location.line, 3);
    assert_eq!(diagnostic.location.column, 1);
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
    let result = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_empty_about.theorem"),
        yaml,
    );
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

#[rstest]
fn decode_failures_preserve_source_error() {
    let yaml = r"
Theorem: InvalidRef
About: Invalid ref target
Let:
  y:
    call:
      action: account.deposit
      args:
        target: 1

  x:
    call:
      action: account.deposit
      args:
        target:
          ref: 'not valid'
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
    let error = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_ref_target.theorem"),
        yaml,
    )
    .expect_err("invalid ref target should fail decoding");

    let source = error.source().expect("decode failure should be preserved");
    let diagnostic = error.diagnostic().expect("diagnostic expected");

    assert!(
        source.to_string().contains("Let binding 'x'"),
        "unexpected source error: {source}"
    );
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_ref_target.theorem"
    );
    assert_eq!(diagnostic.location.line, 15);
}
