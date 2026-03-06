//! Name mangling for deterministic, collision-resistant resolution.
//!
//! Provides two mangling pipelines, both following
//! `docs/name-mangling-rules.md`:
//!
//! # Action name mangling
//!
//! Transforms canonical dot-separated action names into mangled Rust
//! identifiers suitable for binding in `crate::theorem_actions`.
//!
//! 1. Each segment's `_` is escaped to `_u`.
//! 2. `action_slug` splits on `.`, escapes each segment, and joins
//!    with `__`.
//! 3. `hash12` computes the first 12 lowercase hex characters of
//!    the blake3 digest.
//! 4. `mangle_action_name` assembles the full mangled identifier and
//!    resolution path.
//!
//! # Per-file module naming
//!
//! Transforms `.theorem` file paths into deterministic, collision-
//! resistant Rust module names of the form
//! `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}`.
//!
//! 1. `path_stem` removes a trailing `.theorem` extension.
//! 2. `path_mangle` sanitizes the stem into an identifier-safe
//!    fragment (replace separators, collapse underscores, lowercase).
//! 3. `mangle_module_path` assembles the full module name using
//!    `hash12` of the **original** path for disambiguation.

use camino::Utf8Path;

// ── Domain newtypes ───────────────────────────────────────────────

/// Error returned when a string is not a valid canonical action name.
///
/// A valid canonical action name has at least two dot-separated
/// segments, each matching `^[A-Za-z_][A-Za-z0-9_]*$` and not a
/// Rust reserved keyword.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid canonical action name '{name}': {reason}")]
pub struct InvalidCanonicalActionName {
    pub(crate) name: String,
    pub(crate) reason: String,
}

/// A validated canonical action name (e.g. `"account.deposit"`).
///
/// Construct via [`CanonicalActionName::new`] (fallible) or
/// [`TryFrom`].
///
///     use theoremc::mangle::CanonicalActionName;
///     let name = CanonicalActionName::new("account.deposit")
///         .expect("valid canonical name");
///     assert_eq!(name.as_str(), "account.deposit");
///     assert!(CanonicalActionName::new("deposit").is_err());
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalActionName(String);

impl CanonicalActionName {
    /// Validates and wraps a canonical action name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCanonicalActionName`] when the input does
    /// not satisfy the canonical grammar.
    pub fn new(value: &str) -> Result<Self, InvalidCanonicalActionName> {
        validate::validate_canonical(value)?;
        Ok(Self(value.to_owned()))
    }

    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CanonicalActionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CanonicalActionName {
    type Error = InvalidCanonicalActionName;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<String> for CanonicalActionName {
    type Error = InvalidCanonicalActionName;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        validate::validate_canonical(&s)?;
        Ok(Self(s))
    }
}

#[path = "mangle_validate.rs"]
mod validate;

/// Golden test constants for mangling — single source of truth for
/// unit and integration tests.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
#[path = "mangle_golden.rs"]
pub mod golden;

/// A path stem: a `.theorem` file path with its extension removed.
///
///     use theoremc::mangle::PathStem;
///     let stem = PathStem::from("foo/bar");
///     assert_eq!(stem.as_str(), "foo/bar");
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStem(String);

impl PathStem {
    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PathStem {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PathStem {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<String> for PathStem {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// ── Action name mangling ──────────────────────────────────────────

/// The module path into which mangled action identifiers resolve.
pub const RESOLUTION_TARGET: &str = "crate::theorem_actions";

/// The result of mangling a canonical action name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledAction {
    slug: String,
    hash: String,
    identifier: String,
    path: String,
}

impl MangledAction {
    /// The escaped slug portion (segments joined by `__`).
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The 12-character lowercase hex hash suffix.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The full mangled Rust identifier: `{slug}__h{hash12}`.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// The fully qualified resolution path:
    /// `crate::theorem_actions::{identifier}`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Escapes a single action-name segment by replacing `_` with `_u`.
///
///     # use theoremc::mangle::action_slug;
///     // segment_escape("deposit") == "deposit"
///     // segment_escape("attach_node") == "attach_unode"
///     assert_eq!(action_slug("ns.attach_node"), "ns__attach_unode");
#[must_use]
pub(crate) fn segment_escape(segment: &str) -> String {
    let mut result = String::with_capacity(segment.len() + segment.matches('_').count());
    for ch in segment.chars() {
        if ch == '_' {
            result.push_str("_u");
        } else {
            result.push(ch);
        }
    }
    result
}

/// Builds the escaped slug from a canonical action name.
///
/// Splits on `.`, escapes underscores in each segment, and
/// joins with `__`. Accepts `&str`, `String`, or
/// `&CanonicalActionName`.
///
///     use theoremc::mangle::{CanonicalActionName, action_slug};
///     assert_eq!(action_slug("account.deposit"), "account__deposit");
///     let name = CanonicalActionName::new("account.deposit")
///         .expect("valid canonical name");
///     assert_eq!(action_slug(&name), "account__deposit");
#[must_use]
pub fn action_slug(canonical_name: impl AsRef<str>) -> String {
    let s = canonical_name.as_ref();
    let mut slug = String::with_capacity(s.len() * 2);
    let mut segments = s.split('.');
    if let Some(first) = segments.next() {
        slug.push_str(&segment_escape(first));
    }
    for segment in segments {
        slug.push_str("__");
        slug.push_str(&segment_escape(segment));
    }
    slug
}

/// Computes the first 12 lowercase hex characters of the blake3
/// hash of the given value.
///
///     use theoremc::mangle::hash12;
///     assert_eq!(hash12("account.deposit"), "05158894bfb4");
#[must_use]
pub fn hash12(value: &str) -> String {
    let digest = blake3::hash(value.as_bytes());
    let hex = digest.to_hex();
    hex.as_str().get(..12).unwrap_or_default().to_owned()
}

/// Mangles a canonical action name into a [`MangledAction`].
/// Accepts `&str`, `String`, or `&CanonicalActionName`.
///
///     use theoremc::mangle::mangle_action_name;
///     let m = mangle_action_name("account.deposit");
///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
#[must_use]
pub fn mangle_action_name(canonical_name: impl AsRef<str>) -> MangledAction {
    let s = canonical_name.as_ref();
    let slug = action_slug(s);
    let hash = hash12(s);
    let identifier = format!("{slug}__h{hash}");
    let path = format!("{RESOLUTION_TARGET}::{identifier}");
    MangledAction {
        slug,
        hash,
        identifier,
        path,
    }
}

// ── Per-file module naming ─────────────────────────────────────────

/// The prefix for all generated per-file module names.
const MODULE_PREFIX: &str = "__theoremc__file__";

/// The result of mangling a `.theorem` file path into a per-file
/// Rust module name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledModule {
    stem: PathStem,
    mangled_stem: String,
    hash: String,
    module_name: String,
}

impl MangledModule {
    /// The original path stem (`P` with `.theorem` removed).
    #[must_use]
    pub fn stem(&self) -> &str {
        self.stem.as_str()
    }

    /// The sanitized stem after `path_mangle`.
    #[must_use]
    pub fn mangled_stem(&self) -> &str {
        &self.mangled_stem
    }

    /// The 12-character blake3 hash of the original path.
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The full generated module name.
    #[must_use]
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

/// Removes a trailing `.theorem` extension from `path`, returning
/// a [`PathStem`]. Returns the path unchanged when no `.theorem`
/// suffix exists.
///
///     use theoremc::mangle::path_stem;
///     assert_eq!(path_stem("foo/bar.theorem").as_str(), "foo/bar");
///     assert_eq!(path_stem("no_extension").as_str(), "no_extension");
#[must_use]
pub fn path_stem(path: impl AsRef<Utf8Path>) -> PathStem {
    let s = path.as_ref().as_str();
    PathStem(s.strip_suffix(".theorem").unwrap_or(s).to_owned())
}

/// Maps a single character to its mangled equivalent.
///
/// Path separators (`/`, `\`) and non-ASCII-alphanumeric characters
/// become `_`; ASCII alphanumerics and `_` are lowercased.
#[must_use]
const fn mangle_char(ch: char) -> char {
    match ch {
        '/' | '\\' => '_',
        _ if ch.is_ascii_alphanumeric() || ch == '_' => ch.to_ascii_lowercase(),
        _ => '_',
    }
}

/// Sanitizes a [`PathStem`] into a Rust-identifier-safe fragment.
///
/// Algorithm (per `docs/name-mangling-rules.md` §1):
///
/// 1. Map `/` and `\` to `_`.
/// 2. Map any character not in `[A-Za-z0-9_]` to `_`.
/// 3. Collapse consecutive `_` to a single `_`.
/// 4. Lowercase the result.
/// 5. If the result starts with a digit, prefix `_`.
///
/// # Examples
///
///     use theoremc::mangle::{PathStem, path_mangle};
///     assert_eq!(path_mangle(&PathStem::from("theorems/bidirectional")), "theorems_bidirectional");
///     assert_eq!(path_mangle(&PathStem::from("123foo")), "_123foo");
#[must_use]
pub fn path_mangle(stem: &PathStem) -> String {
    let s = stem.as_str();
    let mut buf = String::with_capacity(s.len() + 4);
    let mut prev_underscore = false;

    for ch in s.chars() {
        let mapped = mangle_char(ch);

        // Collapse consecutive underscores.
        if mapped == '_' && prev_underscore {
            continue;
        }

        buf.push(mapped);
        prev_underscore = mapped == '_';
    }

    // Step 5: prefix `_` if leading char is a digit.
    if buf.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        buf.insert(0, '_');
    }

    buf
}

/// Mangles a `.theorem` file path into a [`MangledModule`].
///
/// The `hash12` is computed from the **original** path, not the
/// mangled stem, so paths that sanitize identically still produce
/// distinct module names.
///
///     use theoremc::mangle::mangle_module_path;
///     let m = mangle_module_path("theorems/bidirectional.theorem");
///     assert_eq!(m.mangled_stem(), "theorems_bidirectional");
#[must_use]
pub fn mangle_module_path(path: impl AsRef<Utf8Path>) -> MangledModule {
    let path_ref = path.as_ref();
    let s = path_ref.as_str();
    let stem = path_stem(path_ref);
    let mangled_stem = path_mangle(&stem);
    let hash = hash12(s);
    let module_name = format!("{MODULE_PREFIX}{mangled_stem}__{hash}");
    MangledModule {
        stem,
        mangled_stem,
        hash,
        module_name,
    }
}

#[cfg(test)]
#[path = "mangle_tests.rs"]
mod tests;
