//! Regression tests for fixture package naming.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{first_unused_suffix, suffix_is_taken, unique_suffix, with_unique_package_name};

#[test]
fn renames_only_the_package_name() {
    let manifest = concat!(
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n",
        "[dependencies]\nname = { path = \"other\" }\n",
        "[dependencies.renamed]\nname = \"not-the-package\"\n",
    );

    let (renamed, name) =
        with_unique_package_name(manifest, "abc_1").expect("the manifest names its package");

    assert_eq!(name, "fx_abc_1");
    assert!(renamed.contains("[package]\nname = \"fx_abc_1\"\nversion = \"0.1.0\""));
    assert!(renamed.contains("name = { path = \"other\" }"));
    assert!(renamed.contains("name = \"not-the-package\""));
}

#[test]
fn renames_the_first_package_name_when_two_appear() {
    let manifest = "[package]\nname = \"a\"\nname = \"b\"\n";

    let (renamed, name) =
        with_unique_package_name(manifest, "s").expect("the manifest names its package");

    assert_eq!(name, "fx_s");
    assert!(renamed.contains("name = \"b\""));
}

#[test]
fn rejects_a_manifest_that_names_no_package() {
    let result = with_unique_package_name("[dependencies]\nname = \"x\"\n", "s");

    assert_eq!(
        result,
        Err("fixture manifest must name its package".to_owned())
    );
}

#[test]
fn two_fixtures_never_share_a_suffix() {
    assert_ne!(unique_suffix(), unique_suffix());
}

#[test]
fn a_thousand_suffixes_in_one_process_are_all_distinct() {
    let suffixes: std::collections::HashSet<String> = (0..1000).map(|_| unique_suffix()).collect();

    assert_eq!(suffixes.len(), 1000);
}

#[test]
fn the_suffix_carries_the_process_id_so_concurrent_runs_cannot_collide() {
    let suffix = unique_suffix();

    assert!(
        suffix.starts_with(&format!("{:x}_", std::process::id())),
        "{suffix} must begin with this process's id"
    );
}

#[test]
fn a_fixture_name_stays_short_enough_for_the_files_tools_derive_from_it() {
    let (_, name) = with_unique_package_name("[package]\nname = \"x\"\n", &unique_suffix())
        .expect("the manifest names its package");

    assert!(name.len() <= 20, "{name} is too long to be a safe prefix");
}

#[test]
fn a_suffix_a_stale_build_directory_already_uses_is_skipped() {
    let mut offered = ["taken", "also_taken", "free"]
        .into_iter()
        .map(str::to_owned);

    let chosen = first_unused_suffix(
        || offered.next().expect("a free suffix is offered"),
        |suffix| suffix.ends_with("taken"),
    );

    assert_eq!(chosen, "free");
}

#[test]
fn leftover_build_directories_make_a_suffix_taken_and_only_that_one() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug/build/fx_ab_1_0001-0123abcd/out")
        .expect("leave a stale build directory");

    assert!(suffix_is_taken(root, "ab_1_0001"));
    assert!(
        !suffix_is_taken(root, "ab_1_000"),
        "a prefix of a suffix is not the suffix"
    );
    assert!(!suffix_is_taken(root, "ab_2_0001"));
    assert!(!suffix_is_taken(&root.join("missing"), "ab_1_0001"));
}
