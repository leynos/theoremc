//! Argument decoding and source-preserving raw document conversion errors.

use serde_saphyr::Location;

use crate::canonical_action_name::InvalidCanonicalActionName;

use super::super::raw_action::RawActionCallDecodeError;

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
    /// Returns the typed source location retained by the decode failure.
    ///
    /// `None` means the failure carries no usable span and the caller must
    /// fall back to textual scanning.
    #[must_use]
    pub(crate) fn typed_location(&self) -> Option<Location> {
        match self {
            Self::LetBinding { source, .. } | Self::DoStep { source, .. } => {
                source.typed_location()
            }
            Self::ActionSignature { .. } => None,
        }
    }

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
