//! Contract tests for `.config/nextest.toml`.
//!
//! The fixture suites are serialized twice over: by the cross-process lock the
//! commands take, and by the nextest test group that keeps their tests from
//! starting together. The lock is code and cannot silently disappear; the group
//! is configuration, and a renamed binary or a lost override would quietly turn
//! the fan-out back on (issue #197). These tests read the configuration and fail
//! when the group stops covering the fixture suites or stops bounding them.
//!
//! The resolved membership can be inspected by hand, but not from a test:
//! `cargo nextest show-config test-groups` rebuilds the workspace metadata, which
//! is far too expensive to run inside the suite it describes.

use std::time::Duration;

use test_helpers::DEFAULT_FIXTURE_COMMAND_TIMEOUT;
use toml::Value;

/// The nextest configuration under test.
const CONFIG: &str = include_str!("../.config/nextest.toml");

/// The test group every fixture suite must belong to.
const GROUP: &str = "nested-cargo-builds";

/// The three test binaries that start a fixture `cargo` build.
const FIXTURE_BINARIES: [&str; 3] = [
    "build_discovery_bdd",
    "build_suite_bdd",
    "theorem_file_macro_bdd",
];

/// Parses the configuration, reporting an unreadable file as a test failure.
///
/// The document is parsed as a table, not as a bare value, because the file is a
/// sequence of top-level tables rather than a single `key = value` entry.
fn parse(config: &str) -> Result<Value, String> {
    toml::from_str::<Value>(config)
        .map_err(|error| format!("`.config/nextest.toml` is not valid TOML: {error}"))
}

/// Returns the group's declared `max-threads`.
fn group_max_threads(config: &Value) -> Result<i64, String> {
    config
        .get("test-groups")
        .and_then(|groups| groups.get(GROUP))
        .and_then(|group| group.get("max-threads"))
        .and_then(Value::as_integer)
        .ok_or_else(|| format!("test group `{GROUP}` must declare an integer `max-threads`"))
}

/// Returns the default profile's overrides.
fn overrides(config: &Value) -> Result<&[Value], String> {
    config
        .get("profile")
        .and_then(|profile| profile.get("default"))
        .and_then(|default| default.get("overrides"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| "the default profile must declare an `overrides` array".to_owned())
}

/// Returns the override that assigns the fixture group.
fn group_override(config: &Value) -> Result<&Value, String> {
    overrides(config)?
        .iter()
        .find(|entry| entry.get("test-group").and_then(Value::as_str) == Some(GROUP))
        .ok_or_else(|| format!("no default-profile override assigns test group `{GROUP}`"))
}

/// Returns the override's filter expression.
fn filter_of(entry: &Value) -> Result<&str, String> {
    entry
        .get("filter")
        .and_then(Value::as_str)
        .ok_or_else(|| "the fixture-group override must carry a `filter`".to_owned())
}

/// Parses a nextest duration such as `60s` or `45m`.
fn parse_duration(text: &str) -> Result<Duration, String> {
    let (digits, multipliers) = match text.trim() {
        value if value.ends_with('s') => (value.trim_end_matches('s'), 1_u64),
        value if value.ends_with('m') => (value.trim_end_matches('m'), 60),
        value if value.ends_with('h') => (value.trim_end_matches('h'), 3_600),
        _ => {
            return Err(format!(
                "{text:?} is not a duration with an s, m or h suffix"
            ));
        }
    };
    let amount = digits
        .trim()
        .parse::<u64>()
        .map_err(|error| format!("{text:?} has no whole-number duration: {error}"))?;
    amount
        .checked_mul(multipliers)
        .map(Duration::from_secs)
        .ok_or_else(|| format!("{text:?} overflows a duration"))
}

/// Returns how long the override lets a matching test run before termination.
fn allowance_of(entry: &Value) -> Result<Duration, String> {
    let timeout = entry
        .get("slow-timeout")
        .ok_or_else(|| "the fixture-group override must declare a `slow-timeout`".to_owned())?;
    let period = timeout
        .get("period")
        .and_then(Value::as_str)
        .ok_or_else(|| "`slow-timeout` must declare a `period`".to_owned())?;
    let terminate_after = timeout
        .get("terminate-after")
        .and_then(Value::as_integer)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "`slow-timeout` must declare a positive `terminate-after`".to_owned())?;
    parse_duration(period)?
        .checked_mul(terminate_after)
        .ok_or_else(|| "the group's period and terminate-after overflow a duration".to_owned())
}

#[test]
fn the_fixture_group_runs_one_test_at_a_time() -> Result<(), String> {
    let config = parse(CONFIG)?;

    assert_eq!(
        group_max_threads(&config)?,
        1,
        "test group `{GROUP}` must serialize the fixture suites",
    );
    Ok(())
}

#[test]
fn the_group_override_covers_every_fixture_suite() -> Result<(), String> {
    let config = parse(CONFIG)?;
    let filter = filter_of(group_override(&config)?)?;

    for binary in FIXTURE_BINARIES {
        let expected = format!("binary(={binary})");
        assert!(
            filter.contains(&expected),
            "the fixture-group filter must contain `{expected}`, got: {filter}",
        );
    }
    Ok(())
}

#[test]
fn the_group_allowance_exceeds_the_fixture_deadline() -> Result<(), String> {
    let config = parse(CONFIG)?;
    let allowance = allowance_of(group_override(&config)?)?;

    assert!(
        allowance > DEFAULT_FIXTURE_COMMAND_TIMEOUT,
        "the group allows {allowance:?}, so the exported fixture deadline \
         {DEFAULT_FIXTURE_COMMAND_TIMEOUT:?} must fire first",
    );
    Ok(())
}

/// The serialised fixture suites run one after another, so the whole run is at
/// least the sum of their allowances; a global cap at or below one test's
/// allowance would cancel legitimate work.
#[test]
fn the_run_allowance_exceeds_the_group_allowance() -> Result<(), String> {
    let config = parse(CONFIG)?;
    let group = allowance_of(group_override(&config)?)?;
    let run = config
        .get("profile")
        .and_then(|profile| profile.get("default"))
        .and_then(|default| default.get("global-timeout"))
        .and_then(Value::as_str)
        .ok_or_else(|| "the default profile must declare a `global-timeout`".to_owned())?;
    let run = parse_duration(run)?;

    assert!(
        run > group,
        "the run allows {run:?}, which must exceed the group's {group:?}",
    );
    Ok(())
}

/// A configuration that lost the group's limit must be detected, so the checks
/// above cannot pass by reading nothing.
#[test]
fn a_group_without_a_limit_is_rejected() -> Result<(), String> {
    let mut config = parse(CONFIG)?;
    let group = config
        .get_mut("test-groups")
        .and_then(|groups| groups.get_mut(GROUP))
        .ok_or_else(|| format!("the real configuration must declare `{GROUP}`"))?;
    let table = group
        .as_table_mut()
        .ok_or_else(|| "the group must be a table".to_owned())?;
    table.remove("max-threads");

    assert!(
        group_max_threads(&config).is_err(),
        "a group without `max-threads` must be rejected, not defaulted",
    );
    Ok(())
}

/// The filter must name binaries exactly, so a suite whose name merely contains
/// a fixture suite's name does not join the serial group by accident.
#[test]
fn the_filter_names_each_binary_exactly() -> Result<(), String> {
    let config = parse(CONFIG)?;
    let filter = filter_of(group_override(&config)?)?;

    assert!(
        !filter.contains("binary(theorem_file_macro_bdd)"),
        "an unanchored `binary(...)` filter would also match look-alike names, got: {filter}",
    );
    Ok(())
}

#[test]
fn the_duration_parser_rejects_a_bare_number() {
    assert!(
        parse_duration("60").is_err(),
        "a duration without a unit must not be accepted",
    );
    assert_eq!(
        parse_duration("45m"),
        Ok(Duration::from_secs(2_700)),
        "a minute suffix must be understood",
    );
    assert_eq!(
        parse_duration("60s"),
        Ok(Duration::from_secs(60)),
        "a second suffix must be understood",
    );
}
