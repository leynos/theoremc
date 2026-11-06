//! Capability-scoped build-fixture helpers for integration tests.

use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use filetime::FileTime;

use super::BuildLog;
use super::fixture_isolation::{
    WORKSPACE_LOCK, shared_target_dir, unique_suffix, with_unique_package_name,
};
use super::hermetic_cargo::hermetic_cargo;

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
    _temp_dir: tempfile::TempDir,
    manifest_dir: Utf8PathBuf,
    target_dir: Utf8PathBuf,
    package_name: String,
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
        let (manifest, package_name) = with_unique_package_name(cargo_toml, &unique_suffix())?;
        let target_dir = shared_target_dir()?;
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
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

        fixture.write(Utf8Path::new("Cargo.toml"), &manifest)?;
        fixture.write(Utf8Path::new("Cargo.lock"), WORKSPACE_LOCK)?;
        fixture.write(Utf8Path::new("build.rs"), BUILD_SCRIPT_SOURCE)?;
        fixture.write(Utf8Path::new("src/lib.rs"), lib_rs)?;
        fixture.write(
            Utf8Path::new("build-support/Cargo.toml"),
            &fixture_build_support_cargo_toml(BUILD_SUPPORT_CARGO_TOML)?,
        )?;
        fixture.write(
            Utf8Path::new("build-support/src/lib.rs"),
            BUILD_SUPPORT_LIB_SOURCE,
        )?;
        fixture.write(
            Utf8Path::new("build-support/src/build_script.rs"),
            BUILD_SUPPORT_BUILD_SCRIPT_SOURCE,
        )?;
        fixture.write(
            Utf8Path::new("build-support/src/discovery.rs"),
            BUILD_DISCOVERY_SOURCE,
        )?;
        fixture.write(
            Utf8Path::new("build-support/src/suite.rs"),
            BUILD_SUITE_SOURCE,
        )?;

        Ok(fixture)
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
    /// # Errors
    ///
    /// Returns the combined Cargo output when the command fails or cannot be
    /// executed.
    pub fn cargo_build_log(&self) -> Result<BuildLog, String> {
        let output = hermetic_cargo(&self.manifest_dir, &self.target_dir)
            .args(["build", "-vv", "--color", "never"])
            .output()
            .map_err(|error| error.to_string())?;
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        if output.status.success() {
            Ok(BuildLog(log))
        } else {
            Err(log)
        }
    }

    /// Reads the theorem suite generated by a fixture build.
    ///
    /// # Errors
    ///
    /// Returns an error when the build output cannot be inspected or no
    /// generated theorem suite is present.
    pub fn generated_suite_contents(&self) -> Result<String, String> {
        // The target directory is shared, so only this fixture's own build
        // directories (named after its unique package) can hold its suite.
        let prefix = format!("{}-", self.package_name);
        let build_dir =
            Dir::open_ambient_dir(self.target_dir.join("debug/build"), ambient_authority())
                .map_err(|error| error.to_string())?;
        for entry_result in build_dir.entries().map_err(|error| error.to_string())? {
            let entry = entry_result.map_err(|error| error.to_string())?;
            let name = entry.file_name().map_err(|error| error.to_string())?;
            if !name.starts_with(&prefix) {
                continue;
            }
            match build_dir.read_to_string(format!("{name}/out/theorem_suite.rs")) {
                Ok(contents) => return Ok(contents),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("generated theorem_suite.rs was not found".to_owned())
    }

    fn advance_parent_mtime(&self, path: &Utf8Path) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_str().is_empty()
        {
            self.advance_mtime(parent)?;
        }
        Ok(())
    }

    fn advance_mtime(&self, path: &Utf8Path) -> Result<(), String> {
        let absolute_path = self.manifest_dir.join(path);
        let advanced_mtime = SystemTime::now()
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| "advanced modification time overflowed".to_owned())?;
        filetime::set_file_mtime(&absolute_path, FileTime::from_system_time(advanced_mtime))
            .map_err(|error| error.to_string())
    }
}

fn fixture_build_support_cargo_toml(manifest: &str) -> Result<String, String> {
    let mut removed_lints = false;
    let mut inside_lints = false;
    let mut retained_lines = Vec::new();

    for line in manifest.lines() {
        let trimmed_line = line.trim();
        let is_section_header = trimmed_line.starts_with('[') && trimmed_line.ends_with(']');
        if is_section_header {
            inside_lints = false;
            if trimmed_line == "[lints]" {
                removed_lints = true;
                inside_lints = true;
                continue;
            }
        }
        if !inside_lints {
            retained_lines.push(line);
        }
    }

    if removed_lints {
        Ok(format!("{}\n", retained_lines.join("\n")))
    } else {
        Err("build-support manifest must contain a [lints] table".to_owned())
    }
}

#[cfg(test)]
#[path = "fixture_crate_tests.rs"]
mod tests;
