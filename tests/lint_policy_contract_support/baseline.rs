//! Baseline lint policy data for `lint_policy_contract`.
//!
//! The rule lists live apart from the assertions so each file stays inside the
//! 400-line limit and so a reviewer can compare the expected baseline against
//! `leynos/netsuke` without reading test logic.

/// The `leynos/netsuke` commit the baseline was read from.
pub const BASELINE_COMMIT: &str = "871b609d4228ac3be714ad7ead1ddb0fb4ac17f6";

/// Every Clippy rule the Netsuke baseline denies.
pub const BASELINE_CLIPPY_DENIALS: [&str; 45] = [
    // 1. hygiene
    "allow_attributes",
    "allow_attributes_without_reason",
    "blanket_clippy_restriction_lints",
    "disallowed_methods",
    "needless_pass_by_value",
    "implicit_hasher",
    "missing_assert_message",
    // 2. debugging leftovers
    "dbg_macro",
    "print_stdout",
    "print_stderr",
    // 3. panic-prone operations
    "unwrap_used",
    "expect_used",
    "indexing_slicing",
    "string_slice",
    "integer_division",
    "integer_division_remainder_used",
    "panic_in_result_fn",
    "unreachable",
    // 4. portability
    "host_endian_bytes",
    "little_endian_bytes",
    "big_endian_bytes",
    // 5. nursery idiom polish
    "let_underscore_must_use",
    "or_fun_call",
    "option_if_let_else",
    "self_named_module_files",
    "shadow_reuse",
    "shadow_same",
    "shadow_unrelated",
    "str_to_string",
    "string_lit_as_bytes",
    "try_err",
    "unneeded_field_pattern",
    "use_self",
    // 6. numerical foot-guns
    "float_arithmetic",
    "cast_possible_truncation",
    "cast_possible_wrap",
    "cast_precision_loss",
    "lossy_float_literal",
    // 7. API ergonomics
    "missing_const_for_fn",
    "must_use_candidate",
    "unused_async",
    "missing_docs_in_private_items",
    // 8. error handling
    "missing_panics_doc",
    "error_impl_error",
    "result_large_err",
];

/// Every Rust rule the Netsuke baseline denies.
pub const BASELINE_RUST_DENIALS: [&str; 3] =
    ["unknown_lints", "renamed_and_removed_lints", "missing_docs"];

/// Every rustdoc rule the Netsuke baseline denies.
pub const BASELINE_RUSTDOC_DENIALS: [&str; 7] = [
    "missing_crate_level_docs",
    "broken_intra_doc_links",
    "private_intra_doc_links",
    "bare_urls",
    "invalid_html_tags",
    "invalid_codeblock_attributes",
    "unescaped_backticks",
];

/// The `check-cfg` values the manifest must declare, in order.
pub const BASELINE_CHECK_CFG: [&str; 2] = ["cfg(kani)", "cfg(dylint_lib, values(any()))"];

/// Every environment method the Clippy configuration disallows.
pub const BASELINE_DISALLOWED_METHODS: [&str; 7] = [
    "std::env::var",
    "std::env::var_os",
    "std::env::vars",
    "std::env::vars_os",
    "std::env::set_var",
    "std::env::remove_var",
    "std::env::set_current_dir",
];

/// Every workspace package the contract expects `cargo metadata` to report.
pub const WORKSPACE_PACKAGES: [&str; 6] = [
    "theoremc",
    "cargo-theorem",
    "test-helpers",
    "theoremc-build-support",
    "theoremc-core",
    "theoremc-macros",
];
