//! Regression tests for adapting fixture manifests to standalone crates.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::super::suite_layout::{package_directories, read_suite};
use super::{BUILD_SUPPORT_CARGO_TOML, fixture_build_support_cargo_toml};

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

#[test]
fn removes_the_repository_workspace_lints_table() {
    let fixture_manifest = fixture_build_support_cargo_toml(BUILD_SUPPORT_CARGO_TOML)
        .expect("repository build-support manifest should have workspace lints");

    assert!(!fixture_manifest.contains("[lints]"));
    assert!(fixture_manifest.contains("[dev-dependencies]"));
}

#[test]
fn preserves_sections_after_workspace_lints() {
    let manifest = concat!(
        "[package]\nname = \"fixture\"\n\n",
        "[lints]\nworkspace = true\n\n",
        "[dependencies]\nexample = \"1.0.0\"\n",
    );

    let fixture_manifest = fixture_build_support_cargo_toml(manifest)
        .expect("fixture manifest should have workspace lints");

    assert!(!fixture_manifest.contains("[lints]"));
    assert!(fixture_manifest.contains("[dependencies]\nexample = \"1.0.0\""));
}

#[test]
fn rejects_a_manifest_without_workspace_lints() {
    let result = fixture_build_support_cargo_toml("[package]\nname = \"fixture\"\n");

    assert_eq!(
        result,
        Err("build-support manifest must contain a [lints] table".to_owned())
    );
}
