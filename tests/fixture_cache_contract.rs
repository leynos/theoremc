//! Regressions for persistent dependency caching with isolated fixture sources.

use camino::Utf8Path;
use test_helpers::{
    FixtureCrate, SUITE_FIXTURE_LIB_RS, TRIVIAL_THEOREM, build_support_fixture_manifest,
};

#[test]
fn fixtures_share_a_target_but_keep_sources_and_generated_output_separate() -> Result<(), String> {
    let first = FixtureCrate::new(
        &build_support_fixture_manifest("cache_contract_first"),
        SUITE_FIXTURE_LIB_RS,
    )?;
    let second = FixtureCrate::new(
        &build_support_fixture_manifest("cache_contract_second"),
        SUITE_FIXTURE_LIB_RS,
    )?;
    first.write(Utf8Path::new("theorems/first.theorem"), TRIVIAL_THEOREM)?;
    second.write(Utf8Path::new("theorems/second.theorem"), TRIVIAL_THEOREM)?;

    assert_paths_and_isolation(&first, &second)?;
    assert_generated_suites_are_isolated(&first, &second)
}

/// Checks the shared target and the source-level isolation between fixtures.
fn assert_paths_and_isolation(first: &FixtureCrate, second: &FixtureCrate) -> Result<(), String> {
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
        (
            second
                .modified_time(Utf8Path::new("theorems/first.theorem"))
                .is_err(),
            "writing one fixture must not change another fixture's sources",
        ),
        (
            second.generated_suite_contents().is_err(),
            "an unbuilt fixture must not read a stale shared suite",
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