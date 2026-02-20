//! Post-deserialization semantic validation for theorem documents.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "non-empty after trimming" and "at least one evidence backend".
//! The entry point is [`validate_theorem_doc`], called by the loader after
//! successful YAML deserialization.

use super::error::SchemaError;
use super::expr;
use super::step;
use super::types::{KaniEvidence, LetBinding, TheoremDoc};

// ── Helpers ─────────────────────────────────────────────────────────

/// Returns `true` if the string is empty or contains only whitespace.
fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Constructs a [`SchemaError::ValidationFailed`] for the given theorem.
fn fail(doc: &TheoremDoc, reason: String) -> SchemaError {
    SchemaError::ValidationFailed {
        theorem: doc.theorem.to_string(),
        reason,
    }
}

/// Validates that all labelled string fields within an indexed
/// section entry are non-empty after trimming. Returns an error on
/// the first blank field.
fn require_non_blank_fields(
    doc: &TheoremDoc,
    section: &str,
    pos: usize,
    fields: &[(&str, &str)],
) -> Result<(), SchemaError> {
    for &(label, value) in fields {
        if is_blank(value) {
            return Err(fail(
                doc,
                format!(
                    "{section} {pos}: {label} must be \
                     non-empty after trimming"
                ),
            ));
        }
    }
    Ok(())
}

/// Iterates over a collection, extracting labelled string fields from
/// each item and validating that none are blank.  This eliminates the
/// repeated `for (i, item) … require_non_blank_fields(…)` loop in
/// `validate_assertions`, `validate_assumptions`, and
/// `validate_witnesses`.
fn validate_collection_fields<T>(
    doc: &TheoremDoc,
    section: &str,
    items: &[T],
    extract_fields: impl Fn(&T) -> Vec<(&str, &str)>,
) -> Result<(), SchemaError> {
    for (i, item) in items.iter().enumerate() {
        let fields = extract_fields(item);
        require_non_blank_fields(doc, section, i + 1, &fields)?;
    }
    Ok(())
}

// ── Public entry point ──────────────────────────────────────────────

/// Validates a deserialized theorem document against semantic
/// constraints that `serde` attributes cannot express.
///
/// Checks applied (in order):
///
/// - `About` is non-empty after trimming.
/// - `Prove` contains at least one assertion.
/// - All `Assertion` fields are non-empty after trimming.
/// - All `Assumption` fields are non-empty after trimming.
/// - All `WitnessCheck` fields are non-empty after trimming.
/// - All expression fields (`Assume.expr`, `Prove.assert`,
///   `Witness.cover`) parse as `syn::Expr` and are not statement-like
///   forms (`TFS-1` §1.2, §2.3, `DES-6` §6.2).
/// - All `Let` binding and `Do` step `ActionCall.action` fields are
///   non-empty after trimming (`TFS-4` §3.8, §3.9).
/// - All `MaybeBlock.because` fields are non-empty after trimming and
///   `MaybeBlock.do` lists are non-empty (`TFS-4` §4.2.3, `DES-4`).
/// - At least one evidence backend is specified.
/// - Kani `unwind` is positive (> 0).
/// - Kani `vacuity_because` is non-empty after trimming when present.
/// - Kani `allow_vacuous: true` requires `vacuity_because`.
/// - Kani `allow_vacuous: false` (default) requires non-empty
///   `Witness`.
///
/// # Errors
///
/// Returns [`SchemaError::ValidationFailed`] with the theorem name and
/// a deterministic reason string on the first constraint violation.
pub(crate) fn validate_theorem_doc(doc: &TheoremDoc) -> Result<(), SchemaError> {
    validate_about(doc)?;
    validate_prove_non_empty(doc)?;
    validate_assertions(doc)?;
    validate_assumptions(doc)?;
    validate_witnesses(doc)?;
    validate_expressions(doc)?;
    validate_let_bindings(doc)?;
    validate_do_steps(doc)?;
    validate_evidence(doc)?;
    Ok(())
}

// ── Individual validation helpers ───────────────────────────────────

/// `About` must be non-empty after trimming (`TFS-1` §3.3).
fn validate_about(doc: &TheoremDoc) -> Result<(), SchemaError> {
    if is_blank(&doc.about) {
        return Err(fail(
            doc,
            "About must be non-empty after trimming".to_owned(),
        ));
    }
    Ok(())
}

/// `Prove` must contain at least one assertion (`TFS-1` §3.10).
fn validate_prove_non_empty(doc: &TheoremDoc) -> Result<(), SchemaError> {
    if doc.prove.is_empty() {
        return Err(fail(
            doc,
            concat!("Prove section must contain at least one ", "assertion",).to_owned(),
        ));
    }
    Ok(())
}

/// Every `Assertion` must have non-empty `assert` and `because`
/// fields after trimming (`TFS-1` §3.10).
fn validate_assertions(doc: &TheoremDoc) -> Result<(), SchemaError> {
    validate_collection_fields(doc, "Prove assertion", &doc.prove, |a| {
        vec![
            ("assert", a.assert_expr.as_str()),
            ("because", a.because.as_str()),
        ]
    })
}

/// Every `Assumption` must have non-empty `expr` and `because`
/// fields after trimming (`TFS-1` §3.7).
fn validate_assumptions(doc: &TheoremDoc) -> Result<(), SchemaError> {
    validate_collection_fields(doc, "Assume constraint", &doc.assume, |a| {
        vec![("expr", a.expr.as_str()), ("because", a.because.as_str())]
    })
}

/// Every `WitnessCheck` must have non-empty `cover` and `because`
/// fields after trimming (`TFS-1` §3.7.1).
fn validate_witnesses(doc: &TheoremDoc) -> Result<(), SchemaError> {
    validate_collection_fields(doc, "Witness", &doc.witness, |w| {
        vec![("cover", w.cover.as_str()), ("because", w.because.as_str())]
    })
}

// ── Expression syntax validation ─────────────────────────────────

/// All expression fields parse as valid, non-statement `syn::Expr`
/// forms (`TFS-1` §1.2, §2.3, `DES-6` §6.2).
fn validate_expressions(doc: &TheoremDoc) -> Result<(), SchemaError> {
    for (i, a) in doc.assume.iter().enumerate() {
        expr::validate_rust_expr(a.expr.trim())
            .map_err(|reason| fail(doc, format!("Assume constraint {}: expr {reason}", i + 1)))?;
    }
    for (i, a) in doc.prove.iter().enumerate() {
        expr::validate_rust_expr(a.assert_expr.trim())
            .map_err(|reason| fail(doc, format!("Prove assertion {}: assert {reason}", i + 1)))?;
    }
    for (i, w) in doc.witness.iter().enumerate() {
        expr::validate_rust_expr(w.cover.trim())
            .map_err(|reason| fail(doc, format!("Witness {}: cover {reason}", i + 1)))?;
    }
    Ok(())
}

// ── Step and Let binding validation ──────────────────────────────

/// Every `Let` binding's `ActionCall.action` must be non-empty
/// (`TFS-4` §3.8, `DES-4` §4.4).
fn validate_let_bindings(doc: &TheoremDoc) -> Result<(), SchemaError> {
    for (name, binding) in &doc.let_bindings {
        let ac = match binding {
            LetBinding::Call(c) => &c.call,
            LetBinding::Must(m) => &m.must,
        };
        step::validate_action_call(ac)
            .map_err(|r| fail(doc, format!("Let binding '{name}': {r}")))?;
    }
    Ok(())
}

/// Every `Do` step must have valid shape (`TFS-4` §3.9, §4.2.3).
fn validate_do_steps(doc: &TheoremDoc) -> Result<(), SchemaError> {
    step::validate_step_list(&doc.do_steps, "Do step").map_err(|r| fail(doc, r))
}

/// Evidence section must specify at least one backend, and Kani
/// evidence must satisfy unwind, vacuity, and witness constraints
/// (`TFS-6` §6.2, `ADR-4`).
fn validate_evidence(doc: &TheoremDoc) -> Result<(), SchemaError> {
    if !doc.evidence.has_any_backend() {
        return Err(fail(
            doc,
            concat!(
                "Evidence section must specify at least one ",
                "backend (kani, verus, or stateright)",
            )
            .to_owned(),
        ));
    }

    if let Some(kani) = &doc.evidence.kani {
        validate_kani_unwind(doc, kani)?;
        validate_kani_vacuity(doc, kani)?;
        validate_kani_witnesses(doc, kani)?;
    }

    Ok(())
}

/// Kani `unwind` must be a positive integer (`TFS-6` §6.2).
fn validate_kani_unwind(doc: &TheoremDoc, kani: &KaniEvidence) -> Result<(), SchemaError> {
    if kani.unwind == 0 {
        return Err(fail(
            doc,
            concat!("Evidence.kani.unwind must be a positive ", "integer (> 0)",).to_owned(),
        ));
    }
    Ok(())
}

/// Kani vacuity policy: `allow_vacuous: true` requires a non-empty
/// `vacuity_because`; when present, `vacuity_because` must be
/// non-empty regardless of `allow_vacuous` (`ADR-4`).
fn validate_kani_vacuity(doc: &TheoremDoc, kani: &KaniEvidence) -> Result<(), SchemaError> {
    let requires_reason = kani.allow_vacuous;
    let has_reason = kani.vacuity_because.is_some();
    let reason_is_blank = kani.vacuity_because.as_deref().is_some_and(is_blank);

    if requires_reason && !has_reason {
        return Err(fail(
            doc,
            concat!("vacuity_because is required when ", "allow_vacuous is true",).to_owned(),
        ));
    }

    if has_reason && reason_is_blank {
        return Err(fail(
            doc,
            concat!(
                "Evidence.kani.vacuity_because must be ",
                "non-empty after trimming",
            )
            .to_owned(),
        ));
    }

    Ok(())
}

/// Kani non-vacuity default: `Witness` section must contain at least
/// one witness when `allow_vacuous` is false (`ADR-4`).
fn validate_kani_witnesses(doc: &TheoremDoc, kani: &KaniEvidence) -> Result<(), SchemaError> {
    if !kani.allow_vacuous && doc.witness.is_empty() {
        return Err(fail(
            doc,
            concat!(
                "Witness section must contain at least one ",
                "witness when allow_vacuous is false ",
                "(the default)",
            )
            .to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for post-deserialization semantic validation.
    use crate::schema::load_theorem_docs;
    use rstest::rstest;

    /// Helper: load inline YAML and return the error string.
    fn load_err(yaml: &str) -> String {
        let result = load_theorem_docs(yaml);
        assert!(result.is_err(), "expected YAML to fail validation");
        result.err().map(|e| e.to_string()).unwrap_or_default()
    }

    /// Helper: assert YAML loading fails with an error containing `expected_fragment`.
    fn assert_load_err_contains(yaml: &str, expected_fragment: &str) {
        let msg = load_err(yaml);
        assert!(
            msg.contains(expected_fragment),
            "expected error containing '{expected_fragment}', got: {msg}"
        );
    }

    /// Minimal valid YAML template with placeholders for About,
    /// Prove, Assume, Witness, and Evidence sections.
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
}
