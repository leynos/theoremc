//! Rust expression syntax validation for theorem sections.

use super::{ValidationResult, fail};
use crate::schema::expr;
use crate::schema::types::TheoremDoc;
use crate::schema::validation_reason::{IndexedValidationField, ValidationReasonKind};

/// All expression fields parse as valid, non-statement `syn::Expr` forms
/// (`TFS-1` sections 1.2 and 2.3, `DES-6` section 6.2).
pub(super) fn validate_expressions(doc: &TheoremDoc) -> ValidationResult {
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
