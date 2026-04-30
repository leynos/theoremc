//! Unit tests for duplicate theorem-key rejection during schema loading.

use rstest::{fixture, rstest};

use super::*;
use crate::schema::test_fixtures;

#[derive(Clone, Copy)]
enum DuplicateFixture {
    Single,
    Multi,
}

struct DuplicateCase<'a> {
    fixture: DuplicateFixture,
    source: &'a str,
    expected_key: &'a str,
    expected_collision_messages: &'a [&'a str],
    expected_line: usize,
    expected_column: usize,
}

#[fixture]
fn duplicate_theorem_keys_yaml() -> &'static str {
    test_fixtures::duplicate_theorem_keys_yaml()
}

#[fixture]
fn multi_duplicate_theorem_keys_yaml() -> &'static str {
    test_fixtures::multi_duplicate_theorem_keys_yaml()
}

#[rstest]
#[case(DuplicateCase {
    fixture: DuplicateFixture::Single,
    source: "theorems/duplicate.theorem",
    expected_key: "theorems/duplicate.theorem#SharedName",
    expected_collision_messages: &[concat!(
        "duplicate theorem key 'theorems/duplicate.theorem#SharedName' appears at ",
        "theorems/duplicate.theorem:1:10, theorems/duplicate.theorem:14:10"
    )],
    expected_line: 14,
    expected_column: 10,
})]
#[case(DuplicateCase {
    fixture: DuplicateFixture::Multi,
    source: "theorems/multi-duplicate.theorem",
    expected_key: "theorems/multi-duplicate.theorem#Alpha",
    expected_collision_messages: &[
        concat!(
            "duplicate theorem key 'theorems/multi-duplicate.theorem#Alpha' appears at ",
            "theorems/multi-duplicate.theorem:14:10, theorems/multi-duplicate.theorem:40:10"
        ),
        concat!(
            "duplicate theorem key 'theorems/multi-duplicate.theorem#Zebra' appears at ",
            "theorems/multi-duplicate.theorem:1:10, theorems/multi-duplicate.theorem:27:10"
        ),
    ],
    expected_line: 40,
    expected_column: 10,
})]
fn reject_duplicate_theorem_keys_with_diagnostic(
    #[case] case: DuplicateCase<'_>,
    duplicate_theorem_keys_yaml: &str,
    multi_duplicate_theorem_keys_yaml: &str,
) {
    let yaml = match case.fixture {
        DuplicateFixture::Single => duplicate_theorem_keys_yaml,
        DuplicateFixture::Multi => multi_duplicate_theorem_keys_yaml,
    };
    let source = SourceId::new(case.source);

    let error = load_theorem_docs_with_source(&source, yaml)
        .expect_err("duplicate theorem keys should fail");

    match error {
        SchemaError::DuplicateTheoremKey {
            theorem_key,
            collisions,
            diagnostic,
        } => {
            assert_eq!(theorem_key, case.expected_key);
            assert_eq!(collisions.len(), case.expected_collision_messages.len());

            for (collision, expected_message) in collisions
                .iter()
                .zip(case.expected_collision_messages.iter())
            {
                assert_eq!(collision.message, *expected_message);
            }

            let structured = diagnostic.expect("duplicate theorem keys should expose a diagnostic");
            assert_eq!(structured.code.as_str(), "schema.validation_failure");
            assert_eq!(structured.location.source, case.source);
            assert_eq!(structured.location.line, case.expected_line);
            assert_eq!(structured.location.column, case.expected_column);
        }
        other => panic!("expected duplicate theorem key error, got: {other}"),
    }
}
