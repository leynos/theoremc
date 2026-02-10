//! Post-deserialization semantic validation for theorem documents.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "non-empty after trimming" and "at least one evidence backend".
//! The entry point is [`validate_theorem_doc`], called by the loader after
//! successful YAML deserialization.

use super::error::SchemaError;
use super::types::TheoremDoc;

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
/// - At least one evidence backend is specified.
/// - Kani `unwind` is positive (> 0).
/// - Kani `vacuity_because` is non-empty after trimming when present.
/// - Kani `allow_vacuous: true` requires `vacuity_because`.
/// - Kani `allow_vacuous: false` (default) requires non-empty `Witness`.
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
            "Prove section must contain at least one assertion".to_owned(),
        ));
    }
    Ok(())
}

/// Every `Assertion` must have non-empty `assert` and `because` fields
/// after trimming (`TFS-1` §3.10).
fn validate_assertions(doc: &TheoremDoc) -> Result<(), SchemaError> {
    for (i, assertion) in doc.prove.iter().enumerate() {
        let pos = i + 1;
        if is_blank(&assertion.assert_expr) {
            return Err(fail(
                doc,
                format!("Prove assertion {pos}: assert must be non-empty after trimming"),
            ));
        }
        if is_blank(&assertion.because) {
            return Err(fail(
                doc,
                format!("Prove assertion {pos}: because must be non-empty after trimming"),
            ));
        }
    }
    Ok(())
}

/// Every `Assumption` must have non-empty `expr` and `because` fields
/// after trimming (`TFS-1` §3.7).
fn validate_assumptions(doc: &TheoremDoc) -> Result<(), SchemaError> {
    for (i, assumption) in doc.assume.iter().enumerate() {
        let pos = i + 1;
        if is_blank(&assumption.expr) {
            return Err(fail(
                doc,
                format!("Assume constraint {pos}: expr must be non-empty after trimming"),
            ));
        }
        if is_blank(&assumption.because) {
            return Err(fail(
                doc,
                format!("Assume constraint {pos}: because must be non-empty after trimming",),
            ));
        }
    }
    Ok(())
}

/// Every `WitnessCheck` must have non-empty `cover` and `because`
/// fields after trimming (`TFS-1` §3.7.1).
fn validate_witnesses(doc: &TheoremDoc) -> Result<(), SchemaError> {
    for (i, witness) in doc.witness.iter().enumerate() {
        let pos = i + 1;
        if is_blank(&witness.cover) {
            return Err(fail(
                doc,
                format!("Witness {pos}: cover must be non-empty after trimming"),
            ));
        }
        if is_blank(&witness.because) {
            return Err(fail(
                doc,
                format!("Witness {pos}: because must be non-empty after trimming"),
            ));
        }
    }
    Ok(())
}

/// Evidence section must specify at least one backend, and Kani
/// evidence must satisfy unwind, vacuity, and witness constraints
/// (`TFS-6` §6.2, `ADR-4`).
fn validate_evidence(doc: &TheoremDoc) -> Result<(), SchemaError> {
    if doc.evidence.kani.is_none()
        && doc.evidence.verus.is_none()
        && doc.evidence.stateright.is_none()
    {
        return Err(fail(
            doc,
            concat!(
                "Evidence section must specify at least one backend ",
                "(kani, verus, or stateright)",
            )
            .to_owned(),
        ));
    }

    if let Some(kani) = &doc.evidence.kani {
        if kani.unwind == 0 {
            return Err(fail(
                doc,
                "Evidence.kani.unwind must be a positive integer (> 0)".to_owned(),
            ));
        }

        if !kani.allow_vacuous && doc.witness.is_empty() {
            return Err(fail(
                doc,
                concat!(
                    "Witness section must contain at least one witness ",
                    "when allow_vacuous is false (the default)",
                )
                .to_owned(),
            ));
        }

        if kani.allow_vacuous {
            match &kani.vacuity_because {
                None => {
                    return Err(fail(
                        doc,
                        concat!("vacuity_because is required when ", "allow_vacuous is true",)
                            .to_owned(),
                    ));
                }
                Some(reason) if is_blank(reason) => {
                    return Err(fail(
                        doc,
                        concat!(
                            "Evidence.kani.vacuity_because must be ",
                            "non-empty after trimming",
                        )
                        .to_owned(),
                    ));
                }
                Some(_) => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use crate::schema::load_theorem_docs;

    /// Helper: load inline YAML and return the error string.
    fn load_err(yaml: &str) -> String {
        let result = load_theorem_docs(yaml);
        assert!(result.is_err(), "expected YAML to fail validation");
        result.err().map(|e| e.to_string()).unwrap_or_default()
    }

    // ── About validation ────────────────────────────────────────────

    #[rstest]
    #[case::empty_string(
        r#"
Theorem: T
About: ""
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#
    )]
    #[case::whitespace_only(
        r#"
Theorem: T
About: "   "
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#
    )]
    fn blank_about_is_rejected(#[case] yaml: &str) {
        let msg = load_err(yaml);
        assert!(
            msg.contains("About must be non-empty"),
            "expected About error, got: {msg}"
        );
    }

    // ── Assertion validation ────────────────────────────────────────

    #[test]
    fn blank_assert_expr_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Prove:
  - assert: ""
    because: some reason
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("assert must be non-empty"));
    }

    #[test]
    fn blank_prove_because_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Prove:
  - assert: "true"
    because: ""
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("because must be non-empty"));
    }

    // ── Assumption validation ───────────────────────────────────────

    #[test]
    fn blank_assume_expr_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Assume:
  - expr: ""
    because: some reason
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("expr must be non-empty"));
    }

    #[test]
    fn blank_assume_because_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Assume:
  - expr: "x > 0"
    because: ""
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: always reachable
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("because must be non-empty"));
    }

    // ── Witness validation ──────────────────────────────────────────

    #[test]
    fn blank_witness_cover_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: ""
    because: always reachable
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("cover must be non-empty"));
    }

    #[test]
    fn blank_witness_because_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: "true"
    because: ""
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("because must be non-empty"));
    }

    // ── Kani evidence validation ────────────────────────────────────

    #[test]
    fn zero_unwind_is_rejected() {
        let yaml = r"
Theorem: T
About: valid
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 0
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
";
        let msg = load_err(yaml);
        assert!(msg.contains("unwind must be a positive integer"));
    }

    #[test]
    fn blank_vacuity_because_is_rejected() {
        let yaml = r#"
Theorem: T
About: valid
Prove:
  - assert: "true"
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
    allow_vacuous: true
    vacuity_because: ""
"#;
        let msg = load_err(yaml);
        assert!(msg.contains("vacuity_because must be non-empty"));
    }
}
