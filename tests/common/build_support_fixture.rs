//! Shared Cargo manifest builders for build-support fixture crates.

/// Fixture library source that compiles the generated theorem suite.
///
/// A build-script-enabled fixture must include `OUT_DIR/theorem_suite.rs` for
/// the generated macro invocations to be type-checked. Fixtures that only read
/// the generated file back may use a minimal library instead.
pub const SUITE_FIXTURE_LIB_RS: &str = concat!(
    "//! Fixture crate for build suite behavioural tests.\n",
    "\n",
    "#[doc(hidden)]\n",
    "mod __theoremc_generated_suite {\n",
    "    #[allow(unused_macros)]\n",
    "    macro_rules! theorem_file {\n",
    "        ($path:literal) => {\n",
    "            const _: &str =\n",
    "                include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", $path));\n",
    "        };\n",
    "    }\n",
    "\n",
    "    include!(concat!(env!(\"OUT_DIR\"), \"/theorem_suite.rs\"));\n",
    "}\n",
);

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
