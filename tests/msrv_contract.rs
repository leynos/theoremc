//! Contract tests for the minimum supported Rust version (MSRV) gate.
//!
//! ADR-010 declares Rust 1.89 for every workspace package and widens the gate
//! to every target. These tests read the wiring as text: the exact `make msrv`
//! recipe, the declared floor reported by `cargo metadata`, the MSRV-aware
//! resolver, the locked `cargo-platform` release, and the CI step that runs the
//! gate. The gate itself runs in CI; these tests prove the repository still
//! invokes it as the adopted baseline requires.

use std::ops::Range;
use std::process::Command;

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
const LOCK: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock"));

/// The floor every workspace package must declare.
const MSRV: &str = "1.89";

/// The recipe the `msrv` target must run, exactly as ADR-010 records it.
///
/// `RUSTFLAGS=""` displaces the development rustflags, which the stable Rust
/// 1.89 toolchain cannot accept. `--all-targets` brings the dev-only
/// dependency graph inside the contract, and `--locked` fails the gate if the
/// checked graph would need to change.
const PINNED_RECIPE: &str = concat!(
    "RUSTFLAGS=\"\" $(CARGO) +1.89 check --workspace ",
    "--all-targets --all-features --locked",
);

/// A Makefile target whose recipe a contract test pins.
#[derive(Clone, Copy)]
struct Target(&'static str);

/// A crate whose locked releases a contract test pins.
#[derive(Clone, Copy)]
struct Crate(&'static str);

/// One line of a YAML workflow, with its indentation measured once.
///
/// CI YAML in this repository is space-indented, so the leading-space count is
/// the line's nesting depth. Measuring it on construction keeps the block
/// helpers focused on structure rather than on string arithmetic.
#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a str,
    indent: usize,
}

impl<'a> Line<'a> {
    /// Measures one line's leading spaces.
    fn new(text: &'a str) -> Self {
        Self {
            text,
            indent: text.len() - text.trim_start_matches(' ').len(),
        }
    }

    /// Reports whether the line carries no content.
    fn is_blank(self) -> bool {
        self.text.trim().is_empty()
    }

    /// Reports whether the line opens a sequence item.
    fn is_sequence_item(self) -> bool {
        self.text.trim_start().starts_with("- ")
    }
}

/// A YAML mapping key that can silently excuse a failing step.
#[derive(Clone, Copy)]
enum BypassKey {
    If,
    ContinueOnError,
}

impl BypassKey {
    /// Returns the key as it is written in the workflow.
    const fn as_str(self) -> &'static str {
        match self {
            Self::If => "if",
            Self::ContinueOnError => "continue-on-error",
        }
    }
}

/// Returns the recipe lines of a Makefile target, without the target line.
///
/// The leading tab is stripped so a caller compares the command text alone.
/// An absent target yields an empty list, which fails the equality assertion
/// rather than panicking.
fn recipe_lines(makefile: &str, target: Target) -> Vec<&str> {
    let header = format!("{}:", target.0);
    let mut lines = makefile.lines();
    if !lines.any(|line| line.starts_with(&header)) {
        return Vec::new();
    }
    lines
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// Returns the name and declared `rust_version` of every workspace package.
///
/// `cargo metadata` resolves the member list from `CARGO_MANIFEST_DIR`, so the
/// check follows the real workspace rather than a hand-maintained copy. A
/// package that omits the field reports `<none>`, which fails the comparison
/// with the package named.
///
/// # Errors
///
/// Returns the reason when `cargo metadata` cannot run, fails, or prints no
/// package list, including its standard error in the message.
fn workspace_rust_versions() -> Result<Vec<(String, String)>, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|error| format!("running cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata emitted invalid JSON: {error}"))?;
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata listed no packages".to_owned())?;
    packages.iter().map(package_rust_version).collect()
}

/// Returns one package's name and declared `rust_version` from metadata.
///
/// # Errors
///
/// Returns the reason when the package entry has no name.
fn package_rust_version(package: &serde_json::Value) -> Result<(String, String), String> {
    let name = package
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "a package in cargo metadata has no name".to_owned())?
        .to_owned();
    let rust_version = package
        .get("rust_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<none>")
        .to_owned();
    Ok((name, rust_version))
}

/// Returns every version of the named crate in a `Cargo.lock` document.
///
/// A lockfile lists one `[[package]]` table per resolved package, and a crate
/// can appear more than once when two dependency lines need different major
/// versions. Each matching table contributes its `version` key, so the caller
/// can assert on the whole set rather than on whichever table comes first.
fn locked_versions(lock: &str, package: Crate) -> Vec<&str> {
    let name = format!("name = \"{}\"", package.0);
    lock.split("[[package]]")
        .skip(1)
        .filter(|table| table.lines().any(|line| line.trim() == name))
        .filter_map(|table| {
            table
                .lines()
                .find_map(|line| line.trim().strip_prefix("version = \""))
                .and_then(|rest| rest.split('"').next())
        })
        .collect()
}

/// Splits a workflow document into measured lines.
fn workflow_lines(ci: &str) -> Vec<Line<'_>> {
    ci.lines().map(Line::new).collect()
}

/// Reports whether a line declares a mapping key with the given name.
///
/// The key must sit at the start of a mapping entry, either directly or after
/// a sequence marker such as `- if: ...`. Requiring the colon keeps a value
/// that merely begins with the same word from matching.
fn declares_key(line: Line<'_>, key: BypassKey) -> bool {
    let mapping_entry = line.text.trim_start();
    let candidate = mapping_entry.strip_prefix("- ").unwrap_or(mapping_entry);
    candidate
        .strip_prefix(key.as_str())
        .is_some_and(|rest| rest.starts_with(':'))
}

/// Reports whether a line is a mapping key rather than a nested value.
///
/// Sequence items are excluded, so a step name does not masquerade as a key
/// when the job's own keys are located.
fn is_mapping_key(line: Line<'_>) -> bool {
    let trimmed = line.text.trim();
    !trimmed.starts_with('-') && trimmed.ends_with(':')
}

/// Returns the inclusive line range of the step that owns `run_line`.
///
/// The step starts at the nearest sequence item above the `run:` key and ends
/// before the next line indented no deeper than that item.
///
/// # Errors
///
/// Returns the reason when the run line is outside the file or belongs to no
/// sequence item.
fn step_block(lines: &[Line<'_>], run_line: usize) -> Result<Range<usize>, String> {
    let run_indent = lines
        .get(run_line)
        .map(|line| line.indent)
        .ok_or_else(|| format!("run line {run_line} is outside the CI file"))?;
    let start = (0..run_line)
        .rev()
        .find(|&index| {
            lines
                .get(index)
                .is_some_and(|line| line.indent < run_indent && line.is_sequence_item())
        })
        .ok_or_else(|| "the run key must belong to a steps sequence item".to_owned())?;
    let step_indent = lines.get(start).map_or(0, |line| line.indent);
    let end = (run_line + 1..lines.len())
        .find(|&index| {
            lines
                .get(index)
                .is_some_and(|line| !line.is_blank() && line.indent <= step_indent)
        })
        .unwrap_or(lines.len());
    Ok(start..end)
}

/// Returns the inclusive line range of the job that encloses `step_start`.
///
/// The job key is the shallowest mapping key above the step; the job ends
/// before the next line indented no deeper than that key.
///
/// # Errors
///
/// Returns the reason when the workflow declares no `jobs:` mapping or the
/// step sits outside a job.
fn job_block(lines: &[Line<'_>], step_start: usize) -> Result<Range<usize>, String> {
    let jobs_line = lines
        .iter()
        .position(|line| line.text.trim() == "jobs:")
        .ok_or_else(|| "CI must declare a jobs mapping".to_owned())?;
    let jobs_indent = lines.get(jobs_line).map_or(0, |line| line.indent);
    let candidates = || {
        (jobs_line + 1..step_start).filter(|&index| {
            lines
                .get(index)
                .is_some_and(|line| line.indent > jobs_indent && is_mapping_key(*line))
        })
    };
    let enclosing_indent = candidates()
        .filter_map(|index| lines.get(index).map(|line| line.indent))
        .min()
        .ok_or_else(|| "the step must sit inside a job".to_owned())?;
    let start = candidates()
        .rev()
        .find(|&index| {
            lines
                .get(index)
                .is_some_and(|line| line.indent == enclosing_indent)
        })
        .ok_or_else(|| "the step must sit inside a job".to_owned())?;
    let end = (step_start..lines.len())
        .find(|&index| {
            lines
                .get(index)
                .is_some_and(|line| !line.is_blank() && line.indent <= enclosing_indent)
        })
        .unwrap_or(lines.len());
    Ok(start..end)
}

/// Reports whether a block carries a key that can silently excuse failure.
///
/// Both `if:` and `continue-on-error:` keys are checked, including when they
/// are written on a sequence item's own line.
fn has_bypass_key(lines: &[Line<'_>], block: &Range<usize>) -> bool {
    lines
        .get(block.start..block.end)
        .unwrap_or_default()
        .iter()
        .any(|line| {
            declares_key(*line, BypassKey::If) || declares_key(*line, BypassKey::ContinueOnError)
        })
}

#[test]
fn msrv_target_runs_the_locked_rust_1_89_check() {
    assert_eq!(
        recipe_lines(MAKEFILE, Target("msrv")),
        vec![PINNED_RECIPE],
        "the msrv target must run the locked Rust 1.89 check over every target"
    );
}

#[test]
fn every_workspace_package_declares_the_same_floor() -> Result<(), String> {
    let packages = workspace_rust_versions()?;
    if packages.is_empty() {
        return Err("cargo metadata must report at least one workspace package".to_owned());
    }
    for (name, rust_version) in packages {
        if rust_version != MSRV {
            return Err(format!(
                "{name} must declare Rust {MSRV}, not {rust_version}"
            ));
        }
    }
    Ok(())
}

#[test]
fn the_workspace_uses_the_msrv_aware_resolver() {
    assert!(
        MANIFEST.contains("resolver = \"3\""),
        "the workspace must use resolver 3 so Cargo prefers MSRV-compatible releases"
    );
}

#[test]
fn the_lockfile_selects_the_msrv_compatible_cargo_platform() {
    let versions = locked_versions(LOCK, Crate("cargo-platform"));
    assert!(
        versions.contains(&"0.3.2"),
        "the lockfile must keep the dev-only cargo-platform on 0.3.2, the newest release that builds on Rust 1.89; found {versions:?}"
    );
    assert!(
        !versions.contains(&"0.3.3"),
        "cargo-platform 0.3.3 requires Rust 1.91 and must not be locked; found {versions:?}"
    );
}

#[test]
fn ci_runs_the_msrv_gate() -> Result<(), String> {
    let lines = workflow_lines(CI);
    let run_line = lines
        .iter()
        .position(|line| line.text.trim() == "run: make msrv")
        .ok_or_else(|| "CI must run `make msrv` as its own step".to_owned())?;
    let step = step_block(&lines, run_line)?;
    if has_bypass_key(&lines, &step) {
        return Err("the MSRV step must not carry an `if:` or `continue-on-error:` key".to_owned());
    }
    let job = job_block(&lines, step.start)?;
    if has_bypass_key(&lines, &job) {
        return Err("the MSRV job must not carry an `if:` or `continue-on-error:` key".to_owned());
    }
    Ok(())
}
