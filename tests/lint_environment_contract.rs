//! Contract tests for the workspace environment-boundary policy.
//!
//! Issue #101 adopts `leynos/netsuke`'s environment-injection rule: production
//! code reads the environment through an injected reader, and tests use a stub
//! environment rather than mutating process state. The rule has two halves, and
//! both are needed for it to bite. `clippy.toml` names the six process-
//! environment functions Clippy must refuse, with the advice a contributor
//! sees when they trip one. The workspace lint baseline must actually deny the
//! lint, or the configuration is inert.
//!
//! These tests read both files as text, so neither half can be removed without
//! a failure. `tests/lint_workspace_execution.rs` proves Clippy acts on the
//! configuration rather than merely holding it.

const CLIPPY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml"));
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// Every process-environment function the policy forbids, with its reason.
///
/// The list is the ticket's six entries verbatim. Keeping the path and reason
/// together means a test cannot check one against another entry's reason.
const FORBIDDEN: [(&str, &str); 6] = [
    ("std::env::var", "inject an environment reader"),
    ("std::env::var_os", "inject an environment reader"),
    ("std::env::vars", "inject an environment reader"),
    ("std::env::vars_os", "inject an environment reader"),
    ("std::env::set_var", "use a stub environment in tests"),
    ("std::env::remove_var", "use a stub environment in tests"),
];

/// One `disallowed-methods` entry, as the configuration spells it.
struct DisallowedMethod<'a> {
    /// The function path the entry names.
    path: &'a str,
    /// The advice shown in the diagnostic.
    reason: &'a str,
}

/// The `disallowed-methods` entries, parsed from `clippy.toml`.
fn disallowed_methods(configuration: &str) -> Vec<DisallowedMethod<'_>> {
    configuration
        .lines()
        .filter(|line| line.trim_start().starts_with("{ path ="))
        .filter_map(|line| {
            let path = quoted_value(line, "path =")?;
            let reason = quoted_value(line, "reason =")?;
            Some(DisallowedMethod { path, reason })
        })
        .collect()
}

/// Returns the value of `key "..."` within one line.
fn quoted_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let after_key = line.split_once(key)?.1.trim_start();
    let unquoted = after_key.strip_prefix('"')?;
    unquoted.split_once('"').map(|(value, _)| value)
}

/// The paragraphs of a manifest section, keyed by table header.
///
/// A manifest is not TOML-parsed here: the question is whether a specific key
/// appears under a specific table, and reading the raw text keeps the test
/// independent of formatting and comment changes elsewhere.
fn table<'a>(manifest: &'a str, header: &str) -> Vec<&'a str> {
    manifest
        .lines()
        .skip_while(|line| line.trim() != header)
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['))
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .collect()
}

/// Reports whether `key` is set to `value` in `lines`.
fn declares(lines: &[&str], key: &str, value: &str) -> bool {
    lines.iter().any(|line| {
        let Some((name, rest)) = line.split_once('=') else {
            return false;
        };
        name.trim() == key && rest.trim().trim_matches('"') == value
    })
}

/// Returns the value declared for `key` in `lines`, if any.
fn declared_value<'a>(lines: &'a [&'a str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let (name, rest) = line.split_once('=')?;
        (name.trim() == key).then_some(rest.trim().trim_matches('"'))
    })
}

/// The six Netsuke entries are present with the advice a contributor sees.
#[test]
fn clippy_forbids_every_process_environment_function() {
    let entries = disallowed_methods(CLIPPY);
    for (path, reason) in FORBIDDEN {
        let Some(entry) = entries.iter().find(|candidate| candidate.path == path) else {
            panic!("clippy.toml must disallow `{path}`; the entry is missing");
        };
        assert_eq!(
            entry.reason, reason,
            "the `{path}` entry must tell the contributor what to do instead; found `{}`",
            entry.reason
        );
    }
}

/// The configuration is not inert: the workspace denies the lint.
#[test]
fn the_workspace_denies_disallowed_methods() {
    let clippy = table(MANIFEST, "[workspace.lints.clippy]");
    assert_eq!(
        declared_value(&clippy, "disallowed_methods"),
        Some("deny"),
        "the workspace baseline must deny `clippy::disallowed_methods`, or `clippy.toml` \
         has no effect; found {:?}",
        declared_value(&clippy, "disallowed_methods")
    );
}

/// The policy's premise holds: `unsafe_code` is forbidden workspace-wide.
///
/// The `set_var` and `remove_var` half of the policy relies on no helper
/// needing `unsafe` environment mutation, and a crate-wide exception would
/// silently reopen it.
#[test]
fn the_workspace_forbids_unsafe_code() {
    let rust = table(MANIFEST, "[workspace.lints.rust]");
    assert!(
        declares(&rust, "unsafe_code", "forbid"),
        "the workspace baseline must forbid `unsafe_code`, so the environment policy cannot be \
         reopened with an unsafe mutation helper"
    );
}

/// No crate opts out of the workspace baseline.
///
/// A member without `[lints] workspace = true` would keep compiling, but the
/// disallowed methods would stop being denied there, so the policy would have
/// a hole shaped like that crate.
#[test]
fn every_member_inherits_the_workspace_lints() {
    let members: [(&str, &str); 5] = [
        (
            "crates/test-helpers/Cargo.toml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/test-helpers/Cargo.toml"
            )),
        ),
        (
            "crates/theoremc-build-support/Cargo.toml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/theoremc-build-support/Cargo.toml"
            )),
        ),
        (
            "crates/cargo-theorem/Cargo.toml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/cargo-theorem/Cargo.toml"
            )),
        ),
        (
            "crates/theoremc-core/Cargo.toml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/theoremc-core/Cargo.toml"
            )),
        ),
        (
            "crates/theoremc-macros/Cargo.toml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/crates/theoremc-macros/Cargo.toml"
            )),
        ),
    ];
    for (path, manifest) in members {
        assert!(
            manifest.contains("[lints]\nworkspace = true"),
            "{path} must inherit the workspace lints, or it escapes the policy"
        );
    }
}
