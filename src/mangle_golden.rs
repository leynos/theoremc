//! Golden test constants for action and module name mangling.
//!
//! Single source of truth for golden hash values used by both unit
//! tests (`src/mangle_tests.rs`) and BDD integration tests
//! (`tests/action_mangle_bdd.rs`, `tests/module_naming_bdd.rs`, and
//! `tests/harness_naming_bdd.rs`).

/// `(canonical_name, expected_slug, expected_hash12)`.
pub const ACTION_GOLDEN_TRIPLES: &[(&str, &str, &str)] = &[
    ("account.deposit", "account__deposit", "05158894bfb4"),
    ("hnsw.attach_node", "hnsw__attach_unode", "8d74e77b55f2"),
    (
        "hnsw.graph.with_capacity",
        "hnsw__graph__with_ucapacity",
        "9eafdf8834ec",
    ),
    ("_a._b", "_ua___ub", "0a39aa24f512"),
    ("ns._", "ns___u", "ef4f43e71ce0"),
    ("x.y", "x__y", "f12518d733b0"),
];

/// `(path, expected_stem, expected_mangled_stem, expected_hash12)`.
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

/// `(path, theorem_id, expected_slug)`.
pub const HARNESS_GOLDEN_TUPLES: &[(&str, &str, &str)] = &[
    (
        "theorems/bidirectional.theorem",
        "BidirectionalLinksCommitPath3Nodes",
        "bidirectional_links_commit_path_3_nodes",
    ),
    ("theorems/hnsw.theorem", "HNSWInvariant", "hnsw_invariant"),
    ("theorems/http.theorem", "HTTP2StreamID", "http_2_stream_id"),
    ("theorems/smoke.theorem", "hnsw_smoke", "hnsw_smoke"),
];
