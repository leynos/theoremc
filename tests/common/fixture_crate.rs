//! Capability-scoped build-fixture helpers for integration tests.

use std::cell::RefCell;
use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use filetime::FileTime;
use theoremc_core::schema::StrNewtype;

use super::ExpectedFragment;
use super::fixture_manifest::{canonical_manifest_path, defines_workspace_table};
use super::hermetic_cargo::hermetic_cargo;

#[path = "fixture_output.rs"]
mod output;

#[path = "fixture_cache.rs"]
mod cache;

#[path = "fixture_build_support.rs"]
mod build_support;

use build_support::{
    BUILD_SUPPORT_BUILD_SCRIPT_SOURCE, BUILD_SUPPORT_CARGO_TOML, BUILD_SUPPORT_LIB_SOURCE,
    fixture_build_support_cargo_toml,
};
pub use cache::fixture_target_dir;
use cache::fixture_target_root;

/// Source for the repository build script copied into fixture crates.
pub const BUILD_SCRIPT_SOURCE: &str = include_str!("../../build.rs");

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
    dir: Dir,
    build_out_dir: RefCell<Option<Utf8PathBuf>>,
}

impl FixtureCrate {
    /// Creates a fixture crate with the supplied manifest and library source.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary directory cannot be created or when
    /// fixture files cannot be written.
    pub fn new(cargo_toml: &str, lib_rs: &str) -> Result<Self, String> {
        let source_root = fixture_target_root().join("fixture-sources");
        Dir::create_ambient_dir_all(&source_root, ambient_authority())
            .map_err(|error| error.to_string())?;
        let temp_dir = tempfile::Builder::new()
            .prefix("crate-")
            .tempdir_in(&source_root)
            .map_err(|error| error.to_string())?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| "temp dir path is not valid UTF-8".to_owned())?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority())
            .map_err(|error| error.to_string())?;
        let fixture = Self {
            _temp_dir: temp_dir,
            manifest_dir,
            dir,
            build_out_dir: RefCell::new(None),
        };

        // The source lives under the repository target tree, but remains an
        // independent package rather than joining the enclosing workspace.
        // Appending unconditionally would redefine a `[workspace]` table the
        // caller already supplied, which TOML rejects.
        let manifest = if defines_workspace_table(cargo_toml) {
            cargo_toml.to_owned()
        } else {
            format!("{cargo_toml}\n[workspace]\n")
        };
        fixture.write(Utf8Path::new("Cargo.toml"), &manifest)?;
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
        *self.build_out_dir.borrow_mut() = None;
        // The fixture shares the workspace target so dependencies are reused,
        // but reads its own generated output back from Cargo's JSON messages.
        let output = hermetic_cargo(&self.manifest_dir, &fixture_target_dir())
            .args(["build", "-vv", "--color", "never", "--message-format=json"])
            .output()
            .map_err(|error| error.to_string())?;
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );

        if output.status.success() {
            *self.build_out_dir.borrow_mut() = output::root_build_out_dir(
                &String::from_utf8_lossy(&output.stdout),
                &canonical_manifest_path(&self.manifest_dir)?,
            )?;
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
        let output = self.build_out_dir.borrow();
        let out_dir = output.as_ref().ok_or_else(|| {
            "fixture build did not report its build-script output directory".to_owned()
        })?;
        let dir = Dir::open_ambient_dir(out_dir, ambient_authority())
            .map_err(|error| error.to_string())?;
        dir.read_to_string("theorem_suite.rs")
            .map_err(|error| error.to_string())
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

/// Combined stdout and stderr emitted by a fixture `cargo build` run.
pub struct BuildLog(String);

impl BuildLog {
    /// Returns whether the build script emitted its theorem-directory rerun
    /// instruction.
    #[must_use]
    pub fn ran(&self) -> bool {
        self.0.contains("cargo::rerun-if-changed=theorems")
    }

    /// Asserts that the build log contains `needle`.
    ///
    /// # Errors
    ///
    /// Returns an actionable diff-style message when `needle` is missing.
    pub fn contains(&self, needle: ExpectedFragment<'_>) -> Result<(), String> {
        self.check(true, needle)
    }

    /// Asserts that the build log omits `needle`.
    ///
    /// # Errors
    ///
    /// Returns an actionable diff-style message when `needle` is present.
    pub fn omits(&self, needle: ExpectedFragment<'_>) -> Result<(), String> {
        self.check(false, needle)
    }

    /// Returns the raw combined build log.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn check(&self, expect_present: bool, needle: ExpectedFragment<'_>) -> Result<(), String> {
        let found = self.0.contains(needle.as_str());
        if found == expect_present {
            Ok(())
        } else {
            let verb = if expect_present { "contain" } else { "omit" };
            Err(format!(
                "expected build log to {verb} '{}', got:\n{}",
                needle.as_str(),
                self.0
            ))
        }
    }
}

/// Extracts the body of a top-level TOML section from a document.
///
/// The returned string preserves the section body exactly, including comments
/// and blank lines, and ends with a trailing newline. This helper is intended
/// for fixture manifests that need to copy a known section from the repository
/// manifest without adding a TOML parser as a test dependency.
#[must_use]
pub fn toml_section(document: &str, section_name: &str) -> Option<String> {
    let header_line = format!("[{section_name}]");
    let mut in_section = false;
    let mut body_lines = Vec::new();

    for line in document.lines() {
        if !in_section {
            in_section = line == header_line;
            continue;
        }

        if line.starts_with('[') {
            break;
        }

        body_lines.push(line);
    }

    in_section.then(|| format!("{}\n", body_lines.join("\n")))
}

#[cfg(test)]
#[path = "fixture_crate_tests.rs"]
mod tests;
