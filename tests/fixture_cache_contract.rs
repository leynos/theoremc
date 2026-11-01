//! Regressions for persistent dependency caching with isolated fixture sources.

use camino::Utf8Path;
use test_helpers::FixtureCrate;

#[test]
fn fixtures_share_a_target_but_keep_sources_and_generated_output_separate() -> Result<(), String> {
    let manifest = "[package]\nname = \"cache_contract\"\nversion = \"0.1.0\"\n";
    let first = FixtureCrate::new(manifest, "//! First source.\n")?;
    let second = FixtureCrate::new(manifest, "//! Second source.\n")?;
    first.write(
        Utf8Path::new("theorems/first.theorem"),
        "first fixture only",
    )?;

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