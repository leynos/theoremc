//! Action signature and referenced-action validation.

use super::{ValidationResult, fail};
use crate::collision::referenced_actions;
use crate::schema::action_name::validate_canonical_action_name;
use crate::schema::identifier::validate_identifier;
use crate::schema::types::TheoremDoc;

use super::types::validate_type_without_free_named_lifetime;

/// Every declared action signature must have a canonical name, valid
/// parameter identifiers, and Rust type strings that parse as `syn::Type`.
pub(super) fn validate_action_signatures(doc: &TheoremDoc) -> ValidationResult {
    for (action, signature) in &doc.actions {
        validate_canonical_action_name(action)
            .map_err(|r| fail(doc, format!("Actions entry '{action}': {r}"), None))?;
        for (param, ty) in &signature.params {
            validate_identifier(param)
                .map_err(|r| fail(doc, format!("Actions entry '{action}': param {r}"), None))?;
            validate_type_without_free_named_lifetime(
                doc,
                ty,
                &format!("Actions entry '{action}': {param} type"),
            )?;
        }
        validate_type_without_free_named_lifetime(
            doc,
            &signature.returns,
            &format!("Actions entry '{action}': returns type"),
        )?;
    }
    Ok(())
}

/// Every referenced action must have a theorem-side `Actions` signature
/// declaration before code generation can emit typed probes.
pub(super) fn validate_referenced_action_signatures(doc: &TheoremDoc) -> ValidationResult {
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
