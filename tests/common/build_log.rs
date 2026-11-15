//! The log of a fixture build and the TOML helper fixtures use to copy sections.

use theoremc_core::schema::StrNewtype;

use super::ExpectedFragment;

/// Combined stdout and stderr emitted by a fixture `cargo build` run.
pub struct BuildLog(pub(super) String);

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

    /// Asserts whether the log contains or omits `needle`.
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
