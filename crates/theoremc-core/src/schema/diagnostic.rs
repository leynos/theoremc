//! Structured diagnostics for schema loading failures.
//!
//! This module defines the stable machine-readable diagnostic payload used by
//! the schema loader to report parser and validator failures with source
//! locations.

use super::source_id::SourceId;

/// Stable diagnostic classification codes for schema loading failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaDiagnosticCode {
    /// YAML deserialization or parse failure.
    ParseFailure,
    /// Post-deserialization semantic validation failure.
    ValidationFailure,
}

impl SchemaDiagnosticCode {
    /// Returns the stable, machine-readable code string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParseFailure => "schema.parse_failure",
            Self::ValidationFailure => "schema.validation_failure",
        }
    }
}

/// Source location attached to a schema diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Source file or source identifier.
    pub source: String,
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number.
    pub column: usize,
}

/// Structured schema diagnostic payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiagnostic {
    /// Stable diagnostic code for programmatic handling.
    pub code: SchemaDiagnosticCode,
    /// Primary source location.
    pub location: SourceLocation,
    /// Deterministic human-readable fallback message.
    pub message: String,
}

impl SchemaDiagnostic {
    /// Renders the diagnostic into a deterministic single-line format suitable
    /// for snapshot tests.
    #[must_use]
    pub fn render(&self) -> String {
        format!(
            "{} | {}:{}:{} | {}",
            self.code.as_str(),
            self.location.source,
            self.location.line,
            self.location.column,
            self.message
        )
    }
}

fn location_for_source(source: &SourceId, location: serde_saphyr::Location) -> SourceLocation {
    let line = usize::try_from(location.line()).ok().unwrap_or(usize::MAX);
    let column = usize::try_from(location.column())
        .ok()
        .unwrap_or(usize::MAX);
    SourceLocation {
        source: source.as_str().to_owned(),
        line,
        column,
    }
}

pub(crate) fn create_diagnostic(
    code: SchemaDiagnosticCode,
    source: &SourceId,
    message: String,
    location: serde_saphyr::Location,
) -> SchemaDiagnostic {
    SchemaDiagnostic {
        code,
        location: location_for_source(source, location),
        message,
    }
}

pub(crate) fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).to_owned()
}
