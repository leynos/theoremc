//! Rewrites the build-support manifest for standalone fixture crates.
//!
//! A fixture copies the build-support crate's sources into its own directory,
//! where the workspace's inherited `[lints]` table no longer resolves. Removing
//! that table is the whole job of this module, and it lives apart from
//! `fixture_crate` so the fixture type stays within the module size limit.

/// Builds a fixture manifest with lints disabled for build-script tests.
///
/// # Errors
///
/// Returns an error when the manifest has no `[lints]` table, because copying
/// it unchanged would leave an unresolvable workspace inheritance behind and
/// the fixture would fail to build for a reason the test never intended.
pub(super) fn fixture_build_support_cargo_toml(manifest: &str) -> Result<String, String> {
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
