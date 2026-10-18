//! Shared Cargo manifest builders for build-support fixture crates.

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
