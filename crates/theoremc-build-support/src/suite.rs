//! Theorem suite generation for Cargo build integration.
//!
//! Step 3.1.2 generates `OUT_DIR/theorem_suite.rs` with one `theorem_file!(...)`
//! invocation per discovered theorem path. This module provides deterministic
//! rendering and write-if-changed semantics to minimize build churn.

use camino::Utf8Path;

/// Renders deterministic suite contents from a theorem path iterator.
///
/// Each theorem file produces one line: `theorem_file!("path/to/file.theorem");`
/// The output always ends with a trailing newline, even for empty input.
fn render_theorem_suite<'a>(theorem_files: impl IntoIterator<Item = &'a Utf8Path>) -> String {
    let mut output = String::new();

    for path in theorem_files {
        let escaped = escape_rust_string(path.as_str());
        output.push_str("theorem_file!(\"");
        output.push_str(&escaped);
        output.push_str("\");\n");
    }

    if output.is_empty() {
        output.push('\n');
    }

    output
}

fn escape_rust_string(s: &str) -> String {
    s.chars()
        .map(|ch| match ch {
            '\\' => "\\\\".to_owned(),
            '"' => "\\\"".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            c if c.is_ascii_control() => format!("\\x{:02X}", c as u8),
            c => c.to_string(),
        })
        .collect()
}

mod build_only {
    //! Build-only errors and writing API around the parent module's renderer.

    use std::io;

    use cap_std::fs_utf8::Dir as Utf8Dir;
    use thiserror::Error;

    use super::render_theorem_suite;
    use crate::BuildDiscovery;

    const SUITE_FILENAME: &str = "theorem_suite.rs";

    /// Errors returned while writing the generated theorem suite.
    #[derive(Debug, Error)]
    pub enum BuildSuiteError {
        /// An input or output operation on the generated suite failed.
        #[error("io error: {0}")]
        Io(#[from] io::Error),
    }

    /// Writes `theorem_suite.rs` when its rendered contents changed.
    ///
    /// # Examples
    ///
    /// Discover a fixture's theorem inputs and write their generated macro
    /// callsites into an output directory:
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use camino::Utf8Path;
    /// use cap_std::{ambient_authority, fs_utf8::Dir};
    /// use theoremc_build_support::{discover_theorem_inputs, write_theorem_suite};
    ///
    /// let crate_root = tempfile::tempdir()?;
    /// let output_root = tempfile::tempdir()?;
    /// let crate_root_path = Utf8Path::from_path(crate_root.path())
    ///     .ok_or_else(|| std::io::Error::other("temporary path is not UTF-8"))?;
    /// let output_root_path = Utf8Path::from_path(output_root.path())
    ///     .ok_or_else(|| std::io::Error::other("temporary path is not UTF-8"))?;
    /// let crate_root_dir = Dir::open_ambient_dir(crate_root_path, ambient_authority())?;
    /// crate_root_dir.create_dir_all("theorems")?;
    /// crate_root_dir.write("theorems/example.theorem", "")?;
    /// let discovery = discover_theorem_inputs(crate_root_path)?;
    /// let output_dir = Dir::open_ambient_dir(output_root_path, ambient_authority())?;
    /// write_theorem_suite(&output_dir, &discovery)?;
    /// let generated = output_dir.read_to_string("theorem_suite.rs")?;
    /// assert!(generated.contains("theorems/example.theorem"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`BuildSuiteError`] when the output directory cannot be read or
    /// the generated suite cannot be written.
    pub fn write_theorem_suite(
        out_dir: &Utf8Dir,
        discovery: &BuildDiscovery,
    ) -> Result<(), BuildSuiteError> {
        let rendered = render_theorem_suite(discovery.theorem_files());
        let suite_path = camino::Utf8PathBuf::from(SUITE_FILENAME);

        match out_dir.read_to_string(&suite_path) {
            Ok(existing) if existing == rendered => return Ok(()),
            Ok(_) => {}
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return Err(BuildSuiteError::Io(e));
                }
            }
        }

        out_dir.write(&suite_path, rendered)?;
        Ok(())
    }
}

pub use build_only::{BuildSuiteError, write_theorem_suite};

#[cfg(test)]
#[path = "suite_tests.rs"]
mod tests;
