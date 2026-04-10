//! Error types for `.theorem` schema deserialization and validation.

use super::diagnostic::SchemaDiagnostic;

fn format_duplicate_theorem_key_collisions(collisions: &[SchemaDiagnostic]) -> String {
    collisions
        .iter()
        .map(|collision| collision.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Errors that can occur when loading or validating `.theorem` documents.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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

    /// Two or more different canonical action names produce the same
    /// mangled Rust identifier.
    #[error("mangled identifier collision: {message}")]
    MangledIdentifierCollision {
        /// Human-readable collision report listing all colliding
        /// canonical names per mangled identifier.
        message: String,
    },

    /// Two or more theorem documents from the same source share one or more
    /// theorem keys `{P}#{T}`.
    #[error(
        "duplicate theorem key '{theorem_key}': {}",
        format_duplicate_theorem_key_collisions(.collisions)
    )]
    DuplicateTheoremKey {
        /// The first colliding theorem key in deterministic theorem-key order.
        theorem_key: String,
        /// Structured diagnostics for all colliding theorem keys in
        /// deterministic theorem-key order.
        collisions: Vec<SchemaDiagnostic>,
        /// Optional structured diagnostic payload for the duplicate site of the
        /// first colliding theorem key.
        diagnostic: Option<SchemaDiagnostic>,
    },
}

impl SchemaError {
    /// Returns the structured diagnostic payload when available.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&SchemaDiagnostic> {
        match self {
            Self::Deserialize { diagnostic, .. }
            | Self::ValidationFailed { diagnostic, .. }
            | Self::DuplicateTheoremKey { diagnostic, .. } => diagnostic.as_ref(),
            Self::InvalidIdentifier { .. }
            | Self::InvalidActionName { .. }
            | Self::MangledIdentifierCollision { .. } => None,
        }
    }
}
