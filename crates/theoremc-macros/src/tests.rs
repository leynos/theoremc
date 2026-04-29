//! Unit tests for deterministic theorem-file macro expansion.

use std::env;
use std::sync::{Mutex, MutexGuard, PoisonError};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use tempfile::TempDir;
use theoremc_core::mangle::{mangle_module_path, mangle_theorem_harness};

use super::{MacroExpansionError, expand_theorem_file_at, manifest_dir_from_env};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard<'a> {
    previous: Option<String>,
    _guard: MutexGuard<'a, ()>,
}

impl Drop for EnvVarGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: tests that mutate `CARGO_MANIFEST_DIR` hold `ENV_LOCK`, so
        // this process-global mutation is serialized within this test module.
        unsafe {
            match &self.previous {
                Some(value) => env::set_var("CARGO_MANIFEST_DIR", value),
                None => env::remove_var("CARGO_MANIFEST_DIR"),
            }
        }
    }
}

fn set_cargo_manifest_dir_for_test(value: Option<&str>) -> EnvVarGuard<'_> {
    let guard = ENV_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let previous = env::var("CARGO_MANIFEST_DIR").ok();
    // SAFETY: `ENV_LOCK` prevents concurrent mutation by tests in this module.
    unsafe {
        match value {
            Some(value) => env::set_var("CARGO_MANIFEST_DIR", value),
            None => env::remove_var("CARGO_MANIFEST_DIR"),
        }
    }
    EnvVarGuard {
        previous,
        _guard: guard,
    }
}

struct TheoremSpec<'a> {
    name: &'a str,
    about: &'a str,
}

struct TheoremFixture(String);

impl TheoremFixture {
    fn as_str(&self) -> &str {
        &self.0
    }
}

fn write_fixture(
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

fn temp_fixture_dir() -> Result<(TempDir, camino::Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let fixture_dir = Utf8Path::from_path(temp_dir.path())
        .ok_or_else(|| std::io::Error::other("temp dir path must be UTF-8"))?
        .to_path_buf();
    Ok((temp_dir, fixture_dir))
}

fn expand_fixture(
    path: &Utf8Path,
    contents: &TheoremFixture,
) -> Result<String, Box<dyn std::error::Error>> {
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, contents)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());
    let tokens = expand_theorem_file_at(&fixture_dir, &path_literal)?;
    Ok(normalize(&tokens.to_string()))
}

fn normalize(tokens: &str) -> String {
    tokens.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// Replaces the non-deterministic 12-hex-character hash suffixes appended by
/// the mangling functions with a stable placeholder, making snapshots
/// deterministic across compilations without collapsing structural whitespace.
fn redact_hashes(s: &str) -> String {
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

fn expected_expansion(path: &Utf8Path, theorems: &[&str]) -> String {
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

fn make_single_theorem_fixture(spec: &TheoremSpec<'_>) -> TheoremFixture {
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

fn assert_expansion_matches(
    path: &Utf8Path,
    fixture: &TheoremFixture,
    expected_theorems: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = expand_fixture(path, fixture)?;
    let expected = expected_expansion(path, expected_theorems);
    assert_eq!(actual, expected);
    Ok(())
}

fn assert_expansion_is_stable(
    path: &Utf8Path,
    fixture: &TheoremFixture,
) -> Result<(), Box<dyn std::error::Error>> {
    let first = expand_fixture(path, fixture)?;
    let second = expand_fixture(path, fixture)?;
    assert_eq!(first, second);
    Ok(())
}

fn assert_single_theorem_expansion(
    path: &Utf8Path,
    spec: &TheoremSpec<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(spec);
    assert_expansion_matches(path, &fixture, &[spec.name])
}

fn expansion_error_message(
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

#[test]
fn single_document_expansion_matches_expected_shape() -> Result<(), Box<dyn std::error::Error>> {
    assert_single_theorem_expansion(
        Utf8Path::new("theorems/single.theorem"),
        &TheoremSpec {
            name: "Smoke",
            about: "Macro smoke test",
        },
    )
}

#[test]
fn multi_document_expansion_preserves_document_order() -> Result<(), Box<dyn std::error::Error>> {
    let path = Utf8Path::new("theorems/multi.theorem");
    let theorem = concat!(
        "Theorem: FirstMacro\n",
        "About: First theorem\n",
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
        "---\n",
        "Theorem: SecondMacro\n",
        "About: Second theorem\n",
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
    );

    let actual = expand_fixture(path, &TheoremFixture(theorem.to_owned()))?;
    let expected = expected_expansion(path, &["FirstMacro", "SecondMacro"]);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn nested_path_expansion_uses_stable_module_mangling() -> Result<(), Box<dyn std::error::Error>> {
    assert_single_theorem_expansion(
        Utf8Path::new("theorems/Nested Path/HTTP-2.theorem"),
        &TheoremSpec {
            name: "HTTP2StreamID",
            about: "Path mangling coverage",
        },
    )
}

#[test]
fn expansion_is_stable_for_repeat_calls() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "RepeatableMacro",
        about: "Repeatability test",
    });
    assert_expansion_is_stable(Utf8Path::new("theorems/repeat.theorem"), &fixture)
}

#[test]
fn manifest_dir_from_env_reports_missing_manifest_dir() {
    let _env = set_cargo_manifest_dir_for_test(None);

    let error = manifest_dir_from_env()
        .err()
        .expect("missing CARGO_MANIFEST_DIR should return an error");

    assert!(matches!(error, MacroExpansionError::MissingManifestDir));
}

#[test]
fn manifest_dir_from_env_supports_expansion_from_valid_manifest_dir()
-> Result<(), Box<dyn std::error::Error>> {
    let path = Utf8Path::new("theorems/env.theorem");
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "EnvMacro",
        about: "Environment manifest directory coverage",
    });
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, &fixture)?;
    let _env = set_cargo_manifest_dir_for_test(Some(fixture_dir.as_str()));

    let manifest_dir = manifest_dir_from_env()?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());

    expand_theorem_file_at(&manifest_dir, &path_literal)?;
    Ok(())
}

#[test]
fn expansion_snapshot_matches_golden_output() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = make_single_theorem_fixture(&TheoremSpec {
        name: "SnapshotThm",
        about: "Snapshot coverage",
    });
    let path = Utf8Path::new("theorems/snapshot.theorem");
    let (_temp_dir, fixture_dir) = temp_fixture_dir()?;
    write_fixture(&fixture_dir, path, &fixture)?;
    let path_literal = syn::LitStr::new(path.as_str(), proc_macro2::Span::call_site());
    let tokens = expand_theorem_file_at(&fixture_dir, &path_literal)?;
    // Format via `prettyplease` to produce a readable, structured snapshot.
    let file: syn::File = syn::parse2(tokens)?;
    let formatted = prettyplease::unparse(&file);
    // Redact non-deterministic hash suffixes; preserve structural whitespace.
    insta::assert_snapshot!("expansion_golden", redact_hashes(&formatted));
    Ok(())
}

#[test]
fn invalid_theorem_file_reports_schema_diagnostic_in_compile_error() {
    let (_temp_dir, fixture_dir) =
        temp_fixture_dir().expect("should create temp fixture dir for invalid theorem");
    let path = Utf8Path::new("theorems/invalid.theorem");
    let fixture = TheoremFixture(
        concat!(
            "Theorem: BrokenMacro\n",
            "About: \"\"\n",
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
        )
        .to_owned(),
    );

    write_fixture(&fixture_dir, path, &fixture).expect("should write invalid theorem fixture");

    let error_string =
        expansion_error_message(&fixture_dir, path).expect("should render expansion error");
    assert!(
        error_string.contains("schema.validation_failure|theorems/invalid.theorem:"),
        "expected rendered schema diagnostic in compile error, got: {error_string}"
    );
}

#[test]
fn missing_theorem_file_reports_io_error_in_compile_error() {
    let (_temp_dir, fixture_dir) =
        temp_fixture_dir().expect("should create temp fixture dir for missing theorem");
    let path = Utf8Path::new("theorems/missing.theorem");

    let error_string =
        expansion_error_message(&fixture_dir, path).expect("should render expansion error");
    assert!(
        error_string.contains("failedtoreadtheoremfile'theorems/missing.theorem'"),
        "expected read failure text in compile error, got: {error_string}"
    );
}

#[test]
fn empty_theorem_file_reports_zero_document_error() {
    let (_temp_dir, fixture_dir) =
        temp_fixture_dir().expect("should create temp fixture dir for empty theorem");
    let path = Utf8Path::new("theorems/empty.theorem");
    let fixture = TheoremFixture(String::new());

    write_fixture(&fixture_dir, path, &fixture).expect("should write empty theorem fixture");

    let error_string =
        expansion_error_message(&fixture_dir, path).expect("should render expansion error");
    assert!(
        error_string.contains("doesnotcontainanytheoremdocuments"),
        "expected zero-document error in compile error, got: {error_string}"
    );
}
