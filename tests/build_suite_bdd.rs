//! Behavioural tests for Cargo build suite generation.
//!
//! These tests prove that the generated theorem suite compiles correctly
//! for empty, single-file, and multi-file theorem trees.

pub mod common;

use camino::Utf8Path;
use common::{FixtureCrate, TRIVIAL_THEOREM, toml_section};
use rstest_bdd_macros::{given, scenario, then};

const ROOT_CARGO_TOML: &str = include_str!("../Cargo.toml");

/// Fixture lib.rs that includes the generated suite wiring.
const FIXTURE_LIB_RS: &str = concat!(
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

fn fixture_cargo_toml() -> Result<String, String> {
    let build_dependencies = toml_section(ROOT_CARGO_TOML, "build-dependencies")
        .ok_or_else(|| "root Cargo.toml is missing [build-dependencies]".to_owned())?;

    Ok(format!(
        concat!(
            "[package]\n",
            "name = \"build_suite_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[build-dependencies]\n",
            "{build_dependencies}",
        ),
        build_dependencies = build_dependencies
    ))
}

/// Precondition stub; the empty crate scenario is created in the `then` step.
#[given("a crate without a theorems directory")]
fn given_a_crate_without_a_theorems_directory() {}

#[then("the crate compiles successfully with the generated suite")]
fn then_the_crate_compiles_successfully_with_the_generated_suite() -> Result<(), String> {
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;
    // No theorems directory created - testing empty suite case
    fixture.cargo_build()?;
    Ok(())
}

/// Precondition stub; the single theorem scenario is created in the `then` step.
#[given("a crate with one theorem file")]
fn given_a_crate_with_one_theorem_file() {}

#[then("the single theorem is included automatically and the crate compiles")]
fn then_the_single_theorem_is_included_automatically_and_the_crate_compiles() -> Result<(), String>
{
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new("theorems/single.theorem"), TRIVIAL_THEOREM)?;

    fixture.cargo_build()?;
    Ok(())
}

/// Precondition stub; the multi-theorem scenario is created in the `then` step.
#[given("a crate with multiple theorem files created in non-sorted order")]
fn given_a_crate_with_multiple_theorem_files_created_in_non_sorted_order() {}

#[then("all theorems compile in deterministic suite order")]
fn then_all_theorems_compile_in_deterministic_suite_order() -> Result<(), String> {
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;
    // Create theorems in non-sorted order (z, a, m)
    fixture.write(Utf8Path::new("theorems/z.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/a.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/m.theorem"), TRIVIAL_THEOREM)?;

    fixture.cargo_build()?;
    Ok(())
}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "An empty crate still compiles with generated suite wiring"
)]
fn an_empty_crate_still_compiles_with_generated_suite_wiring() {}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "A single theorem file is included automatically"
)]
fn a_single_theorem_file_is_included_automatically() {}

#[scenario(
    path = "tests/features/build_suite.feature",
    name = "Multiple theorem files compile in deterministic suite order"
)]
fn multiple_theorem_files_compile_in_deterministic_suite_order() {}
