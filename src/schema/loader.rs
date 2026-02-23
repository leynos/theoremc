//! Multi-document `.theorem` file loading.
//!
//! Provides [`load_theorem_docs`] which deserializes one or more YAML
//! documents from a single string into a `Vec<TheoremDoc>`, validating
//! identifiers at deserialization time (via `TheoremName` / `ForallVar`
//! newtypes) and enforcing structural constraints post-deserialization.

use super::diagnostic::{SchemaDiagnostic, SchemaDiagnosticCode, create_diagnostic, first_line};
use super::error::SchemaError;
use super::raw::{RawTheoremDoc, ValidationReason};
use super::source_id::SourceId;
use super::types::TheoremDoc;
use super::validate::validate_theorem_doc;

/// Synthetic source identifier used by [`load_theorem_docs`].
const INLINE_SOURCE: &str = "<inline>";

/// Loads one or more theorem documents from a YAML string.
///
/// A `.theorem` file may contain a single YAML document or multiple
/// documents separated by `---`. Each document is deserialized into a
/// [`TheoremDoc`] with strict unknown-key rejection. Theorem names
/// and `Forall` keys are validated at deserialization time via the
/// [`TheoremName`](super::newtypes::TheoremName) and
/// [`ForallVar`](super::newtypes::ForallVar) newtypes. Additional
/// structural constraints are checked post-deserialization, including
/// non-empty `About`, non-empty
/// `Prove`, at-least-one Evidence backend, positive Kani `unwind`,
/// non-blank string fields, non-empty `Witness` when Kani
/// `allow_vacuous` is false, and `vacuity_because` when
/// `allow_vacuous` is true.
///
/// # Errors
///
/// Returns [`SchemaError::Deserialize`] if the YAML is malformed,
/// does not match the theorem schema, or contains invalid identifiers.
/// Returns [`SchemaError::ValidationFailed`] if a structural
/// constraint is violated.
///
/// # Examples
///
///     use theoremc::schema::load_theorem_docs;
///
///     let yaml = r#"
///     Theorem: MyTheorem
///     About: A simple example
///     Prove:
///       - assert: "x > 0"
///         because: "x is positive"
///     Evidence:
///       kani:
///         unwind: 10
///         expect: SUCCESS
///     Witness:
///       - cover: "x == 1"
///         because: "at least one positive value"
///     "#;
///     let docs = load_theorem_docs(yaml).unwrap();
///     assert_eq!(docs.len(), 1);
pub fn load_theorem_docs(input: &str) -> Result<Vec<TheoremDoc>, SchemaError> {
    load_theorem_docs_with_source(&SourceId::new(INLINE_SOURCE), input)
}

/// Loads theorem documents from YAML and records diagnostics against an
/// explicit source identifier.
///
/// This function behaves like [`load_theorem_docs`] but associates parser and
/// validator diagnostics with `source` in structured diagnostic payloads.
///
/// # Errors
///
/// Returns [`SchemaError::Deserialize`] when YAML parsing or deserialization
/// fails and [`SchemaError::ValidationFailed`] when semantic validation fails.
pub fn load_theorem_docs_with_source(
    source: &SourceId,
    input: &str,
) -> Result<Vec<TheoremDoc>, SchemaError> {
    let raw_docs: Vec<RawTheoremDoc> = serde_saphyr::from_multiple(input).map_err(|error| {
        let message = error.to_string();
        let diagnostic = build_parse_diagnostic(source, input, &error, &message);
        SchemaError::Deserialize {
            message,
            diagnostic,
        }
    })?;

    let mut docs = Vec::with_capacity(raw_docs.len());
    for raw_doc in raw_docs {
        let doc = raw_doc.to_theorem_doc();
        validate_theorem_doc(&doc)
            .map_err(|error| attach_validation_diagnostic(error, source, &raw_doc))?;
        docs.push(doc);
    }

    Ok(docs)
}

fn attach_validation_diagnostic(
    error: SchemaError,
    source: &SourceId,
    raw_doc: &RawTheoremDoc,
) -> SchemaError {
    match error {
        SchemaError::ValidationFailed {
            theorem, reason, ..
        } => {
            let location = raw_doc.location_for_validation_reason(ValidationReason::new(&reason));
            let diagnostic = validation_diagnostic(source, &reason, location);
            SchemaError::ValidationFailed {
                theorem,
                reason,
                diagnostic: Some(diagnostic),
            }
        }
        other => other,
    }
}

fn build_parse_diagnostic(
    source: &SourceId,
    input: &str,
    error: &serde_saphyr::Error,
    message: &str,
) -> Option<SchemaDiagnostic> {
    let location = error.location()?;
    let mut diagnostic = parse_diagnostic(source, message, location);

    if should_reanchor_unknown_field(&diagnostic) {
        // `serde_saphyr` may report unknown-field deserialization failures at
        // document-start (1:1). Re-anchor to the offending key when possible.
        if let Some((line, column)) = locate_unknown_field(input, message) {
            diagnostic.location.line = line;
            diagnostic.location.column = column;
        }
    }

    Some(diagnostic)
}

fn parse_diagnostic(
    source: &SourceId,
    message: &str,
    location: serde_saphyr::Location,
) -> SchemaDiagnostic {
    create_diagnostic(
        SchemaDiagnosticCode::ParseFailure,
        source,
        first_line(message),
        location,
    )
}

fn validation_diagnostic(
    source: &SourceId,
    reason: &str,
    location: serde_saphyr::Location,
) -> SchemaDiagnostic {
    create_diagnostic(
        SchemaDiagnosticCode::ValidationFailure,
        source,
        reason.to_owned(),
        location,
    )
}

const fn should_reanchor_unknown_field(diagnostic: &SchemaDiagnostic) -> bool {
    diagnostic.location.line == 1 && diagnostic.location.column == 1
}

fn locate_unknown_field(input: &str, message: &str) -> Option<(usize, usize)> {
    let field = unknown_field_name(message)?;

    for (line_index, line) in input.lines().enumerate() {
        if let Some(column) = mapping_key_column(line, field) {
            return Some((line_index + 1, column));
        }
    }

    None
}

fn unknown_field_name(message: &str) -> Option<&str> {
    let (_, tail) = message.split_once("unknown field `")?;
    let (field, _) = tail.split_once('`')?;
    Some(field)
}

fn mapping_key_column(line: &str, field: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let leading = line.len() - trimmed.len();
    if is_plain_mapping_key(trimmed, field) {
        return Some(leading + 1);
    }

    if is_single_quoted_mapping_key(trimmed, field) {
        return Some(leading + 1);
    }

    if is_double_quoted_mapping_key(trimmed, field) {
        return Some(leading + 1);
    }

    None
}

fn is_plain_mapping_key(line: &str, field: &str) -> bool {
    line.strip_prefix(field)
        .is_some_and(|tail| tail.starts_with(':'))
}

fn is_single_quoted_mapping_key(line: &str, field: &str) -> bool {
    line.strip_prefix('\'')
        .and_then(|tail| tail.strip_prefix(field))
        .is_some_and(|tail| tail.starts_with("':"))
}

fn is_double_quoted_mapping_key(line: &str, field: &str) -> bool {
    line.strip_prefix('"')
        .and_then(|tail| tail.strip_prefix(field))
        .is_some_and(|tail| tail.starts_with("\":"))
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
