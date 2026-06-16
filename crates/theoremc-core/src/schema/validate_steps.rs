//! `Let` binding and `Do` step validation.

use super::{ValidationResult, fail};
use crate::schema::step;
use crate::schema::types::{LetBinding, TheoremDoc};

/// Every `Let` binding's `ActionCall.action` must be non-empty
/// (`TFS-4` section 3.8, `DES-4` section 4.4).
pub(super) fn validate_let_bindings(doc: &TheoremDoc) -> ValidationResult {
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

/// Every `Do` step must have valid shape (`TFS-4` sections 3.9 and 4.2.3).
pub(super) fn validate_do_steps(doc: &TheoremDoc) -> ValidationResult {
    step::validate_step_list(&doc.do_steps, "Do step").map_err(|r| fail(doc, r, None))
}
