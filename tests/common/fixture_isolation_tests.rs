//! Regression tests for fixture package naming.

use super::{unique_suffix, with_unique_package_name};

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
fn a_fixture_name_stays_short_enough_for_the_files_tools_derive_from_it() {
    let (_, name) = with_unique_package_name("[package]\nname = \"x\"\n", &unique_suffix())
        .expect("the manifest names its package");

    assert!(name.len() <= 20, "{name} is too long to be a safe prefix");
}
