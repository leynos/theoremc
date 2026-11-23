//! Deterministic tests for the three fixture failure shapes.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{FixtureIdentity, build_log, generated_suite_contents};

/// Describes a fixture rooted in `root` that builds into `target`, so the tests
/// can point the helpers at a scratch directory tree.
fn identity<'a>(
    package_name: &'a str,
    root: &'a Utf8Path,
    target: &'a Utf8Path,
) -> FixtureIdentity<'a> {
    FixtureIdentity {
        package_name,
        root,
        target_dir: target,
    }
}

fn utf8_path(temp: &tempfile::TempDir) -> camino::Utf8PathBuf {
    Utf8Path::from_path(temp.path())
        .expect("a temporary directory path should be UTF-8")
        .to_path_buf()
}

#[test]
fn an_absent_build_directory_names_its_full_path() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let base = utf8_path(&scratch);
    let fixture = identity("fx_ab_1_0001", &base, &base);

    let error = generated_suite_contents(&fixture)
        .expect_err("a target with no build directory cannot yield a suite");

    let expected = base.join("debug/build");
    assert!(
        error.contains(&expected.to_string()),
        "the error must name the full build directory {expected}: {error}",
    );
    assert!(
        error.contains("fx_ab_1_0001"),
        "the error must name the fixture package: {error}",
    );
}

#[test]
fn a_build_root_without_a_suite_names_the_package_target_and_candidates() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let base = utf8_path(&scratch);
    let dir =
        Dir::open_ambient_dir(&base, ambient_authority()).expect("open the scratch directory");
    dir.create_dir_all("debug/build/fx_ab_1_0001-0123abcd/out")
        .expect("create an empty build output directory");
    dir.create_dir_all("debug/build/fx_ab_1_0001/4567cdef/out")
        .expect("create an empty nested build output directory");
    let fixture = identity("fx_ab_1_0001", &base, &base);

    let error = generated_suite_contents(&fixture)
        .expect_err("build directories without a suite must be reported");

    for fragment in [
        "fx_ab_1_0001",
        "fx_ab_1_0001-0123abcd",
        "fx_ab_1_0001/4567cdef",
        base.as_str(),
    ] {
        assert!(
            error.contains(fragment),
            "the error must name {fragment}: {error}",
        );
    }
}

#[test]
fn a_build_that_cannot_start_names_the_command_and_the_missing_directory() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let base = utf8_path(&scratch);
    let absent_root = base.join("fixture-root-that-does-not-exist");
    let fixture = identity("fx_ab_1_0001", &absent_root, &base);

    let error = match build_log(&fixture) {
        Ok(log) => panic!(
            "a build in a missing root must fail, got log:\n{}",
            log.as_str()
        ),
        Err(error) => error,
    };

    assert!(
        error.contains(&absent_root.to_string()),
        "the error must name the working directory: {error}",
    );
    assert!(
        error.contains("running `cargo build -vv --color never`"),
        "the error must name the command: {error}",
    );
    assert!(
        error.contains("fx_ab_1_0001"),
        "the error must name the fixture package: {error}",
    );
}
