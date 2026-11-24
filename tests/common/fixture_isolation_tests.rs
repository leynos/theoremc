//! Regression tests for fixture package naming.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use proptest::prelude::*;

use super::{
    first_unused_suffix, fixture_target_dir, suffix_is_taken, unique_suffix,
    with_unique_package_name,
};

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
        |suffix| Ok(suffix.ends_with("taken")),
    );

    assert_eq!(chosen, Ok("free".to_owned()));
}

#[test]
fn a_scan_that_fails_stops_the_allocation_instead_of_calling_the_name_free() {
    let chosen = first_unused_suffix(|| "any".to_owned(), |_| Err("cannot read".to_owned()));

    assert_eq!(chosen, Err("cannot read".to_owned()));
}

#[test]
fn leftover_build_directories_make_a_suffix_taken_and_only_that_one() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug/build/fx_ab_1_0001-0123abcd/out")
        .expect("leave a stale build directory");

    assert_eq!(suffix_is_taken(root, "ab_1_0001"), Ok(true));
    assert_eq!(
        suffix_is_taken(root, "ab_1_000"),
        Ok(false),
        "a prefix of a suffix is not the suffix"
    );
    assert_eq!(suffix_is_taken(root, "ab_2_0001"), Ok(false));
    assert_eq!(
        suffix_is_taken(&root.join("missing"), "ab_1_0001"),
        Ok(false)
    );
}

#[test]
fn a_stale_directory_in_the_nested_layout_takes_the_suffix() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug/build/fx_ab_1_0001/0123abcd/out")
        .expect("leave a stale nested build directory");

    assert_eq!(suffix_is_taken(root, "ab_1_0001"), Ok(true));
    assert_eq!(suffix_is_taken(root, "ab_1_000"), Ok(false));
    assert_eq!(suffix_is_taken(root, "ab_2_0001"), Ok(false));
}

#[test]
fn a_build_directory_that_cannot_be_listed_is_an_error_not_an_absence() {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug").expect("create debug");
    // A file where the build directory belongs: it exists, but is not a directory.
    dir.write("debug/build", "not a directory")
        .expect("write file");

    assert!(suffix_is_taken(root, "ab_1_0001").is_err());
}

proptest! {
    /// Whatever stale names exist and however the clock and process ids
    /// repeat, the suffix chosen is never a stale one.
    #[test]
    fn a_stale_suffix_is_never_chosen_across_pid_reuse_and_clock_wrap(
        offered in proptest::collection::vec(0u16..40, 1..60),
        stale in proptest::collection::hash_set(0u16..40, 0..39),
    ) {
        let free = (0u16..40)
            .find(|candidate| !stale.contains(candidate))
            .expect("a free name");
        let mut sequence = offered
            .into_iter()
            .map(u32::from)
            .chain(std::iter::repeat(u32::from(free)));
        let taken: std::collections::HashSet<String> =
            stale.iter().map(|value| format!("s{value}")).collect();

        let chosen = first_unused_suffix(
            || format!("s{}", sequence.next().expect("an endless sequence")),
            |candidate| Ok(taken.contains(candidate)),
        )
        .expect("the model scan cannot fail");

        prop_assert!(!taken.contains(&chosen), "{chosen} is stale");
    }
}

/// The shared fixture directory inside an absolute injected base.
#[test]
fn an_absolute_base_selects_a_canonical_shared_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    let base = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");

    let target = fixture_target_dir(Some(base)).expect("an absolute base is usable");

    assert_eq!(
        target,
        base.canonicalize_utf8()
            .expect("the temporary directory canonicalizes")
            .join("theoremc-fixture-builds"),
        "the shared directory must sit inside the injected base and be canonical"
    );
    assert!(target.is_dir(), "{target} must be created");
}

/// A base spelled with `.` and `..` segments is canonicalized before it is
/// returned, so child Cargo invocations receive one absolute path however the
/// caller spelled it. A relative base travels the same canonicalization step.
#[test]
fn a_non_canonical_base_is_canonicalized_before_it_is_returned() {
    let temp = tempfile::tempdir().expect("temp dir");
    let base = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let spelled = base.join("nested").join("..");

    let target = fixture_target_dir(Some(&spelled)).expect("a non-canonical base is usable");

    assert_eq!(
        target,
        base.canonicalize_utf8()
            .expect("the temporary directory canonicalizes")
            .join("theoremc-fixture-builds"),
        "canonicalization must remove the `..` segment before the path is returned"
    );
}

/// With no injected base, the fixture target falls inside the repository's
/// `target`, which is what keeps it warm and removable by `cargo clean`.
#[test]
fn an_absent_base_falls_back_to_the_repository_target() {
    let target = fixture_target_dir(None).expect("the repository target is usable");

    assert!(
        target.ends_with("theoremc-fixture-builds"),
        "{target} must end with the shared directory name"
    );
    assert!(
        target.is_absolute(),
        "{target} must be absolute so child commands agree on one location"
    );
}
