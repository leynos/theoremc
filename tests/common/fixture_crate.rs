//! Capability-scoped build-fixture helpers for integration tests.

use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use filetime::FileTime;

use super::BuildLog;
use super::fixture_command::{FixtureOutcome, run_fixture_command};
use super::fixture_isolation::{
    WORKSPACE_LOCK, first_unused_suffix, shared_target_dir, shared_target_sources_dir,
    suffix_is_taken, unique_suffix, with_unique_package_name,
};
use super::hermetic_cargo::hermetic_cargo;
use super::suite_layout::{package_directories, read_suite};

/// Source for the repository build script copied into fixture crates.
pub const BUILD_SCRIPT_SOURCE: &str = include_str!("../../build.rs");

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
    /// third-party dependencies compile once per warm target rather than once
    /// per fixture; a `cargo clean` or a different `CARGO_TARGET_DIR` starts a
    /// new one. Its package gets a short unique name, which keeps each fixture's
    /// build artefacts apart inside that directory, and the workspace lock is
    /// copied in so every fixture resolves the same versions. The sources live
    /// under that target too, so an interrupted run cannot leave them in
    /// `$TMPDIR`.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary directory cannot be created, when
    /// the manifest names no package, or when fixture files cannot be written.
    pub fn new(cargo_toml: &str, lib_rs: &str) -> Result<Self, String> {
        let target_dir = shared_target_dir()?;
        let suffix =
            first_unused_suffix(unique_suffix, |taken| suffix_is_taken(&target_dir, taken))?;
        let (manifest, package_name) = with_unique_package_name(cargo_toml, &suffix)?;
        let sources_dir = shared_target_sources_dir()?;
        let temp_dir = tempfile::Builder::new()
            .prefix("fixture-")
            .tempdir_in(&sources_dir)
            .map_err(|error| error.to_string())?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| "temp dir path is not valid UTF-8".to_owned())?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority())
            .map_err(|error| error.to_string())?;
        let fixture = Self {
            _temp_dir: temp_dir,
            manifest_dir,
            target_dir,
            package_name,
            dir,
        };

        fixture.write_scaffold(&manifest, lib_rs)?;

        Ok(fixture)
    }

    /// Writes the manifest, lock, build script and library source into the
    /// fixture directory.
    ///
    /// The build-support crate is not copied: the manifest names the checkout's
    /// copy by path, so every fixture resolves it to the same unit in the shared
    /// target and reuses the compiled artefact.
    fn write_scaffold(&self, manifest: &str, lib_rs: &str) -> Result<(), String> {
        let files = [
            ("Cargo.toml", manifest),
            ("Cargo.lock", WORKSPACE_LOCK),
            ("build.rs", BUILD_SCRIPT_SOURCE),
            ("src/lib.rs", lib_rs),
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
            .map_err(|error| error.to_string())
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
                .map_err(|error| error.to_string())?;
        }
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| error.to_string())
    }

    /// Creates a directory relative to the fixture crate root.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory cannot be created.
    pub fn create_dir(&self, path: &Utf8Path) -> Result<(), String> {
        self.dir
            .create_dir_all(path)
            .map_err(|error| error.to_string())
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
            .map_err(|error| error.to_string())
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
        self.dir
            .metadata(path.as_str())
            .map_err(|error| error.to_string())?;
        self.dir
            .write(path.as_str(), contents)
            .map_err(|error| error.to_string())
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

    /// Returns the unique package name this fixture builds under.
    #[must_use]
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Runs `cargo build` in the fixture crate.
    ///
    /// # Errors
    ///
    /// Returns the combined Cargo output when the command fails or cannot be
    /// executed.
    pub fn cargo_build(&self) -> Result<(), String> {
        self.cargo_build_log().map(|_| ())
    }

    /// Runs `cargo build -vv --color never` and returns the combined log.
    ///
    /// The command runs under the fixture lock and a deadline: it is a real
    /// Cargo invocation that shares the fixture target and Cargo's package cache
    /// with every other fixture, and it can stall on either. An overrun fails the
    /// test with the command, the deadline and whatever the build printed,
    /// instead of waiting for nextest to kill the whole run.
    ///
    /// # Errors
    ///
    /// Returns the combined Cargo output when the command fails, overruns, or
    /// cannot be executed.
    pub fn cargo_build_log(&self) -> Result<BuildLog, String> {
        let mut command = hermetic_cargo(&self.manifest_dir, &self.target_dir);
        command.args(["build", "-vv", "--color", "never"]);
        match run_fixture_command(command)? {
            FixtureOutcome::Completed(output) if output.is_success() => {
                Ok(BuildLog(output.combined()))
            }
            FixtureOutcome::Completed(output) => Err(format!(
                "cargo build failed with status {}\n{}",
                output.status,
                output.combined()
            )),
            FixtureOutcome::TimedOut {
                deadline,
                elapsed,
                output,
            } => Err(format!(
                "cargo build did not finish within {deadline:?} (ran for {elapsed:?})\n{output}"
            )),
        }
    }

    /// Reads the theorem suite generated by a fixture build.
    ///
    /// # Errors
    ///
    /// Returns an error when the build output cannot be inspected or no
    /// generated theorem suite is present.
    pub fn generated_suite_contents(&self) -> Result<String, String> {
        let build_dir =
            Dir::open_ambient_dir(self.target_dir.join("debug/build"), ambient_authority())
                .map_err(|error| error.to_string())?;
        for name in package_directories(&build_dir, &self.package_name)? {
            if let Some(contents) = read_suite(&build_dir, &name)? {
                return Ok(contents);
            }
        }
        Err("generated theorem_suite.rs was not found".to_owned())
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
        let absolute_path = self.manifest_dir.join(path);
        let advanced_mtime = SystemTime::now()
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| "advanced modification time overflowed".to_owned())?;
        filetime::set_file_mtime(&absolute_path, FileTime::from_system_time(advanced_mtime))
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "fixture_crate_tests.rs"]
mod tests;
