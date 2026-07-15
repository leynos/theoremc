//! Evidence backend policy validation.

use super::{ValidationResult, fail, is_blank};
use crate::schema::types::{KaniEvidence, TheoremDoc};
use crate::schema::validation_reason::ValidationReasonKind;

/// Evidence section must specify at least one backend, and Kani evidence must
/// satisfy unwind, vacuity, and witness constraints (`TFS-6` section 6.2,
/// `ADR-4`).
pub(super) fn validate_evidence(doc: &TheoremDoc) -> ValidationResult {
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

/// Kani `unwind` must be a positive integer (`TFS-6` section 6.2).
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
/// `vacuity_because`; when present, `vacuity_because` must be non-empty
/// regardless of `allow_vacuous` (`ADR-4`).
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

/// Kani non-vacuity default: `Witness` section must contain at least one
/// witness when `allow_vacuous` is false (`ADR-4`).
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
