//! Regressions for persistent dependency caching with isolated fixture sources.

use camino::Utf8Path;
use test_helpers::{FixtureCrate, fixture_target_dir};

#[test]
fn fixtures_share_dependencies_but_keep_sources_and_generated_output_separate() -> Result<(), String>
{
    let manifest = "[package]\nname = \"cache_contract\"\nversion = \"0.1.0\"\n";
    let first = FixtureCrate::new(manifest, "//! First source.\n")?;
    let second = FixtureCrate::new(manifest, "//! Second source.\n")?;
    first.write(
        Utf8Path::new("theorems/first.theorem"),
        "first fixture only",
    )?;
    [
        (
            first.manifest_dir() != second.manifest_dir(),
            "fixture source directories must be distinct",
        ),
        (
            first.manifest_dir().parent() == second.manifest_dir().parent(),
            "fixture sources must share their persistent source parent",
        ),
        (
            first.manifest_dir() != fixture_target_dir(),
            "fixture sources must be separate from the dependency target",
        ),
        (
            first.manifest_dir().ancestors().nth(2) == Some(fixture_target_dir().as_path()),
            "fixture packages must share the workspace Cargo target",
        ),
        (
            first
                .manifest_dir()
                .parent()
                .is_some_and(|path| path.file_name() == Some("fixture-sources")),
            "fixture sources must live under fixture-sources",
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
    ]
    .into_iter()
    .try_for_each(|(condition, message)| {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    })
}
