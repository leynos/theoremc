//! Shared test helpers for theorem-file macro expansion tests.

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use tempfile::TempDir;
pub(super) use test_helpers::set_cargo_manifest_dir_for_test;
use theoremc_core::mangle::{mangle_module_path, mangle_theorem_harness};

use super::expand_theorem_file_at;

pub(super) struct TheoremSpec<'a> {
    pub(super) name: &'a str,
    pub(super) about: &'a str,
}

pub(super) struct TheoremFixture(pub(super) String);

impl TheoremFixture {
    fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) fn write_fixture(
    fixture_dir: &Utf8Path,
    path: &Utf8Path,
    contents: &TheoremFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_root = Dir::open_ambient_dir(fixture_dir, ambient_authority())?;
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fixture_root.create_dir_all(parent)?;
    }
    fixture_root.write(path.as_str(), contents.as_str())?;
    Ok(())
}

pub(super) fn temp_fixture_dir()
-> Result<(TempDir, camino::Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let fixture_dir = Utf8Path::from_path(temp_dir.path())
        .ok_or_else(|| std::io::Error::other("temp dir path must be UTF-8"))?
        .to_path_buf();
    Ok((temp_dir, fixture_dir))
}

pub(super) fn expand_fixture(
    path: &Utf8Path,
    contents: &TheoremFixture,
) -> Result<String, Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, contents)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());
    let tokens = expand_theorem_file_at(&fixture_dir, &path_literal)?;
    Ok(normalize(&tokens.to_string()))
}

pub(super) fn normalize(tokens: &str) -> String {
    tokens.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// Replaces non-deterministic mangling hash suffixes with a stable placeholder.
pub(super) fn redact_hashes(s: &str) -> String {
    // Mangle suffixes are `__h` plus exactly 12 lowercase hex characters.
    let mut result = String::with_capacity(s.len());
    let marker = "__h";
    let mut rest = s;
    while let Some(pos) = rest.find(marker) {
        result.push_str(&rest[..pos]);
        let after_marker = &rest[pos + marker.len()..];
        let suffix = &after_marker[..after_marker.len().min(12)];
        if suffix.len() == 12 && suffix.bytes().all(|b| b.is_ascii_hexdigit()) {
            result.push_str("__hXXXXXXXXXXXX");
            rest = &after_marker[12..];
        } else {
            result.push_str(marker);
            rest = after_marker;
        }
    }
    result.push_str(rest);
    result
}

pub(super) fn expected_expansion(path: &Utf8Path, theorems: &[&str]) -> String {
    let module_name = mangle_module_path(path.as_str()).module_name().to_owned();
    let harnesses: Vec<String> = theorems
        .iter()
        .map(|theorem| {
            mangle_theorem_harness(path.as_str(), theorem)
                .identifier()
                .to_owned()
        })
        .collect();
    let harness_defs = harnesses
        .iter()
        .map(|harness| format!("pub(crate) fn {harness} () {{ }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let harness_refs = harnesses
        .iter()
        .map(|harness| format!("kani :: {harness}"))
        .collect::<Vec<_>>()
        .join(" , ");

    normalize(&format!(
        "mod {module_name} {{
            const _: & str = include_str! ( concat! ( env! (\"CARGO_MANIFEST_DIR\") , \"/\" , \"{path}\" ) ) ;
            pub(super) mod kani {{ {harness_defs} }}
            const _: [fn(); {}] = [ {} ] ;
        }}",
        harnesses.len(),
        harness_refs,
    ))
}

pub(super) fn make_single_theorem_fixture(spec: &TheoremSpec<'_>) -> TheoremFixture {
    TheoremFixture(format!(
        concat!(
            "Theorem: {name}\n",
            "About: {about}\n",
            "Witness:\n",
            "  - cover: \"true\"\n",
            "    because: \"reachable\"\n",
            "Prove:\n",
            "  - assert: \"true\"\n",
            "    because: \"trivial\"\n",
            "Evidence:\n",
            "  kani:\n",
            "    unwind: 1\n",
            "    expect: SUCCESS\n",
        ),
        name = spec.name,
        about = spec.about,
    ))
}

pub(super) fn assert_expansion_matches(
    path: &Utf8Path,
    fixture: &TheoremFixture,
    expected_theorems: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = expand_fixture(path, fixture)?;
    let expected = expected_expansion(path, expected_theorems);
    assert_eq!(actual, expected);
    Ok(())
}

pub(super) fn assert_expansion_is_stable(
    path: &Utf8Path,
    fixture: &TheoremFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    let first = expand_fixture(path, fixture)?;
    let second = expand_fixture(path, fixture)?;
    assert_eq!(first, second);
    Ok(())
}

pub(super) fn assert_single_theorem_expansion(
    path: &Utf8Path,
    spec: &TheoremSpec<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(spec);
    assert_expansion_matches(path, &fixture, &[spec.name])
}

pub(super) fn expansion_error_message(
    manifest_dir: &Utf8Path,
    path: &Utf8Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());
    let error = expand_theorem_file_at(manifest_dir, &path_literal)
        .err()
        .ok_or_else(|| std::io::Error::other("fixture unexpectedly expanded successfully"))?;
    Ok(normalize(
        &error
            .to_compile_error(proc_macro2::Span::call_site())
            .to_string(),
    ))
}
