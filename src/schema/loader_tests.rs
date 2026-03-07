//! Unit tests for schema document loading.

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
    let fixtures_dir =
        Dir::open_ambient_dir("tests/fixtures", ambient_authority()).expect("should open fixtures");
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
fn reject_duplicate_theorem_keys_with_diagnostic() {
    let yaml = concat!(
        "Theorem: SharedName\n",
        "About: First theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
        "---\n",
        "Theorem: SharedName\n",
        "About: Second theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    );
    let source = SourceId::new("theorems/duplicate.theorem");

    let error = load_theorem_docs_with_source(&source, yaml)
        .expect_err("duplicate theorem keys should fail");

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            message,
            diagnostic,
        } => {
            assert_eq!(theorem_key, "theorems/duplicate.theorem#SharedName");
            assert!(message.contains(
                "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at \
theorems/duplicate.theorem:1:10, theorems/duplicate.theorem:14:10"
            ));

            let structured = diagnostic.expect("duplicate theorem keys should expose a diagnostic");
            assert_eq!(structured.code.as_str(), "schema.validation_failure");
            assert_eq!(structured.location.source, "theorems/duplicate.theorem");
            assert_eq!(structured.location.line, 14);
            assert_eq!(structured.location.column, 10);
            assert!(structured.message.contains(
                "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at"
            ));
        }
        other => panic!("expected duplicate theorem key error, got: {other}"),
    }
}

#[rstest]
fn reject_all_duplicate_theorem_keys_in_stable_order() {
    let yaml = concat!(
        "Theorem: Zebra\n",
        "About: First zebra theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
        "---\n",
        "Theorem: Alpha\n",
        "About: First alpha theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
        "---\n",
        "Theorem: Zebra\n",
        "About: Second zebra theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
        "---\n",
        "Theorem: Alpha\n",
        "About: Second alpha theorem\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivially true\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    );
    let source = SourceId::new("theorems/multi-duplicate.theorem");

    let error = load_theorem_docs_with_source(&source, yaml)
        .expect_err("duplicate theorem keys should fail");

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            message,
            diagnostic,
        } => {
            assert_eq!(theorem_key, "theorems/multi-duplicate.theorem#Alpha");
            let alpha_idx = message
                .find("duplicate theorem key 'theorems/multi-duplicate.theorem#Alpha'")
                .expect("message should mention Alpha");
            let zebra_idx = message
                .find("duplicate theorem key 'theorems/multi-duplicate.theorem#Zebra'")
                .expect("message should mention Zebra");
            assert!(
                alpha_idx < zebra_idx,
                "collisions should be reported in key order"
            );

            let structured = diagnostic.expect("duplicate theorem keys should expose a diagnostic");
            assert_eq!(structured.location.line, 40);
            assert_eq!(structured.location.column, 10);
        }
        other => panic!("expected duplicate theorem key error, got: {other}"),
    }
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
