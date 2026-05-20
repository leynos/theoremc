//! Unit tests for deterministic theorem-file macro expansion.
//!
//! These tests exercise the private expansion pipeline in `lib.rs` directly,
//! using `tests_support.rs` for temporary theorem fixtures and golden token
//! rendering. The separate trybuild suite covers public compiler diagnostics;
//! this module concentrates on internal expansion shape, ordering, and
//! harness metadata before those tokens reach rustc.

use super::tests_support::{
    TheoremFixture, TheoremSpec, assert_expansion_is_stable, assert_single_theorem_expansion,
    expand_fixture, expansion_error_message, expected_expansion_with_unwinds,
    make_single_theorem_fixture, redact_hashes, set_cargo_manifest_dir_for_test, temp_fixture_dir,
    write_fixture,
};
use super::{
    MacroExpansionError, expand_theorem_file_at, generated_harnesses, manifest_dir_from_env,
};
use camino::Utf8Path;
use proptest::prelude::{prop, prop_assert_eq, proptest};
use proptest::{prop_assert, prop_assume};
use rstest::rstest;
use theoremc_core::{
    mangle::mangle_theorem_harness,
    schema::{
        Assertion, Evidence, KaniEvidence, KaniExpectation, TheoremDoc, TheoremName, TheoremValue,
        WitnessCheck,
    },
};

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
        "    unwind: 3\n",
        "    expect: SUCCESS\n",
    );

    let actual = expand_fixture(path, &TheoremFixture(theorem.to_owned()))?;
    let expected = expected_expansion_with_unwinds(path, &["FirstMacro", "SecondMacro"], &[1, 3]);
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
fn generated_harnesses_reports_missing_kani_evidence() {
    let doc = TheoremDoc {
        schema: None,
        theorem: TheoremName::new("NoKaniEvidence".to_owned()).expect("valid theorem name"),
        about: "Missing Kani evidence coverage".to_owned(),
        tags: Vec::new(),
        given: Vec::new(),
        forall: Default::default(),
        assume: Vec::new(),
        witness: vec![WitnessCheck {
            cover: "true".to_owned(),
            because: "reachable".to_owned(),
        }],
        let_bindings: Default::default(),
        do_steps: Vec::new(),
        prove: vec![Assertion {
            assert_expr: "true".to_owned(),
            because: "trivial".to_owned(),
        }],
        evidence: Evidence {
            kani: None,
            verus: Some(TheoremValue::String("future backend".to_owned())),
            stateright: None,
        },
    };

    let error = generated_harnesses("theorems/no-kani.theorem", &[doc])
        .err()
        .expect("missing Kani evidence should fail harness generation");

    assert!(matches!(
        error,
        MacroExpansionError::MissingKaniEvidence { theorem }
            if theorem == "NoKaniEvidence"
    ));
}

fn theorem_doc_with_unwind(name: String, unwind: u32) -> TheoremDoc {
    TheoremDoc {
        schema: None,
        theorem: TheoremName::new(name).expect("generated theorem name should be valid"),
        about: "Generated theorem".to_owned(),
        tags: Vec::new(),
        given: Vec::new(),
        forall: Default::default(),
        assume: Vec::new(),
        witness: vec![WitnessCheck {
            cover: "true".to_owned(),
            because: "reachable".to_owned(),
        }],
        let_bindings: Default::default(),
        do_steps: Vec::new(),
        prove: vec![Assertion {
            assert_expr: "true".to_owned(),
            because: "trivial".to_owned(),
        }],
        evidence: Evidence {
            kani: Some(KaniEvidence {
                unwind,
                expect: KaniExpectation::Success,
                allow_vacuous: false,
                vacuity_because: None,
            }),
            verus: None,
            stateright: None,
        },
    }
}

proptest! {
    #[test]
    fn generated_harnesses_preserve_count_order_and_unwinds(
        unwinds in prop::collection::vec(1_u32..=u32::MAX, 1..8),
    ) {
        let theorem_path = "theorems/generated.theorem";
        let docs = unwinds
            .iter()
            .enumerate()
            .map(|(index, unwind)| theorem_doc_with_unwind(format!("Generated{index}"), *unwind))
            .collect::<Vec<_>>();

        let harnesses = generated_harnesses(theorem_path, &docs)
            .expect("generated theorem documents should produce harnesses");

        prop_assert_eq!(harnesses.len(), docs.len());
        for ((harness, doc), unwind) in harnesses.iter().zip(&docs).zip(&unwinds) {
            let expected_ident = mangle_theorem_harness(theorem_path, doc.theorem.as_str())
                .identifier()
                .to_owned();
            let actual_unwind = harness
                .unwind_literal
                .base10_parse::<u32>()
                .expect("generated unwind literal should parse as u32");

            prop_assert_eq!(harness.ident.to_string(), expected_ident);
            prop_assert_eq!(actual_unwind, *unwind);
        }
    }
}

proptest! {
    #[test]
    fn mangle_theorem_harness_is_deterministic(
        path in "theorems/[a-zA-Z0-9_/]{1,32}\\.theorem",
        name in "[A-Z][a-zA-Z0-9]{1,31}",
    ) {
        let first = mangle_theorem_harness(&path, &name);
        let second = mangle_theorem_harness(&path, &name);
        prop_assert_eq!(
            first.identifier(),
            second.identifier(),
            "mangle_theorem_harness must return identical identifiers for identical inputs"
        );
    }
}

proptest! {
    #[test]
    fn generated_harnesses_fails_when_any_doc_lacks_kani_evidence(
        has_kani in prop::collection::vec(prop::bool::ANY, 1..8),
    ) {
        prop_assume!(!has_kani.iter().all(|&b| b));

        let theorem_path = "theorems/mixed.theorem";
        let docs: Vec<TheoremDoc> = has_kani
            .iter()
            .enumerate()
            .map(|(index, &present)| {
                let name = format!("Mixed{index}");
                if present {
                    theorem_doc_with_unwind(name, 1)
                } else {
                    TheoremDoc {
                        schema: None,
                        theorem: TheoremName::new(name).expect("valid theorem name"),
                        about: "Missing kani".to_owned(),
                        tags: Vec::new(),
                        given: Vec::new(),
                        forall: Default::default(),
                        assume: Vec::new(),
                        witness: vec![WitnessCheck {
                            cover: "true".to_owned(),
                            because: "reachable".to_owned(),
                        }],
                        let_bindings: Default::default(),
                        do_steps: Vec::new(),
                        prove: vec![Assertion {
                            assert_expr: "true".to_owned(),
                            because: "trivial".to_owned(),
                        }],
                        evidence: Evidence {
                            kani: None,
                            verus: None,
                            stateright: None,
                        },
                    }
                }
            })
            .collect();
        let missing_names: Vec<String> = has_kani
            .iter()
            .enumerate()
            .filter(|(_, present)| !**present)
            .map(|(index, _)| format!("Mixed{index}"))
            .collect();

        let error = generated_harnesses(theorem_path, &docs)
            .err()
            .expect("at least one missing-kani doc must cause failure");

        let MacroExpansionError::MissingKaniEvidence { theorem } = error else {
            panic!("expected MissingKaniEvidence, got {error:?}");
        };
        prop_assert!(
            missing_names.contains(&theorem),
            "error named {theorem:?}, expected one of {missing_names:?}"
        );
    }
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
#[case::zero_unwind(
    "theorems/zero-unwind.theorem",
    Some(concat!(
        "Theorem: ZeroUnwindMacro\n",
        "About: Invalid Kani evidence coverage\n",
        "Witness:\n",
        "  - cover: \"true\"\n",
        "    because: \"reachable\"\n",
        "Prove:\n",
        "  - assert: \"true\"\n",
        "    because: \"trivial\"\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 0\n",
        "    expect: SUCCESS\n",
    )),
    "Evidence.kani.unwindmustbeapositiveinteger"
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
