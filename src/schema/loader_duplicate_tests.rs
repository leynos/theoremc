//! Unit tests for duplicate theorem-key rejection during schema loading.

use rstest::{fixture, rstest};

use super::*;
use crate::schema::test_fixtures;

#[fixture]
fn duplicate_theorem_keys_yaml() -> &'static str {
    test_fixtures::duplicate_theorem_keys_yaml()
}

#[fixture]
fn multi_duplicate_theorem_keys_yaml() -> &'static str {
    test_fixtures::multi_duplicate_theorem_keys_yaml()
}

#[rstest]
fn reject_duplicate_theorem_keys_with_diagnostic(duplicate_theorem_keys_yaml: &str) {
    let source = SourceId::new("theorems/duplicate.theorem");

    let error = load_theorem_docs_with_source(&source, duplicate_theorem_keys_yaml)
        .expect_err("duplicate theorem keys should fail");

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            collisions,
            diagnostic,
        } => {
            assert_eq!(theorem_key, "theorems/duplicate.theorem#SharedName");
            assert_eq!(collisions.len(), 1);
            let collision = collisions
                .first()
                .expect("duplicate theorem-key collision should be present");
            assert!(collision.message.contains(
                "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at \
theorems/duplicate.theorem:1:10, theorems/duplicate.theorem:14:10"
            ));

            let structured = diagnostic.expect("duplicate theorem keys should expose a diagnostic");
            assert_eq!(structured.code.as_str(), "schema.validation_failure");
            assert_eq!(structured.location.source, "theorems/duplicate.theorem");
            assert_eq!(structured.location.line, 14);
            assert_eq!(structured.location.column, 10);
            assert!(structured.message.contains(
                "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at"
            ));
        }
        other => panic!("expected duplicate theorem key error, got: {other}"),
    }
}

#[rstest]
fn reject_all_duplicate_theorem_keys_in_stable_order(multi_duplicate_theorem_keys_yaml: &str) {
    let source = SourceId::new("theorems/multi-duplicate.theorem");

    let error = load_theorem_docs_with_source(&source, multi_duplicate_theorem_keys_yaml)
        .expect_err("duplicate theorem keys should fail");

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            collisions,
            diagnostic,
        } => {
            assert_eq!(theorem_key, "theorems/multi-duplicate.theorem#Alpha");
            assert_eq!(collisions.len(), 2);
            let alpha = collisions
                .first()
                .expect("alpha duplicate theorem-key collision should be present");
            let zebra = collisions
                .get(1)
                .expect("zebra duplicate theorem-key collision should be present");
            assert!(
                alpha
                    .message
                    .contains("duplicate theorem key 'theorems/multi-duplicate.theorem#Alpha'")
            );
            assert!(
                zebra
                    .message
                    .contains("duplicate theorem key 'theorems/multi-duplicate.theorem#Zebra'")
            );

            let structured = diagnostic.expect("duplicate theorem keys should expose a diagnostic");
            assert_eq!(structured.location.line, 40);
            assert_eq!(structured.location.column, 10);
        }
        other => panic!("expected duplicate theorem key error, got: {other}"),
    }
}
