//! Behaviour-driven tests for theorem document deserialization and
//! validation.
//!
//! These tests use `rstest` parameterization to express Given/When/Then
//! acceptance criteria for the schema validation rules.

mod common;

use common::load_fixture;
use rstest::rstest;
use theoremc::schema::load_theorem_docs;

// ── Given valid documents, deserialization succeeds ──────────────────

#[rstest]
#[case::minimal_document("valid_minimal.theorem")]
#[case::full_document("valid_full.theorem")]
#[case::multi_document("valid_multi.theorem")]
#[case::lowercase_aliases("valid_lowercase.theorem")]
#[case::vacuous_allowed("valid_vacuous.theorem")]
fn given_a_valid_theorem_file_when_loaded_then_it_succeeds(#[case] fixture: &str) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_ok(),
        "expected {fixture} to parse successfully, got: {:?}",
        result.err()
    );
}

// ── Given an unknown key, deserialization fails ─────────────────────

#[rstest]
#[case::unknown_top_level("invalid_unknown_key.theorem", "unknown field")]
#[case::bad_expect_value("invalid_bad_expect.theorem", "YAML deserialization failed")]
fn given_a_structurally_invalid_file_when_loaded_then_error_is_actionable(
    #[case] fixture: &str,
    #[case] expected_fragment: &str,
) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_err(),
        "expected {fixture} to fail deserialization"
    );
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        msg.contains(expected_fragment),
        "error for {fixture} should contain '{expected_fragment}', \
         got: {msg}"
    );
}

// ── Given a missing required field, deserialization fails ────────────

#[rstest]
#[case::missing_theorem("invalid_missing_theorem.theorem")]
#[case::missing_about("invalid_missing_about.theorem")]
#[case::missing_prove("invalid_missing_prove.theorem")]
#[case::missing_evidence("invalid_missing_evidence.theorem")]
fn given_a_missing_required_field_when_loaded_then_it_fails(#[case] fixture: &str) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_err(),
        "expected {fixture} to fail due to missing field"
    );
}

// ── Given an invalid identifier, validation fails ───────────────────

#[rstest]
#[case::keyword_name("invalid_keyword_name.theorem", "Rust reserved keyword")]
#[case::digit_start("invalid_bad_identifier.theorem", "must match the pattern")]
fn given_an_invalid_theorem_name_when_loaded_then_error_mentions_reason(
    #[case] fixture: &str,
    #[case] expected_fragment: &str,
) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_err(),
        "expected {fixture} to fail identifier validation"
    );
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        msg.contains(expected_fragment),
        "error for {fixture} should contain '{expected_fragment}', \
         got: {msg}"
    );
}

// ── Given a wrong scalar type, deserialization fails ────────────────

#[rstest]
#[case::tags_as_string("invalid_wrong_type.theorem")]
fn given_wrong_scalar_type_when_loaded_then_it_fails(#[case] fixture: &str) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(
        result.is_err(),
        "expected {fixture} to fail due to wrong type"
    );
}

// ── Given multi-document YAML, document order is preserved ──────────

#[test]
fn given_multi_doc_yaml_when_loaded_then_order_is_preserved() {
    let yaml = load_fixture("valid_multi.theorem");
    let docs = load_theorem_docs(&yaml).expect("should parse");
    let names: Vec<&str> = docs.iter().map(|d| d.theorem.as_str()).collect();
    assert_eq!(names, vec!["FirstTheorem", "SecondTheorem", "ThirdTheorem"]);
}

// ── Given Rust keyword identifiers, they are all rejected ───────────

#[rstest]
#[case::keyword_as("as")]
#[case::keyword_break("break")]
#[case::keyword_const("const")]
#[case::keyword_continue("continue")]
#[case::keyword_crate("crate")]
#[case::keyword_else("else")]
#[case::keyword_enum("enum")]
#[case::keyword_extern("extern")]
#[case::keyword_false("false")]
#[case::keyword_fn("fn")]
#[case::keyword_for("for")]
#[case::keyword_if("if")]
#[case::keyword_impl("impl")]
#[case::keyword_in("in")]
#[case::keyword_let("let")]
#[case::keyword_loop("loop")]
#[case::keyword_match("match")]
#[case::keyword_mod("mod")]
#[case::keyword_move("move")]
#[case::keyword_mut("mut")]
#[case::keyword_pub("pub")]
#[case::keyword_ref("ref")]
#[case::keyword_return("return")]
#[case::keyword_self("self")]
#[case::keyword_static("static")]
#[case::keyword_struct("struct")]
#[case::keyword_super("super")]
#[case::keyword_trait("trait")]
#[case::keyword_true("true")]
#[case::keyword_type("type")]
#[case::keyword_unsafe("unsafe")]
#[case::keyword_use("use")]
#[case::keyword_where("where")]
#[case::keyword_while("while")]
#[case::keyword_async("async")]
#[case::keyword_await("await")]
#[case::keyword_dyn("dyn")]
#[case::keyword_abstract("abstract")]
#[case::keyword_become("become")]
#[case::keyword_do("do")]
#[case::keyword_final("final")]
#[case::keyword_macro("macro")]
#[case::keyword_override("override")]
#[case::keyword_priv("priv")]
#[case::keyword_try("try")]
#[case::keyword_typeof("typeof")]
#[case::keyword_unsized("unsized")]
#[case::keyword_virtual("virtual")]
#[case::keyword_yield("yield")]
#[case::keyword_union("union")]
#[case::keyword_gen("gen")]
#[case::keyword_self_upper("Self")]
fn given_a_rust_keyword_as_theorem_name_when_loaded_then_it_fails(#[case] keyword: &str) {
    let yaml = format!(
        "
Theorem: {keyword}
About: testing keyword rejection
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
    assert!(
        result.is_err(),
        "Rust keyword '{keyword}' should be rejected as theorem name"
    );
}

// ── Given empty or blank fields, validation fails ────────────────────

#[rstest]
#[case::empty_about("invalid_empty_about.theorem", "About must be non-empty")]
#[case::whitespace_about("invalid_whitespace_about.theorem", "About must be non-empty")]
#[case::empty_assert(
    "invalid_empty_assert.theorem",
    "Prove assertion 1: assert must be non-empty"
)]
#[case::second_empty_assert(
    "invalid_second_empty_assert.theorem",
    "Prove assertion 2: assert must be non-empty"
)]
#[case::empty_prove_because(
    "invalid_empty_prove_because.theorem",
    "Prove assertion 1: because must be non-empty"
)]
#[case::empty_assume_expr(
    "invalid_empty_assume_expr.theorem",
    "Assume constraint 1: expr must be non-empty"
)]
#[case::second_empty_assume_expr(
    "invalid_second_empty_assume_expr.theorem",
    "Assume constraint 2: expr must be non-empty"
)]
#[case::empty_assume_because(
    "invalid_empty_assume_because.theorem",
    "Assume constraint 1: because must be non-empty"
)]
#[case::empty_witness_cover(
    "invalid_empty_witness_cover.theorem",
    "Witness 1: cover must be non-empty"
)]
#[case::second_empty_witness_cover(
    "invalid_second_empty_witness_cover.theorem",
    "Witness 2: cover must be non-empty"
)]
#[case::empty_witness_because(
    "invalid_empty_witness_because.theorem",
    "Witness 1: because must be non-empty"
)]
#[case::zero_unwind("invalid_zero_unwind.theorem", "unwind must be a positive integer")]
#[case::empty_vacuity_because(
    "invalid_empty_vacuity_because.theorem",
    "vacuity_because must be non-empty"
)]
fn given_empty_or_blank_fields_when_loaded_then_validation_fails(
    #[case] fixture: &str,
    #[case] expected_fragment: &str,
) {
    let yaml = load_fixture(fixture);
    let result = load_theorem_docs(&yaml);
    assert!(result.is_err(), "expected {fixture} to fail validation");
    let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        msg.contains(expected_fragment),
        "error for {fixture} should contain '{expected_fragment}', \
         got: {msg}"
    );
}
