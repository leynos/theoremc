//! Post-deserialization semantic validation for theorem documents.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "non-empty after trimming" and "at least one evidence backend".
//! The entry point is [`validate_theorem_doc`], called by the loader after
//! successful YAML deserialization.

use super::types::TheoremDoc;
use super::validation_reason::{ValidationFailure, ValidationReasonKind};

#[path = "validate_actions.rs"]
mod actions;
#[path = "validate_evidence.rs"]
mod evidence;
#[path = "validate_expressions.rs"]
mod expressions;
#[path = "validate_fields.rs"]
mod fields;
#[path = "validate_steps.rs"]
mod steps;

use actions::{validate_action_signatures, validate_referenced_action_signatures};
use evidence::validate_evidence;
use expressions::validate_expressions;
use fields::{
    validate_about, validate_assertions, validate_assumptions, validate_prove_non_empty,
    validate_witnesses,
};
use steps::{validate_do_steps, validate_let_bindings};

type ValidationResult = Result<(), ValidationFailure>;

/// Returns `true` if the string is empty or contains only whitespace.
fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Constructs an internal validation failure for the given theorem.
fn fail(
    doc: &TheoremDoc,
    reason: String,
    reason_kind: Option<ValidationReasonKind>,
) -> ValidationFailure {
    ValidationFailure::new(doc, reason, reason_kind)
}

/// Validates a deserialized theorem document against semantic constraints that
/// `serde` attributes cannot express.
///
/// Checks applied in order:
///
/// - `About` is non-empty after trimming.
/// - `Prove` contains at least one assertion.
/// - All `Assertion` fields are non-empty after trimming.
/// - All `Assumption` fields are non-empty after trimming.
/// - All `WitnessCheck` fields are non-empty after trimming.
/// - All expression fields (`Assume.expr`, `Prove.assert`, `Witness.cover`)
///   parse as `syn::Expr` and are not statement-like forms.
/// - All `Let` binding and `Do` step `ActionCall.action` fields are non-empty
///   after trimming.
/// - All `MaybeBlock.because` fields are non-empty after trimming and
///   `MaybeBlock.do` lists are non-empty.
/// - At least one evidence backend is specified.
/// - Kani `unwind` is positive.
/// - Kani `vacuity_because` is non-empty after trimming when present.
/// - Kani `allow_vacuous: true` requires `vacuity_because`.
/// - Kani `allow_vacuous: false` requires non-empty `Witness`.
///
/// # Errors
///
/// Returns [`ValidationFailure`] with the theorem name, deterministic reason
/// string, and typed diagnostic reason on the first constraint violation.
pub(crate) fn validate_theorem_doc(doc: &TheoremDoc) -> ValidationResult {
    validate_about(doc)?;
    validate_prove_non_empty(doc)?;
    validate_assertions(doc)?;
    validate_assumptions(doc)?;
    validate_witnesses(doc)?;
    validate_expressions(doc)?;
    validate_action_signatures(doc)?;
    validate_let_bindings(doc)?;
    validate_do_steps(doc)?;
    validate_referenced_action_signatures(doc)?;
    validate_evidence(doc)?;
    Ok(())
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
