//! Unit tests for action name mangling and per-file module naming.
//!
//! Golden hash values are computed from blake3 and hardcoded to
//! detect any accidental algorithm or dependency changes.

use rstest::rstest;

use super::*;

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

/// Expected golden values for a mangled action name.
struct ActionGolden<'a> {
    canonical: &'a str,
    slug: &'a str,
    hash: &'a str,
    identifier: &'a str,
    path: &'a str,
}

impl ActionGolden<'_> {
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
    ActionGolden {
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
    ActionGolden {
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
    ActionGolden {
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
    ActionGolden {
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
    assert_eq!(path_stem(input), expected);
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
fn path_mangle_cases(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(path_mangle(input), expected);
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

#[test]
fn golden_simple_path() {
    ModuleGolden {
        path: "theorems/bidirectional.theorem",
        stem: "theorems/bidirectional",
        mangled_stem: "theorems_bidirectional",
        hash: "1fc14bdf614f",
        module_name: "__theoremc__file__theorems_bidirectional__1fc14bdf614f",
    }
    .assert();
}

#[test]
fn golden_nested_path() {
    ModuleGolden {
        path: "theorems/nested/deep/path.theorem",
        stem: "theorems/nested/deep/path",
        mangled_stem: "theorems_nested_deep_path",
        hash: "5cb0a56a3468",
        module_name: "__theoremc__file__theorems_nested_deep_path__5cb0a56a3468",
    }
    .assert();
}

#[test]
fn golden_backslash_path() {
    ModuleGolden {
        path: "theorems\\windows\\style.theorem",
        stem: "theorems\\windows\\style",
        mangled_stem: "theorems_windows_style",
        hash: "38b12c01ea29",
        module_name: "__theoremc__file__theorems_windows_style__38b12c01ea29",
    }
    .assert();
}

#[test]
fn golden_uppercase_path() {
    ModuleGolden {
        path: "theorems/UPPER-case.theorem",
        stem: "theorems/UPPER-case",
        mangled_stem: "theorems_upper_case",
        hash: "7ee5f747b4c1",
        module_name: "__theoremc__file__theorems_upper_case__7ee5f747b4c1",
    }
    .assert();
}

#[test]
fn golden_no_extension() {
    ModuleGolden {
        path: "no_extension",
        stem: "no_extension",
        mangled_stem: "no_extension",
        hash: "afb36ed5206f",
        module_name: "__theoremc__file__no_extension__afb36ed5206f",
    }
    .assert();
}

#[test]
fn golden_digit_leading() {
    ModuleGolden {
        path: "theorems/123_digit_leading.theorem",
        stem: "theorems/123_digit_leading",
        mangled_stem: "theorems_123_digit_leading",
        hash: "76c6c1009e0d",
        module_name: concat!(
            "__theoremc__file__theorems_123_digit_leading",
            "__76c6c1009e0d",
        ),
    }
    .assert();
}

// ── Disambiguation tests ─────────────────────────────────────────

#[test]
fn hyphen_vs_underscore_disambiguation() {
    // "my-file" and "my_file" both mangle to the same stem but
    // the hash of the original path disambiguates.
    let m_hyphen = mangle_module_path("theorems/my-file.theorem");
    let m_under = mangle_module_path("theorems/my_file.theorem");
    assert_eq!(
        m_hyphen.mangled_stem(),
        m_under.mangled_stem(),
        "mangled stems should be identical",
    );
    assert_ne!(
        m_hyphen.module_name(),
        m_under.module_name(),
        "module names must differ due to hash",
    );
}

#[test]
fn forward_vs_backslash_disambiguation() {
    // Forward and backslash paths mangle identically but the
    // original-path hash keeps module names distinct.
    let m_fwd = mangle_module_path("theorems/windows/style.theorem");
    let m_back = mangle_module_path("theorems\\windows\\style.theorem");
    assert_eq!(
        m_fwd.mangled_stem(),
        m_back.mangled_stem(),
        "mangled stems should be identical",
    );
    assert_ne!(
        m_fwd.module_name(),
        m_back.module_name(),
        "module names must differ due to hash",
    );
}

#[test]
fn module_name_starts_with_prefix() {
    let m = mangle_module_path("theorems/bidirectional.theorem");
    assert!(
        m.module_name().starts_with("__theoremc__file__"),
        "module name must start with prefix: {}",
        m.module_name(),
    );
}

#[test]
fn module_name_ends_with_hash() {
    let m = mangle_module_path("theorems/bidirectional.theorem");
    assert!(
        m.module_name().ends_with(m.hash()),
        "module name must end with hash: {}",
        m.module_name(),
    );
}

#[test]
fn module_name_is_deterministic() {
    let first = mangle_module_path("theorems/bidirectional.theorem");
    let second = mangle_module_path("theorems/bidirectional.theorem");
    assert_eq!(first, second);
}
