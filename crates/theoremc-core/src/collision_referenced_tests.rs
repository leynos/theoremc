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

mod referenced_types_proptests {
    //! Property-based tests for referenced type handling.

    use super::super::referenced_types;
    use super::super::test_helpers::{DocBoilerplate, boilerplate, theorem_doc};
    use super::forall_var;
    use crate::schema::rust_type::canonical_token_stream;
    use crate::schema::{ActionSignature, TheoremDoc};
    use indexmap::IndexMap;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn distinct_type_pool() -> impl Strategy<Value = Vec<String>> {
        proptest::collection::hash_set("T[a-z][a-z0-9]{0,6}", 2..6)
            .prop_map(|types| types.into_iter().collect())
    }

    fn build_doc_with_type(
        name: &str,
        position: usize,
        ty: &str,
        boilerplate: &DocBoilerplate,
    ) -> TheoremDoc {
        let mut doc = theorem_doc(name, IndexMap::new(), Vec::new(), boilerplate);
        let action_name = format!("action_{position}");

        match position % 3 {
            0 => {
                doc.forall
                    .insert(forall_var(&format!("value_{position}")), ty.to_owned());
            }
            1 => {
                doc.actions.insert(
                    action_name,
                    ActionSignature {
                        params: IndexMap::from([("value".to_owned(), ty.to_owned())]),
                        returns: ty.to_owned(),
                    },
                );
            }
            _ => {
                doc.actions.insert(
                    action_name,
                    ActionSignature {
                        params: IndexMap::new(),
                        returns: ty.to_owned(),
                    },
                );
            }
        }

        doc
    }

    fn expected_first_seen_types<'a>(types: impl IntoIterator<Item = &'a String>) -> Vec<String> {
        let mut seen = BTreeSet::new();
        types
            .into_iter()
            .filter(|ty| {
                let key = canonical_token_stream(ty).unwrap_or_else(|| ty.trim().to_owned());
                seen.insert(key)
            })
            .cloned()
            .collect()
    }

    proptest! {
        #[test]
        fn referenced_types_preserves_first_seen_order_across_arbitrary_placement(
            pool in distinct_type_pool(),
            raw_indices in proptest::collection::vec(0usize..64, 3..24),
        ) {
            let indices: Vec<_> = raw_indices
                .into_iter()
                .map(|index| index % pool.len())
                .collect();
            let boilerplate = boilerplate();
            let docs: Vec<_> = indices
                .iter()
                .enumerate()
                .map(|(position, index)| {
                    build_doc_with_type(
                        &format!("Theorem{position}"),
                        position,
                        &pool[*index],
                        &boilerplate,
                    )
                })
                .collect();
            let expected = expected_first_seen_types(indices.iter().map(|index| &pool[*index]));
            let actual: Vec<_> = referenced_types(&docs)
                .into_iter()
                .map(str::to_owned)
                .collect();

            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn referenced_types_dedupes_whitespace_variant_spellings(
            padding in proptest::collection::vec(0usize..3, 4),
        ) {
            let variants: Vec<_> = padding
                .iter()
                .map(|padding| format!("Vec{}<{}u8{}>", " ".repeat(*padding), " ".repeat(*padding), " ".repeat(*padding)))
                .collect();
            let boilerplate = boilerplate();
            let mut doc = theorem_doc("Whitespace", IndexMap::new(), Vec::new(), &boilerplate);
            for (position, ty) in variants.iter().enumerate() {
                doc.forall
                    .insert(forall_var(&format!("value_{position}")), ty.clone());
            }

            let docs = [doc];
            let actual = referenced_types(&docs);
            prop_assert_eq!(actual, vec![variants[0].as_str()]);
        }
    }
}
