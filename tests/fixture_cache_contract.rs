//! Regressions for persistent dependency caching with isolated fixture sources.

use camino::Utf8Path;
use test_helpers::{
    FixtureCrate, SUITE_FIXTURE_LIB_RS, TRIVIAL_THEOREM, build_support_fixture_manifest,
};

#[test]
fn fixtures_share_a_target_but_keep_sources_and_generated_output_separate() -> Result<(), String> {
    let first = new_suite_fixture("cache_contract_first", "theorems/first.theorem")?;
    let second = new_suite_fixture("cache_contract_second", "theorems/second.theorem")?;

    assert_sources_are_isolated(&first, &second)?;
    assert_generated_suites_are_isolated(&first, &second)
}

/// Creates a build-script-enabled fixture owning exactly one theorem.
fn new_suite_fixture(package_name: &str, theorem_path: &str) -> Result<FixtureCrate, String> {
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest(package_name),
        SUITE_FIXTURE_LIB_RS,
    )?;
    fixture.write(Utf8Path::new(theorem_path), TRIVIAL_THEOREM)?;
    Ok(fixture)
}

/// Checks that the fixtures share a target but not their source directories.
fn assert_sources_are_isolated(first: &FixtureCrate, second: &FixtureCrate) -> Result<(), String> {
    assert_path_relationships(first, second)?;
    assert_second_fixture_is_unbuilt(second)
}

/// Checks the directory and target relationships between two fixtures.
fn assert_path_relationships(first: &FixtureCrate, second: &FixtureCrate) -> Result<(), String> {
    let checks = [
        (
            first.manifest_dir() != second.manifest_dir(),
            "fixture source directories must be distinct",
        ),
        (
            first.target_dir() == second.target_dir(),
            "fixtures must share one dependency target",
        ),
        (
            first.target_dir().file_name() == Some("theoremc-fixture-builds"),
            "fixtures must build into the shared fixture target",
        ),
    ];

    checks.into_iter().try_for_each(|(condition, message)| {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    })
}

/// Checks that a source written into one fixture is absent from the other.
///
/// The suite read must fail before either fixture is built, so a cached suite
/// from a sibling fixture cannot satisfy it.
fn assert_second_fixture_is_unbuilt(second: &FixtureCrate) -> Result<(), String> {
    if second
        .modified_time(Utf8Path::new("theorems/first.theorem"))
        .is_ok()
    {
        return Err("writing one fixture must not change another fixture's sources".to_owned());
    }
    if second.generated_suite_contents().is_ok() {
        return Err("an unbuilt fixture must not read a stale shared suite".to_owned());
    }
    Ok(())
}

/// Builds both fixtures and checks each suite lists only its own theorem.
fn assert_generated_suites_are_isolated(
    first: &FixtureCrate,
    second: &FixtureCrate,
) -> Result<(), String> {
    first.cargo_build()?;
    second.cargo_build()?;

    let first_suite = first.generated_suite_contents()?;
    let second_suite = second.generated_suite_contents()?;
    assert_suite_contains(
        &first_suite,
        "theorems/first.theorem",
        "theorems/second.theorem",
    )?;
    assert_suite_contains(
        &second_suite,
        "theorems/second.theorem",
        "theorems/first.theorem",
    )
}

/// Checks that `suite` contains `expected` and omits the sibling's theorem.
fn assert_suite_contains(suite: &str, expected: &str, forbidden: &str) -> Result<(), String> {
    if !suite.contains(expected) {
        return Err(format!("generated suite missing {expected}:\n{suite}"));
    }
    if suite.contains(forbidden) {
        return Err(format!(
            "generated suite leaked another fixture's theorem {forbidden}:\n{suite}"
        ));
    }
    Ok(())
}