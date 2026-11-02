//! Regression tests for adapting fixture manifests to standalone crates.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::super::fixture_manifest::{canonical_manifest_path, defines_workspace_table};
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

#[test]
fn recognises_existing_workspace_tables() {
    assert!(defines_workspace_table(
        "[package]\nname = \"fixture\"\n\n[workspace]\n"
    ));
    assert!(defines_workspace_table(
        "[workspace.package]\nversion = \"0.1.0\"\n"
    ));
    assert!(!defines_workspace_table("[package]\nname = \"fixture\"\n"));
}

#[test]
fn canonicalizes_manifest_paths_containing_lexical_components() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = Utf8Path::from_path(temp.path())
        .ok_or_else(|| "temporary directory path is not UTF-8".to_owned())?;
    let source = root.join("nested");
    Dir::create_ambient_dir_all(&source, ambient_authority()).map_err(|error| error.to_string())?;
    let dir =
        Dir::open_ambient_dir(&source, ambient_authority()).map_err(|error| error.to_string())?;
    dir.write("Cargo.toml", "[package]\nname = \"fixture\"\n")
        .map_err(|error| error.to_string())?;

    let lexical = root.join("nested").join("..").join("nested");
    let canonical = canonical_manifest_path(&lexical)?;
    let expected = source
        .join("Cargo.toml")
        .canonicalize_utf8()
        .map_err(|error| error.to_string())?;

    if canonical != expected {
        return Err(format!(
            "expected canonical manifest path {expected}, got {canonical}"
        ));
    }
    Ok(())
}
