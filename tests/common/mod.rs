//! Shared test helpers for integration tests.

use std::process::Command;
use std::time::{Duration, SystemTime};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use theoremc::schema::{
    SchemaDiagnosticCode, SourceId, TheoremDoc, load_theorem_docs, load_theorem_docs_with_source,
};

/// Source for the repository build script copied into fixture crates.
pub const BUILD_SCRIPT_SOURCE: &str = include_str!("../../build.rs");

/// Source for build-time theorem discovery copied into fixture crates.
pub const BUILD_DISCOVERY_SOURCE: &str = include_str!("../../src/build_discovery.rs");

/// Source for build-time theorem-suite generation copied into fixture crates.
pub const BUILD_SUITE_SOURCE: &str = include_str!("../../src/build_suite.rs");

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
}

impl FixtureCrate {
    /// Creates a fixture crate with the supplied manifest and library source.
    ///
    /// # Errors
    ///
    /// Returns an error when the temporary directory cannot be created or when
    /// fixture files cannot be written.
    pub fn new(cargo_toml: &str, lib_rs: &str) -> Result<Self, String> {
        let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| "temp dir path is not valid UTF-8".to_owned())?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority())
            .map_err(|error| error.to_string())?;
        let fixture = Self {
            _temp_dir: temp_dir,
            manifest_dir,
            dir,
        };

        fixture.write(Utf8Path::new("Cargo.toml"), cargo_toml)?;
        fixture.write(Utf8Path::new("build.rs"), BUILD_SCRIPT_SOURCE)?;
        fixture.write(Utf8Path::new("src/lib.rs"), lib_rs)?;
        fixture.write(
            Utf8Path::new("src/build_discovery.rs"),
            BUILD_DISCOVERY_SOURCE,
        )?;
        fixture.write(Utf8Path::new("src/build_suite.rs"), BUILD_SUITE_SOURCE)?;

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
    /// Returns an error when the target file cannot be written.
    pub fn overwrite_in_place(&self, path: &Utf8Path, contents: &str) -> Result<(), String> {
        std::fs::write(self.manifest_dir.join(path), contents).map_err(|error| error.to_string())
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
        let output = Command::new("cargo")
            .current_dir(&self.manifest_dir)
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
        let file = std::fs::File::open(&absolute_path).map_err(|error| error.to_string())?;
        let advanced_mtime = SystemTime::now()
            .checked_add(Duration::from_secs(2))
            .ok_or_else(|| "advanced modification time overflowed".to_owned())?;
        file.set_modified(advanced_mtime)
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
    pub fn contains(&self, needle: &str) -> Result<(), String> {
        self.check(true, needle)
    }

    /// Asserts that the build log omits `needle`.
    ///
    /// # Errors
    ///
    /// Returns an actionable diff-style message when `needle` is present.
    pub fn omits(&self, needle: &str) -> Result<(), String> {
        self.check(false, needle)
    }

    /// Returns the raw combined build log.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn check(&self, expect_present: bool, needle: &str) -> Result<(), String> {
        let found = self.0.contains(needle);
        if found == expect_present {
            Ok(())
        } else {
            let verb = if expect_present { "contain" } else { "omit" };
            Err(format!(
                "expected build log to {verb} '{needle}', got:\n{}",
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

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn load_fixture(name: &str) -> std::io::Result<String> {
    Dir::open_ambient_dir("tests/fixtures", ambient_authority())?.read_to_string(name)
}

/// Loads a fixture file and formats I/O failures as test-friendly strings.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read.
pub fn load_fixture_text(name: &str) -> Result<String, String> {
    load_fixture(name).map_err(|error| format!("failed to load fixture {name}: {error}"))
}

/// Loads and parses a theorem fixture.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or parsed as theorem YAML.
pub fn load_fixture_docs(name: &str) -> Result<Vec<TheoremDoc>, String> {
    let yaml = load_fixture_text(name)?;
    load_theorem_docs(&yaml).map_err(|error| error.to_string())
}

/// Asserts that a theorem fixture loads successfully.
///
/// # Errors
///
/// Returns the parser or validation error when loading fails.
pub fn assert_fixture_loads(name: &str) -> Result<(), String> {
    load_fixture_docs(name).map(|_| ())
}

/// Returns the parser or validation error message for an invalid fixture.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or unexpectedly succeeds.
pub fn fixture_error_message(name: &str) -> Result<String, String> {
    let yaml = load_fixture_text(name)?;
    load_theorem_docs(&yaml)
        .err()
        .map(|error| error.to_string())
        .ok_or_else(|| format!("fixture should fail: {name}"))
}

/// Asserts that a theorem fixture fails to load.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or unexpectedly succeeds.
pub fn assert_fixture_fails(name: &str) -> Result<(), String> {
    fixture_error_message(name).map(|_| ())
}

/// Asserts that an invalid fixture error contains `expected_fragment`.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read, unexpectedly succeeds, or
/// fails with a different message.
pub fn assert_fixture_error_contains(name: &str, expected_fragment: &str) -> Result<(), String> {
    let message = fixture_error_message(name)?;
    if message.contains(expected_fragment) {
        Ok(())
    } else {
        Err(format!(
            "expected '{expected_fragment}' in error for {name}, got: {message}"
        ))
    }
}

/// Asserts that an invalid fixture reports the expected typed diagnostic code.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read, unexpectedly succeeds, or
/// reports a diagnostic without the expected code and source location.
pub fn assert_diagnostic_failure(
    fixture_name: &str,
    expected_code: SchemaDiagnosticCode,
) -> Result<(), String> {
    let source = format!("tests/fixtures/{fixture_name}");
    let yaml = load_fixture_text(fixture_name)?;
    let error = load_theorem_docs_with_source(&SourceId::new(&source), &yaml)
        .err()
        .ok_or_else(|| format!("fixture should fail: {fixture_name}"))?;
    let diagnostic = error
        .diagnostic()
        .ok_or_else(|| String::from("diagnostic should be present"))?;

    if diagnostic.code != expected_code {
        return Err(format!(
            "unexpected diagnostic code: expected {}, got {}",
            expected_code.as_str(),
            diagnostic.code.as_str()
        ));
    }
    if diagnostic.location.source != source {
        return Err(format!(
            "unexpected diagnostic source: expected {source}, got {}",
            diagnostic.location.source
        ));
    }
    if diagnostic.location.line == 0 {
        return Err(String::from("diagnostic line should be greater than 0"));
    }
    if diagnostic.location.column == 0 {
        return Err(String::from("diagnostic column should be greater than 0"));
    }

    Ok(())
}
