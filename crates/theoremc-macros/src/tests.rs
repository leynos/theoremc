//! Unit tests for deterministic theorem-file macro expansion.

use super::tests_support::{
    TheoremFixture, TheoremSpec, assert_expansion_is_stable, assert_single_theorem_expansion,
    expand_fixture, expansion_error_message, expected_expansion, make_single_theorem_fixture,
    redact_hashes, set_cargo_manifest_dir_for_test, temp_fixture_dir, write_fixture,
};
use super::{MacroExpansionError, expand_theorem_file_at, manifest_dir_from_env};
use camino::Utf8Path;
use rstest::rstest;

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

#[rstest]
#[case::invalid_schema(
    "theorems/invalid.theorem",
    Some(concat!(
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
    )),
    "schema.validation_failure|theorems/invalid.theorem:",
)]
#[case::missing_file(
    "theorems/missing.theorem",
    None,
    "failedtoreadtheoremfile'theorems/missing.theorem'"
)]
#[case::empty_file(
    "theorems/empty.theorem",
    Some(""),
    "doesnotcontainanytheoremdocuments"
)]
fn theorem_file_errors_report_expected_compile_error(
    #[case] path: &str,
    #[case] fixture_content: Option<&str>,
    #[case] expected_fragment: &str,
) {
    let (_temp_dir, fixture_dir) =
        temp_fixture_dir().expect("should create temp fixture dir for error fixture");
    let path = Utf8Path::new(path);

    if let Some(content) = fixture_content {
        write_fixture(&fixture_dir, path, &TheoremFixture(content.to_owned()))
            .expect("should write theorem fixture");
    }

    let error_string =
        expansion_error_message(&fixture_dir, path).expect("should render expansion error");
    assert!(
        error_string.contains(expected_fragment),
        "expected '{expected_fragment}' in compile error, got: {error_string}"
    );
}
