//! Typed validation failure reasons used before diagnostic rendering.
//!
//! Validation failures keep the rendered message separate from the semantic
//! field that failed. This lets diagnostics choose source locations from typed
//! data instead of reparsing human-readable text.

use super::diagnostic::SchemaDiagnostic;
use super::error::SchemaError;
use super::types::TheoremDoc;

/// Indexed field within a repeated validation section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexedValidationField {
    /// The section's primary expression field: `assert`, `expr`, or `cover`.
    Value,
    /// The section's `because` field.
    Because,
}

/// Repeated theorem section whose entries have source locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexedValidationSection {
    /// An entry in the `Prove` section.
    Prove,
    /// An entry in the `Assume` section.
    Assume,
    /// An entry in the `Witness` section.
    Witness,
}

impl IndexedValidationSection {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Prove => "Prove assertion",
            Self::Assume => "Assume constraint",
            Self::Witness => "Witness",
        }
    }

    pub(crate) const fn reason_kind(
        self,
        index: usize,
        field: IndexedValidationField,
    ) -> ValidationReasonKind {
        match self {
            Self::Prove => ValidationReasonKind::Prove { index, field },
            Self::Assume => ValidationReasonKind::Assume { index, field },
            Self::Witness => ValidationReasonKind::Witness { index, field },
        }
    }
}

/// Semantic classification for validation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidationReasonKind {
    /// The `About` field is blank.
    AboutEmpty,
    /// A field in one `Prove` entry failed validation.
    Prove {
        /// Zero-based entry index.
        index: usize,
        /// Field within the entry.
        field: IndexedValidationField,
    },
    /// A field in one `Assume` entry failed validation.
    Assume {
        /// Zero-based entry index.
        index: usize,
        /// Field within the entry.
        field: IndexedValidationField,
    },
    /// A field in one `Witness` entry failed validation.
    Witness {
        /// Zero-based entry index.
        index: usize,
        /// Field within the entry.
        field: IndexedValidationField,
    },
    /// Kani `unwind` is zero.
    KaniUnwind,
    /// Kani `allow_vacuous: true` omitted `vacuity_because`.
    KaniAllowVacuousRequired,
    /// Kani `vacuity_because` is present but blank.
    KaniVacuityBecauseNonEmpty,
    /// Kani non-vacuous policy requires at least one witness.
    KaniWitnessRequired,
}

/// Internal validation failure before conversion to the public error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidationFailure {
    theorem: String,
    reason: String,
    reason_kind: Option<ValidationReasonKind>,
}

impl ValidationFailure {
    pub(crate) fn new(
        doc: &TheoremDoc,
        reason: String,
        reason_kind: Option<ValidationReasonKind>,
    ) -> Self {
        Self {
            theorem: doc.theorem.to_string(),
            reason,
            reason_kind,
        }
    }

    pub(crate) fn reason(&self) -> &str {
        &self.reason
    }

    pub(crate) const fn reason_kind(&self) -> Option<ValidationReasonKind> {
        self.reason_kind
    }

    pub(crate) fn into_schema_error(self, diagnostic: Option<SchemaDiagnostic>) -> SchemaError {
        SchemaError::ValidationFailed {
            theorem: self.theorem,
            reason: self.reason,
            diagnostic: diagnostic.map(Box::new),
            source: None,
        }
    }
}
