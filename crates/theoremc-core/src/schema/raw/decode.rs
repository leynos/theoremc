//! Argument decoding and source-preserving raw document conversion errors.

use indexmap::IndexMap;

use crate::canonical_action_name::{CanonicalActionName, InvalidCanonicalActionName};

use super::super::raw_action::{self, RawActionCallDecodeError, RawLetBinding, RawStep};
use super::super::types::ActionSignature;

/// Errors raised during the raw-to-public conversion in
/// [`RawTheoremDoc::to_theorem_doc`](super::RawTheoremDoc::to_theorem_doc).
///
/// Each variant identifies the location (binding name or step index)
/// and wraps the underlying [`RawActionCallDecodeError`] as a `#[source]`
/// so the full error chain is preserved. Stringification is deferred to
/// the loader boundary when building
/// [`SchemaError::ValidationFailed`](super::super::error::SchemaError::ValidationFailed).
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum RawDocDecodeError {
    /// An `Actions` map key failed canonical action-name validation.
    #[error("Actions entry '{action}': {source}")]
    ActionSignature {
        /// Rejected action-name string.
        action: String,
        /// Underlying typed validation failure.
        #[source]
        source: InvalidCanonicalActionName,
    },

    /// An argument in a `Let` binding failed decoding.
    #[error("Let binding '{name}': {source}")]
    LetBinding {
        /// The binding name from the `Let` map.
        name: String,
        /// The underlying action-call decoding failure.
        #[source]
        source: Box<RawActionCallDecodeError>,
    },

    /// An argument in a `Do` step failed decoding.
    #[error("Do step {index}: {source}")]
    DoStep {
        /// One-based step index in the `Do` list.
        index: usize,
        /// The underlying action-call decoding failure.
        #[source]
        source: Box<RawActionCallDecodeError>,
    },
}

impl RawDocDecodeError {
    /// Returns the argument breadcrumb associated with the decode failure.
    #[must_use]
    pub(crate) fn param(&self) -> &str {
        match self {
            // An invalid `Actions` key has no argument breadcrumb; the
            // caller falls back to the theorem-level location.
            Self::ActionSignature { .. } => "",
            Self::LetBinding { source, .. } | Self::DoStep { source, .. } => source.param(),
        }
    }

    /// Returns the `Let` binding name when this failure came from `Let`.
    #[must_use]
    pub(crate) fn let_binding_name(&self) -> Option<&str> {
        match self {
            Self::LetBinding { name, .. } => Some(name),
            Self::DoStep { .. } | Self::ActionSignature { .. } => None,
        }
    }

    /// Returns the one-based top-level `Do` step index for `Do` failures.
    #[must_use]
    pub(crate) const fn do_step_index(&self) -> Option<usize> {
        match self {
            Self::LetBinding { .. } | Self::ActionSignature { .. } => None,
            Self::DoStep { index, .. } => Some(*index),
        }
    }
}

// ── Domain conversion helpers ────────────────────────────────────────

/// Converts raw action signature keys into canonical action-name keys.
pub(super) fn convert_actions(
    raw: &IndexMap<String, ActionSignature>,
) -> Result<IndexMap<CanonicalActionName, ActionSignature>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (action, signature) in raw {
        let canonical = CanonicalActionName::new(action).map_err(|source| {
            RawDocDecodeError::ActionSignature {
                action: action.clone(),
                source,
            }
        })?;
        out.insert(canonical, signature.clone());
    }
    Ok(out)
}

/// Converts a map of raw `Let` bindings, decoding argument values.
pub(super) fn convert_let_bindings(
    raw: &IndexMap<String, RawLetBinding>,
) -> Result<IndexMap<String, super::super::types::LetBinding>, RawDocDecodeError> {
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
pub(super) fn convert_steps(
    raw: &[RawStep],
) -> Result<Vec<super::super::types::Step>, RawDocDecodeError> {
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
