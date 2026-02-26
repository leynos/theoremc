//! Action name mangling for deterministic, injective resolution.
//!
//! Transforms canonical dot-separated action names into mangled Rust
//! identifiers suitable for binding in `crate::theorem_actions`. The
//! algorithm follows `docs/name-mangling-rules.md` and produces stable,
//! injective identifiers from validated canonical action names.
//!
//! # Mangling pipeline
//!
//! 1. `segment_escape` replaces `_` with `_u` in each segment.
//! 2. `action_slug` splits on `.`, escapes each segment, and joins
//!    with `__`.
//! 3. `hash12` computes the first 12 lowercase hex characters of
//!    the blake3 digest.
//! 4. `mangle_action_name` assembles the full mangled identifier and
//!    resolution path.

/// The module path into which mangled action identifiers resolve.
///
/// All mangled action names resolve to identifiers within this module.
/// Use this constant (or [`MangledAction::path`]) instead of
/// hard-coding the resolution target string.
pub const RESOLUTION_TARGET: &str = "crate::theorem_actions";

/// The result of mangling a canonical action name.
///
/// Contains the individual components (slug, hash) and the fully
/// assembled mangled identifier and resolution path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledAction {
    slug: String,
    hash: String,
    identifier: String,
    path: String,
}

impl MangledAction {
    /// The escaped slug portion (segments joined by `__`).
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.slug(), "account__deposit");
    #[must_use]
    pub fn slug(&self) -> &str {
        &self.slug
    }

    /// The 12-character lowercase hex hash suffix.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.hash().len(), 12);
    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// The full mangled Rust identifier: `{slug}__h{hash12}`.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// The fully qualified resolution path:
    /// `crate::theorem_actions::{identifier}`.
    ///
    /// # Examples
    ///
    ///     use theoremc::mangle::mangle_action_name;
    ///
    ///     let m = mangle_action_name("account.deposit");
    ///     assert!(m.path().starts_with("crate::theorem_actions::"));
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Escapes a single action-name segment by replacing `_` with `_u`.
///
/// ASCII letters and digits are left unchanged. This function assumes
/// the segment has already passed canonical action-name validation
/// (i.e. matches `^[A-Za-z_][A-Za-z0-9_]*$`).
///
/// # Examples
///
///     use theoremc::mangle::segment_escape;
///
///     assert_eq!(segment_escape("deposit"), "deposit");
///     assert_eq!(segment_escape("attach_node"), "attach_unode");
///     assert_eq!(segment_escape("_private"), "_uprivate");
///     assert_eq!(segment_escape("__double"), "_u_udouble");
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

/// Builds the escaped slug from a canonical dot-separated action name.
///
/// Splits `canonical_name` on `.`, applies [`segment_escape`] to each
/// segment, and joins the escaped segments with `__`.
///
/// This function assumes the input has already passed canonical
/// action-name validation.
///
/// # Examples
///
///     use theoremc::mangle::action_slug;
///
///     assert_eq!(action_slug("account.deposit"), "account__deposit");
///     assert_eq!(
///         action_slug("hnsw.attach_node"),
///         "hnsw__attach_unode",
///     );
///     assert_eq!(
///         action_slug("hnsw.graph.with_capacity"),
///         "hnsw__graph__with_ucapacity",
///     );
#[must_use]
pub fn action_slug(canonical_name: &str) -> String {
    let mut slug = String::with_capacity(canonical_name.len() * 2);
    let mut segments = canonical_name.split('.');
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

/// Mangles a canonical action name into a [`MangledAction`].
///
/// Produces a deterministic, injective Rust identifier and a fully
/// qualified resolution path into `crate::theorem_actions`.
///
/// This function assumes the input has already passed canonical
/// action-name validation (see
/// `validate_canonical_action_name` from the `schema` module).
///
/// # Examples
///
///     use theoremc::mangle::mangle_action_name;
///
///     let m = mangle_action_name("account.deposit");
///     assert_eq!(m.slug(), "account__deposit");
///     assert_eq!(m.hash(), "05158894bfb4");
///     assert_eq!(m.identifier(), "account__deposit__h05158894bfb4");
///     assert_eq!(
///         m.path(),
///         "crate::theorem_actions::account__deposit__h05158894bfb4",
///     );
#[must_use]
pub fn mangle_action_name(canonical_name: &str) -> MangledAction {
    let slug = action_slug(canonical_name);
    let hash = hash12(canonical_name);
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
mod tests {
    //! Unit tests for action name mangling.
    //!
    //! Golden hash values are computed from blake3 and hardcoded to
    //! detect any accidental algorithm or dependency changes.

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::no_underscores("deposit", "deposit")]
    #[case::single_underscore("attach_node", "attach_unode")]
    #[case::multiple_underscores("graph_with_capacity", "graph_uwith_ucapacity")]
    #[case::leading_underscore("_private", "_uprivate")]
    #[case::consecutive_underscores("__double", "_u_udouble")]
    #[case::lone_underscore("_", "_u")]
    #[case::single_char("a", "a")]
    #[case::alphanumeric("A1", "A1")]
    #[case::already_escaped_looking("_u", "_uu")]
    #[case::multiple_mid_underscores("a_b_c", "a_ub_uc")]
    fn segment_escape_cases(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(segment_escape(input), expected);
    }

    #[rstest]
    #[case::two_segments("account.deposit", "account__deposit")]
    #[case::underscore_segment("hnsw.attach_node", "hnsw__attach_unode")]
    #[case::three_segments("hnsw.graph.with_capacity", "hnsw__graph__with_ucapacity")]
    #[case::leading_underscores("_a._b", "_ua___ub")]
    #[case::lone_underscore_segment("ns._", "ns___u")]
    #[case::minimal("x.y", "x__y")]
    fn action_slug_cases(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(action_slug(input), expected);
    }

    #[rstest]
    #[case::account_deposit("account.deposit", "05158894bfb4")]
    #[case::hnsw_attach_node("hnsw.attach_node", "8d74e77b55f2")]
    #[case::three_segments("hnsw.graph.with_capacity", "9eafdf8834ec")]
    #[case::leading_underscores("_a._b", "0a39aa24f512")]
    #[case::lone_underscore("ns._", "ef4f43e71ce0")]
    #[case::minimal("x.y", "f12518d733b0")]
    fn hash12_golden_values(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(hash12(input), expected);
    }

    #[test]
    fn hash12_length_is_always_twelve() {
        let inputs = ["a.b", "very.long.multi.segment.action.name", "_._"];
        for input in &inputs {
            assert_eq!(hash12(input).len(), 12, "hash12({input})");
        }
    }

    #[test]
    fn hash12_is_lowercase_hex() {
        let result = hash12("account.deposit");
        assert!(
            result
                .chars()
                .all(|c| c.is_ascii_hexdigit()
                    && (!c.is_ascii_alphabetic() || c.is_ascii_lowercase())),
            "expected lowercase hex, got: {result}",
        );
    }

    #[test]
    fn hash12_is_deterministic() {
        let first = hash12("account.deposit");
        let second = hash12("account.deposit");
        assert_eq!(first, second);
    }

    /// Expected golden values for a mangled action name.
    struct Golden<'a> {
        canonical: &'a str,
        slug: &'a str,
        hash: &'a str,
        identifier: &'a str,
        path: &'a str,
    }

    impl Golden<'_> {
        fn assert(&self) {
            let m = mangle_action_name(self.canonical);
            assert_eq!(m.slug(), self.slug, "slug");
            assert_eq!(m.hash(), self.hash, "hash");
            assert_eq!(m.identifier(), self.identifier, "identifier");
            assert_eq!(m.path(), self.path, "path");
        }
    }

    /// Builds the expected resolution path from `RESOLUTION_TARGET`
    /// and the given identifier, so the target is not duplicated.
    fn expected_path(identifier: &str) -> String {
        format!("{RESOLUTION_TARGET}::{identifier}")
    }

    #[test]
    fn golden_account_deposit() {
        Golden {
            canonical: "account.deposit",
            slug: "account__deposit",
            hash: "05158894bfb4",
            identifier: "account__deposit__h05158894bfb4",
            path: &expected_path("account__deposit__h05158894bfb4"),
        }
        .assert();
    }

    #[test]
    fn golden_hnsw_attach_node() {
        Golden {
            canonical: "hnsw.attach_node",
            slug: "hnsw__attach_unode",
            hash: "8d74e77b55f2",
            identifier: "hnsw__attach_unode__h8d74e77b55f2",
            path: &expected_path("hnsw__attach_unode__h8d74e77b55f2"),
        }
        .assert();
    }

    #[test]
    fn golden_three_segments() {
        Golden {
            canonical: "hnsw.graph.with_capacity",
            slug: "hnsw__graph__with_ucapacity",
            hash: "9eafdf8834ec",
            identifier: "hnsw__graph__with_ucapacity__h9eafdf8834ec",
            path: &expected_path("hnsw__graph__with_ucapacity__h9eafdf8834ec"),
        }
        .assert();
    }

    #[test]
    fn golden_leading_underscores() {
        Golden {
            canonical: "_a._b",
            slug: "_ua___ub",
            hash: "0a39aa24f512",
            identifier: "_ua___ub__h0a39aa24f512",
            path: &expected_path("_ua___ub__h0a39aa24f512"),
        }
        .assert();
    }

    #[test]
    fn underscore_placement_produces_distinct_slugs() {
        // "a.b_c" and "a_b.c" must produce different slugs to preserve
        // injectivity.
        let slug_a = action_slug("a.b_c");
        let slug_b = action_slug("a_b.c");
        assert_ne!(
            slug_a, slug_b,
            "different canonical names must produce different slugs: \
             {slug_a} vs {slug_b}",
        );
    }

    #[test]
    fn underscore_placement_produces_distinct_identifiers() {
        let m_a = mangle_action_name("a.b_c");
        let m_b = mangle_action_name("a_b.c");
        assert_ne!(
            m_a.identifier(),
            m_b.identifier(),
            "different canonical names must produce different identifiers",
        );
    }

    #[test]
    fn path_starts_with_resolution_target() {
        let m = mangle_action_name("account.deposit");
        let prefix = format!("{RESOLUTION_TARGET}::");
        assert!(
            m.path().starts_with(&prefix),
            "path must begin with resolution target: {}",
            m.path(),
        );
    }

    #[test]
    fn path_ends_with_identifier() {
        let m = mangle_action_name("account.deposit");
        assert!(
            m.path().ends_with(m.identifier()),
            "path must end with identifier: {}",
            m.path(),
        );
    }
}
