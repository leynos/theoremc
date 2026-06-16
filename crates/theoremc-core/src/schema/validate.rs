//! Post-deserialization semantic validation for theorem documents.
//!
//! These checks enforce constraints that `serde` attributes cannot express,
//! such as "non-empty after trimming" and "at least one evidence backend".
//! The entry point is [`validate_theorem_doc`], called by the loader after
//! successful YAML deserialization.

use super::action_name::validate_canonical_action_name;
use super::expr;
use super::identifier::validate_identifier;
use super::step;
use super::types::{KaniEvidence, LetBinding, TheoremDoc};
use super::validation_reason::{
    IndexedValidationField, IndexedValidationSection, ValidationFailure, ValidationReasonKind,
};
use crate::collision::referenced_actions;

// ── Helpers ─────────────────────────────────────────────────────────

type ValidationResult = Result<(), ValidationFailure>;

/// Returns `true` if the string is empty or contains only whitespace.
fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Constructs a [`SchemaError::ValidationFailed`] for the given theorem.
fn fail(
    doc: &TheoremDoc,
    reason: String,
    reason_kind: Option<ValidationReasonKind>,
) -> ValidationFailure {
    ValidationFailure::new(doc, reason, reason_kind)
}

/// Validates that all labelled string fields within an indexed
/// section entry are non-empty after trimming. Returns an error on
/// the first blank field.
fn require_non_blank_fields(
    doc: &TheoremDoc,
    section: IndexedValidationSection,
    pos: usize,
    fields: &[(IndexedValidationField, &str, &str)],
) -> ValidationResult {
    for &(field, label, value) in fields {
        if is_blank(value) {
            return Err(fail(
                doc,
                format!(
                    "{} {pos}: {label} must be non-empty after trimming",
                    section.label(),
                ),
                Some(section.reason_kind(pos - 1, field)),
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
    section: IndexedValidationSection,
    items: &[T],
    extract_fields: impl Fn(&T) -> Vec<(IndexedValidationField, &'static str, &str)>,
) -> ValidationResult {
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

// ── Individual validation helpers ───────────────────────────────────

/// `About` must be non-empty after trimming (`TFS-1` §3.3).
fn validate_about(doc: &TheoremDoc) -> ValidationResult {
    if is_blank(&doc.about) {
        return Err(fail(
            doc,
            "About must be non-empty after trimming".to_owned(),
            Some(ValidationReasonKind::AboutEmpty),
        ));
    }
    Ok(())
}

/// `Prove` must contain at least one assertion (`TFS-1` §3.10).
fn validate_prove_non_empty(doc: &TheoremDoc) -> ValidationResult {
    if doc.prove.is_empty() {
        return Err(fail(
            doc,
            concat!("Prove section must contain at least one ", "assertion",).to_owned(),
            None,
        ));
    }
    Ok(())
}

/// Every `Assertion` must have non-empty `assert` and `because`
/// fields after trimming (`TFS-1` §3.10).
fn validate_assertions(doc: &TheoremDoc) -> ValidationResult {
    validate_collection_fields(doc, IndexedValidationSection::Prove, &doc.prove, |a| {
        vec![
            (
                IndexedValidationField::Value,
                "assert",
                a.assert_expr.as_str(),
            ),
            (
                IndexedValidationField::Because,
                "because",
                a.because.as_str(),
            ),
        ]
    })
}

/// Every `Assumption` must have non-empty `expr` and `because`
/// fields after trimming (`TFS-1` §3.7).
fn validate_assumptions(doc: &TheoremDoc) -> ValidationResult {
    validate_collection_fields(doc, IndexedValidationSection::Assume, &doc.assume, |a| {
        vec![
            (IndexedValidationField::Value, "expr", a.expr.as_str()),
            (
                IndexedValidationField::Because,
                "because",
                a.because.as_str(),
            ),
        ]
    })
}

/// Every `WitnessCheck` must have non-empty `cover` and `because`
/// fields after trimming (`TFS-1` §3.7.1).
fn validate_witnesses(doc: &TheoremDoc) -> ValidationResult {
    validate_collection_fields(doc, IndexedValidationSection::Witness, &doc.witness, |w| {
        vec![
            (IndexedValidationField::Value, "cover", w.cover.as_str()),
            (
                IndexedValidationField::Because,
                "because",
                w.because.as_str(),
            ),
        ]
    })
}

// ── Expression syntax validation ─────────────────────────────────

/// All expression fields parse as valid, non-statement `syn::Expr`
/// forms (`TFS-1` §1.2, §2.3, `DES-6` §6.2).
fn validate_expressions(doc: &TheoremDoc) -> ValidationResult {
    for (i, a) in doc.assume.iter().enumerate() {
        expr::validate_rust_expr(a.expr.trim()).map_err(|reason| {
            fail(
                doc,
                format!("Assume constraint {}: expr {reason}", i + 1),
                Some(ValidationReasonKind::Assume {
                    index: i,
                    field: IndexedValidationField::Value,
                }),
            )
        })?;
    }
    for (i, a) in doc.prove.iter().enumerate() {
        expr::validate_rust_expr(a.assert_expr.trim()).map_err(|reason| {
            fail(
                doc,
                format!("Prove assertion {}: assert {reason}", i + 1),
                Some(ValidationReasonKind::Prove {
                    index: i,
                    field: IndexedValidationField::Value,
                }),
            )
        })?;
    }
    for (i, w) in doc.witness.iter().enumerate() {
        expr::validate_rust_expr(w.cover.trim()).map_err(|reason| {
            fail(
                doc,
                format!("Witness {}: cover {reason}", i + 1),
                Some(ValidationReasonKind::Witness {
                    index: i,
                    field: IndexedValidationField::Value,
                }),
            )
        })?;
    }
    Ok(())
}

/// Every declared action signature must have a canonical name, valid parameter
/// identifiers, and Rust type strings that parse as `syn::Type`.
fn validate_action_signatures(doc: &TheoremDoc) -> ValidationResult {
    for (action, signature) in &doc.actions {
        validate_canonical_action_name(action)
            .map_err(|r| fail(doc, format!("Actions entry '{action}': {r}"), None))?;
        for (param, ty) in &signature.params {
            validate_identifier(param)
                .map_err(|r| fail(doc, format!("Actions entry '{action}': param {r}"), None))?;
            validate_rust_type(doc, action, param, ty)?;
        }
        validate_rust_type(doc, action, "returns", &signature.returns)?;
    }
    Ok(())
}

fn validate_rust_type(doc: &TheoremDoc, action: &str, field: &str, ty: &str) -> ValidationResult {
    syn::parse_str::<syn::Type>(ty.trim()).map_err(|error| {
        fail(
            doc,
            format!("Actions entry '{action}': {field} type is not a valid Rust type: {error}"),
            None,
        )
    })?;
    Ok(())
}

// ── Step and Let binding validation ──────────────────────────────

/// Every `Let` binding's `ActionCall.action` must be non-empty
/// (`TFS-4` §3.8, `DES-4` §4.4).
fn validate_let_bindings(doc: &TheoremDoc) -> ValidationResult {
    for (name, binding) in &doc.let_bindings {
        let ac = match binding {
            LetBinding::Call(c) => &c.call,
            LetBinding::Must(m) => &m.must,
        };
        step::validate_action_call(ac)
            .map_err(|r| fail(doc, format!("Let binding '{name}': {r}"), None))?;
    }
    Ok(())
}

/// Every `Do` step must have valid shape (`TFS-4` §3.9, §4.2.3).
fn validate_do_steps(doc: &TheoremDoc) -> ValidationResult {
    step::validate_step_list(&doc.do_steps, "Do step").map_err(|r| fail(doc, r, None))
}

/// Every referenced action must have a theorem-side `Actions` signature
/// declaration before code generation can emit typed probes.
fn validate_referenced_action_signatures(doc: &TheoremDoc) -> ValidationResult {
    let docs = std::slice::from_ref(doc);
    for action in referenced_actions(docs) {
        if !doc.actions.contains_key(action) {
            return Err(fail(
                doc,
                format!("referenced action '{action}' is missing an Actions signature entry"),
                None,
            ));
        }
    }
    Ok(())
}

/// Evidence section must specify at least one backend, and Kani
/// evidence must satisfy unwind, vacuity, and witness constraints
/// (`TFS-6` §6.2, `ADR-4`).
fn validate_evidence(doc: &TheoremDoc) -> ValidationResult {
    if !doc.evidence.has_any_backend() {
        return Err(fail(
            doc,
            concat!(
                "Evidence section must specify at least one ",
                "backend (kani, verus, or stateright)",
            )
            .to_owned(),
            None,
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
fn validate_kani_unwind(doc: &TheoremDoc, kani: &KaniEvidence) -> ValidationResult {
    if kani.unwind == 0 {
        return Err(fail(
            doc,
            "Evidence.kani.unwind must be a positive integer (> 0)".to_owned(),
            Some(ValidationReasonKind::KaniUnwind),
        ));
    }
    Ok(())
}

/// Kani vacuity policy: `allow_vacuous: true` requires a non-empty
/// `vacuity_because`; when present, `vacuity_because` must be
/// non-empty regardless of `allow_vacuous` (`ADR-4`).
fn validate_kani_vacuity(doc: &TheoremDoc, kani: &KaniEvidence) -> ValidationResult {
    let requires_reason = kani.allow_vacuous;
    let has_reason = kani.vacuity_because.is_some();
    let reason_is_blank = kani.vacuity_because.as_deref().is_some_and(is_blank);

    if requires_reason && !has_reason {
        return Err(fail(
            doc,
            "vacuity_because is required when allow_vacuous is true".to_owned(),
            Some(ValidationReasonKind::KaniAllowVacuousRequired),
        ));
    }

    if has_reason && reason_is_blank {
        return Err(fail(
            doc,
            "Evidence.kani.vacuity_because must be non-empty after trimming".to_owned(),
            Some(ValidationReasonKind::KaniVacuityBecauseNonEmpty),
        ));
    }

    Ok(())
}

/// Kani non-vacuity default: `Witness` section must contain at least
/// one witness when `allow_vacuous` is false (`ADR-4`).
fn validate_kani_witnesses(doc: &TheoremDoc, kani: &KaniEvidence) -> ValidationResult {
    if !kani.allow_vacuous && doc.witness.is_empty() {
        return Err(fail(
            doc,
            concat!(
                "Witness section must contain at least one ",
                "witness when allow_vacuous is false ",
                "(the default)",
            )
            .to_owned(),
            Some(ValidationReasonKind::KaniWitnessRequired),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;
