//! Behavioural tests for deterministic theorem harness naming.

use rstest_bdd_macros::{given, scenario, then};
use theoremc::mangle::golden::HARNESS_GOLDEN_TUPLES;
use theoremc::mangle::{hash12, mangle_theorem_harness, theorem_key};
use theoremc::schema::test_fixtures;
use theoremc::schema::{SchemaError, SourceId, load_theorem_docs_with_source};

#[given("representative theorem paths and theorem identifiers")]
fn given_representative_theorem_paths_and_theorem_identifiers() {}

#[then("each theorem produces the expected harness identifier")]
fn then_each_theorem_produces_the_expected_harness_identifier() {
    for (path, theorem, expected_slug) in HARNESS_GOLDEN_TUPLES {
        let harness = mangle_theorem_harness(path, theorem);
        let expected_key = theorem_key(path, theorem);
        let expected_hash = hash12(&expected_key);

        assert_eq!(
            harness.slug(),
            *expected_slug,
            "slug mismatch for {theorem}"
        );
        assert_eq!(
            harness.theorem_key(),
            expected_key,
            "key mismatch for {theorem}"
        );
        assert_eq!(harness.hash(), expected_hash, "hash mismatch for {theorem}");
        assert_eq!(
            harness.identifier(),
            format!("theorem__{expected_slug}__h{expected_hash}"),
            "identifier mismatch for {theorem}",
        );
    }
}

#[given("theorem identifiers that are already snake case")]
fn given_theorem_identifiers_that_are_already_snake_case() {}

#[then("the harness slug stays unchanged")]
fn then_the_harness_slug_stays_unchanged() {
    let harness = mangle_theorem_harness("theorems/smoke.theorem", "already_snake_42");
    assert_eq!(harness.slug(), "already_snake_42");
}

#[given("a multi-document theorem source with duplicate theorem identifiers")]
fn given_a_multi_document_theorem_source_with_duplicate_theorem_identifiers() {}

#[then("loading fails with a duplicate theorem key diagnostic")]
fn then_loading_fails_with_a_duplicate_theorem_key_diagnostic() -> Result<(), String> {
    let source = SourceId::new("theorems/duplicate.theorem");
    let error =
        load_theorem_docs_with_source(&source, test_fixtures::duplicate_theorem_keys_yaml())
            .err()
            .ok_or_else(|| "duplicate theorem keys should fail".to_owned())?;

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            diagnostic,
            ..
        } => {
            if theorem_key != "theorems/duplicate.theorem#SharedName" {
                return Err(format!(
                    "expected theorem key theorems/duplicate.theorem#SharedName, got {theorem_key}"
                ));
            }

            let structured = diagnostic
                .ok_or_else(|| "duplicate theorem key should carry a diagnostic".to_owned())?;
            if structured.location.source != "theorems/duplicate.theorem" {
                return Err(format!(
                    "expected diagnostic source theorems/duplicate.theorem, got {}",
                    structured.location.source
                ));
            }
            if structured.location.line != 14 {
                return Err(format!(
                    "expected diagnostic line 14, got {}",
                    structured.location.line
                ));
            }
            if structured.location.column != 10 {
                return Err(format!(
                    "expected diagnostic column 10, got {}",
                    structured.location.column
                ));
            }
            if !structured.message.contains(
                "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at",
            ) {
                return Err(format!(
                    "expected duplicate theorem-key message, got {}",
                    structured.message
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "expected duplicate theorem key error, got: {other}"
        )),
    }
}

#[scenario(
    path = "tests/features/harness_naming.feature",
    name = "Representative theorem identifiers produce deterministic harness names"
)]
fn representative_theorem_identifiers_produce_deterministic_harness_names() {}

#[scenario(
    path = "tests/features/harness_naming.feature",
    name = "Theorem slugs preserve snake-case identifiers"
)]
fn theorem_slugs_preserve_snake_case_identifiers() {}

#[scenario(
    path = "tests/features/harness_naming.feature",
    name = "Duplicate theorem keys are rejected during loading"
)]
fn duplicate_theorem_keys_are_rejected_during_loading() {}
