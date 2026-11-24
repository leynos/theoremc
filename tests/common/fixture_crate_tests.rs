//! Regression tests for shared fixture crate layout.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{package_directories, read_suite};

/// A shared `target/debug/build` holding two builds of `fx_ab`, one of the
/// similarly named `fx_abc`, and a generated suite for the first `fx_ab` only.
fn shared_build_directory() -> (tempfile::TempDir, Dir) {
    let temp = tempfile::tempdir().expect("temp dir");
    let root = Utf8Path::from_path(temp.path()).expect("temp dir path is UTF-8");
    let dir = Dir::open_ambient_dir(root, ambient_authority()).expect("open temp dir");
    for name in ["fx_ab-111/out", "fx_ab-333", "fx_abc-222/out"] {
        dir.create_dir_all(name).expect("create build directory");
    }
    dir.write("fx_ab-111/out/theorem_suite.rs", "// suite")
        .expect("write suite");
    (temp, dir)
}

#[test]
fn a_package_claims_only_directories_named_after_it() {
    let (_temp, dir) = shared_build_directory();

    let names = package_directories(&dir, "fx_ab").expect("list build directories");

    assert_eq!(
        names,
        ["fx_ab-111", "fx_ab-333"],
        "fx_abc-222 is another package"
    );
}

#[test]
fn a_package_with_no_build_directory_claims_none() {
    let (_temp, dir) = shared_build_directory();

    assert_eq!(package_directories(&dir, "fx_zz"), Ok(Vec::new()));
}

#[test]
fn a_generated_suite_is_read_from_its_build_directory() {
    let (_temp, dir) = shared_build_directory();

    assert_eq!(
        read_suite(&dir, "fx_ab-111"),
        Ok(Some("// suite".to_owned()))
    );
}

#[test]
fn a_build_directory_without_a_suite_reads_as_none() {
    let (_temp, dir) = shared_build_directory();

    assert_eq!(read_suite(&dir, "fx_ab-333"), Ok(None));
    assert_eq!(read_suite(&dir, "fx_abc-222"), Ok(None));
}
