//! Name mangling for deterministic, collision-resistant resolution.
//!
//! Provides three mangling pipelines, all following
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
//!
//! # Theorem harness naming
//!
//! Transforms theorem identifiers and literal source paths into stable Kani
//! harness identifiers of the form
//! `theorem__{theorem_slug(T)}__h{hash12(P#T)}`.
//!
//! 1. `theorem_key` builds the exact key `{P}#{T}`.
//! 2. `theorem_slug` preserves snake-case identifiers and otherwise performs
//!    deterministic acronym-aware snake-case conversion.
//! 3. `mangle_theorem_harness` assembles the final harness identifier.

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
///     use theoremc_core::mangle::CanonicalActionName;
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

#[path = "mangle_harness.rs"]
mod harness;
#[path = "mangle_path.rs"]
mod path;

pub use harness::{MangledHarness, mangle_theorem_harness, theorem_key, theorem_slug};
pub use path::{MangledModule, PathStem, mangle_module_path, path_mangle, path_stem};

#[cfg(test)]
pub(crate) use path::MODULE_PREFIX;

#[cfg(test)]
mod prop_tests {
    //! Property-based tests for mangling determinism and uniqueness.

    use super::{mangle_module_path, mangle_theorem_harness};
    use proptest::prelude::{prop_assert_eq, prop_assert_ne, prop_assume, proptest};

    proptest! {
        /// `mangle_module_path` must be deterministic: same input always
        /// produces same output.
        #[test]
        fn mangle_module_path_is_deterministic(
            path in "[a-z0-9/._-]{1,40}\\.theorem",
        ) {
            let first = mangle_module_path(&path);
            let second = mangle_module_path(&path);
            prop_assert_eq!(first.module_name(), second.module_name());
        }

        /// `mangle_theorem_harness` must be deterministic.
        #[test]
        fn mangle_theorem_harness_is_deterministic(
            path in "[a-z0-9/._-]{1,40}\\.theorem",
            theorem in "[A-Za-z][A-Za-z0-9]{0,30}",
        ) {
            let first = mangle_theorem_harness(&path, &theorem);
            let second = mangle_theorem_harness(&path, &theorem);
            prop_assert_eq!(first.identifier(), second.identifier());
        }

        /// Different theorem names within the same file must not produce the
        /// same harness identifier.
        #[test]
        fn distinct_theorem_names_produce_distinct_harnesses(
            path in "[a-z]{1,8}\\.theorem",
            a in "[A-Za-z][A-Za-z0-9]{1,15}",
            b in "[A-Za-z][A-Za-z0-9]{1,15}",
        ) {
            prop_assume!(a != b);
            let ha = mangle_theorem_harness(&path, &a);
            let hb = mangle_theorem_harness(&path, &b);
            prop_assert_ne!(ha.identifier(), hb.identifier());
        }
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
///     # use theoremc_core::mangle::action_slug;
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
///     use theoremc_core::mangle::{CanonicalActionName, action_slug};
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
///     use theoremc_core::mangle::hash12;
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
///     use theoremc_core::mangle::mangle_action_name;
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

#[cfg(test)]
#[path = "mangle_tests.rs"]
mod tests;
