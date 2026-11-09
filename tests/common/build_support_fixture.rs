//! Shared crate inputs for build-support fixture crates.
//!
//! A build-support fixture needs two things that every such fixture would
//! otherwise repeat: the manifest that pulls in `theoremc-build-support` as a
//! build dependency ([`build_support_fixture_manifest`]), and, for fixtures
//! that must compile the suite its build script generates, the library source
//! that includes `OUT_DIR/theorem_suite.rs` ([`SUITE_FIXTURE_LIB_RS`]).
//! Including the generated file is what type-checks its macro invocations;
//! a fixture that only reads the suite back may pass a minimal library
//! instead.

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
    "    macro_rules! theorem_file {\n",
    "        ($path:literal) => {\n",
    "            const _: &str =\n",
    "                include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", $path));\n",
    "        };\n",
    "    }\n",
    "\n",
    "    // An empty generated suite invokes no theorem, so a guaranteed\n",
    "    // invocation keeps the macro used without silencing a lint.\n",
    "    theorem_file!(\"Cargo.toml\");\n",
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
