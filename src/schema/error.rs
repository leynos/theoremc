//! Error types for `.theorem` schema deserialization and validation.

use super::diagnostic::SchemaDiagnostic;

/// Errors that can occur when loading or validating `.theorem` documents.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// YAML deserialization failed (malformed YAML or schema mismatch).
    #[error("YAML deserialization failed: {message}")]
    Deserialize {
        /// Deserialization error message.
        message: String,
        /// Optional structured diagnostic payload.
        diagnostic: Option<SchemaDiagnostic>,
    },

    /// A theorem identifier failed lexical or keyword validation.
    #[error("invalid identifier '{identifier}': {reason}")]
    InvalidIdentifier {
        /// The identifier string that failed validation.
        identifier: String,
        /// A human-readable explanation of why the identifier is invalid.
        reason: String,
    },

    /// An action name failed canonical grammar or keyword validation.
    #[error("invalid action name '{action}': {reason}")]
    InvalidActionName {
        /// The action name string that failed validation.
        action: String,
        /// A human-readable explanation of why the action name is invalid.
        reason: String,
    },

    /// A structural constraint was violated after deserialization.
    #[error("validation failed for theorem '{theorem}': {reason}")]
    ValidationFailed {
        /// The theorem name that failed validation.
        theorem: String,
        /// A human-readable explanation of the violation.
        reason: String,
        /// Optional structured diagnostic payload.
        diagnostic: Option<SchemaDiagnostic>,
    },

    /// Two or more action names collide on canonical form or mangled
    /// identifier.
    #[error("action name collision: {message}")]
    DuplicateActionName {
        /// Human-readable collision report listing all colliding names
        /// and their source theorems.
        message: String,
    },
}

impl SchemaError {
    /// Returns the structured diagnostic payload when available.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&SchemaDiagnostic> {
        match self {
            Self::Deserialize { diagnostic, .. } | Self::ValidationFailed { diagnostic, .. } => {
                diagnostic.as_ref()
            }
            Self::InvalidIdentifier { .. }
            | Self::InvalidActionName { .. }
            | Self::DuplicateActionName { .. } => None,
        }
    }
}
