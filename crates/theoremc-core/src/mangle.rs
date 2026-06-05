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
//!
//! # Collision safety
//!
//! Collision resistance of the 12-hex truncation is not relied upon for
//! correctness. The build performs compile-time collision detection over all
//! generated identifiers and fails fast if a collision is found. The truncated
//! hash is used as a disambiguator to reduce collision probability; safety is
//! enforced by detection.

pub use crate::canonical_action_name::{CanonicalActionName, InvalidCanonicalActionName};

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

    use super::{
        CanonicalActionName, hash12, mangle_action_name, mangle_module_path,
        mangle_theorem_harness, try_action_slug, try_mangle_action_name,
    };
    use proptest::prelude::{
        Strategy, prop_assert, prop_assert_eq, prop_assert_ne, prop_assume, proptest,
    };

    fn valid_segment_strategy() -> impl Strategy<Value = String> {
        ("[A-Za-z_]", "[A-Za-z0-9_]{0,8}")
            .prop_map(|(first, rest)| format!("{first}{rest}"))
            .prop_filter("segment must not be a Rust keyword", |segment| {
                !crate::canonical_action_name::is_rust_reserved_keyword(segment)
            })
    }

    fn valid_canonical_action_name_strategy() -> impl Strategy<Value = String> {
        (
            valid_segment_strategy(),
            valid_segment_strategy(),
            proptest::collection::vec(valid_segment_strategy(), 0..4),
        )
            .prop_map(|(first, second, rest)| {
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join(".")
            })
    }

    fn is_identifier_safe(identifier: &str) -> bool {
        let mut chars = identifier.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        (first.is_ascii_alphabetic() || first == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    proptest! {
        /// Valid generated canonical action names round-trip through the
        /// checked mangle APIs and produce Rust-identifier-safe names.
        #[test]
        fn valid_canonical_names_round_trip_through_checked_mangling(
            name in valid_canonical_action_name_strategy(),
        ) {
            let canonical = CanonicalActionName::new(&name)?;
            let checked_slug = try_action_slug(&name)?;
            let checked_mangle = try_mangle_action_name(&name)?;
            let typed_mangle = mangle_action_name(&canonical);

            prop_assert_eq!(canonical.as_str(), name);
            prop_assert_eq!(checked_slug, typed_mangle.slug());
            prop_assert!(is_identifier_safe(typed_mangle.identifier()));
            prop_assert_eq!(checked_mangle, typed_mangle);
        }

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

        /// `hash12` must produce distinct outputs for distinct inputs across a
        /// wide variety of canonical action names; empirically validates
        /// collision-resistance of the 12-hex-character truncation.
        #[test]
        fn hash12_is_distinct_for_diverse_canonical_names(
            a in "[a-z][a-z0-9]{0,10}\\.[a-z][a-z0-9]{0,10}",
            b in "[a-z][a-z0-9]{0,10}\\.[a-z][a-z0-9]{0,10}",
        ) {
            prop_assume!(a != b);
            prop_assert_ne!(
                hash12(&a),
                hash12(&b),
                "hash12 collision between '{}' and '{}'",
                a,
                b,
            );
        }

        /// `hash12` must be deterministic.
        #[test]
        fn hash12_is_deterministic(
            input in "[a-z0-9._-]{1,50}",
        ) {
            prop_assert_eq!(hash12(&input), hash12(&input));
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
///     # use theoremc_core::mangle::{CanonicalActionName, action_slug};
///     // segment_escape("deposit") == "deposit"
///     // segment_escape("attach_node") == "attach_unode"
///     let name = CanonicalActionName::new("ns.attach_node")
///         .expect("valid canonical action name");
///     assert_eq!(action_slug(&name), "ns__attach_unode");
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
/// Splits on `.`, escapes underscores in each segment, and joins with `__`.
///
///     use theoremc_core::mangle::{CanonicalActionName, action_slug};
///     let name = CanonicalActionName::new("account.deposit")
///         .expect("valid canonical name");
///     assert_eq!(action_slug(&name), "account__deposit");
#[must_use]
pub fn action_slug(canonical_name: &CanonicalActionName) -> String {
    let s = canonical_name.as_str();
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

/// Validates and builds the escaped slug from a canonical action-name string.
///
/// # Errors
///
/// Returns [`InvalidCanonicalActionName`] when the input does not satisfy the
/// canonical action-name grammar.
pub fn try_action_slug(name: &str) -> Result<String, InvalidCanonicalActionName> {
    let canonical_name = CanonicalActionName::new(name)?;
    Ok(action_slug(&canonical_name))
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
///
///     use theoremc_core::mangle::{CanonicalActionName, mangle_action_name};
///     let name = CanonicalActionName::new("account.deposit")
///         .expect("valid canonical action name");
///     let m = mangle_action_name(&name);
///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
#[must_use]
pub fn mangle_action_name(canonical_name: &CanonicalActionName) -> MangledAction {
    let s = canonical_name.as_str();
    let slug = action_slug(canonical_name);
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

/// Validates and mangles a canonical action-name string into a
/// [`MangledAction`].
///
/// # Errors
///
/// Returns [`InvalidCanonicalActionName`] when the input does not satisfy the
/// canonical action-name grammar.
pub fn try_mangle_action_name(name: &str) -> Result<MangledAction, InvalidCanonicalActionName> {
    let canonical_name = CanonicalActionName::new(name)?;
    Ok(mangle_action_name(&canonical_name))
}

#[cfg(test)]
#[path = "mangle_tests.rs"]
mod tests;
