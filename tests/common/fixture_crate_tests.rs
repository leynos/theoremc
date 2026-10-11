//! Regression tests for adapting fixture manifests to standalone crates.

use super::{BUILD_SUPPORT_CARGO_TOML, fixture_build_support_cargo_toml};

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
