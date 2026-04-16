//! Self-contained canonical action-name validation.
//!
//! Duplicates the grammar rules from `schema::action_name` so the
//! mangle module does not depend on the schema layer (per ADR-003).

use super::InvalidCanonicalActionName;

/// Rust reserved keywords that cannot appear as action-name segments.
#[rustfmt::skip]
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate",
    "dyn", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual",
    "yield", "gen", "union",
];

/// Validates the canonical grammar: `Segment ("." Segment)+`.
pub(super) fn validate_canonical(name: &str) -> Result<(), InvalidCanonicalActionName> {
    if !name.contains('.') {
        return Err(InvalidCanonicalActionName {
            name: name.to_owned(),
            reason: "must contain at least two dot-separated segments".into(),
        });
    }
    for (i, seg) in name.split('.').enumerate() {
        validate_segment(name, seg, i + 1)?;
    }
    Ok(())
}

fn validate_segment(name: &str, seg: &str, pos: usize) -> Result<(), InvalidCanonicalActionName> {
    if seg.is_empty() {
        return Err(InvalidCanonicalActionName {
            name: name.to_owned(),
            reason: format!("segment {pos} must be non-empty"),
        });
    }
    let mut chars = seg.chars();
    // SAFETY: `seg.is_empty()` was checked above.
    let Some(first) = chars.next() else {
        return Ok(());
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(InvalidCanonicalActionName {
            name: name.to_owned(),
            reason: format!("segment {pos} ('{seg}') must start with a letter or underscore",),
        });
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(InvalidCanonicalActionName {
            name: name.to_owned(),
            reason: format!("segment {pos} ('{seg}') contains invalid characters",),
        });
    }
    if RUST_KEYWORDS.contains(&seg) {
        return Err(InvalidCanonicalActionName {
            name: name.to_owned(),
            reason: format!("segment {pos} ('{seg}') is a Rust reserved keyword",),
        });
    }
    Ok(())
}
