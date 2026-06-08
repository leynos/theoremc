//! Tests for `referenced_actions` deduplication semantics.

use super::test_helpers::{
    DocBoilerplate, boilerplate, doc_with_do_actions, doc_with_let_actions, theorem_doc,
};
use super::*;
use crate::schema::{ActionSignature, ForallVar};
use indexmap::IndexMap;
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

#[rstest]
fn referenced_types_collects_forall_params_and_returns_in_first_seen_order(
    boilerplate: DocBoilerplate,
) {
    let mut doc = theorem_doc("First", IndexMap::new(), Vec::new(), &boilerplate);
    doc.forall
        .insert(forall_var("account"), "crate::Account".to_owned());
    doc.forall.insert(forall_var("limit"), "u64".to_owned());
    doc.actions.insert(
        "account.deposit".to_owned(),
        ActionSignature {
            params: IndexMap::from([
                ("command".to_owned(), "crate::DepositCommand".to_owned()),
                ("audit".to_owned(), "crate::AuditRecord".to_owned()),
            ]),
            returns: "crate::DepositOutcome".to_owned(),
        },
    );

    assert_eq!(
        referenced_types(&[doc]),
        vec![
            "crate::Account",
            "u64",
            "crate::DepositCommand",
            "crate::AuditRecord",
            "crate::DepositOutcome",
        ],
    );
}

#[rstest]
fn referenced_types_deduplicate_by_canonical_type_tokens(boilerplate: DocBoilerplate) {
    let mut first = theorem_doc("First", IndexMap::new(), Vec::new(), &boilerplate);
    first
        .forall
        .insert(forall_var("payload"), "Vec<u8>".to_owned());
    first.actions.insert(
        "payload.write".to_owned(),
        ActionSignature {
            params: IndexMap::from([("buffer".to_owned(), "Vec <u8>".to_owned())]),
            returns: "u64".to_owned(),
        },
    );
    let mut second = theorem_doc("Second", IndexMap::new(), Vec::new(), &boilerplate);
    second.actions.insert(
        "payload.read".to_owned(),
        ActionSignature {
            params: IndexMap::from([("buffer".to_owned(), "Vec<u8>".to_owned())]),
            returns: "u64".to_owned(),
        },
    );

    assert_eq!(referenced_types(&[first, second]), vec!["Vec<u8>", "u64"]);
}

fn forall_var(name: &str) -> ForallVar {
    ForallVar::new(name.to_owned()).expect("valid Forall var")
}
