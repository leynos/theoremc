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
//! 1. `segment_escape` replaces `_` with `_u` in each segment.
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
//! 2. `path_mangle` sanitises the stem into an identifier-safe
//!    fragment (replace separators, collapse underscores, lowercase).
//! 3. `mangle_module_path` assembles the full module name using
//!    `hash12` of the **original** path for disambiguation.

// ── Domain newtypes ───────────────────────────────────────────────

/// A validated canonical action name (e.g. `"account.deposit"`).
///
/// # Examples
///
///     use theoremc::mangle::CanonicalActionName;
///
///     let name = CanonicalActionName::new_unchecked("account.deposit");
///     assert_eq!(name.as_str(), "account.deposit");
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalActionName(String);

impl CanonicalActionName {
    /// Wraps a pre-validated canonical action name.
    #[must_use]
    pub fn new_unchecked(value: &str) -> Self {
        Self(value.to_owned())
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

/// A path stem: a `.theorem` file path with its extension removed.
///
/// # Examples
///
///     use theoremc::mangle::PathStem;
///
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
/// # Examples
///
///     use theoremc::mangle::segment_escape;
///
///     assert_eq!(segment_escape("deposit"), "deposit");
///     assert_eq!(segment_escape("attach_node"), "attach_unode");
#[must_use]
pub fn segment_escape(segment: &str) -> String {
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

/// Builds the escaped slug from a [`CanonicalActionName`].
///
/// Splits the name on `.`, applies [`segment_escape`] to each
/// segment, and joins the escaped segments with `__`.
///
/// # Examples
///
///     use theoremc::mangle::{CanonicalActionName, action_slug};
///
///     let name = CanonicalActionName::new_unchecked("account.deposit");
///     assert_eq!(action_slug(&name), "account__deposit");
#[must_use]
pub fn action_slug(canonical_name: &CanonicalActionName) -> String {
    let mut slug = String::with_capacity(canonical_name.as_str().len() * 2);
    let mut segments = canonical_name.as_str().split('.');
    if let Some(first) = segments.next() {
        slug.push_str(&segment_escape(first));
    }
    for segment in segments {
        slug.push_str("__");
        slug.push_str(&segment_escape(segment));
    }
    slug
}

/// Computes the first 12 lowercase hex characters of the blake3 hash
/// of the given value.
///
/// # Examples
///
///     use theoremc::mangle::hash12;
///
///     let h = hash12("account.deposit");
///     assert_eq!(h, "05158894bfb4");
#[must_use]
pub fn hash12(value: &str) -> String {
    let digest = blake3::hash(value.as_bytes());
    let hex = digest.to_hex();
    hex.as_str().get(..12).unwrap_or_default().to_owned()
}

/// Mangles a [`CanonicalActionName`] into a [`MangledAction`].
///
/// # Examples
///
///     use theoremc::mangle::{CanonicalActionName, mangle_action_name};
///
///     let name = CanonicalActionName::new_unchecked("account.deposit");
///     let m = mangle_action_name(&name);
///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
#[must_use]
pub fn mangle_action_name(canonical_name: &CanonicalActionName) -> MangledAction {
    let slug = action_slug(canonical_name);
    let hash = hash12(canonical_name.as_str());
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

    /// The sanitised stem after `path_mangle`.
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
/// # Examples
///
///     use theoremc::mangle::path_stem;
///
///     assert_eq!(path_stem("foo/bar.theorem").as_str(), "foo/bar");
///     assert_eq!(path_stem("no_extension").as_str(), "no_extension");
#[must_use]
pub fn path_stem(path: &str) -> PathStem {
    PathStem(path.strip_suffix(".theorem").unwrap_or(path).to_owned())
}

fn replace_separators_and_special_chars(stem: &str) -> String {
    let mut buf = String::with_capacity(stem.len() + 4);
    for ch in stem.chars() {
        match ch {
            '/' | '\\' => buf.push_str("__"),
            _ if ch.is_ascii_alphanumeric() || ch == '_' => buf.push(ch),
            _ => buf.push('_'),
        }
    }
    buf
}

fn collapse_consecutive_underscores(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_underscore = false;
    for ch in s.chars() {
        if ch == '_' {
            if !prev_underscore {
                result.push('_');
            }
            prev_underscore = true;
        } else {
            result.push(ch);
            prev_underscore = false;
        }
    }
    result
}

fn prefix_if_digit(mut s: String) -> String {
    if s.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        s.insert(0, '_');
    }
    s
}

/// Sanitises a [`PathStem`] into a Rust-identifier-safe fragment.
///
/// Algorithm (per `docs/name-mangling-rules.md` §1):
///
/// 1. Replace `/` and `\` with `__`.
/// 2. Replace any character not in `[A-Za-z0-9_]` with `_`.
/// 3. Collapse consecutive `_` to a single `_`.
/// 4. Lowercase the result.
/// 5. If the result starts with a digit, prefix `_`.
///
/// # Examples
///
///     use theoremc::mangle::{PathStem, path_mangle};
///
///     assert_eq!(path_mangle(&PathStem::from("theorems/bidirectional")), "theorems_bidirectional");
///     assert_eq!(path_mangle(&PathStem::from("123foo")), "_123foo");
#[must_use]
pub fn path_mangle(stem: &PathStem) -> String {
    let sanitized = replace_separators_and_special_chars(stem.as_str());
    let collapsed = collapse_consecutive_underscores(&sanitized);
    let lowered = collapsed.to_ascii_lowercase();
    prefix_if_digit(lowered)
}

/// Mangles a `.theorem` file path into a [`MangledModule`].
///
/// The `hash12` is computed from the **original** path, not the
/// mangled stem, so paths that sanitise identically still produce
/// distinct module names.
///
/// # Examples
///
///     use theoremc::mangle::mangle_module_path;
///
///     let m = mangle_module_path("theorems/bidirectional.theorem");
///     assert_eq!(m.mangled_stem(), "theorems_bidirectional");
#[must_use]
pub fn mangle_module_path(path: &str) -> MangledModule {
    let stem = path_stem(path);
    let mangled_stem = path_mangle(&stem);
    let hash = hash12(path);
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
