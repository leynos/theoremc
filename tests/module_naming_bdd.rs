//! Behavioural tests for per-file module naming.

use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::golden::MODULE_GOLDEN_TUPLES;
use theoremc::mangle::{hash12, mangle_module_path};

// ── Scenario: Simple paths produce deterministic module names ─────

#[given("representative .theorem file paths")]
fn given_representative_theorem_file_paths() {}

#[then("each path produces the expected module name")]
fn then_each_path_produces_the_expected_module_name() {
    for (path, expected_stem, expected_mangled, expected_hash) in MODULE_GOLDEN_TUPLES {
        let m = mangle_module_path(path);
        assert_eq!(m.stem(), *expected_stem, "stem mismatch for {path}");
        assert_eq!(
            m.mangled_stem(),
            *expected_mangled,
            "mangled_stem mismatch for {path}",
        );
        assert_eq!(m.hash(), *expected_hash, "hash mismatch for {path}");

        let expected_name = format!("__theoremc__file__{expected_mangled}__{expected_hash}",);
        assert_eq!(
            m.module_name(),
            expected_name,
            "module_name mismatch for {path}",
        );
    }
}

// ── Scenario: Mixed separators produce stable names ──────────────

#[given("paths with forward slashes and backslashes")]
fn given_paths_with_forward_slashes_and_backslashes() {}

#[then("the mangled stems are identical but module names differ")]
fn then_mangled_stems_identical_but_module_names_differ() {
    let m_fwd = mangle_module_path("theorems/windows/style.theorem");
    let m_back = mangle_module_path("theorems\\windows\\style.theorem");

    assert_eq!(
        m_fwd.mangled_stem(),
        m_back.mangled_stem(),
        "mangled stems should match across separator styles",
    );
    assert_ne!(
        m_fwd.module_name(),
        m_back.module_name(),
        "module names must differ due to different hash inputs",
    );
    // Verify hashes are independently correct.
    assert_eq!(m_fwd.hash(), hash12("theorems/windows/style.theorem"),);
    assert_eq!(m_back.hash(), hash12("theorems\\windows\\style.theorem"),);
}

// ── Scenario: Punctuation-heavy paths disambiguated by hash ──────

#[given("paths that differ only in punctuation")]
fn given_paths_that_differ_only_in_punctuation() {}

#[then("their module names are distinct because hash suffixes differ")]
fn then_module_names_distinct_because_hash_suffixes_differ() {
    let m_hyphen = mangle_module_path("theorems/my-file.theorem");
    let m_under = mangle_module_path("theorems/my_file.theorem");

    // Both sanitize to the same mangled stem.
    assert_eq!(
        m_hyphen.mangled_stem(),
        m_under.mangled_stem(),
        "mangled stems should be identical",
    );
    // But module names are distinct.
    assert_ne!(
        m_hyphen.module_name(),
        m_under.module_name(),
        "module names must differ due to different hash inputs",
    );
}

// ── Scenario wiring ──────────────────────────────────────────────

#[scenario(
    path = "tests/features/module_naming.feature",
    name = "Simple paths produce deterministic module names"
)]
fn simple_paths_produce_deterministic_module_names() {}

#[scenario(
    path = "tests/features/module_naming.feature",
    name = "Mixed separators produce stable human-recognizable names"
)]
fn mixed_separators_produce_stable_human_recognizable_names() {}

#[scenario(
    path = "tests/features/module_naming.feature",
    name = "Punctuation-heavy paths are disambiguated by hash"
)]
fn punctuation_heavy_paths_are_disambiguated_by_hash() {}
