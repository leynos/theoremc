//! Behavioural tests for Cargo build suite generation.
//!
//! These tests prove that the generated theorem suite compiles correctly
//! for empty, single-file, and multi-file theorem trees.

use camino::Utf8Path;
use rstest_bdd_macros::{given, scenario, then};
use test_helpers::{FixtureCrate, TRIVIAL_THEOREM, build_support_fixture_manifest};

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

fn generated_suite_contents(fixture: &FixtureCrate) -> Result<String, String> {
    fixture.generated_suite_contents()
}

fn assert_sorted_suite_order(contents: &str) -> Result<(), String> {
    let a = contents
        .find("theorems/a.theorem")
        .ok_or_else(|| "generated suite missing theorems/a.theorem".to_owned())?;
    let m = contents
        .find("theorems/m.theorem")
        .ok_or_else(|| "generated suite missing theorems/m.theorem".to_owned())?;
    let z = contents
        .find("theorems/z.theorem")
        .ok_or_else(|| "generated suite missing theorems/z.theorem".to_owned())?;

    if a < m && m < z {
        Ok(())
    } else {
        Err(format!(
            "generated suite order was not sorted a, m, z:\n{contents}"
        ))
    }
}

/// Precondition stub; the empty crate scenario is created in the `then` step.
#[given("a crate without a theorems directory")]
fn given_a_crate_without_a_theorems_directory() {}

#[then("the crate compiles successfully with the generated suite")]
fn then_the_crate_compiles_successfully_with_the_generated_suite() -> Result<(), String> {
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest("build_suite_fixture"),
        FIXTURE_LIB_RS,
    )?;
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
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest("build_suite_fixture"),
        FIXTURE_LIB_RS,
    )?;
    fixture.write(Utf8Path::new("theorems/single.theorem"), TRIVIAL_THEOREM)?;

    fixture.cargo_build()?;
    Ok(())
}

/// Precondition stub; the multi-theorem scenario is created in the `then` step.
#[given("a crate with multiple theorem files created in non-sorted order")]
fn given_a_crate_with_multiple_theorem_files_created_in_non_sorted_order() {}

#[then("all theorems compile in deterministic suite order")]
fn then_all_theorems_compile_in_deterministic_suite_order() -> Result<(), String> {
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest("build_suite_fixture"),
        FIXTURE_LIB_RS,
    )?;
    // Create theorems in non-sorted order (z, a, m)
    fixture.write(Utf8Path::new("theorems/z.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/a.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/m.theorem"), TRIVIAL_THEOREM)?;

    fixture.cargo_build()?;
    assert_sorted_suite_order(&generated_suite_contents(&fixture)?)?;
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

/// Fixtures that declare the same package name still keep their artefacts
/// apart in the shared target directory: each reads only its own suite.
#[test]
fn fixtures_sharing_a_target_read_only_their_own_generated_suite() -> Result<(), String> {
    let first = FixtureCrate::new(
        &build_support_fixture_manifest("build_suite_fixture"),
        FIXTURE_LIB_RS,
    )?;
    first.write(
        Utf8Path::new("theorems/only_first.theorem"),
        TRIVIAL_THEOREM,
    )?;
    let second = FixtureCrate::new(
        &build_support_fixture_manifest("build_suite_fixture"),
        FIXTURE_LIB_RS,
    )?;
    second.write(
        Utf8Path::new("theorems/only_second.theorem"),
        TRIVIAL_THEOREM,
    )?;

    if first.target_dir() != second.target_dir()
        || !first
            .target_dir()
            .as_str()
            .ends_with("theoremc-fixture-builds")
    {
        return Err(format!(
            "fixtures must share the warm target, got {} and {}",
            first.target_dir(),
            second.target_dir()
        ));
    }
    for fixture in [&first, &second] {
        if fixture.read_to_string(Utf8Path::new("Cargo.lock"))? != include_str!("../Cargo.lock") {
            return Err("a fixture must start from the workspace lock".to_owned());
        }
    }

    first.cargo_build()?;
    second.cargo_build()?;
    let first_suite = generated_suite_contents(&first)?;
    let second_suite = generated_suite_contents(&second)?;

    if !first_suite.contains("only_first") || first_suite.contains("only_second") {
        return Err(format!(
            "the first suite read another fixture's:\n{first_suite}"
        ));
    }
    if !second_suite.contains("only_second") || second_suite.contains("only_first") {
        return Err(format!(
            "the second suite read another fixture's:\n{second_suite}"
        ));
    }
    Ok(())
}
