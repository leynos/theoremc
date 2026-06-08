//! Rust type and lifetime validation for theorem declarations.
//!
//! This module centralizes type checks shared by `Forall` declarations and
//! action signatures while the parent validation module owns check ordering.

use super::{ValidationResult, fail};
use crate::schema::rust_type;
use crate::schema::types::TheoremDoc;

/// Validates all `Forall` type strings and rejects free named lifetimes.
pub(super) fn validate_forall_types(doc: &TheoremDoc) -> ValidationResult {
    for (name, ty) in &doc.forall {
        validate_type_without_free_named_lifetime(
            doc,
            ty,
            &format!("Forall entry '{name}': type"),
        )?;
    }
    Ok(())
}

/// Validates a Rust type string and rejects free named lifetimes.
pub(super) fn validate_type_without_free_named_lifetime(
    doc: &TheoremDoc,
    ty: &str,
    context: &str,
) -> ValidationResult {
    rust_type::validate(ty, |error| {
        fail(
            doc,
            format!("{context} is not a valid Rust type: {error}"),
            None,
        )
    })?;
    if let Some(lifetime) = rust_type::free_named_lifetime(ty) {
        return Err(fail(
            doc,
            format!(
                "{context} contains a free named lifetime parameter '{lifetime}'; use an owned \
                 type or an elided lifetime"
            ),
            None,
        ));
    }
    Ok(())
}
