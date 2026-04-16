//! Deterministic theorem-harness mangling helpers.
//!
//! These functions implement the theorem-key and harness-name derivation rules
//! from `docs/name-mangling-rules.md`.

use camino::Utf8Path;

/// Prefix for generated theorem harness identifiers.
const HARNESS_PREFIX: &str = "theorem__";

/// The result of mangling a theorem identifier into a Kani harness name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledHarness {
    theorem: String,
    slug: String,
    theorem_key: String,
    hash: String,
    identifier: String,
}

impl MangledHarness {
    /// The original theorem identifier.
    #[must_use]
    pub fn theorem(&self) -> &str {
        &self.theorem
    }

    /// The deterministic snake-case theorem slug.
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The exact theorem key `{P}#{T}`.
    #[must_use]
    pub fn theorem_key(&self) -> &str {
        &self.theorem_key
    }

    /// The 12-character lowercase hex hash of the theorem key.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The generated harness identifier.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }
}

fn is_preserved_snake_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first.is_ascii_lowercase() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum CharKind {
    Upper,
    Lower,
    Digit,
    Underscore,
    Other,
}

impl CharKind {
    const fn of(ch: char) -> Self {
        match ch {
            '_' => Self::Underscore,
            'A'..='Z' => Self::Upper,
            'a'..='z' => Self::Lower,
            '0'..='9' => Self::Digit,
            _ => Self::Other,
        }
    }
}

fn should_insert_slug_separator(previous: char, current: char, next: Option<char>) -> bool {
    use CharKind::{Digit, Lower, Underscore, Upper};

    let prev = CharKind::of(previous);
    let curr = CharKind::of(current);

    if prev == Underscore || curr == Underscore {
        return false;
    }

    match (prev, curr) {
        (Lower | Digit, Upper) | (Upper | Lower, Digit) | (Digit, Lower) => true,
        (Upper, Upper) => next.is_some_and(|n| n.is_ascii_lowercase()),
        _ => false,
    }
}

fn sanitize_slug_fragment(raw_slug: &str) -> String {
    let mut sanitized = String::with_capacity(raw_slug.len());

    for ch in raw_slug.chars() {
        let normalized = match ch.to_ascii_lowercase() {
            'a'..='z' | '0'..='9' | '_' => ch.to_ascii_lowercase(),
            _ => '_',
        };

        if normalized == '_' && sanitized.ends_with('_') {
            continue;
        }

        sanitized.push(normalized);
    }

    if sanitized.is_empty() {
        sanitized.push('_');
    }

    if sanitized
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }

    sanitized
}

/// Builds the exact theorem key `{P}#{T}` from the literal theorem path and
/// theorem identifier.
///
///     use theoremc_core::mangle::theorem_key;
///     assert_eq!(
///         theorem_key("theorems/bidirectional.theorem", "BidirectionalLinks"),
///         "theorems/bidirectional.theorem#BidirectionalLinks",
///     );
#[must_use]
pub fn theorem_key(path: impl AsRef<Utf8Path>, theorem: impl AsRef<str>) -> String {
    format!("{}#{}", path.as_ref().as_str(), theorem.as_ref())
}

/// Converts a theorem identifier into the deterministic harness slug.
///
/// Identifiers already matching `^[a-z_][a-z0-9_]*$` are preserved exactly.
/// Other identifiers are converted to snake case using deterministic acronym
/// and numeric-boundary splitting rules, then sanitized so the returned slug
/// always matches `^[a-z_][a-z0-9_]*$`. Any non-ASCII-alphanumeric character
/// becomes `_`, consecutive underscores collapse, and digit-leading slugs are
/// prefixed with `_`.
///
///     use theoremc_core::mangle::theorem_slug;
///     assert_eq!(
///         theorem_slug("BidirectionalLinksCommitPath3Nodes"),
///         "bidirectional_links_commit_path_3_nodes",
///     );
///     assert_eq!(theorem_slug("hnsw_smoke"), "hnsw_smoke");
#[must_use]
pub fn theorem_slug(theorem: impl AsRef<str>) -> String {
    let theorem_name = theorem.as_ref();
    if is_preserved_snake_identifier(theorem_name) {
        return theorem_name.to_owned();
    }

    let chars: Vec<char> = theorem_name.chars().collect();
    let mut slug = String::with_capacity(chars.len() + 4);

    for (index, current) in chars.iter().copied().enumerate() {
        if let Some(previous) = index.checked_sub(1).and_then(|idx| chars.get(idx).copied()) {
            let next = chars.get(index + 1).copied();
            if should_insert_slug_separator(previous, current, next) && !slug.ends_with('_') {
                slug.push('_');
            }
        }

        slug.push(current.to_ascii_lowercase());
    }

    sanitize_slug_fragment(&slug)
}

/// Mangles a theorem identifier into a deterministic Kani harness name.
///
/// Harnesses follow the normative format
/// `theorem__{theorem_slug(T)}__h{hash12(P#T)}`. The slug portion is always an
/// identifier-safe fragment matching `^[a-z_][a-z0-9_]*$`, even when the input
/// theorem string contains punctuation or starts with digits.
///
///     use theoremc_core::mangle::mangle_theorem_harness;
///     let harness = mangle_theorem_harness(
///         "theorems/bidirectional.theorem",
///         "BidirectionalLinksCommitPath3Nodes",
///     );
///     assert_eq!(
///         harness.slug(),
///         "bidirectional_links_commit_path_3_nodes",
///     );
///     assert!(harness.identifier().starts_with("theorem__"));
#[must_use]
pub fn mangle_theorem_harness(
    path: impl AsRef<Utf8Path>,
    theorem: impl AsRef<str>,
) -> MangledHarness {
    let theorem_name = theorem.as_ref();
    let slug = theorem_slug(theorem_name);
    let theorem_key = theorem_key(path, theorem_name);
    let hash = crate::mangle::hash12(&theorem_key);
    let identifier = format!("{HARNESS_PREFIX}{slug}__h{hash}");

    MangledHarness {
        theorem: theorem_name.to_owned(),
        slug,
        theorem_key,
        hash,
        identifier,
    }
}
