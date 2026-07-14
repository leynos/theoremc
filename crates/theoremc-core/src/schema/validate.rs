//! Post-deserialization semantic validation for theorem documents.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "non-empty after trimming" and "at least one evidence backend".
//! The entry point is [`validate_theorem_doc`], called by the loader after
//! successful YAML deserialization.

use super::expr;
use super::identifier::validate_identifier;
use super::step;
use super::types::{KaniEvidence, TheoremDoc};
use crate::collision::referenced_actions;

#[path = "validate_reason_markers.rs"]
pub(crate) mod reason_markers;

/// Field names used inside typed validation paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationField {
    /// `assert` field in a `Prove` assertion.
    Assert,
    /// `because` field in proof, assumption, or witness entries.
    Because,
    /// `expr` field in an `Assume` constraint.
    Expr,
    /// `cover` field in a `Witness` entry.
    Cover,
}

impl ValidationField {
    const fn label(self) -> &'static str {
        match self {
            Self::Assert => "assert",
            Self::Because => "because",
            Self::Expr => "expr",
            Self::Cover => "cover",
        }
    }
}

/// Typed location path for a validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationPath {
    /// The theorem as a whole.
    Theorem,
    /// Top-level `About`.
    About,
    /// A `Prove` assertion field.
    Prove {
        index: usize,
        field: ValidationField,
    },
    /// An `Assume` constraint field.
    Assume {
        index: usize,
        field: ValidationField,
    },
    /// A `Witness` entry field.
    Witness {
        index: usize,
        field: ValidationField,
    },
    /// Kani `unwind`.
    KaniUnwind,
    /// Kani `allow_vacuous`.
    KaniAllowVacuous,
    /// Kani `vacuity_because`.
    KaniVacuityBecause,
}

impl ValidationPath {
    fn section_label(self) -> String {
        match self {
            Self::Theorem => "Theorem".to_owned(),
            Self::About => "About".to_owned(),
            Self::Prove { index, field } => {
                format!("Prove assertion {index}: {}", field.label())
            }
            Self::Assume { index, field } => {
                format!("Assume constraint {index}: {}", field.label())
            }
            Self::Witness { index, field } => format!("Witness {index}: {}", field.label()),
            Self::KaniUnwind => "unwind".to_owned(),
            Self::KaniAllowVacuous => "allow_vacuous".to_owned(),
            Self::KaniVacuityBecause => "vacuity_because".to_owned(),
        }
    }
}

/// Typed reason for a validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ValidationKind {
    /// A string field was empty or blank.
    NonEmpty,
    /// `Prove` contained no assertions.
    EmptyProve,
    /// A Rust expression field failed syntax or form validation.
    InvalidRustExpression(String),
    /// An action signature parameter name failed identifier validation.
    InvalidActionParam {
        /// Canonical action name.
        action: String,
        /// Parameter name failure reason.
        reason: String,
    },
    /// An action signature type failed parsing.
    InvalidActionType {
        /// Canonical action name.
        action: String,
        /// Signature field name.
        field: String,
        /// Parser failure reason.
        reason: String,
    },
    /// A `Do` step shape failed validation.
    InvalidStep(String),
    /// A referenced action lacked an `Actions` signature.
    MissingActionSignature(String),
    /// `Evidence` declared no backend.
    EmptyEvidence,
    /// Kani `unwind` was zero.
    KaniUnwindNonZero,
    /// Kani vacuity was allowed without a reason.
    KaniVacuityReasonRequired,
    /// Kani vacuity reason was blank.
    KaniVacuityReasonNonEmpty,
    /// Non-vacuous Kani policy lacked witnesses.
    MissingWitness,
}

/// Typed validation failure carrying both diagnostic path and reason kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidationError {
    path: ValidationPath,
    kind: ValidationKind,
}

impl ValidationError {
    const fn new(path: ValidationPath, kind: ValidationKind) -> Self {
        Self { path, kind }
    }

    /// Returns the typed source path associated with this validation failure.
    #[must_use]
    pub(crate) const fn path(&self) -> ValidationPath {
        self.path
    }

    /// Renders the same deterministic reason strings exposed by
    /// [`SchemaError::ValidationFailed`](super::error::SchemaError::ValidationFailed).
    #[must_use]
    pub(crate) fn reason(&self) -> String {
        match &self.kind {
            ValidationKind::NonEmpty => {
                format!(
                    "{} must be non-empty after trimming",
                    self.path.section_label()
                )
            }
            ValidationKind::EmptyProve => {
                concat!("Prove section must contain at least one ", "assertion").to_owned()
            }
            ValidationKind::InvalidRustExpression(reason) => {
                format!("{} {reason}", self.path.section_label())
            }
            ValidationKind::InvalidActionParam { action, reason } => {
                format!("Actions entry '{action}': param {reason}")
            }
            ValidationKind::InvalidActionType {
                action,
                field,
                reason,
            } => {
                format!("Actions entry '{action}': {field} type is not a valid Rust type: {reason}")
            }
            ValidationKind::InvalidStep(reason) => reason.clone(),
            ValidationKind::MissingActionSignature(action) => {
                format!("referenced action '{action}' is missing an Actions signature entry")
            }
            ValidationKind::EmptyEvidence => concat!(
                "Evidence section must specify at least one ",
                "backend (kani, verus, or stateright)"
            )
            .to_owned(),
            ValidationKind::KaniUnwindNonZero => reason_markers::KANI_UNWIND_NON_ZERO.to_owned(),
            ValidationKind::KaniVacuityReasonRequired => {
                reason_markers::KANI_VACUITY_REASON_REQUIRED.to_owned()
            }
            ValidationKind::KaniVacuityReasonNonEmpty => {
                reason_markers::KANI_VACUITY_REASON_NON_EMPTY.to_owned()
            }
            ValidationKind::MissingWitness => concat!(
                "Witness section must contain at least one ",
                "witness when allow_vacuous is false ",
                "(the default)"
            )
            .to_owned(),
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Returns `true` if the string is empty or contains only whitespace.
fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Validates that all labelled string fields within an indexed
/// section entry are non-empty after trimming. Returns an error on
/// the first blank field.
fn require_non_blank_fields(
    path_for_label: impl Fn(&str) -> ValidationPath,
    pos: usize,
    fields: &[(&str, &str)],
) -> Result<(), ValidationError> {
    let _ = pos;
    for &(label, value) in fields {
        if is_blank(value) {
            return Err(ValidationError::new(
                path_for_label(label),
                ValidationKind::NonEmpty,
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
    items: &[T],
    path_for_field: impl Fn(usize, &str) -> ValidationPath,
    extract_fields: impl Fn(&T) -> Vec<(&str, &str)>,
) -> Result<(), ValidationError> {
    for (i, item) in items.iter().enumerate() {
        let fields = extract_fields(item);
        let pos = i + 1;
        require_non_blank_fields(|label| path_for_field(pos, label), pos, &fields)?;
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
/// - All `Let` binding and `Do` step `ActionCall` values remain structurally
///   valid after raw conversion (`TFS-4` §3.8, §3.9).
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
pub(crate) fn validate_theorem_doc(doc: &TheoremDoc) -> Result<(), ValidationError> {
    validate_about(doc)?;
    validate_prove_non_empty(doc)?;
    validate_assertions(doc)?;
    validate_assumptions(doc)?;
    validate_witnesses(doc)?;
    validate_expressions(doc)?;
    validate_action_signatures(doc)?;
    validate_do_steps(doc)?;
    validate_referenced_action_signatures(doc)?;
    validate_evidence(doc)?;
    Ok(())
}

// ── Individual validation helpers ───────────────────────────────────

/// `About` must be non-empty after trimming (`TFS-1` §3.3).
fn validate_about(doc: &TheoremDoc) -> Result<(), ValidationError> {
    if is_blank(&doc.about) {
        return Err(ValidationError::new(
            ValidationPath::About,
            ValidationKind::NonEmpty,
        ));
    }
    Ok(())
}

/// `Prove` must contain at least one assertion (`TFS-1` §3.10).
fn validate_prove_non_empty(doc: &TheoremDoc) -> Result<(), ValidationError> {
    if doc.prove.is_empty() {
        return Err(ValidationError::new(
            ValidationPath::Theorem,
            ValidationKind::EmptyProve,
        ));
    }
    Ok(())
}

/// Every `Assertion` must have non-empty `assert` and `because`
/// fields after trimming (`TFS-1` §3.10).
fn validate_assertions(doc: &TheoremDoc) -> Result<(), ValidationError> {
    validate_collection_fields(&doc.prove, prove_path, |a| {
        vec![
            ("assert", a.assert_expr.as_str()),
            ("because", a.because.as_str()),
        ]
    })
}

/// Every `Assumption` must have non-empty `expr` and `because`
/// fields after trimming (`TFS-1` §3.7).
fn validate_assumptions(doc: &TheoremDoc) -> Result<(), ValidationError> {
    validate_collection_fields(&doc.assume, assume_path, |a| {
        vec![("expr", a.expr.as_str()), ("because", a.because.as_str())]
    })
}

/// Every `WitnessCheck` must have non-empty `cover` and `because`
/// fields after trimming (`TFS-1` §3.7.1).
fn validate_witnesses(doc: &TheoremDoc) -> Result<(), ValidationError> {
    validate_collection_fields(&doc.witness, witness_path, |w| {
        vec![("cover", w.cover.as_str()), ("because", w.because.as_str())]
    })
}

fn prove_path(index: usize, label: &str) -> ValidationPath {
    let field = if label == "because" {
        ValidationField::Because
    } else {
        ValidationField::Assert
    };
    ValidationPath::Prove { index, field }
}

fn assume_path(index: usize, label: &str) -> ValidationPath {
    let field = if label == "because" {
        ValidationField::Because
    } else {
        ValidationField::Expr
    };
    ValidationPath::Assume { index, field }
}

fn witness_path(index: usize, label: &str) -> ValidationPath {
    let field = if label == "because" {
        ValidationField::Because
    } else {
        ValidationField::Cover
    };
    ValidationPath::Witness { index, field }
}

// ── Expression syntax validation ─────────────────────────────────

/// All expression fields parse as valid, non-statement `syn::Expr`
/// forms (`TFS-1` §1.2, §2.3, `DES-6` §6.2).
fn validate_expressions(doc: &TheoremDoc) -> Result<(), ValidationError> {
    for (i, a) in doc.assume.iter().enumerate() {
        expr::validate_rust_expr(a.expr.trim()).map_err(|reason| {
            ValidationError::new(
                ValidationPath::Assume {
                    index: i + 1,
                    field: ValidationField::Expr,
                },
                ValidationKind::InvalidRustExpression(reason),
            )
        })?;
    }
    for (i, a) in doc.prove.iter().enumerate() {
        expr::validate_rust_expr(a.assert_expr.trim()).map_err(|reason| {
            ValidationError::new(
                ValidationPath::Prove {
                    index: i + 1,
                    field: ValidationField::Assert,
                },
                ValidationKind::InvalidRustExpression(reason),
            )
        })?;
    }
    for (i, w) in doc.witness.iter().enumerate() {
        expr::validate_rust_expr(w.cover.trim()).map_err(|reason| {
            ValidationError::new(
                ValidationPath::Witness {
                    index: i + 1,
                    field: ValidationField::Cover,
                },
                ValidationKind::InvalidRustExpression(reason),
            )
        })?;
    }
    Ok(())
}

/// Every declared action signature must have valid parameter identifiers and
/// Rust type strings that parse as `syn::Type`.
fn validate_action_signatures(doc: &TheoremDoc) -> Result<(), ValidationError> {
    for (canonical_action, signature) in &doc.actions {
        let action = canonical_action.as_str();
        for (param, ty) in &signature.params {
            validate_identifier(param).map_err(|reason| {
                ValidationError::new(
                    ValidationPath::Theorem,
                    ValidationKind::InvalidActionParam {
                        action: action.to_owned(),
                        reason: reason.to_string(),
                    },
                )
            })?;
            validate_rust_type(action, param, ty)?;
        }
        validate_rust_type(action, "returns", &signature.returns)?;
    }
    Ok(())
}

fn validate_rust_type(action: &str, field: &str, ty: &str) -> Result<(), ValidationError> {
    syn::parse_str::<syn::Type>(ty.trim()).map_err(|error| {
        ValidationError::new(
            ValidationPath::Theorem,
            ValidationKind::InvalidActionType {
                action: action.to_owned(),
                field: field.to_owned(),
                reason: error.to_string(),
            },
        )
    })?;
    Ok(())
}

// ── Step and Let binding validation ──────────────────────────────

/// Every `Do` step must have valid shape (`TFS-4` §3.9, §4.2.3).
fn validate_do_steps(doc: &TheoremDoc) -> Result<(), ValidationError> {
    step::validate_step_list(&doc.do_steps, "Do step").map_err(|reason| {
        ValidationError::new(ValidationPath::Theorem, ValidationKind::InvalidStep(reason))
    })
}

/// Every referenced action must have a theorem-side `Actions` signature
/// declaration before code generation can emit typed probes.
fn validate_referenced_action_signatures(doc: &TheoremDoc) -> Result<(), ValidationError> {
    let docs = std::slice::from_ref(doc);
    for action in referenced_actions(docs) {
        if !doc.actions.contains_key(action) {
            return Err(ValidationError::new(
                ValidationPath::Theorem,
                ValidationKind::MissingActionSignature(action.to_string()),
            ));
        }
    }
    Ok(())
}

/// Evidence section must specify at least one backend, and Kani
/// evidence must satisfy unwind, vacuity, and witness constraints
/// (`TFS-6` §6.2, `ADR-4`).
fn validate_evidence(doc: &TheoremDoc) -> Result<(), ValidationError> {
    if !doc.evidence.has_any_backend() {
        return Err(ValidationError::new(
            ValidationPath::Theorem,
            ValidationKind::EmptyEvidence,
        ));
    }

    if let Some(kani) = &doc.evidence.kani {
        validate_kani_unwind(kani)?;
        validate_kani_vacuity(kani)?;
        validate_kani_witnesses(doc, kani)?;
    }

    Ok(())
}

/// Kani `unwind` must be a positive integer (`TFS-6` §6.2).
const fn validate_kani_unwind(kani: &KaniEvidence) -> Result<(), ValidationError> {
    if kani.unwind == 0 {
        return Err(ValidationError::new(
            ValidationPath::KaniUnwind,
            ValidationKind::KaniUnwindNonZero,
        ));
    }
    Ok(())
}

/// Kani vacuity policy: `allow_vacuous: true` requires a non-empty
/// `vacuity_because`; when present, `vacuity_because` must be
/// non-empty regardless of `allow_vacuous` (`ADR-4`).
fn validate_kani_vacuity(kani: &KaniEvidence) -> Result<(), ValidationError> {
    let requires_reason = kani.allow_vacuous;
    let has_reason = kani.vacuity_because.is_some();
    let reason_is_blank = kani.vacuity_because.as_deref().is_some_and(is_blank);

    if requires_reason && !has_reason {
        return Err(ValidationError::new(
            ValidationPath::KaniAllowVacuous,
            ValidationKind::KaniVacuityReasonRequired,
        ));
    }

    if has_reason && reason_is_blank {
        return Err(ValidationError::new(
            ValidationPath::KaniVacuityBecause,
            ValidationKind::KaniVacuityReasonNonEmpty,
        ));
    }

    Ok(())
}

/// Kani non-vacuity default: `Witness` section must contain at least
/// one witness when `allow_vacuous` is false (`ADR-4`).
fn validate_kani_witnesses(doc: &TheoremDoc, kani: &KaniEvidence) -> Result<(), ValidationError> {
    if !kani.allow_vacuous && doc.witness.is_empty() {
        return Err(ValidationError::new(
            ValidationPath::Theorem,
            ValidationKind::MissingWitness,
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
