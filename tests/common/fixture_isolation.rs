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

use super::io_context::path_error;

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

/// Returns a short suffix that is unlikely to clash with any other fixture's.
///
/// Three parts make a clash unlikely: the process id separates processes that
/// run at once, the sequence number separates fixtures within a process, and
/// the clock's low 16 bits separate runs that reuse a process id. They do not
/// make it impossible, because the clock bits wrap; `first_unused_suffix`
/// closes that gap by claiming a suffix the shared target has not already
/// allocated.
pub(super) fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let clock = u32::try_from(nanos & 0xffff).unwrap_or(0);
    let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{:x}_{sequence:x}_{clock:04x}", std::process::id())
}

/// Whether a candidate suffix could be claimed for a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Claim {
    /// No other allocator or stale build holds the suffix.
    Reserved,
    /// Another allocator or an earlier run already holds the suffix.
    Taken,
}

/// Returns the first suffix `claim` reports as reserved.
///
/// A name is claimed, not merely checked: the claim is an atomic `create_dir`
/// that loses to any other allocator racing for the same name. A scan that
/// failed, by contrast, cannot say a name is free, so its error stops the
/// allocation.
///
/// # Errors
///
/// Returns the first error `claim` reports.
pub(super) fn first_unused_suffix(
    mut next: impl FnMut() -> String,
    mut claim: impl FnMut(&str) -> Result<Claim, String>,
) -> Result<String, String> {
    loop {
        let suffix = next();
        if claim(&suffix)? == Claim::Reserved {
            return Ok(suffix);
        }
    }
}

/// Names the reservation root inside a shared target directory.
const RESERVATIONS: &str = "fixture-reservations";

/// The marker directory names every allocated fixture suffix.
///
/// Markers are directories rather than files because `create_dir` fails with
/// `AlreadyExists` instead of silently succeeding, which is what makes a claim
/// atomic across processes. A file created with `create_new` would give the
/// same signal; a directory simply matches the build directories it stands for.
/// `cargo clean` removes markers with the rest of the target.
pub(super) struct Reservations {
    /// The shared target the markers live in, used in error context.
    target_dir: Utf8PathBuf,
    /// Capability-scoped handle to the marker root.
    root: Dir,
}

impl Reservations {
    /// Opens (creating if needed) the marker root inside `target_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error when the root cannot be created or opened.
    pub(super) fn open(target_dir: &Utf8Path) -> Result<Self, String> {
        let root_path = target_dir.join(RESERVATIONS);
        Dir::create_ambient_dir_all(&root_path, ambient_authority())
            .map_err(|error| path_error("creating", &root_path, &error))?;
        let root = Dir::open_ambient_dir(&root_path, ambient_authority())
            .map_err(|error| path_error("opening", &root_path, &error))?;
        Ok(Self {
            target_dir: target_dir.to_path_buf(),
            root,
        })
    }

    /// Claims `suffix` for the fixture, or reports that another holder has it.
    ///
    /// The stale-output scan runs first so a name left by an earlier process is
    /// skipped without leaving a marker, and the marker then settles any race
    /// between allocators that both scanned before either had written anything.
    ///
    /// # Errors
    ///
    /// Returns an error when the scan fails, or when a marker cannot be created
    /// for any reason other than already existing.
    pub(super) fn claim(&self, suffix: &str) -> Result<Claim, String> {
        if suffix_is_taken(&self.target_dir, suffix)? {
            return Ok(Claim::Taken);
        }
        let marker = format!("fx_{suffix}");
        match self.root.create_dir(&marker) {
            Ok(()) => Ok(Claim::Reserved),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(Claim::Taken),
            Err(error) => Err(path_error(
                "reserving",
                &self.target_dir.join(RESERVATIONS).join(&marker),
                &error,
            )),
        }
    }
}

/// Returns whether `target_dir` already holds build directories for `suffix`,
/// in the classic layout (`build/fx_<suffix>-<hash>`) or the newer one
/// (`build/fx_<suffix>/<hash>`).
///
/// # Errors
///
/// Returns an error when the build directory exists but cannot be read; a build
/// directory that does not exist yet holds nothing.
pub(super) fn suffix_is_taken(target_dir: &Utf8Path, suffix: &str) -> Result<bool, String> {
    let package = format!("fx_{suffix}");
    let prefix = format!("{package}-");
    let build_path = target_dir.join("debug/build");
    let build = match Dir::open_ambient_dir(&build_path, ambient_authority()) {
        Ok(build) => build,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(path_error("opening", &build_path, &error)),
    };
    let entries = build
        .entries()
        .map_err(|error| path_error("listing", &build_path, &error))?;
    for listing in entries {
        if entry_matches(listing, &package, &prefix, &build_path)? == Some(true) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Classifies one entry yielded by a scan of a shared build directory.
///
/// Returns `Some(true)` when the entry names the package, `Some(false)` when it
/// does not, and `None` when the entry vanished between the listing and the
/// read. A directory can be removed by Cargo or a sibling fixture during the
/// scan, and that is absence rather than failure; every other I/O error is
/// fail-closed, because a scan that could not run must not report a name free.
///
/// # Errors
///
/// Returns the operation, the build path and the underlying error when the
/// entry cannot be read for a reason other than having vanished.
fn entry_matches(
    entry: std::io::Result<cap_std::fs_utf8::DirEntry>,
    package: &str,
    prefix: &str,
    build_path: &Utf8Path,
) -> Result<Option<bool>, String> {
    let found = match entry {
        Ok(found) => found,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(path_error("reading an entry of", build_path, &error)),
    };
    let name = found
        .file_name()
        .map_err(|error| path_error("reading an entry of", build_path, &error))?;
    Ok(Some(name.starts_with(prefix) || name == package))
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
#[expect(
    clippy::disallowed_methods,
    reason = "fixture crates must share the target directory the host Cargo invocation is using, so the helper reads the ambient CARGO_TARGET_DIR"
)]
pub(super) fn shared_target_dir() -> Result<Utf8PathBuf, String> {
    let base = std::env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| repository_root().join("target"), Utf8PathBuf::from);
    let directory = base.join("theoremc-fixture-builds");
    Dir::create_ambient_dir_all(&directory, ambient_authority())
        .map_err(|error| path_error("creating", &directory, &error))?;
    directory
        .canonicalize_utf8()
        .map_err(|error| path_error("canonicalizing", &directory, &error))
}

#[cfg(test)]
#[path = "fixture_isolation_tests.rs"]
mod tests;
