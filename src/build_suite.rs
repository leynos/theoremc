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
pub(crate) fn render_theorem_suite<'a>(
    theorem_files: impl IntoIterator<Item = &'a Utf8Path>,
) -> String {
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
            c if c.is_ascii_control() => format!("\\x{{{:02X}}}", c as u8),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(not(test))]
mod build_only {
    use std::io;

    use cap_std::fs_utf8::Dir as Utf8Dir;
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

#[cfg(not(test))]
pub(crate) use build_only::write_theorem_suite;

#[cfg(test)]
#[path = "build_suite_tests.rs"]
mod tests;
