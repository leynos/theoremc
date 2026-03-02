//! Unit tests for action name mangling and per-file module naming.
//!
//! Golden hash values are computed from blake3 and hardcoded to
//! detect any accidental algorithm or dependency changes.

use rstest::rstest;

use super::*;

// ── Helpers ─────────────────────────────────────────────────────

/// Shorthand for constructing a [`CanonicalActionName`] in tests.
fn can(name: &str) -> CanonicalActionName {
    CanonicalActionName::new_unchecked(name)
}

// ── Action name mangling ──────────────────────────────────────────

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
    assert_eq!(action_slug(can(input)), expected);
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
            .all(|c| c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_ascii_lowercase())),
        "expected lowercase hex, got: {result}",
    );
}

#[test]
fn hash12_is_deterministic() {
    let first = hash12("account.deposit");
    let second = hash12("account.deposit");
    assert_eq!(first, second);
}

#[rstest]
#[case::account_deposit("account.deposit", "account__deposit", "05158894bfb4")]
#[case::hnsw_attach_node("hnsw.attach_node", "hnsw__attach_unode", "8d74e77b55f2")]
#[case::three_segments(
    "hnsw.graph.with_capacity",
    "hnsw__graph__with_ucapacity",
    "9eafdf8834ec"
)]
#[case::leading_underscores("_a._b", "_ua___ub", "0a39aa24f512")]
fn mangle_action_name_golden_cases(
    #[case] canonical: &str,
    #[case] expected_slug: &str,
    #[case] expected_hash: &str,
) {
    let m = mangle_action_name(can(canonical));
    assert_eq!(m.slug(), expected_slug, "slug");
    assert_eq!(m.hash(), expected_hash, "hash");
    let expected_ident = format!("{expected_slug}__h{expected_hash}");
    assert_eq!(m.identifier(), expected_ident, "identifier");
    let expected_path = format!("{RESOLUTION_TARGET}::{expected_ident}");
    assert_eq!(m.path(), expected_path, "path");
}

#[test]
fn underscore_placement_produces_distinct_slugs() {
    let slug_a = action_slug(can("a.b_c"));
    let slug_b = action_slug(can("a_b.c"));
    assert_ne!(slug_a, slug_b);
}

#[test]
fn underscore_placement_produces_distinct_identifiers() {
    let m_a = mangle_action_name(can("a.b_c"));
    let m_b = mangle_action_name(can("a_b.c"));
    assert_ne!(m_a.identifier(), m_b.identifier());
}

#[test]
fn action_path_structural_properties() {
    let m = mangle_action_name(can("account.deposit"));
    let prefix = format!("{RESOLUTION_TARGET}::");
    assert!(m.path().starts_with(&prefix));
    assert!(m.path().ends_with(m.identifier()));
}

// ── path_stem tests ───────────────────────────────────────────────

#[rstest]
#[case::removes_theorem_ext("foo/bar.theorem", "foo/bar")]
#[case::no_extension("no_extension", "no_extension")]
#[case::double_extension("foo.theorem.theorem", "foo.theorem")]
#[case::empty_string("", "")]
#[case::only_extension(".theorem", "")]
#[case::different_extension("foo.txt", "foo.txt")]
#[case::theorem_mid_path("theorem/bar.theorem", "theorem/bar")]
fn path_stem_cases(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(path_stem(input).as_str(), expected);
}

// ── path_mangle tests ────────────────────────────────────────────

#[rstest]
#[case::simple("theorems/bidirectional", "theorems_bidirectional")]
#[case::nested("theorems/nested/deep/path", "theorems_nested_deep_path")]
#[case::backslash("theorems\\windows\\style", "theorems_windows_style")]
#[case::hyphen("theorems/my-file", "theorems_my_file")]
#[case::underscore("theorems/my_file", "theorems_my_file")]
#[case::uppercase("theorems/UPPER-case", "theorems_upper_case")]
#[case::digit_leading("123foo", "_123foo")]
#[case::all_digits("42", "_42")]
#[case::mixed_separators("a/b\\c", "a_b_c")]
#[case::consecutive_specials("a--b..c", "a_b_c")]
#[case::trailing_separator("dir/", "dir_")]
#[case::no_transform("simple", "simple")]
#[case::unicode_non_ascii("théorèmes/αβ", "th_or_mes_")]
fn path_mangle_cases(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(path_mangle(&PathStem::from(input)), expected);
}

// ── mangle_module_path golden tests ──────────────────────────────

/// Expected golden values for a mangled module path.
struct ModuleGolden<'a> {
    path: &'a str,
    stem: &'a str,
    mangled_stem: &'a str,
    hash: &'a str,
    module_name: &'a str,
}

impl ModuleGolden<'_> {
    fn assert(&self) {
        let m = mangle_module_path(self.path);
        assert_eq!(m.stem(), self.stem, "stem");
        assert_eq!(m.mangled_stem(), self.mangled_stem, "mangled_stem");
        assert_eq!(m.hash(), self.hash, "hash");
        assert_eq!(m.module_name(), self.module_name, "module_name");
    }
}

#[rstest]
#[case::simple_path(
    "theorems/bidirectional.theorem",
    "theorems_bidirectional",
    "1fc14bdf614f",
    "__theoremc__file__theorems_bidirectional__1fc14bdf614f"
)]
#[case::nested_path(
    "theorems/nested/deep/path.theorem",
    "theorems_nested_deep_path",
    "5cb0a56a3468",
    "__theoremc__file__theorems_nested_deep_path__5cb0a56a3468"
)]
#[case::backslash_path(
    "theorems\\windows\\style.theorem",
    "theorems_windows_style",
    "38b12c01ea29",
    "__theoremc__file__theorems_windows_style__38b12c01ea29"
)]
#[case::uppercase_path(
    "theorems/UPPER-case.theorem",
    "theorems_upper_case",
    "7ee5f747b4c1",
    "__theoremc__file__theorems_upper_case__7ee5f747b4c1"
)]
#[case::no_extension(
    "no_extension",
    "no_extension",
    "afb36ed5206f",
    "__theoremc__file__no_extension__afb36ed5206f"
)]
#[case::digit_leading(
    "theorems/123_digit_leading.theorem",
    "theorems_123_digit_leading",
    "76c6c1009e0d",
    concat!(
        "__theoremc__file__theorems_123_digit_leading",
        "__76c6c1009e0d",
    ),
)]
#[case::empty_path("", "", "af1349b9f5f9", "__theoremc__file____af1349b9f5f9")]
#[case::dot_theorem(".theorem", "", "f9d6885cf913", "__theoremc__file____f9d6885cf913")]
fn mangle_module_path_golden_cases(
    #[case] path: &str,
    #[case] expected_mangled_stem: &str,
    #[case] expected_hash: &str,
    #[case] expected_module_name: &str,
) {
    let stem = path_stem(path);
    ModuleGolden {
        path,
        stem: stem.as_str(),
        mangled_stem: expected_mangled_stem,
        hash: expected_hash,
        module_name: expected_module_name,
    }
    .assert();
}

// ── Disambiguation tests ─────────────────────────────────────────

#[test]
fn hyphen_vs_underscore_disambiguation() {
    let m_hyphen = mangle_module_path("theorems/my-file.theorem");
    let m_under = mangle_module_path("theorems/my_file.theorem");
    assert_eq!(m_hyphen.mangled_stem(), m_under.mangled_stem());
    assert_ne!(m_hyphen.module_name(), m_under.module_name());
}

#[test]
fn forward_vs_backslash_disambiguation() {
    let m_fwd = mangle_module_path("theorems/windows/style.theorem");
    let m_back = mangle_module_path("theorems\\windows\\style.theorem");
    assert_eq!(m_fwd.mangled_stem(), m_back.mangled_stem());
    assert_ne!(m_fwd.module_name(), m_back.module_name());
}

#[test]
fn module_name_structural_properties() {
    let m = mangle_module_path("theorems/bidirectional.theorem");
    assert!(m.module_name().starts_with("__theoremc__file__"));
    assert!(m.module_name().ends_with(m.hash()));
    assert_eq!(m, mangle_module_path("theorems/bidirectional.theorem"));
}
