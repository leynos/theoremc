//! Deterministic tests for the three fixture failure shapes.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{FixtureIdentity, build_log, generated_suite_contents, spawn_redirected};

/// Describes a fixture rooted in `root` that builds into `target`, so the tests
/// can point the helpers at a scratch directory tree.
fn identity(package_name: &str, root: &Utf8Path, target: &Utf8Path) -> FixtureIdentity {
    FixtureIdentity::new(package_name, root, target)
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

/// A successful build's log must carry what the redirected streams wrote.
///
/// The streams go to files, so `wait_with_output` would return nothing. This
/// pins that regression: a stubbed child writes both streams, and the returned
/// log must contain both.
#[cfg(unix)]
#[test]
fn a_completed_build_returns_the_output_it_redirected_to_files() {
    let scratch = tempfile::tempdir().expect("create a scratch directory");
    let base = utf8_path(&scratch);
    let fixture = identity("fx_ab_1_0001", &base, &base);
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "printf 'out-text'; printf 'err-text' >&2"]);

    let log = spawn_redirected(&fixture, command)
        .expect("the stub command should start")
        .wait()
        .expect("the stub command should succeed and both logs should be readable");

    assert!(
        log.as_str().contains("out-text"),
        "the log must carry the redirected stdout, got:\n{}",
        log.as_str(),
    );
    assert!(
        log.as_str().contains("err-text"),
        "the log must carry the redirected stderr, got:\n{}",
        log.as_str(),
    );
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
