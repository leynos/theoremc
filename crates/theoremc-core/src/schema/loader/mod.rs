//! Multi-document `.theorem` file loading.
//!
//! Provides [`load_theorem_docs`] which deserializes one or more YAML
//! documents from a single string into a `Vec<TheoremDoc>`, validating
//! identifiers at deserialization time (via `TheoremName` / `ForallVar`
//! newtypes) and enforcing structural constraints post-deserialization.

use super::diagnostic::{SchemaDiagnostic, SchemaDiagnosticCode, create_diagnostic, first_line};
use super::error::SchemaError;
use super::loader_decode_location::locate_decode_failure;
use super::loader_message::{ErrorMessage, FieldName};
use super::raw::{RawDocDecodeError, RawTheoremDoc};
use super::source_id::SourceId;
use super::types::TheoremDoc;
use super::validate::validate_theorem_doc;
use super::validation_reason::ValidationFailure;

mod duplicate;

use duplicate::check_duplicate_theorem_keys;

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
/// constraint is violated. Returns [`SchemaError::DuplicateTheoremKey`] if two
/// or more documents in the same loaded source share the same literal theorem
/// key `{P}#{T}`.
///
/// # Examples
///
///     use theoremc_core::schema::load_theorem_docs;
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
/// fails, [`SchemaError::ValidationFailed`] when semantic validation fails,
/// and [`SchemaError::DuplicateTheoremKey`] when the same source declares a
/// duplicate literal theorem key `{P}#{T}`.
pub fn load_theorem_docs_with_source(
    source: &SourceId,
    input: &str,
) -> Result<Vec<TheoremDoc>, SchemaError> {
    let raw_docs: Vec<RawTheoremDoc> = serde_saphyr::from_multiple(input).map_err(|error| {
        let message = error.to_string();
        let diagnostic = build_parse_diagnostic(source, input, &error, ErrorMessage::new(&message));
        SchemaError::Deserialize {
            message,
            diagnostic,
        }
    })?;
    check_duplicate_theorem_keys(source, &raw_docs)?;

    let mut docs = Vec::with_capacity(raw_docs.len());
    for raw_doc in &raw_docs {
        let doc = raw_doc.to_theorem_doc().map_err(|decode_err| {
            attach_decode_failure_diagnostic(decode_err, source, input, raw_doc)
        })?;
        validate_theorem_doc(&doc)
            .map_err(|failure| attach_validation_failure_diagnostic(failure, source, raw_doc))?;
        docs.push(doc);
    }

    crate::collision::check_action_collisions(&docs)?;

    Ok(docs)
}

fn attach_decode_failure_diagnostic(
    error: RawDocDecodeError,
    source: &SourceId,
    input: &str,
    raw_doc: &RawTheoremDoc,
) -> SchemaError {
    let reason = error.to_string();
    let mut diagnostic = create_diagnostic(
        SchemaDiagnosticCode::ValidationFailure,
        source,
        reason.clone(),
        raw_doc.theorem_location(),
    );
    if let Some((line, column)) = locate_decode_failure(input, raw_doc, &error) {
        diagnostic.location.line = line;
        diagnostic.location.column = column;
    }

    SchemaError::ValidationFailed {
        theorem: raw_doc.theorem.value.to_string(),
        reason,
        diagnostic: Some(Box::new(diagnostic)),
        source: Some(Box::new(error)),
    }
}

fn attach_validation_failure_diagnostic(
    failure: ValidationFailure,
    source: &SourceId,
    raw_doc: &RawTheoremDoc,
) -> SchemaError {
    let location = failure.reason_kind().map_or_else(
        || raw_doc.theorem_location(),
        |reason| raw_doc.location_for_validation_reason(reason),
    );
    let diagnostic = create_diagnostic(
        SchemaDiagnosticCode::ValidationFailure,
        source,
        failure.reason().to_owned(),
        location,
    );
    failure.into_schema_error(Some(diagnostic))
}

fn build_parse_diagnostic(
    source: &SourceId,
    input: &str,
    error: &serde_saphyr::Error,
    message: ErrorMessage<'_>,
) -> Option<SchemaDiagnostic> {
    let location = error.location()?;
    let mut diagnostic = create_diagnostic(
        SchemaDiagnosticCode::ParseFailure,
        source,
        first_line(message.as_str()),
        location,
    );

    // `serde_saphyr` may report unknown-field deserialization failures at
    // document-start (1:1). Re-anchor to the offending key when possible.
    if should_reanchor_unknown_field(&diagnostic)
        && let Some((line, column)) = locate_unknown_field(input, message)
    {
        diagnostic.location.line = line;
        diagnostic.location.column = column;
    }

    Some(diagnostic)
}

const fn should_reanchor_unknown_field(diagnostic: &SchemaDiagnostic) -> bool {
    diagnostic.location.line == 1 && diagnostic.location.column == 1
}

fn locate_unknown_field(input: &str, message: ErrorMessage<'_>) -> Option<(usize, usize)> {
    let field = unknown_field_name(message)?;

    for (line_index, line) in input.lines().enumerate() {
        if let Some(column) = mapping_key_column(line, field) {
            return Some((line_index + 1, column));
        }
    }

    None
}

fn unknown_field_name(message: ErrorMessage<'_>) -> Option<FieldName<'_>> {
    let (_, tail) = message.as_str().split_once("unknown field `")?;
    let (field, _) = tail.split_once('`')?;
    Some(FieldName::new(field))
}

fn mapping_key_column(line: &str, field: FieldName<'_>) -> Option<usize> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    if is_mapping_key_for_field(trimmed, field) {
        let leading = line.len() - trimmed.len();
        return Some(leading + 1);
    }

    None
}

fn is_plain_mapping_key(line: &str, field: FieldName<'_>) -> bool {
    line.strip_prefix(field.as_str())
        .is_some_and(|tail| tail.starts_with(':'))
}

fn is_single_quoted_mapping_key(line: &str, field: FieldName<'_>) -> bool {
    line.strip_prefix('\'')
        .and_then(|tail| tail.strip_prefix(field.as_str()))
        .is_some_and(|tail| tail.starts_with("':"))
}

fn is_double_quoted_mapping_key(line: &str, field: FieldName<'_>) -> bool {
    line.strip_prefix('"')
        .and_then(|tail| tail.strip_prefix(field.as_str()))
        .is_some_and(|tail| tail.starts_with("\":"))
}

fn is_mapping_key_for_field(line: &str, field: FieldName<'_>) -> bool {
    is_plain_mapping_key(line, field)
        || is_single_quoted_mapping_key(line, field)
        || is_double_quoted_mapping_key(line, field)
}

#[cfg(test)]
#[path = "../loader_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../loader_action_tests.rs"]
mod action_tests;

#[cfg(test)]
#[path = "../loader_diagnostic_tests.rs"]
mod diagnostic_tests;

#[cfg(test)]
#[path = "../loader_duplicate_tests.rs"]
mod duplicate_theorem_key_tests;
