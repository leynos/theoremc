//! Semantic contract for the workspace lint policy.
//!
//! Issue #98 makes the `leynos/netsuke` lint tables the minimum `TheoremC` must
//! enforce. The behavioural test proves a copied policy is live; this test
//! proves the manifest still *declares* that minimum. It parses the manifest
//! and `clippy.toml` rather than scanning text, because a text search cannot
//! tell a denied lint from one named in a comment.
//!
//! Deleting or downgrading a baseline rule fails here even when no violation
//! exists yet, which is the window a behavioural test alone cannot close.
//!
//! The expected baseline and the manifest readers live in support modules,
//! apart from the assertions, so every file stays inside the 400-line limit and
//! a reviewer can compare the expected baseline against `leynos/netsuke`
//! without reading test logic. Those helpers return `Result` and carry no
//! assertions, so a test body is the only place that can panic; that keeps
//! `clippy::panic_in_result_fn` satisfied without an expectation.

use camino::Utf8Path;
use toml::{Table, Value};

#[path = "lint_policy_contract_support/baseline.rs"]
mod baseline;

#[path = "lint_policy_contract_support/policy.rs"]
mod policy;

use baseline::{
    BASELINE_CHECK_CFG, BASELINE_CLIPPY_DENIALS, BASELINE_DISALLOWED_METHODS,
    BASELINE_RUST_DENIALS, BASELINE_RUSTDOC_DENIALS, WORKSPACE_PACKAGES,
};
use policy::{
    LintNames, clippy_config, disallowed_method_paths, inherits_workspace_lints, lint_group,
    lint_level, lint_tables, rules_not_denied, rules_outranked_by_a_group, workspace_manifests,
};

/// The Clippy configuration text, scanned for recorded divergences.
const CLIPPY_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml"));

/// The ADR that records the baseline and its retrieval.
const BASELINE_ADR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/adrs/adr-0014-netsuke-lint-baseline-alignment.md"
));

#[test]
fn the_minimum_predicate_accepts_stricter_and_rejects_weaker_levels() {
    // The baseline is a floor. A promotion to `forbid` must satisfy it, while a
    // downgrade to `warn` or `allow`, or a deleted entry, must not, or the
    // contract would block a policy increase or hide a decrease.
    let mut table = Table::new();
    for (name, level) in [
        ("denied", "deny"),
        ("forbidden", "forbid"),
        ("warned", "warn"),
        ("allowed", "allow"),
    ] {
        table.insert(name.to_owned(), Value::String(level.to_owned()));
    }
    // Cargo's table spelling must read the same as the bare string, or a rule
    // rewritten the long way would look deleted.
    for (name, level) in [("table_denied", "deny"), ("table_forbidden", "forbid")] {
        let mut entry = Table::new();
        entry.insert("level".to_owned(), Value::String(level.to_owned()));
        entry.insert("priority".to_owned(), Value::Integer(0));
        table.insert(name.to_owned(), Value::Table(entry));
    }
    let names = [
        "denied",
        "forbidden",
        "table_denied",
        "table_forbidden",
        "warned",
        "allowed",
        "absent",
    ];

    let offending = rules_not_denied(&table, LintNames::new(&names));

    assert_eq!(
        offending,
        vec![
            "warned = warn".to_owned(),
            "allowed = allow".to_owned(),
            "absent = <missing>".to_owned(),
        ],
        "only weaker or missing levels should be reported"
    );
}

#[test]
fn the_precedence_check_rejects_a_denial_below_a_weaker_group() {
    // A group at priority -1 outranks an individual entry at a lower
    // priority, whatever that entry's level says, so `deny` at -2 is not a
    // denial in effect. The same entry at 0 wins, which is Cargo's default.
    let mut table = Table::new();
    let mut pedantic = Table::new();
    pedantic.insert("level".to_owned(), Value::String("warn".to_owned()));
    pedantic.insert("priority".to_owned(), Value::Integer(-1));
    table.insert("pedantic".to_owned(), Value::Table(pedantic));
    for (name, priority) in [("outranked", -2), ("default_priority", 0)] {
        let mut entry = Table::new();
        entry.insert("level".to_owned(), Value::String("deny".to_owned()));
        entry.insert("priority".to_owned(), Value::Integer(priority));
        table.insert(name.to_owned(), Value::Table(entry));
    }
    // A bare string entry has no priority key, so it is Cargo's default 0 and
    // outranks the -1 group.
    table.insert("bare".to_owned(), Value::String("deny".to_owned()));

    let outranked = rules_outranked_by_a_group(
        &table,
        LintNames::new(&["outranked", "default_priority", "bare", "absent"]),
    );

    assert_eq!(
        outranked,
        vec!["outranked = priority -2".to_owned()],
        "only the denial below the group's priority should be reported"
    );
}

#[test]
fn clippy_table_denies_the_netsuke_baseline() {
    let lints = lint_tables().expect("the root manifest should declare workspace lints");
    let clippy = lint_group(&lints, "clippy").expect("the Clippy lint table should exist");
    let downgraded = rules_not_denied(clippy, LintNames::new(&BASELINE_CLIPPY_DENIALS));

    assert!(
        downgraded.is_empty(),
        "every baseline Clippy rule must be denied; these are not: {downgraded:?}"
    );
    let outranked = rules_outranked_by_a_group(clippy, LintNames::new(&BASELINE_CLIPPY_DENIALS));
    assert!(
        outranked.is_empty(),
        "these denials are outranked by a weaker group: {outranked:?}"
    );

    let pedantic = clippy
        .get("pedantic")
        .and_then(Value::as_table)
        .expect("clippy::pedantic should be a table with a level and priority");
    assert_eq!(
        pedantic.get("level").and_then(Value::as_str),
        Some("warn"),
        "clippy::pedantic should stay at warn"
    );
    assert_eq!(
        pedantic.get("priority").and_then(Value::as_integer),
        Some(-1),
        "clippy::pedantic should keep priority -1 so explicit denials win"
    );
}

#[test]
fn rust_table_denies_the_baseline_and_forbids_unsafe() {
    let lints = lint_tables().expect("the root manifest should declare workspace lints");
    let rust = lint_group(&lints, "rust").expect("the Rust lint table should exist");
    let downgraded = rules_not_denied(rust, LintNames::new(&BASELINE_RUST_DENIALS));

    assert!(
        downgraded.is_empty(),
        "every baseline Rust rule must be denied; these are not: {downgraded:?}"
    );
    let outranked = rules_outranked_by_a_group(rust, LintNames::new(&BASELINE_RUST_DENIALS));
    assert!(
        outranked.is_empty(),
        "these denials are outranked by a weaker group: {outranked:?}"
    );
    assert_eq!(
        lint_level(rust, "unsafe_code").ok(),
        Some("forbid"),
        "unsafe_code should be forbidden so no expectation can re-open it"
    );
}

#[test]
fn unexpected_cfgs_declares_the_baseline_check_cfg_values() {
    let lints = lint_tables().expect("the root manifest should declare workspace lints");
    let rust = lint_group(&lints, "rust").expect("the Rust lint table should exist");
    let unexpected = rust
        .get("unexpected_cfgs")
        .and_then(Value::as_table)
        .expect("unexpected_cfgs should be a table with a level and check-cfg");

    assert_eq!(
        unexpected.get("level").and_then(Value::as_str),
        Some("warn"),
        "unexpected_cfgs should stay at warn; the gates supply -D warnings"
    );
    let declared: Vec<&str> = unexpected
        .get("check-cfg")
        .and_then(Value::as_array)
        .expect("unexpected_cfgs should declare a check-cfg array")
        .iter()
        .map(|value| value.as_str().expect("every check-cfg entry is a string"))
        .collect();
    assert_eq!(
        declared, BASELINE_CHECK_CFG,
        "check-cfg should declare exactly the baseline cfg values"
    );
}

#[test]
fn rustdoc_table_denies_the_complete_baseline() {
    let lints = lint_tables().expect("the root manifest should declare workspace lints");
    let rustdoc = lint_group(&lints, "rustdoc").expect("the rustdoc lint table should exist");
    let downgraded = rules_not_denied(rustdoc, LintNames::new(&BASELINE_RUSTDOC_DENIALS));

    assert!(
        downgraded.is_empty(),
        "every baseline rustdoc rule must be denied; these are not: {downgraded:?}"
    );
    let outranked = rules_outranked_by_a_group(rustdoc, LintNames::new(&BASELINE_RUSTDOC_DENIALS));
    assert!(
        outranked.is_empty(),
        "these denials are outranked by a weaker group: {outranked:?}"
    );
}

#[test]
fn the_adr_records_the_baseline_commit() {
    assert!(
        BASELINE_ADR.contains(baseline::BASELINE_COMMIT),
        "the baseline ADR should name the Netsuke commit this contract expects"
    );
}

#[test]
fn clippy_config_disallows_every_environment_method_with_a_reason() {
    let config = clippy_config().expect("clippy.toml should parse as TOML");
    let declared =
        disallowed_method_paths(&config).expect("every disallowed method should carry a reason");

    // Membership, not equality: the baseline is a floor, so an additional
    // restriction is an improvement rather than a contract break.
    let absent: Vec<&&str> = BASELINE_DISALLOWED_METHODS
        .iter()
        .filter(|method| !declared.iter().any(|entry| entry == *method))
        .collect();
    assert!(
        absent.is_empty(),
        "clippy.toml must disallow every baseline environment method; absent: {absent:?}"
    );
}

#[test]
fn clippy_config_records_its_intentional_divergences() {
    // Comments cannot be asserted semantically, so the contract pins the two
    // entries TheoremC deliberately omits. Without them, a reader would have
    // to guess whether the omission is policy or an oversight.
    assert!(
        CLIPPY_CONFIG.contains("shell_quote::QuoteRefExt::quoted"),
        "clippy.toml should record why the shell_quote method is omitted"
    );
    assert!(
        CLIPPY_CONFIG.contains("std::io::IsTerminal::is_terminal"),
        "clippy.toml should record why terminal detection is omitted"
    );
}

#[test]
fn every_workspace_package_inherits_the_workspace_lints() {
    let manifests = workspace_manifests().expect("cargo metadata should list the workspace");
    let discovered: Vec<&str> = manifests.iter().map(|(name, _)| name.as_str()).collect();
    for expected in WORKSPACE_PACKAGES {
        assert!(
            discovered.contains(&expected),
            "cargo metadata did not report workspace package `{expected}`"
        );
    }

    let not_inheriting: Vec<&str> = manifests
        .iter()
        .filter(|(_, manifest_path)| {
            !inherits_workspace_lints(Utf8Path::new(manifest_path)).unwrap_or(false)
        })
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        not_inheriting.is_empty(),
        "these packages do not opt into the workspace lints: {not_inheriting:?}"
    );
}
