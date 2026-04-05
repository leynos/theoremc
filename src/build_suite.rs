//! Theorem suite generation for Cargo build integration.
//!
//! Step 3.1.2 generates `OUT_DIR/theorem_suite.rs` with one `theorem_file!(...)`
//! invocation per discovered theorem path. This module provides deterministic
//! rendering and write-if-changed semantics to minimise build churn.

use camino::Utf8Path;

/// Renders deterministic suite contents from a theorem path iterator.
///
/// Each theorem file produces one line: `theorem_file!("path/to/file.theorem");`
/// The output always ends with a trailing newline, even for empty input.
pub(crate) fn render_theorem_suite<'a>(
    theorem_files: impl IntoIterator<Item = &'a Utf8Path>,
) -> String {
    let mut output = String::new();

    for path in theorem_files {
        let escaped = escape_rust_string(path.as_str());
        #[expect(
            clippy::format_push_string,
            reason = "write! is fallible; push_str+format is infallible and clearer here"
        )]
        output.push_str(&format!("theorem_file!(\"{escaped}\");\n"));
    }

    if output.is_empty() {
        output.push('\n');
    }

    output
}

fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(not(test))]
mod build_only {
    use std::io;

    use camino::Utf8Path;
    use thiserror::Error;

    use super::render_theorem_suite;
    use crate::build_discovery::BuildDiscovery;

    const SUITE_FILENAME: &str = "theorem_suite.rs";

    #[derive(Debug, Error)]
    pub(crate) enum BuildSuiteError {
        #[error("io error: {0}")]
        Io(#[from] io::Error),
    }

    pub(crate) fn write_theorem_suite(
        out_dir: &Utf8Path,
        discovery: &BuildDiscovery,
    ) -> Result<(), BuildSuiteError> {
        let rendered = render_theorem_suite(discovery.theorem_files());
        let suite_path = out_dir.join(SUITE_FILENAME);

        if let Ok(existing) = std::fs::read_to_string(&suite_path) {
            if existing == rendered {
                return Ok(());
            }
        }

        std::fs::write(&suite_path, rendered)?;
        Ok(())
    }
}

#[cfg(not(test))]
pub(crate) use build_only::write_theorem_suite;

#[cfg(test)]
#[path = "build_suite_tests.rs"]
mod tests;
