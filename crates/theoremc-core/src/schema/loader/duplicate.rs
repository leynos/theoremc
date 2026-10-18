//! Duplicate theorem-key collision detection and diagnostic rendering.

use std::collections::BTreeMap;

use super::{RawTheoremDoc, SchemaDiagnosticCode, SchemaError, SourceId, create_diagnostic};

#[derive(Debug, Clone, Copy)]
struct DuplicateTheoremLocation {
    location: serde_saphyr::Location,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone)]
struct DuplicateTheoremCollision {
    first: DuplicateTheoremLocation,
    duplicates: Vec<DuplicateTheoremLocation>,
}

fn build_duplicate_theorem_key_error(
    source: &SourceId,
    theorem: &str,
    first_collision: &DuplicateTheoremCollision,
    collisions: &BTreeMap<&str, DuplicateTheoremCollision>,
) -> SchemaError {
    let theorem_key = crate::mangle::theorem_key(source.as_str(), theorem);
    let first_diagnostic = create_diagnostic(
        SchemaDiagnosticCode::ValidationFailure,
        source,
        format_duplicate_theorem_key_summary(source, theorem, first_collision),
        first_collision
            .duplicates
            .first()
            .copied()
            .unwrap_or(first_collision.first)
            .location,
    );
    let mut collision_diagnostics = Vec::with_capacity(collisions.len());
    collision_diagnostics.push(first_diagnostic.clone());
    collision_diagnostics.extend(collisions.iter().skip(1).map(
        |(other_theorem, other_collision)| {
            create_diagnostic(
                SchemaDiagnosticCode::ValidationFailure,
                source,
                format_duplicate_theorem_key_summary(source, other_theorem, other_collision),
                other_collision
                    .duplicates
                    .first()
                    .copied()
                    .unwrap_or(other_collision.first)
                    .location,
            )
        },
    ));
    SchemaError::DuplicateTheoremKey {
        theorem_key,
        collisions: collision_diagnostics,
        diagnostic: Some(first_diagnostic),
    }
}

pub(super) fn check_duplicate_theorem_keys(
    source: &SourceId,
    raw_docs: &[RawTheoremDoc],
) -> Result<(), SchemaError> {
    let mut first_seen: BTreeMap<&str, DuplicateTheoremLocation> = BTreeMap::new();
    let mut collisions: BTreeMap<&str, DuplicateTheoremCollision> = BTreeMap::new();

    for raw_doc in raw_docs {
        let theorem = raw_doc.theorem.value.as_str();
        let location = raw_doc.theorem_location();
        let duplicate = DuplicateTheoremLocation {
            location,
            line: usize::try_from(location.line()).ok().unwrap_or(usize::MAX),
            column: usize::try_from(location.column())
                .ok()
                .unwrap_or(usize::MAX),
        };

        if let Some(first) = first_seen.get(theorem) {
            collisions
                .entry(theorem)
                .and_modify(|collision| collision.duplicates.push(duplicate))
                .or_insert_with(|| DuplicateTheoremCollision {
                    first: *first,
                    duplicates: vec![duplicate],
                });
        } else {
            first_seen.insert(theorem, duplicate);
        }
    }

    collisions
        .first_key_value()
        .map_or(Ok(()), |(theorem, first_collision)| {
            Err(build_duplicate_theorem_key_error(
                source,
                theorem,
                first_collision,
                &collisions,
            ))
        })
}

fn format_duplicate_theorem_key_summary(
    source: &SourceId,
    theorem: &str,
    collision: &DuplicateTheoremCollision,
) -> String {
    let theorem_key = crate::mangle::theorem_key(source.as_str(), theorem);
    let mut locations = Vec::with_capacity(collision.duplicates.len() + 1);
    locations.push(render_duplicate_location(source, collision.first));
    locations.extend(
        collision
            .duplicates
            .iter()
            .copied()
            .map(|location| render_duplicate_location(source, location)),
    );

    format!(
        "duplicate theorem key '{theorem_key}' appears at {}",
        locations.join(", "),
    )
}

fn render_duplicate_location(source: &SourceId, location: DuplicateTheoremLocation) -> String {
    format!("{}:{}:{}", source.as_str(), location.line, location.column,)
}
