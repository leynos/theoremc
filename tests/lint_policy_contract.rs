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
//! Helpers return `Result` and carry no assertions, so a test body is the only
//! place that can panic. That keeps `clippy::panic_in_result_fn` satisfied
//! without an expectation, and lets each test name the fact it checks.

use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};
use toml::{Table, Value};

// The expected baseline lives apart from the assertions so both files stay
// inside the 400-line limit.
#[path = "lint_policy_contract_support/baseline.rs"]
mod baseline;

use baseline::{
    BASELINE_CHECK_CFG, BASELINE_CLIPPY_DENIALS, BASELINE_DISALLOWED_METHODS,
    BASELINE_RUST_DENIALS, BASELINE_RUSTDOC_DENIALS, WORKSPACE_PACKAGES,
};

/// The root manifest, parsed by every table assertion.
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// The Clippy configuration, parsed for its disallowed methods.
const CLIPPY_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml"));

/// The ADR that records the baseline and its retrieval.
const BASELINE_ADR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/adrs/adr-0014-netsuke-lint-baseline-alignment.md"
));

/// Parses the root manifest and returns its `[workspace.lints]` table.
fn lint_tables() -> Result<Table, String> {
    let manifest: Table = toml::from_str(MANIFEST)
        .map_err(|error| format!("root Cargo.toml is not valid TOML: {error}"))?;
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"))
        .and_then(Value::as_table)
        .cloned()
        .ok_or_else(|| "root manifest has no [workspace.lints] table".to_owned())
}

/// Returns one `[workspace.lints.*]` table by its group name.
fn lint_group<'a>(lints: &'a Table, group: &str) -> Result<&'a Table, String> {
    lints
        .get(group)
        .and_then(Value::as_table)
        .ok_or_else(|| format!("[workspace.lints.{group}] is missing"))
}

/// Returns a lint's declared level.
///
/// Cargo accepts a bare level string or a table with a `level` key, and both
/// mean the same thing, so a policy that spells a rule the long way must not
/// read as missing.
fn lint_level<'a>(group: &'a Table, name: &str) -> Result<&'a str, String> {
    let entry = group
        .get(name)
        .ok_or_else(|| format!("lint `{name}` is not declared"))?;
    entry
        .as_str()
        .or_else(|| entry.get("level").and_then(Value::as_str))
        .ok_or_else(|| format!("lint `{name}` must declare a level"))
}

/// Returns every lint in `group` that is missing or weaker than required.
///
/// `deny` and `forbid` both satisfy the baseline, because the policy is a
/// minimum: promoting a lint to `forbid` raises it, and promoting it to `allow`
/// or `warn` lowers it. Reporting the whole group at once means a failure names
/// every offending rule rather than only the first, which is what a reviewer
/// needs after a bulk edit.
fn rules_not_denied(group: &Table, names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| {
            let level = lint_level(group, name).unwrap_or("<missing>");
            (!matches!(level, "deny" | "forbid")).then(|| format!("{name} = {level}"))
        })
        .collect()
}

/// Parses the Clippy configuration.
fn clippy_config() -> Result<Table, String> {
    toml::from_str(CLIPPY_CONFIG).map_err(|error| format!("clippy.toml is not valid TOML: {error}"))
}

/// Returns each disallowed method's path, failing when one carries no reason.
fn disallowed_method_paths(config: &Table) -> Result<Vec<String>, String> {
    let methods = config
        .get("disallowed-methods")
        .and_then(Value::as_array)
        .ok_or_else(|| "clippy.toml must declare disallowed-methods".to_owned())?;
    methods
        .iter()
        .map(|method| {
            let entry = method
                .as_table()
                .ok_or_else(|| "every disallowed method must be a table".to_owned())?;
            let path = entry
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "every disallowed method must name a path".to_owned())?;
            let reason = entry
                .get("reason")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("disallowed method `{path}` must carry a reason"))?;
            if reason.trim().is_empty() {
                return Err(format!("disallowed method `{path}` has an empty reason"));
            }
            Ok(path.to_owned())
        })
        .collect()
}

/// Returns the name and manifest path of every workspace package.
fn workspace_manifests() -> Result<Vec<(String, String)>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| format!("running cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    metadata_packages(&output.stdout)
}

/// Extracts each package's name and manifest path from `cargo metadata` JSON.
fn metadata_packages(json: &[u8]) -> Result<Vec<(String, String)>, String> {
    let metadata: serde_json::Value = serde_json::from_slice(json)
        .map_err(|error| format!("cargo metadata emitted invalid JSON: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata listed no packages".to_owned())?;
    packages
        .iter()
        .map(|package| {
            let name = package
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "a package entry carries no name".to_owned())?;
            let manifest_path = package
                .get("manifest_path")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("package `{name}` carries no manifest path"))?;
            Ok((name.to_owned(), manifest_path.to_owned()))
        })
        .collect()
}

/// Reports whether a package manifest opts into the workspace lints.
fn inherits_workspace_lints(manifest_path: &str) -> Result<bool, String> {
    let text = read_manifest(manifest_path)?;
    let manifest: Table = toml::from_str(&text)
        .map_err(|error| format!("{manifest_path} is not valid TOML: {error}"))?;
    Ok(manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

/// Reads a package manifest through a capability-scoped directory handle.
fn read_manifest(manifest_path: &str) -> Result<String, String> {
    let path = Utf8Path::new(manifest_path);
    let parent = path
        .parent()
        .ok_or_else(|| format!("{manifest_path} has no parent directory"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("{manifest_path} has no file name"))?;
    let directory = Utf8Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|error| format!("opening {parent}: {error}"))?;
    directory
        .read_to_string(file_name)
        .map_err(|error| format!("reading {manifest_path}: {error}"))
}

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

    let offending = rules_not_denied(&table, &names);

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
fn clippy_table_denies_the_netsuke_baseline() {
    let lints = lint_tables().expect("the root manifest should declare workspace lints");
    let clippy = lint_group(&lints, "clippy").expect("the Clippy lint table should exist");
    let downgraded = rules_not_denied(clippy, &BASELINE_CLIPPY_DENIALS);

    assert!(
        downgraded.is_empty(),
        "every baseline Clippy rule must be denied; these are not: {downgraded:?}"
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
    let downgraded = rules_not_denied(rust, &BASELINE_RUST_DENIALS);

    assert!(
        downgraded.is_empty(),
        "every baseline Rust rule must be denied; these are not: {downgraded:?}"
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
    let downgraded = rules_not_denied(rustdoc, &BASELINE_RUSTDOC_DENIALS);

    assert!(
        downgraded.is_empty(),
        "every baseline rustdoc rule must be denied; these are not: {downgraded:?}"
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
        .filter(|(_, manifest_path)| !inherits_workspace_lints(manifest_path).unwrap_or(false))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        not_inheriting.is_empty(),
        "these packages do not opt into the workspace lints: {not_inheriting:?}"
    );
}
