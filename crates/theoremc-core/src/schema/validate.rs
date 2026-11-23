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
#[path = "validate_types.rs"]
mod types;

use actions::{validate_action_signatures, validate_referenced_action_signatures};
use evidence::validate_evidence;
use expressions::validate_expressions;
use fields::{
    validate_about, validate_assertions, validate_assumptions, validate_prove_non_empty,
    validate_witnesses,
};
use steps::validate_do_steps;
use types::validate_forall_types;

/// Result type for one validation pass.
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
/// Passes run in the order the body lists them, and the first violation wins,
/// so a document with two faults reports the earlier one. Earlier passes check
/// the plain text fields, because a later parse of the same field would report
/// a syntax error for what is really a blank value.
///
/// 1. `About` is non-empty after trimming.
/// 2. `Prove` contains at least one assertion.
/// 3. Every `Assertion`, `Assumption`, and `WitnessCheck` field is non-empty
///    after trimming.
/// 4. Every expression field (`Assume.expr`, `Prove.assert`, `Witness.cover`)
///    parses as a `syn::Expr` and is not a statement-like form.
/// 5. Every `Actions` signature has valid parameter identifiers, and every
///    parameter and return type string parses as a `syn::Type` without a free
///    named lifetime.
/// 6. Every `Forall` type string parses as a `syn::Type` without a free named
///    lifetime.
/// 7. Every `Do` step is structurally valid: a `maybe` step needs a non-empty
///    `because` and a non-empty `do`, and its nested steps are checked
///    recursively. `call` and `must` steps are already validated by
///    raw-to-domain conversion.
/// 8. Every action referenced by a step or `Let` binding has an `Actions`
///    signature entry.
/// 9. At least one evidence backend is named; Kani `unwind` is positive,
///    `vacuity_because` is non-empty when present, `allow_vacuous: true`
///    requires `vacuity_because`, and `allow_vacuous: false` requires a
///    non-empty `Witness`.
///
/// Validation reads the domain model, so it never re-parses YAML and never
/// consults the filesystem: a failure names the theorem, a deterministic
/// reason, and a typed diagnostic reason, which lets callers render a
/// source-located diagnostic without re-deriving the cause.
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
    validate_forall_types(doc)?;
    validate_do_steps(doc)?;
    validate_referenced_action_signatures(doc)?;
    validate_evidence(doc)?;
    Ok(())
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
