//! Regression tests for fixture package naming.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use proptest::prelude::*;

use super::{
    FixtureLock, first_unused_suffix, resolved_cargo_home, seeded_lock, suffix_is_taken,
    unique_suffix, with_unique_package_name,
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

#[test]
fn a_seeded_lock_renames_only_the_template_root() {
    let seeded = seeded_lock(FixtureLock::BuildSupport, "fx_ab_1_0001")
        .expect("the build-support template names its root");

    assert!(
        seeded.contains("name = \"fx_ab_1_0001\"\nversion = \"0.1.0\""),
        "the seeded lock must name the generated package:\n{seeded}",
    );
    assert!(
        !seeded.contains("name = \"build_support_fixture\""),
        "the template's own root name must not survive",
    );
    let registry = seeded.matches("source = \"registry+").count();
    let template = super::BUILD_SUPPORT_TEMPLATE
        .lock
        .matches("source = \"registry+")
        .count();
    assert_eq!(
        registry, template,
        "a rename must not add or remove a registry entry",
    );
}

#[test]
fn the_dependency_free_seed_names_only_the_fixture() {
    let seeded = seeded_lock(FixtureLock::DependencyFree, "fx_ab_1_0001")
        .expect("the dependency-free template names its root");

    assert_eq!(
        seeded.matches("[[package]]").count(),
        1,
        "a fixture with no dependencies resolves nothing but itself:\n{seeded}",
    );
    assert!(seeded.contains("name = \"fx_ab_1_0001\""));
}

#[test]
fn each_template_seeds_a_distinct_graph() {
    let support = seeded_lock(FixtureLock::BuildSupport, "fx_a").expect("build-support seed");
    let macros = seeded_lock(FixtureLock::TheoremFileMacro, "fx_a").expect("macro seed");

    assert_ne!(
        support, macros,
        "the two templates describe different dependency graphs",
    );
}

#[test]
fn a_template_whose_root_is_missing_is_rejected() {
    let result = super::with_renamed_root(
        "[[package]]\nname = \"other\"\nversion = \"0.1.0\"\n",
        "fixture_template",
        "fx_a",
    );

    assert_eq!(
        result,
        Err("fixture lock names no root package \"fixture_template\"".to_owned()),
        "a template without its root cannot be seeded",
    );
}

#[test]
fn a_template_naming_its_root_twice_is_rejected() {
    let lock = concat!(
        "[[package]]\nname = \"fixture_template\"\nversion = \"0.1.0\"\n",
        "\n",
        "[[package]]\nname = \"fixture_template\"\nversion = \"0.2.0\"\n",
    );

    let result = super::with_renamed_root(lock, "fixture_template", "fx_a");

    assert!(
        matches!(&result, Err(reason) if reason.contains("more than once")),
        "an ambiguous root must not be renamed, got {result:?}",
    );
}

#[test]
fn a_registry_package_sharing_the_root_name_is_not_renamed() {
    let lock = concat!(
        "[[package]]\nname = \"fixture_template\"\nversion = \"0.1.0\"\n",
        "source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
        "\n",
        "[[package]]\nname = \"fixture_template\"\nversion = \"0.2.0\"\n",
    );

    let seeded = super::with_renamed_root(lock, "fixture_template", "fx_a")
        .expect("the source-less entry is the root");

    assert!(
        seeded.contains(
            "name = \"fixture_template\"\nversion = \"0.1.0\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\""
        ),
        "a registry package keeps its name:\n{seeded}",
    );
    assert!(
        seeded.contains("name = \"fx_a\"\nversion = \"0.2.0\""),
        "the source-less entry is the root:\n{seeded}",
    );
}

#[test]
fn the_fixture_home_falls_back_beside_the_shared_target() {
    let home = resolved_cargo_home(None);

    assert!(
        home.as_str()
            .ends_with("target/theoremc-fixture-cargo-home"),
        "{home} must sit beside the shared target directory",
    );
}

#[test]
fn an_exported_fixture_home_wins() {
    let home = resolved_cargo_home(Some("/elsewhere/fixture-home"));

    assert_eq!(
        home,
        Utf8Path::new("/elsewhere/fixture-home"),
        "the fetch step exports the home it populated",
    );
}

/// A relative configured home must come back absolute.
///
/// Cargo resolves a relative `CARGO_HOME` against its own working directory,
/// and every fixture build runs in the fixture's temporary directory. A
/// relative home would therefore send each fixture to a different, empty
/// cache, and every frozen build would fail. The value is injected rather than
/// exported, so the test mutates no process state.
#[test]
fn a_relative_fixture_home_is_resolved_to_an_absolute_path() -> Result<(), String> {
    // A unique directory in the working directory, so a relative name can be
    // handed over without two runs sharing one path. Keeping the `TempDir` in
    // scope removes the directory when the test ends.
    let temp = tempfile::Builder::new()
        .prefix("relative-fixture-home-")
        .tempdir_in(".")
        .map_err(|error| error.to_string())?;
    let name = temp
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "temp dir name is not valid UTF-8".to_owned())?
        .to_owned();

    let home = super::prepared_cargo_home(Some(&name))?;
    let expected_name = Utf8Path::new(&name);

    if home.is_absolute() && home.file_name() == Some(expected_name.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "{home} must be absolute and keep the configured name \
             so a build cannot resolve it from its own directory"
        ))
    }
}

/// An absolute configured home is kept as it is.
#[test]
fn an_absolute_fixture_home_is_unchanged() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    // Compare against the canonical spelling: the helper canonicalises, and a
    // temporary root containing a symlink would otherwise compare unequal.
    let root = Utf8Path::from_path(temp.path())
        .ok_or_else(|| "temp dir path is not valid UTF-8".to_owned())?
        .canonicalize_utf8()
        .map_err(|error| error.to_string())?;
    let configured = root.join("fixture-home");

    let home = super::prepared_cargo_home(Some(configured.as_str()))?;

    if home == configured {
        Ok(())
    } else {
        Err(format!(
            "an absolute configuration must survive unchanged, expected {configured}, got {home}"
        ))
    }
}
