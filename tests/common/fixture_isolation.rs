//! Isolation helpers for fixture crates built by integration tests.
//!
//! Fixture crates share one warm target directory so third-party dependencies
//! compile once per run, and keep their own artefacts apart through unique
//! package names. These helpers decide the target directory, the package name
//! and the lock a fixture starts from; `fixture_crate` owns everything else.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};

/// The workspace's resolved dependency graph, copied into every fixture crate.
///
/// Without it each fixture resolves the registry afresh, so two fixtures can
/// resolve a crate to different versions and compile different units. Seeding
/// the lock makes them resolve to the versions the workspace already built, so
/// the units a fixture compiles match across fixtures and the shared target
/// directory reuses them instead of starting cold.
pub(super) const WORKSPACE_LOCK: &str = include_str!("../../Cargo.lock");

/// Counts fixtures in this process, so package names stay unique within it.
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Returns a short suffix no other fixture, in this or an earlier run, has used.
///
/// Three parts keep it unique where each alone would not be. The process id
/// separates processes that run at once (two live processes never share one),
/// the sequence number separates fixtures within a process, and the clock's low
/// 16 bits separate runs that reuse a process id over the life of a persistent
/// target directory.
pub(super) fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let clock = u32::try_from(nanos & 0xffff).unwrap_or(0);
    let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{:x}_{sequence:x}_{clock:04x}", std::process::id())
}

/// Returns the first suffix `in_use` does not report as taken.
///
/// A name is taken when the shared target already holds build directories for
/// it, left by an earlier process whose id was reused. Checking the directory
/// makes a name safe by construction instead of by probability.
pub(super) fn first_unused_suffix(
    mut next: impl FnMut() -> String,
    in_use: impl Fn(&str) -> bool,
) -> String {
    loop {
        let suffix = next();
        if !in_use(&suffix) {
            return suffix;
        }
    }
}

/// Returns whether `target_dir` already holds build directories for `suffix`.
pub(super) fn suffix_is_taken(target_dir: &Utf8Path, suffix: &str) -> bool {
    let prefix = format!("fx_{suffix}-");
    Dir::open_ambient_dir(target_dir.join("debug/build"), ambient_authority())
        .and_then(|build| build.entries())
        .is_ok_and(|mut entries| {
            entries.any(|listing| {
                listing
                    .and_then(|found| found.file_name())
                    .is_ok_and(|name| name.starts_with(&prefix))
            })
        })
}

/// Reads the value of a `name = "..."` line.
fn quoted_name(line: &str) -> Option<&str> {
    let value = line
        .trim()
        .strip_prefix("name")?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    value.strip_prefix('"')?.strip_suffix('"')
}

/// Names the `[package]` after *suffix* and returns the manifest and the name.
///
/// Fixtures share one target directory, so two fixtures with the same package
/// name would share build directories and a test could read another fixture's
/// generated suite. The name is short on purpose: Kani writes a file named
/// after the crate and the proof harness, and a long crate name pushed that
/// file name past the filesystem limit. Only the first `name` inside
/// `[package]` changes.
pub(super) fn with_unique_package_name(
    manifest: &str,
    suffix: &str,
) -> Result<(String, String), String> {
    let mut in_package = false;
    let mut unique = None;
    let mut lines = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
        }
        match quoted_name(line).filter(|_| in_package && unique.is_none()) {
            Some(_) => {
                let name = format!("fx_{suffix}");
                lines.push(format!("name = \"{name}\""));
                unique = Some(name);
            }
            None => lines.push(line.to_owned()),
        }
    }
    let name = unique.ok_or_else(|| "fixture manifest must name its package".to_owned())?;
    Ok((format!("{}\n", lines.join("\n")), name))
}

/// Returns the repository checkout, which is two levels above this crate.
fn repository_root() -> Utf8PathBuf {
    let crate_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .unwrap_or(crate_dir)
        .to_path_buf()
}

/// Returns the absolute target directory shared by every fixture crate.
///
/// It sits inside the caller's `CARGO_TARGET_DIR` when one is exported, else
/// inside the repository's `target`, so it is warm on a second run and `cargo
/// clean` removes it.
pub(super) fn shared_target_dir() -> Result<Utf8PathBuf, String> {
    let base = std::env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| repository_root().join("target"), Utf8PathBuf::from);
    let directory = base.join("theoremc-fixture-builds");
    Dir::create_ambient_dir_all(&directory, ambient_authority())
        .map_err(|error| error.to_string())?;
    directory
        .canonicalize_utf8()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "fixture_isolation_tests.rs"]
mod tests;
