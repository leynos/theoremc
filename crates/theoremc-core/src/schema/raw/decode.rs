//! Argument decoding and source-preserving raw document conversion errors.

use indexmap::IndexMap;

use super::super::arg_value::ArgDecodeError;
use super::super::raw_action::{self, RawLetBinding, RawStep};

/// Errors raised during the raw-to-public conversion in
/// [`RawTheoremDoc::to_theorem_doc`](super::RawTheoremDoc::to_theorem_doc).
///
/// Each variant identifies the location (binding name or step index)
/// and wraps the underlying [`ArgDecodeError`] as a `#[source]` so
/// the full error chain is preserved. Stringification is deferred to
/// the loader boundary when building
/// [`SchemaError::ValidationFailed`](super::super::error::SchemaError::ValidationFailed).
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum RawDocDecodeError {
    /// An argument in a `Let` binding failed decoding.
    #[error("Let binding '{name}': {source}")]
    LetBinding {
        /// The binding name from the `Let` map.
        name: String,
        /// The underlying argument decoding failure.
        #[source]
        source: ArgDecodeError,
    },

    /// An argument in a `Do` step failed decoding.
    #[error("Do step {index}: {source}")]
    DoStep {
        /// One-based step index in the `Do` list.
        index: usize,
        /// The underlying argument decoding failure.
        #[source]
        source: ArgDecodeError,
    },
}

impl RawDocDecodeError {
    /// Returns the argument breadcrumb associated with the decode failure.
    #[must_use]
    pub(crate) fn param(&self) -> &str {
        match self {
            Self::LetBinding { source, .. } | Self::DoStep { source, .. } => source.param(),
        }
    }

    /// Returns the `Let` binding name when this failure came from `Let`.
    #[must_use]
    pub(crate) fn let_binding_name(&self) -> Option<&str> {
        match self {
            Self::LetBinding { name, .. } => Some(name),
            Self::DoStep { .. } => None,
        }
    }

    /// Returns the one-based top-level `Do` step index for `Do` failures.
    #[must_use]
    pub(crate) const fn do_step_index(&self) -> Option<usize> {
        match self {
            Self::LetBinding { .. } => None,
            Self::DoStep { index, .. } => Some(*index),
        }
    }
}

// ── Argument decoding helpers ────────────────────────────────────────

/// Converts a map of raw `Let` bindings, decoding argument values.
pub(super) fn convert_let_bindings(
    raw: &IndexMap<String, RawLetBinding>,
) -> Result<IndexMap<String, super::super::types::LetBinding>, RawDocDecodeError> {
    let mut out = IndexMap::with_capacity(raw.len());
    for (name, binding) in raw {
        let converted = raw_action::convert_let_binding(binding).map_err(|source| {
            RawDocDecodeError::LetBinding {
                name: name.clone(),
                source,
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
                source,
            })?;
        out.push(converted);
    }
    Ok(out)
}
