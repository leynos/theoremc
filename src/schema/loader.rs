//! Multi-document `.theorem` file loading.
//!
//! Provides [`load_theorem_docs`] which deserializes one or more YAML
//! documents from a single string into a `Vec<TheoremDoc>`, validating
//! identifiers at deserialization time (via `TheoremName` / `ForallVar`
//! newtypes) and enforcing structural constraints post-deserialization.

use super::diagnostic::{SchemaDiagnostic, SchemaDiagnosticCode, SourceLocation};
use super::error::SchemaError;
use super::raw::RawTheoremDoc;
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
    load_theorem_docs_with_source(INLINE_SOURCE, input)
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
    source: &str,
    input: &str,
) -> Result<Vec<TheoremDoc>, SchemaError> {
    let raw_docs: Vec<RawTheoremDoc> = serde_saphyr::from_multiple(input).map_err(|error| {
        let message = error.to_string();
        let diagnostic = error
            .location()
            .map(|location| parse_diagnostic(source, &message, location));
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
    source: &str,
    raw_doc: &RawTheoremDoc,
) -> SchemaError {
    match error {
        SchemaError::ValidationFailed {
            theorem, reason, ..
        } => {
            let location = raw_doc.location_for_validation_reason(&reason);
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

fn parse_diagnostic(
    source: &str,
    message: &str,
    location: serde_saphyr::Location,
) -> SchemaDiagnostic {
    SchemaDiagnostic {
        code: SchemaDiagnosticCode::ParseFailure,
        location: location_for_source(source, location),
        message: first_line(message),
    }
}

fn validation_diagnostic(
    source: &str,
    reason: &str,
    location: serde_saphyr::Location,
) -> SchemaDiagnostic {
    SchemaDiagnostic {
        code: SchemaDiagnosticCode::ValidationFailure,
        location: location_for_source(source, location),
        message: reason.to_owned(),
    }
}

fn location_for_source(source: &str, location: serde_saphyr::Location) -> SourceLocation {
    let line = usize::try_from(location.line()).ok().unwrap_or(usize::MAX);
    let column = usize::try_from(location.column())
        .ok()
        .unwrap_or(usize::MAX);
    SourceLocation {
        source: source.to_owned(),
        line,
        column,
    }
}

fn first_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).to_owned()
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;
