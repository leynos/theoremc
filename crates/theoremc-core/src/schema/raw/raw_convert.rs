//! Domain conversion helpers for the raw schema adapter.
//!
//! The raw adapter keeps source locations beside every value it decodes.
//! These helpers perform the actual translation into the public schema domain,
//! attributing each failure to the binding name or one-based step index that
//! produced it so the loader can render an actionable diagnostic.

use indexmap::IndexMap;

use crate::canonical_action_name::CanonicalActionName;

use super::super::raw_action::{self, RawLetBinding, RawStep};
use super::super::raw_evidence::RawActionSignature;
use super::super::types::{ActionSignature, LetBinding, Step};
use super::RawDocDecodeError;

/// Converts raw action signature keys into canonical action-name keys.
pub(super) fn convert_actions(
    raw: &IndexMap<String, RawActionSignature>,
) -> Result<IndexMap<CanonicalActionName, ActionSignature>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (action, signature) in raw {
        let canonical = CanonicalActionName::new(action).map_err(|source| {
            RawDocDecodeError::ActionSignature {
                action: action.clone(),
                source,
            }
        })?;
        out.insert(canonical, signature.to_action_signature());
    }
    Ok(out)
}

/// Converts a map of raw `Let` bindings, decoding argument values.
pub(super) fn convert_let_bindings(
    raw: &IndexMap<String, RawLetBinding>,
) -> Result<IndexMap<String, LetBinding>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (name, binding) in raw {
        let converted = raw_action::convert_let_binding(binding).map_err(|source| {
            RawDocDecodeError::LetBinding {
                name: name.clone(),
                source: Box::new(source),
            }
        })?;
        out.insert(name.clone(), converted);
    }
    Ok(out)
}

/// Converts a list of raw `Do` steps, decoding argument values.
pub(super) fn convert_steps(raw: &[RawStep]) -> Result<Vec<Step>, RawDocDecodeError> {
    let mut out = Vec::with_capacity(raw.len());
    for (i, step) in raw.iter().enumerate() {
        let converted =
            raw_action::convert_step(step).map_err(|source| RawDocDecodeError::DoStep {
                index: i + 1,
                source: Box::new(source),
            })?;
        out.push(converted);
    }
    Ok(out)
}
