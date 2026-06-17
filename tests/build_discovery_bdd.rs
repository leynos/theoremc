//! Behavioural tests for Cargo build discovery of theorem files.

pub mod common;

use camino::Utf8Path;
use common::{FixtureCrate, TRIVIAL_THEOREM, toml_section};
use rstest_bdd_macros::{given, scenario, then};

const ROOT_CARGO_TOML: &str = include_str!("../Cargo.toml");
const FIXTURE_LIB_RS: &str = "//! Fixture crate for build discovery behavioural tests.\n";

fn fixture_cargo_toml() -> Result<String, String> {
    let build_dependencies = toml_section(ROOT_CARGO_TOML, "build-dependencies")
        .ok_or_else(|| "root Cargo.toml is missing [build-dependencies]".to_owned())?;

    Ok(format!(
        concat!(
            "[package]\n",
            "name = \"build_discovery_fixture\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2024\"\n\n",
            "[build-dependencies]\n",
            "{build_dependencies}",
        ),
        build_dependencies = build_dependencies
    ))
}

/// Precondition stub; the nested theorem fixture is created in the `then` step.
#[given("a crate with nested theorem files")]
fn given_a_crate_with_nested_theorem_files() {}

#[then("building twice stays fresh and editing a theorem reruns the build script")]
fn then_building_twice_stays_fresh_and_editing_a_theorem_reruns_the_build_script()
-> Result<(), String> {
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new("theorems/root.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(
        Utf8Path::new("theorems/nested/alpha.theorem"),
        TRIVIAL_THEOREM,
    )?;

    let first_build = fixture.cargo_build_log()?;
    if !first_build.ran() {
        return Err(format!(
            "first build should run the build script, got:\n{}",
            first_build.as_str()
        ));
    }
    first_build.contains("cargo::rerun-if-changed=theorems/nested/alpha.theorem")?;
    first_build.contains("cargo::rerun-if-changed=theorems/nested")?;

    let second_build = fixture.cargo_build_log()?;
    if second_build.ran() {
        return Err(format!(
            "second unchanged build should stay fresh, got:\n{}",
            second_build.as_str()
        ));
    }

    fixture.overwrite_in_place_with_advanced_mtime(
        Utf8Path::new("theorems/nested/alpha.theorem"),
        &format!("{TRIVIAL_THEOREM}\n# edited\n"),
    )?;

    let third_build = fixture.cargo_build_log()?;
    if !third_build.ran() {
        return Err(format!(
            "editing a theorem file should rerun the build script, got:\n{}",
            third_build.as_str()
        ));
    }
    Ok(())
}

/// Precondition stub; the non-theorem fixture is created in the `then` step.
#[given("a crate with ignored non-theorem files under theorems")]
fn given_a_crate_with_ignored_non_theorem_files_under_theorems() {}

#[then("the build script emits only theorem inputs")]
fn then_the_build_script_emits_only_theorem_inputs() -> Result<(), String> {
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;
    fixture.write(Utf8Path::new("theorems/kept.theorem"), TRIVIAL_THEOREM)?;
    fixture.write(Utf8Path::new("theorems/ignored.txt"), "not a theorem")?;

    let first_build = fixture.cargo_build_log()?;
    if !first_build.ran() {
        return Err(format!(
            "first build should run the build script, got:\n{}",
            first_build.as_str()
        ));
    }
    first_build.omits("cargo::rerun-if-changed=theorems/ignored.txt")?;
    first_build.contains("cargo::rerun-if-changed=theorems/kept.theorem")?;
    Ok(())
}

/// Precondition stub; the missing-directory precondition is verified in the `then` step.
#[given("a crate without a theorems directory")]
fn given_a_crate_without_a_theorems_directory() {}

#[then("creating theorems later reruns the build script without manual seeding")]
fn then_creating_theorems_later_reruns_the_build_script_without_manual_seeding()
-> Result<(), String> {
    let fixture = FixtureCrate::new(&fixture_cargo_toml()?, FIXTURE_LIB_RS)?;

    let first_build = fixture.cargo_build_log()?;
    if !first_build.ran() {
        return Err(format!(
            "first build should run the build script, got:\n{}",
            first_build.as_str()
        ));
    }
    first_build.contains("cargo::rerun-if-changed=theorems")?;

    fixture.write_with_advanced_mtime(Utf8Path::new("theorems/first.theorem"), TRIVIAL_THEOREM)?;

    let second_build = fixture.cargo_build_log()?;
    if !second_build.ran() {
        return Err(format!(
            "creating theorems after the first build should rerun the build script, got:\n{}",
            second_build.as_str()
        ));
    }
    second_build.contains("cargo::rerun-if-changed=theorems/first.theorem")?;
    Ok(())
}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Existing theorem files are discovered recursively"
)]
fn existing_theorem_files_are_discovered_recursively() {}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Non-theorem files do not participate in discovery"
)]
fn non_theorem_files_do_not_participate_in_discovery() {}

#[scenario(
    path = "tests/features/build_discovery.feature",
    name = "Missing theorem directory is handled without manual seeding"
)]
fn missing_theorem_directory_is_handled_without_manual_seeding() {}
