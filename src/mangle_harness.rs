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

fn should_insert_slug_separator(previous: char, current: char, next: Option<char>) -> bool {
    if current == '_' || previous == '_' {
        return false;
    }

    let current_is_upper = current.is_ascii_uppercase();
    let current_is_lower = current.is_ascii_lowercase();
    let current_is_digit = current.is_ascii_digit();
    let previous_is_upper = previous.is_ascii_uppercase();
    let previous_is_lower = previous.is_ascii_lowercase();
    let previous_is_digit = previous.is_ascii_digit();

    (current_is_upper && (previous_is_lower || previous_is_digit))
        || (current_is_upper
            && previous_is_upper
            && next.is_some_and(|peek| peek.is_ascii_lowercase()))
        || (current_is_digit && previous.is_ascii_alphabetic())
        || ((current_is_lower || current_is_upper) && previous_is_digit)
}

/// Builds the exact theorem key `{P}#{T}` from the literal theorem path and
/// theorem identifier.
///
///     use theoremc::mangle::theorem_key;
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
/// and numeric-boundary splitting rules.
///
///     use theoremc::mangle::theorem_slug;
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

    slug
}

/// Mangles a theorem identifier into a deterministic Kani harness name.
///
/// Harnesses follow the normative format
/// `theorem__{theorem_slug(T)}__h{hash12(P#T)}`.
///
///     use theoremc::mangle::mangle_theorem_harness;
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
