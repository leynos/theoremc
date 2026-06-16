//! Required text-field validation for theorem documents.

use super::{ValidationResult, fail, is_blank};
use crate::schema::types::TheoremDoc;
use crate::schema::validation_reason::{
    IndexedValidationField, IndexedValidationSection, ValidationReasonKind,
};

/// Validates that all labelled string fields within an indexed section entry
/// are non-empty after trimming. Returns an error on the first blank field.
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

/// `About` must be non-empty after trimming (`TFS-1` section 3.3).
pub(super) fn validate_about(doc: &TheoremDoc) -> ValidationResult {
    if is_blank(&doc.about) {
        return Err(fail(
            doc,
            "About must be non-empty after trimming".to_owned(),
            Some(ValidationReasonKind::AboutEmpty),
        ));
    }
    Ok(())
}

/// `Prove` must contain at least one assertion (`TFS-1` section 3.10).
pub(super) fn validate_prove_non_empty(doc: &TheoremDoc) -> ValidationResult {
    if doc.prove.is_empty() {
        return Err(fail(
            doc,
            concat!("Prove section must contain at least one ", "assertion",).to_owned(),
            None,
        ));
    }
    Ok(())
}

/// Every `Assertion` must have non-empty `assert` and `because` fields after
/// trimming (`TFS-1` section 3.10).
pub(super) fn validate_assertions(doc: &TheoremDoc) -> ValidationResult {
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

/// Every `Assumption` must have non-empty `expr` and `because` fields after
/// trimming (`TFS-1` section 3.7).
pub(super) fn validate_assumptions(doc: &TheoremDoc) -> ValidationResult {
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

/// Every `WitnessCheck` must have non-empty `cover` and `because` fields after
/// trimming (`TFS-1` section 3.7.1).
pub(super) fn validate_witnesses(doc: &TheoremDoc) -> ValidationResult {
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
