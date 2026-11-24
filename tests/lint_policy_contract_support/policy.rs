//! Manifest and configuration readers for `lint_policy_contract`.
//!
//! The parsing lives apart from the assertions so each file stays inside the
//! 400-line limit. These helpers return `Result` and never assert: a test body
//! is the only place that may panic, which keeps `clippy::panic_in_result_fn`
//! satisfied without an expectation.

use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};
use toml::{Table, Value};

/// The root manifest, parsed by every table assertion.
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// The Clippy configuration, parsed for its disallowed methods.
const CLIPPY_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clippy.toml"));

/// Parses the root manifest and returns its `[workspace.lints]` table.
pub fn lint_tables() -> Result<Table, String> {
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
pub fn lint_group<'a>(lints: &'a Table, group: &str) -> Result<&'a Table, String> {
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
pub fn lint_level<'a>(group: &'a Table, name: &str) -> Result<&'a str, String> {
    let entry = group
        .get(name)
        .ok_or_else(|| format!("lint `{name}` is not declared"))?;
    entry
        .as_str()
        .or_else(|| entry.get("level").and_then(Value::as_str))
        .ok_or_else(|| format!("lint `{name}` must declare a level"))
}

/// The rule names one group assertion checks.
///
/// A newtype keeps the predicates' signatures from being dominated by string
/// parameters, which the code-health rules measure across the module.
pub struct LintNames<'a>(&'a [&'a str]);

impl<'a> LintNames<'a> {
    /// Wraps a slice of rule names.
    pub const fn new(names: &'a [&'a str]) -> Self {
        Self(names)
    }

    /// Iterates over the names.
    pub fn iter(self) -> std::slice::Iter<'a, &'a str> {
        self.0.iter()
    }
}

/// Returns every lint in `group` that is missing or weaker than required.
///
/// `deny` and `forbid` both satisfy the baseline, because the policy is a
/// minimum: promoting a lint to `forbid` raises it, and promoting it to `allow`
/// or `warn` lowers it. Reporting the whole group at once means a failure names
/// every offending rule rather than only the first, which is what a reviewer
/// needs after a bulk edit.
pub fn rules_not_denied(group: &Table, names: LintNames<'_>) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| {
            let level = lint_level(group, name).unwrap_or("<missing>");
            (!matches!(level, "deny" | "forbid")).then(|| format!("{name} = {level}"))
        })
        .collect()
}

/// Returns every denied lint whose priority a weaker group would override.
///
/// Cargo's table spelling carries a `priority`; the highest wins. A group such
/// as `pedantic` at priority -1 therefore overrides an individual denial left
/// at Cargo's default of 0 only when that denial declares a lower priority.
/// A denial that survives on paper but loses to a `warn` group is not a
/// denial, so the contract has to check the precedence, not just the level.
pub fn rules_outranked_by_a_group(group: &Table, names: LintNames<'_>) -> Vec<String> {
    let group_priority = highest_weakening_group_priority(group);
    names
        .iter()
        .filter_map(|name| {
            let entry = group.get(*name).and_then(Value::as_table)?;
            if !matches!(
                entry.get("level").and_then(Value::as_str),
                Some("deny" | "forbid")
            ) {
                return None;
            }
            let priority = entry.get("priority").and_then(Value::as_integer)?;
            (priority < group_priority).then(|| format!("{name} = priority {priority}"))
        })
        .collect()
}

/// Returns the priority of the weakest non-denying group entry.
///
/// This is the highest priority among entries that do not deny, because any
/// individual denial below it loses. When no such entry exists, the minimum
/// possible priority is returned, so nothing is reported.
pub fn highest_weakening_group_priority(group: &Table) -> i64 {
    group
        .values()
        .filter_map(Value::as_table)
        .filter(|entry| {
            matches!(
                entry.get("level").and_then(Value::as_str),
                Some("warn" | "allow")
            )
        })
        .filter_map(|entry| entry.get("priority").and_then(Value::as_integer))
        .max()
        .unwrap_or(i64::MIN)
}

/// Parses the Clippy configuration.
pub fn clippy_config() -> Result<Table, String> {
    toml::from_str(CLIPPY_CONFIG).map_err(|error| format!("clippy.toml is not valid TOML: {error}"))
}

/// Returns each disallowed method's path, failing when one carries no reason.
pub fn disallowed_method_paths(config: &Table) -> Result<Vec<String>, String> {
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
pub fn workspace_manifests() -> Result<Vec<(String, String)>, String> {
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
pub fn metadata_packages(json: &[u8]) -> Result<Vec<(String, String)>, String> {
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
pub fn inherits_workspace_lints(manifest_path: &Utf8Path) -> Result<bool, String> {
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
pub fn read_manifest(path: &Utf8Path) -> Result<String, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{path} has no parent directory"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("{path} has no file name"))?;
    let directory = Utf8Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|error| format!("opening {parent}: {error}"))?;
    directory
        .read_to_string(file_name)
        .map_err(|error| format!("reading {path}: {error}"))
}
