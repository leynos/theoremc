//! Embedded build-support sources and manifest preparation for test fixtures.

/// Manifest for the internal build-support crate copied into fixture crates.
pub(super) const BUILD_SUPPORT_CARGO_TOML: &str =
    include_str!("../../crates/theoremc-build-support/Cargo.toml");

/// Root source for the internal build-support crate copied into fixture crates.
pub(super) const BUILD_SUPPORT_LIB_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/lib.rs");

/// Build-script source for the internal build-support crate copied into fixtures.
pub(super) const BUILD_SUPPORT_BUILD_SCRIPT_SOURCE: &str =
    include_str!("../../crates/theoremc-build-support/src/build_script.rs");

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
