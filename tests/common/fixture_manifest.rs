//! Manifest construction and path matching for fixture crates.
//!
//! Fixture manifests are generated rather than copied, and Cargo reports the
//! resolved manifest path rather than the lexical one the fixture was created
//! with. These helpers keep both concerns together and out of the fixture
//! crate module, which owns crate construction and builds.

use camino::{Utf8Path, Utf8PathBuf};

/// Returns whether `cargo_toml` already declares workspace configuration.
///
/// `[workspace.package]` and similar sub-tables define the same TOML table as
/// `[workspace]`, so either form must suppress an appended table.
pub(super) fn defines_workspace_table(cargo_toml: &str) -> bool {
    cargo_toml.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "[workspace]" || trimmed.starts_with("[workspace.")
    })
}

/// Resolves the fixture manifest to the absolute path Cargo reports.
///
/// Cargo canonicalizes its working directory, so a `CARGO_TARGET_DIR`
/// containing lexical components such as `..` would otherwise not match the
/// `manifest_path` field of the emitted compiler artefacts.
pub(super) fn canonical_manifest_path(manifest_dir: &Utf8Path) -> Result<Utf8PathBuf, String> {
    manifest_dir
        .join("Cargo.toml")
        .canonicalize_utf8()
        .map_err(|error| error.to_string())
}
