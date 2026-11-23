//! Regression tests for fixture package naming and suffix reservation.

use std::collections::HashSet;
use std::sync::{Arc, Barrier};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use proptest::prelude::*;

use super::{
    Claim, Reservations, first_unused_suffix, suffix_is_taken, unique_suffix,
    with_unique_package_name,
};

/// Opens a scratch directory as the shared target a reservation lives in.
fn scratch_target() -> (tempfile::TempDir, camino::Utf8PathBuf) {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path())
        .expect("temp dir path is UTF-8")
        .to_path_buf();
    (temp, root)
}

/// Runs `count` allocators that all offer the same `offered` names, starting
/// together on a barrier, and returns the suffixes they reserved.
fn race_allocators(target: &Utf8Path, count: usize, offered: Vec<String>) -> HashSet<String> {
    let barrier = Arc::new(Barrier::new(count));
    let mut workers = Vec::with_capacity(count);
    for _ in 0..count {
        let barrier = Arc::clone(&barrier);
        let target = target.to_path_buf();
        let offered = offered.clone();
        workers.push(std::thread::spawn(move || {
            let reservations = Reservations::open(&target).expect("open the reservation root");
            let mut candidates = offered.into_iter().chain(std::iter::repeat_with(|| {
                format!("spare-{}", unique_suffix())
            }));
            barrier.wait();
            first_unused_suffix(
                || candidates.next().expect("an endless candidate stream"),
                |candidate| reservations.claim(candidate),
            )
            .expect("every allocator must eventually reserve a name")
        }));
    }
    workers
        .into_iter()
        .map(|worker| worker.join().expect("the allocator thread must not panic"))
        .collect()
}

#[test]
fn renames_only_the_package_name() {
    let manifest = concat!(
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n\n",
        "[dependencies]\nname = { path = \"other\" }\n",
        "[dependencies.renamed]\nname = \"not-the-package\"\n",
    );

    let (renamed, name) =
        with_unique_package_name(manifest, "abc_1").expect("the manifest names its package");

    assert_eq!(
        name, "fx_abc_1",
        "the fixture package must be renamed uniquely"
    );
    assert!(
        renamed.contains("[package]\nname = \"fx_abc_1\"\nversion = \"0.1.0\""),
        "only the package name must change, got:\n{renamed}",
    );
    assert!(
        renamed.contains("name = { path = \"other\" }"),
        "dependency keys must survive, got:\n{renamed}",
    );
    assert!(
        renamed.contains("name = \"not-the-package\""),
        "dependency table names must survive, got:\n{renamed}",
    );
}

#[test]
fn renames_the_first_package_name_when_two_appear() {
    let manifest = "[package]\nname = \"a\"\nname = \"b\"\n";

    let (renamed, name) =
        with_unique_package_name(manifest, "s").expect("the manifest names its package");

    assert_eq!(name, "fx_s", "the first package name must be rewritten");
    assert!(
        renamed.contains("name = \"b\""),
        "a later duplicate must be left alone, got:\n{renamed}",
    );
}

#[test]
fn rejects_a_manifest_that_names_no_package() {
    let result = with_unique_package_name("[dependencies]\nname = \"x\"\n", "s");

    assert_eq!(
        result,
        Err("fixture manifest must name its package".to_owned()),
        "a manifest without a [package] name must be rejected",
    );
}

#[test]
fn two_fixtures_never_share_a_suffix() {
    assert_ne!(
        unique_suffix(),
        unique_suffix(),
        "two suffixes in one process must differ by sequence number",
    );
}

#[test]
fn a_thousand_suffixes_in_one_process_are_all_distinct() {
    let suffixes: HashSet<String> = (0..1000).map(|_| unique_suffix()).collect();

    assert_eq!(
        suffixes.len(),
        1000,
        "a thousand suffixes must all be distinct"
    );
}

#[test]
fn the_suffix_carries_the_process_id_so_concurrent_runs_cannot_collide() {
    let suffix = unique_suffix();

    assert!(
        suffix.starts_with(&format!("{:x}_", std::process::id())),
        "{suffix} must begin with this process's id",
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
        |suffix| {
            Ok(if suffix.ends_with("taken") {
                Claim::Taken
            } else {
                Claim::Reserved
            })
        },
    );

    assert_eq!(
        chosen,
        Ok("free".to_owned()),
        "a claimed suffix must be passed over for the next candidate",
    );
}

#[test]
fn a_scan_that_fails_stops_the_allocation_instead_of_calling_the_name_free() {
    let chosen = first_unused_suffix(|| "any".to_owned(), |_| Err("cannot read".to_owned()));

    assert_eq!(
        chosen,
        Err("cannot read".to_owned()),
        "a scan that cannot run must not report a name free",
    );
}

#[test]
fn a_pre_existing_reservation_marker_is_skipped() {
    let (_temp, root) = scratch_target();
    let reservations = Reservations::open(&root).expect("open the reservation root");
    assert_eq!(
        reservations.claim("already").expect("claim the first name"),
        Claim::Reserved,
        "the first claim must succeed",
    );

    let chosen = first_unused_suffix(
        {
            let mut offered = ["already", "fresh"].into_iter().map(str::to_owned);
            move || offered.next().expect("a free suffix is offered")
        },
        |candidate| reservations.claim(candidate),
    );

    assert_eq!(
        chosen,
        Ok("fresh".to_owned()),
        "a suffix another allocator holds must be skipped",
    );
}

#[test]
fn competing_allocators_never_receive_the_same_suffix() {
    let (_temp, root) = scratch_target();
    // Every worker offers the same names first, so a non-atomic check would
    // hand the same suffix to more than one of them.
    let contested: Vec<String> = (0..8).map(|index| format!("contested{index}")).collect();

    let reserved = race_allocators(&root, 6, contested);

    assert_eq!(
        reserved.len(),
        6,
        "six allocators must reserve six distinct suffixes, got {reserved:?}",
    );
}

#[test]
fn leftover_build_directories_make_a_suffix_taken_and_only_that_one() {
    let (_temp, root) = scratch_target();
    let dir = Dir::open_ambient_dir(&root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug/build/fx_ab_1_0001-0123abcd/out")
        .expect("leave a stale build directory");

    assert_eq!(
        suffix_is_taken(&root, "ab_1_0001"),
        Ok(true),
        "a stale classic-layout build directory must take the suffix",
    );
    assert_eq!(
        suffix_is_taken(&root, "ab_1_000"),
        Ok(false),
        "a prefix of a suffix is not the suffix",
    );
    assert_eq!(
        suffix_is_taken(&root, "ab_2_0001"),
        Ok(false),
        "an unrelated suffix must remain free",
    );
    assert_eq!(
        suffix_is_taken(&root.join("missing"), "ab_1_0001"),
        Ok(false),
        "a target with no build directory holds nothing",
    );
}

#[test]
fn a_stale_directory_in_the_nested_layout_takes_the_suffix() {
    let (_temp, root) = scratch_target();
    let dir = Dir::open_ambient_dir(&root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug/build/fx_ab_1_0001/0123abcd/out")
        .expect("leave a stale nested build directory");

    assert_eq!(
        suffix_is_taken(&root, "ab_1_0001"),
        Ok(true),
        "a stale nested-layout build directory must take the suffix",
    );
    assert_eq!(
        suffix_is_taken(&root, "ab_1_000"),
        Ok(false),
        "a prefix of a suffix is not the suffix",
    );
    assert_eq!(
        suffix_is_taken(&root, "ab_2_0001"),
        Ok(false),
        "an unrelated suffix must remain free",
    );
}

#[test]
fn a_build_directory_that_cannot_be_listed_is_an_error_not_an_absence() {
    let (_temp, root) = scratch_target();
    let dir = Dir::open_ambient_dir(&root, ambient_authority()).expect("open temp dir");
    dir.create_dir_all("debug").expect("create debug");
    // A file where the build directory belongs: it exists, but is not a directory.
    dir.write("debug/build", "not a directory")
        .expect("write file");

    assert!(
        suffix_is_taken(&root, "ab_1_0001").is_err(),
        "an unreadable build directory must fail the scan, not report free",
    );
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
        let taken: HashSet<String> =
            stale.iter().map(|value| format!("s{value}")).collect();

        let chosen = first_unused_suffix(
            || format!("s{}", sequence.next().expect("an endless sequence")),
            |candidate| {
                Ok(if taken.contains(candidate) { Claim::Taken } else { Claim::Reserved })
            },
        )
        .expect("the model scan cannot fail");

        prop_assert!(!taken.contains(&chosen), "{chosen} is stale");
    }
}
