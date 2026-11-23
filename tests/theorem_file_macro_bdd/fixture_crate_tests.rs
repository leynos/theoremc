//! Tests for the manifest a theorem-file macro fixture uses.
//!
//! The manifest is what makes the fixture reuse a warm target: it has to be its
//! own workspace root, and it has to name the checkout's build-support crate by a
//! path that does not change between fixtures.

use camino::Utf8Path;

use super::fixture_cargo_toml_for;

#[test]
fn the_manifest_is_its_own_workspace_root() {
    let manifest = fixture_cargo_toml_for(Utf8Path::new("/checkout"));

    assert!(
        manifest.contains("\n[workspace]\n"),
        "a fixture inside the repository must not join the repository workspace; got:\n{manifest}",
    );
}

#[test]
fn the_manifest_names_the_checkout_build_support_by_path() {
    let manifest = fixture_cargo_toml_for(Utf8Path::new("/checkout"));

    assert!(
        manifest.contains("path = \"/checkout/crates/theoremc-build-support\""),
        "the build-support dependency must point at the checkout; got:\n{manifest}",
    );
}

#[test]
fn the_manifest_names_the_checkout_root_by_path() {
    let manifest = fixture_cargo_toml_for(Utf8Path::new("/checkout"));

    assert!(
        manifest.contains("theoremc = { path = \"/checkout\", features = [\"test-support\"] }"),
        "the theorem crate must come from the checkout; got:\n{manifest}",
    );
}

#[test]
fn the_manifest_normalizes_windows_path_separators() {
    let manifest = fixture_cargo_toml_for(Utf8Path::new(r"C:\Users\user\projects\theoremc"));

    assert!(
        !manifest.contains('\\'),
        "a manifest must not carry backslashes; got:\n{manifest}",
    );
    assert!(
        manifest.contains("path = \"C:/Users/user/projects/theoremc\""),
        "the path must be normalised to forward slashes; got:\n{manifest}",
    );
}

#[test]
fn the_manifest_escapes_quotes_in_the_checkout_path() {
    let manifest = fixture_cargo_toml_for(Utf8Path::new("/home/user/project's/\"theoremc\""));

    assert!(
        manifest.contains("path = \"/home/user/project's/\\\"theoremc\\\"\""),
        "a quote in the checkout path must be escaped for TOML; got:\n{manifest}",
    );
}
