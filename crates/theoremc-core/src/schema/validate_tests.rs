//! Unit tests for post-deserialization semantic validation.

use crate::schema::load_theorem_docs;
use rstest::rstest;

/// Helper: load inline YAML and return the error string.
fn load_err(yaml: &str) -> String {
    let result = load_theorem_docs(yaml);
    assert!(result.is_err(), "expected YAML to fail validation");
    result.err().map(|e| e.to_string()).unwrap_or_default()
}

/// Helper: assert YAML loading fails with an error containing
/// `expected_fragment`.
fn assert_load_err_contains(yaml: &str, expected_fragment: &str) {
    let msg = load_err(yaml);
    assert!(
        msg.contains(expected_fragment),
        "expected error containing '{expected_fragment}', got: {msg}"
    );
}

/// Minimal valid YAML template with placeholders for About, Prove, Assume,
/// Witness, and Evidence sections.
const VALID_BASE: &str = r"
Theorem: T
About: valid
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

#[rstest]
#[case::empty_about(
    "Theorem: T\nAbout: \"\"\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "About must be non-empty"
)]
#[case::whitespace_about(
    "Theorem: T\nAbout: \"   \"\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "About must be non-empty"
)]
#[case::empty_assert_expr(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: \"\"\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Prove assertion 1: assert must be non-empty"
)]
#[case::empty_prove_because(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: \"\"\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Prove assertion 1: because must be non-empty"
)]
#[case::empty_assume_expr(
    "Theorem: T\nAbout: ok\nAssume:\n  - expr: \"\"\n    because: r\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Assume constraint 1: expr must be non-empty"
)]
#[case::empty_assume_because(
    "Theorem: T\nAbout: ok\nAssume:\n  - expr: 'x > 0'\n    because: \"\"\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Assume constraint 1: because must be non-empty"
)]
#[case::empty_witness_cover(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: \"\"\n    because: r",
    "Witness 1: cover must be non-empty"
)]
#[case::empty_witness_because(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: \"\"",
    "Witness 1: because must be non-empty"
)]
#[case::zero_unwind(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 0\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "unwind must be a positive integer"
)]
#[case::blank_vacuity_because_when_vacuous(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n    allow_vacuous: true\n    vacuity_because: \"\"",
    "vacuity_because must be non-empty"
)]
#[case::blank_vacuity_because_when_not_vacuous(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n    vacuity_because: \"  \"\nWitness:\n  - cover: 'true'\n    because: r",
    "vacuity_because must be non-empty"
)]
#[case::block_assume_expr(
    "Theorem: T\nAbout: ok\nAssume:\n  - expr: '{ let x = 1; x }'\n    because: r\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Assume constraint 1: expr must be a single expression"
)]
#[case::for_loop_assert(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'for i in 0..10 { }'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Prove assertion 1: assert must be a single expression"
)]
#[case::block_witness_cover(
    "Theorem: T\nAbout: ok\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: '{ true }'\n    because: r",
    "Witness 1: cover must be a single expression"
)]
#[case::invalid_syntax_assume(
    "Theorem: T\nAbout: ok\nAssume:\n  - expr: 'not rust %%'\n    because: r\nProve:\n  - assert: 'true'\n    because: t\nEvidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\nWitness:\n  - cover: 'true'\n    because: r",
    "Assume constraint 1: expr is not a valid Rust expression"
)]
fn given_invalid_field_when_loaded_then_rejected(
    #[case] yaml: &str,
    #[case] expected_fragment: &str,
) {
    assert_load_err_contains(yaml, expected_fragment);
}

#[test]
fn valid_base_parses_successfully() {
    let result = load_theorem_docs(VALID_BASE);
    assert!(result.is_ok(), "VALID_BASE should parse: {result:?}");
}
