//! Tests for `referenced_actions` deduplication semantics.

use super::test_helpers::{DocBoilerplate, boilerplate, doc_with_do_actions, doc_with_let_actions};
use super::*;
use rstest::rstest;

#[rstest]
fn referenced_actions_deduplicate_in_first_seen_order(boilerplate: DocBoilerplate) {
    let first = doc_with_let_actions(
        "First",
        &["account.params", "account.deposit", "account.params"],
        &boilerplate,
    );
    let second = doc_with_do_actions(
        "Second",
        &["account.validate", "account.deposit"],
        &boilerplate,
    );

    assert_eq!(
        referenced_actions(&[first, second]),
        vec!["account.params", "account.deposit", "account.validate"]
    );
}
