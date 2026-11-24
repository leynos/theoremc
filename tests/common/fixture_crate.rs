//! Capability-scoped build-fixture helpers for integration tests.

use std::io;
use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use filetime::FileTime;

use super::BuildLog;
use super::build_support_manifest::fixture_build_support_cargo_toml;
use super::fixture_build::{self, FixtureIdentity, RunningBuild};
use super::fixture_isolation::{
    Reservations, WORKSPACE_LOCK, first_unused_suffix, shared_target_dir, unique_suffix,
    with_unique_package_name,
};
use super::io_context::path_error;

/// Source for the repository build script copied into fixture crates.
pub const BUILD_SCRIPT_SOURCE: &str = include_str!("../../build.rs");

/// Manifest for the internal build-support crate copied into fixture crates.
pub const BUILD_SUPPORT_CARGO_TOML: &str =
    include_str!("../../crates/theoremc-build-support/Cargo.toml");

/// Root source for the internal build-support crate copied into fixture crates.
pub const BUILD_SUPPORT_LIB_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/lib.rs");

/// Build-script source for the internal build-support crate copied into fixtures.
pub const BUILD_SUPPORT_BUILD_SCRIPT_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/build_script.rs");

/// Discovery source for the internal build-support crate copied into fixtures.
pub const BUILD_DISCOVERY_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/discovery.rs");

/// Suite source for the internal build-support crate copied into fixtures.
pub const BUILD_SUITE_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/suite.rs");

/// Minimal valid theorem document used by build-script fixture crates.
pub const TRIVIAL_THEOREM: &str = concat!(
    "Theorem: Smoke\n",
    "About: Smoke theorem\n",
    "Witness:\n",
    "  - cover: \"true\"\n",
    "    because: \"reachable\"\n",
    "Prove:\n",
    "  - assert: \"true\"\n",
    "    because: \"trivial\"\n",
    "Evidence:\n",
    "  kani:\n",
    "    unwind: 1\n",
    "    expect: SUCCESS\n",
);

/// Temporary Cargo crate used by integration tests that exercise build scripts.
pub struct FixtureCrate {
    /// Temporary directory kept alive for the fixture's lifetime.
    _temp_dir: tempfile::TempDir,
    /// Absolute path to the fixture crate's manifest directory.
    manifest_dir: Utf8PathBuf,
    /// Shared target directory used by all fixture builds.
    target_dir: Utf8PathBuf,
    /// Package name used in generated manifests and lookups.
    package_name: String,
    /// Capability-scoped handle to the fixture's manifest directory.
    dir: Dir,
}

impl FixtureCrate {
    /// Creates a fixture crate with the supplied manifest and library source.
    ///
    /// The crate builds into a target directory shared by every fixture, so
    /// third-party dependencies compile once per run rather than once per
    /// fixture. Its package gets a short unique name, which keeps each
    /// fixture's build artefacts apart inside that directory, and the
    /// workspace lock is copied in so every fixture resolves the same versions.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary directory cannot be created, when
    /// the manifest names no package, or when fixture files cannot be written.
    pub fn new(cargo_toml: &str, lib_rs: &str) -> Result<Self, String> {
        let target_dir = shared_target_dir()?;
        let reservations = Reservations::open(&target_dir)?;
        let suffix = first_unused_suffix(unique_suffix, |candidate| reservations.claim(candidate))?;
        let (manifest, package_name) = with_unique_package_name(cargo_toml, &suffix)?;
        let temp_parent = std::env::temp_dir();
        let temp_dir = tempfile::tempdir().map_err(|error| {
            format!(
                "creating a temporary fixture root under {}: {error} (package {package_name}, shared target {target_dir})",
                temp_parent.display(),
            )
        })?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| {
                format!(
                    "temporary fixture root {} is not valid UTF-8 (package {package_name})",
                    temp_dir.path().display(),
                )
            })?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority()).map_err(|error| {
            format!(
                "{} (package {package_name}, shared target {target_dir})",
                path_error("opening the fixture root", &manifest_dir, &error),
            )
        })?;
        let fixture = Self {
            _temp_dir: temp_dir,
            manifest_dir,
            target_dir,
            package_name,
            dir,
        };

        fixture.write_scaffold(&manifest, lib_rs, BUILD_SCRIPT_SOURCE)?;

        Ok(fixture)
    }

    /// Writes the manifest, lock, build script, library source and the
    /// build-support crate into the fixture directory.
    ///
    /// `build_script` lets a test substitute a script that synchronises with
    /// the test, such as the barrier build in `fixture_build`.
    fn write_scaffold(
        &self,
        manifest: &str,
        lib_rs: &str,
        build_script: &str,
    ) -> Result<(), String> {
        let build_support_manifest = fixture_build_support_cargo_toml(BUILD_SUPPORT_CARGO_TOML)?;
        let files = [
            ("Cargo.toml", manifest),
            ("Cargo.lock", WORKSPACE_LOCK),
            ("build.rs", build_script),
            ("src/lib.rs", lib_rs),
            ("build-support/Cargo.toml", build_support_manifest.as_str()),
            ("build-support/src/lib.rs", BUILD_SUPPORT_LIB_SOURCE),
            (
                "build-support/src/build_script.rs",
                BUILD_SUPPORT_BUILD_SCRIPT_SOURCE,
            ),
            ("build-support/src/discovery.rs", BUILD_DISCOVERY_SOURCE),
            ("build-support/src/suite.rs", BUILD_SUITE_SOURCE),
        ];
        for (path, contents) in files {
            self.write(Utf8Path::new(path), contents)?;
        }
        Ok(())
    }

    /// Reads a file relative to the fixture crate root.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read.
    pub fn read_to_string(&self, path: &Utf8Path) -> Result<String, String> {
        self.dir
            .read_to_string(path.as_str())
            .map_err(|error| self.io_error("reading", &self.absolute(path), &error))
    }

    /// Writes `contents` to a path relative to the fixture crate root.
    ///
    /// # Errors
    ///
    /// Returns an error when the parent directory or target file cannot be
    /// created.
    pub fn write(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_str().is_empty()
        {
            self.dir
                .create_dir_all(parent)
                .map_err(|error| self.io_error("creating", &self.absolute(parent), &error))?;
        }
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| self.io_error("writing", &self.absolute(path), &error))
    }

    /// Creates a directory relative to the fixture crate root.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory cannot be created.
    pub fn create_dir(&self, path: &Utf8Path) -> Result<(), String> {
        self.dir
            .create_dir_all(path)
            .map_err(|error| self.io_error("creating", &self.absolute(path), &error))
    }

    /// Returns a fixture path's modification time.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata cannot be read or has no modification
    /// time.
    pub fn modified_time(&self, path: &Utf8Path) -> Result<SystemTime, String> {
        self.dir
            .metadata(path)
            .and_then(|metadata| metadata.modified())
            .map(cap_std::time::SystemTime::into_std)
            .map_err(|error| {
                self.io_error(
                    "reading the modification time of",
                    &self.absolute(path),
                    &error,
                )
            })
    }

    /// Writes `contents` and advances the target and parent mtimes.
    ///
    /// This is intended for writes performed after a fixture crate has already
    /// been built. Cargo uses modification times to decide whether
    /// `rerun-if-changed` inputs are dirty, so tests can make that dirtiness
    /// explicit instead of sleeping for the filesystem timestamp tick.
    ///
    /// # Errors
    ///
    /// Returns an error when the target file cannot be written or when its
    /// modification time cannot be updated.
    pub fn write_with_advanced_mtime(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        self.write(path, contents)?;
        self.advance_mtime(path)?;
        self.advance_parent_mtime(path)
    }

    /// Overwrites an existing fixture file through the ambient filesystem.
    ///
    /// # Errors
    ///
    /// Returns an error when the target file does not exist or cannot be
    /// written.
    pub fn overwrite_in_place(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        self.dir.metadata(path.as_str()).map_err(|error| {
            self.io_error("reading the metadata of", &self.absolute(path), &error)
        })?;
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| self.io_error("overwriting", &self.absolute(path), &error))
    }

    /// Overwrites an existing fixture file and advances its mtime.
    ///
    /// This is intended for after-build edits in Cargo rerun tests where the
    /// edited file is already listed by `cargo::rerun-if-changed`.
    ///
    /// # Errors
    ///
    /// Returns an error when the target file cannot be written or when its
    /// modification time cannot be updated.
    pub fn overwrite_in_place_with_advanced_mtime(
        &self,
        path: &Utf8Path,
        contents: &str,
    ) -> Result<(), String> {
        self.overwrite_in_place(path, contents)?;
        self.advance_mtime(path)?;
        self.advance_parent_mtime(path)
    }

    /// Returns whether `path` exists relative to the fixture crate root.
    ///
    /// This is a capability-scoped existence probe for tests that must wait for
    /// a build to create a file, without reaching for the ambient filesystem.
    #[must_use]
    pub fn has_file(&self, path: &Utf8Path) -> bool {
        self.dir.metadata(path).is_ok()
    }

    /// Removes a file relative to the fixture crate root.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be removed.
    pub fn remove_file(&self, path: &Utf8Path) -> Result<(), String> {
        self.dir
            .remove_file(path.as_str())
            .map_err(|error| self.io_error("removing", &self.absolute(path), &error))
    }

    /// Returns the fixture crate manifest directory.
    #[must_use]
    pub fn manifest_dir(&self) -> &Utf8Path {
        &self.manifest_dir
    }

    /// Returns the target directory every fixture crate builds into.
    #[must_use]
    pub fn target_dir(&self) -> &Utf8Path {
        &self.target_dir
    }

    /// Runs `cargo build` in the fixture crate.
    ///
    /// # Errors
    ///
    /// Returns the same diagnostic as [`Self::cargo_build_log`].
    pub fn cargo_build(&self) -> Result<(), String> {
        self.cargo_build_log().map(|_| ())
    }

    /// Runs `cargo build -vv --color never` and returns the combined log.
    ///
    /// # Errors
    ///
    /// Returns the command and working directory with the underlying error when
    /// Cargo cannot start, or the exit status with separately labelled stdout
    /// and stderr when Cargo runs and fails.
    pub fn cargo_build_log(&self) -> Result<BuildLog, String> {
        fixture_build::build_log(&self.identity())
    }

    /// Starts `cargo build -vv --color never` without waiting for it.
    ///
    /// This exists for concurrency tests that must observe the build while it is
    /// running, for example to remove a sibling fixture during it. Every other
    /// caller should prefer [`Self::cargo_build`], which cannot leave a child
    /// behind.
    ///
    /// # Errors
    ///
    /// Returns the command and working directory with the underlying error when
    /// Cargo cannot start.
    pub fn spawn_cargo_build(&self) -> Result<RunningBuild, String> {
        fixture_build::spawn_build(&self.identity())
    }

    /// Reads the theorem suite generated by a fixture build.
    ///
    /// # Errors
    ///
    /// Returns an error when the build output cannot be inspected or no
    /// generated theorem suite is present.
    pub fn generated_suite_contents(&self) -> Result<String, String> {
        fixture_build::generated_suite_contents(&self.identity())
    }

    /// Returns the fixture's resolved absolute path for a relative path.
    fn absolute(&self, path: &Utf8Path) -> Utf8PathBuf {
        self.manifest_dir.join(path)
    }

    /// Describes the fixture identity for build and lookup errors.
    fn identity(&self) -> FixtureIdentity {
        FixtureIdentity::new(&self.package_name, &self.manifest_dir, &self.target_dir)
    }

    /// Formats an I/O error with the operation, path and fixture identity.
    fn io_error(&self, operation: &str, path: &Utf8Path, error: &io::Error) -> String {
        format!(
            "{} (fixture package {} at {}, shared target {})",
            path_error(operation, path, error),
            self.package_name,
            self.manifest_dir,
            self.target_dir,
        )
    }

    /// Advances the modification time of `path`'s parent directory.
    fn advance_parent_mtime(&self, path: &Utf8Path) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_str().is_empty()
        {
            self.advance_mtime(parent)?;
        }
        Ok(())
    }

    /// Advances `path`'s modification time to defeat Cargo caching.
    fn advance_mtime(&self, path: &Utf8Path) -> Result<(), String> {
        let absolute_path = self.absolute(path);
        let advanced_mtime = SystemTime::now()
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| "advanced modification time overflowed".to_owned())?;
        filetime::set_file_mtime(&absolute_path, FileTime::from_system_time(advanced_mtime))
            .map_err(|error| {
                self.io_error("advancing the modification time of", &absolute_path, &error)
            })
    }
}

#[cfg(test)]
#[path = "fixture_crate_tests.rs"]
mod tests;
