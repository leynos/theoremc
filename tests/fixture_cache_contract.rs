//! Regressions for persistent dependency caching with isolated fixture sources.

use camino::Utf8Path;
use test_helpers::{
    FixtureCrate, SUITE_FIXTURE_LIB_RS, TRIVIAL_THEOREM, build_support_fixture_manifest,
};

/// One fixture in the isolation contract: a package owning a single theorem.
struct FixtureSpec {
    package_name: &'static str,
    theorem: &'static str,
}

const FIRST_FIXTURE: FixtureSpec = FixtureSpec {
    package_name: "cache_contract_first",
    theorem: "theorems/first.theorem",
};

const SECOND_FIXTURE: FixtureSpec = FixtureSpec {
    package_name: "cache_contract_second",
    theorem: "theorems/second.theorem",
};

#[test]
fn fixtures_share_a_target_but_keep_sources_and_generated_output_separate() -> Result<(), String> {
    let first = new_suite_fixture(&FIRST_FIXTURE)?;
    let second = new_suite_fixture(&SECOND_FIXTURE)?;

    assert_sources_are_isolated(&first, &second)?;
    assert_generated_suites_are_isolated(&first, &second)
}

/// Creates a build-script-enabled fixture owning exactly one theorem.
fn new_suite_fixture(spec: &FixtureSpec) -> Result<FixtureCrate, String> {
    let fixture = FixtureCrate::new(
        &build_support_fixture_manifest(spec.package_name),
        SUITE_FIXTURE_LIB_RS,
    )?;
    fixture.write(Utf8Path::new(spec.theorem), TRIVIAL_THEOREM)?;
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
/// Both checks read absence through helpers that report an inspection failure
/// separately from an absent path, so an unreadable fixture cannot pass as an
/// unbuilt one. The suite must also be absent before either fixture is built,
/// so a cached suite from a sibling fixture cannot satisfy it.
fn assert_second_fixture_is_unbuilt(second: &FixtureCrate) -> Result<(), String> {
    assert_unbuilt(
        second.path_exists(Utf8Path::new(FIRST_FIXTURE.theorem)),
        second.find_generated_suite(),
    )
}

/// Applies the unbuilt-fixture rules to one pair of lookup outcomes.
///
/// Taking the outcomes as arguments keeps the failure policy testable without
/// provoking a real filesystem error. An inspection error is propagated
/// unchanged, so it can never be mistaken for an absent path.
fn assert_unbuilt(
    source_present: Result<bool, String>,
    suite: Result<Option<String>, String>,
) -> Result<(), String> {
    if source_present? {
        return Err("writing one fixture must not change another fixture's sources".to_owned());
    }
    if suite?.is_some() {
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
    assert_suite_contains(&first_suite, &FIRST_FIXTURE, &SECOND_FIXTURE)?;
    assert_suite_contains(&second_suite, &SECOND_FIXTURE, &FIRST_FIXTURE)
}

/// Checks that `suite` contains `expected` and omits the sibling's theorem.
fn assert_suite_contains(
    suite: &str,
    expected: &FixtureSpec,
    forbidden: &FixtureSpec,
) -> Result<(), String> {
    if !suite.contains(expected.theorem) {
        return Err(format!(
            "generated suite missing {}:\n{suite}",
            expected.theorem
        ));
    }
    if suite.contains(forbidden.theorem) {
        return Err(format!(
            "generated suite leaked another fixture's theorem {}:\n{suite}",
            forbidden.theorem
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Unit tests for the isolation check's failure policy.

    use super::assert_unbuilt;

    #[test]
    fn a_source_inspection_error_fails_the_isolation_check() {
        let error = assert_unbuilt(
            Err("cannot inspect theorems/first.theorem".to_owned()),
            Ok(None),
        )
        .expect_err("a source inspection failure must not read as absence");

        assert_eq!(error, "cannot inspect theorems/first.theorem");
    }

    #[test]
    fn a_suite_inspection_error_fails_the_isolation_check() {
        let error = assert_unbuilt(Ok(false), Err("cannot inspect build".to_owned()))
            .expect_err("a suite inspection failure must not read as absence");

        assert_eq!(error, "cannot inspect build");
    }

    #[test]
    fn an_absent_source_and_suite_pass_the_isolation_check() {
        assert_unbuilt(Ok(false), Ok(None)).expect("an unbuilt fixture is isolated");
    }
}
