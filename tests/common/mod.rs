//! Shared test helpers and golden constants for integration tests.
//!
//! Each integration test crate compiles this module independently, so
//! not every item is used in every crate.
#![allow(dead_code, reason = "each integration test crate uses only a subset")]

use cap_std::{ambient_authority, fs_utf8::Dir};

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn load_fixture(name: &str) -> std::io::Result<String> {
    Dir::open_ambient_dir("tests/fixtures", ambient_authority())?.read_to_string(name)
}

// ── Golden constants for action-name mangling ────────────────────
//
// Each tuple is `(canonical_name, expected_slug, expected_hash12)`.
// Shared by unit tests in `src/mangle_tests.rs` and BDD tests in
// `tests/action_mangle_bdd.rs`.

/// Representative golden triples for action-name mangling.
pub const ACTION_GOLDEN_TRIPLES: &[(&str, &str, &str)] = &[
    ("account.deposit", "account__deposit", "05158894bfb4"),
    ("hnsw.attach_node", "hnsw__attach_unode", "8d74e77b55f2"),
    (
        "hnsw.graph.with_capacity",
        "hnsw__graph__with_ucapacity",
        "9eafdf8834ec",
    ),
];

// ── Golden constants for per-file module naming ──────────────────
//
// Each tuple is `(path, expected_stem, expected_mangled_stem,
// expected_hash12)`. The module name is derived as
// `__theoremc__file__{mangled_stem}__{hash12}`.

/// Representative golden tuples for per-file module naming.
pub const MODULE_GOLDEN_TUPLES: &[(&str, &str, &str, &str)] = &[
    (
        "theorems/bidirectional.theorem",
        "theorems/bidirectional",
        "theorems_bidirectional",
        "1fc14bdf614f",
    ),
    (
        "theorems/nested/deep/path.theorem",
        "theorems/nested/deep/path",
        "theorems_nested_deep_path",
        "5cb0a56a3468",
    ),
    (
        "no_extension",
        "no_extension",
        "no_extension",
        "afb36ed5206f",
    ),
];
