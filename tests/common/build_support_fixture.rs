//! Shared Cargo manifest builders for build-support fixture crates.

use super::manifest_paths::build_support_dependency;

/// Builds a minimal manifest for a build-support integration fixture.
///
/// The fixture is its own workspace root, because it is created inside the
/// repository tree and would otherwise be treated as a candidate member of this
/// repository's workspace. It depends on the checkout's build-support crate by
/// path, so every fixture shares one identity for that crate in the warm target.
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
            "[workspace]\n\n",
            "[build-dependencies]\n",
            "{dependency}\n",
        ),
        package_name = package_name,
        dependency = build_support_dependency(),
    )
}
