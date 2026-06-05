//! `Do` step validation.

use super::{ValidationResult, fail};
use crate::schema::step;
use crate::schema::types::TheoremDoc;

/// Every `Do` step must have valid shape (`TFS-4` sections 3.9 and 4.2.3).
pub(super) fn validate_do_steps(doc: &TheoremDoc) -> ValidationResult {
    step::validate_step_list(&doc.do_steps, "Do step").map_err(|r| fail(doc, r, None))
}
