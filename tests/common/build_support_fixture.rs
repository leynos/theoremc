//! Shared Cargo manifest builders for build-support fixture crates.

use super::fixture_crate::FixtureCrate;
use super::fixture_isolation::FixtureLock;

/// Builds a minimal manifest for a build-support integration fixture.
///
/// # Examples
///
/// ```
/// let manifest = test_helpers::build_support_fixture_manifest("sample_fixture");
/// assert!(manifest.contains("name = \"sample_fixture\""));
/// ```
#[must_use]
pub fn build_support_fixture_manifest(package_name: &str) -> String {
    format!(
        concat!(
            "[package]\n",
            "name = \"{package_name}\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[build-dependencies]\n",
            "theoremc-build-support = {{ path = \"build-support\" }}\n",
        ),
        package_name = package_name,
    )
}

/// Creates a build-support fixture crate with the supplied library source.
///
/// The manifest and the lock it resolves against are chosen together, so a
/// caller cannot pair this manifest with another dependency graph's template.
/// The lock must match, because every fixture build runs `--frozen`.
///
/// # Errors
///
/// Returns the error [`FixtureCrate::new`] reports.
pub fn build_support_fixture(package_name: &str, lib_rs: &str) -> Result<FixtureCrate, String> {
    FixtureCrate::new(
        &build_support_fixture_manifest(package_name),
        lib_rs,
        FixtureLock::BuildSupport,
    )
}
