//! Focused unit tests for the action-signature index.

use super::super::{MacroExpansionError, action_signature_index::ActionSignatureIndex};
use googletest::prelude::*;
use pretty_assertions::assert_eq as pretty_assert_eq;
use rstest::rstest;
use theoremc_core::schema::load_theorem_docs;

/// Renders an `Actions` block declaring one action with one parameter.
fn declared_action_yaml(action: &str, declaration: (&str, &str)) -> String {
    let (param, returns) = declaration;
    let (param_name, param_type) = param
        .split_once(": ")
        .expect("test cases always declare a named parameter");
    format!(
        concat!(
            "Actions:\n",
            "  {action}:\n",
            "    params:\n",
            "      {param_name}: {param_type}\n",
            "    returns: {returns}\n",
        ),
        action = action,
        param_name = param_name,
        param_type = param_type,
        returns = returns,
    )
}

#[test]
fn action_signature_index_finds_one_action_in_one_document()
-> Result<(), Box<dyn std::error::Error>> {
    let docs = load_theorem_docs(&theorem_yaml(
        "IndexedAction",
        concat!(
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: u64\n",
            "    returns: bool\n",
        ),
    ))?;
    let selected = vec!["account.deposit"];

    let index = ActionSignatureIndex::for_actions(&docs, &selected)?;
    let signature = index.signature_for("account.deposit")?;

    assert_that!(signature.params.len(), eq(1_usize));
    pretty_assert_eq!(signature.returns, "bool");
    Ok(())
}

#[test]
fn action_signature_index_accepts_equivalent_repeated_signatures()
-> Result<(), Box<dyn std::error::Error>> {
    let docs = load_theorem_docs(&format!(
        "{}---\n{}",
        theorem_yaml(
            "FirstIndexedAction",
            concat!(
                "Actions:\n",
                "  payload.write:\n",
                "    params:\n",
                "      buffer: Vec<u8>\n",
                "    returns: u64\n",
            ),
        ),
        theorem_yaml(
            "SecondIndexedAction",
            concat!(
                "Actions:\n",
                "  payload.write:\n",
                "    params:\n",
                "      buffer: \"Vec <u8>\"\n",
                "    returns: \"u64 \"\n",
            ),
        ),
    ))?;
    let selected = vec!["payload.write"];

    let index = ActionSignatureIndex::for_actions(&docs, &selected)?;
    let signature = index.signature_for("payload.write")?;

    let buffer_type = signature
        .params
        .get("buffer")
        .expect("first signature should define buffer");
    pretty_assert_eq!(buffer_type, "Vec<u8>");
    pretty_assert_eq!(signature.returns, "u64");
    Ok(())
}

#[rstest]
#[case::parameter_type_drift(
    "account.deposit",
    ("account: u64", "bool"),
    ("account: u32", "bool")
)]
#[case::return_type_drift(
    "payload.write",
    ("buffer: u64", "u64"),
    ("buffer: u64", "String")
)]
fn action_signature_index_reports_conflicting_signatures_with_provenance(
    #[case] action: &str,
    #[case] first: (&str, &str),
    #[case] conflicting: (&str, &str),
) -> Result<(), Box<dyn std::error::Error>> {
    let docs = load_theorem_docs(&format!(
        "{}---\n{}",
        theorem_yaml("FirstIndexConflict", &declared_action_yaml(action, first)),
        theorem_yaml(
            "SecondIndexConflict",
            &declared_action_yaml(action, conflicting),
        ),
    ))?;
    let selected = vec![action];

    let error = ActionSignatureIndex::for_actions(&docs, &selected)
        .expect_err("conflicting selected signatures should fail");

    let expected_first = format!("fn({}) -> {}", first.0, first.1);
    let expected_conflicting = format!("fn({}) -> {}", conflicting.0, conflicting.1);

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature {
            action: eq(action),
            first_theorem: eq("FirstIndexConflict"),
            conflicting_theorem: eq("SecondIndexConflict"),
            first_signature: eq(expected_first.as_str()),
            conflicting_signature: eq(expected_conflicting.as_str()),
        })
    );

    // The rendered message is what a theorem author actually reads, so it must
    // carry the same provenance as the typed fields.
    let message = error.to_string();
    for fragment in [
        action,
        "FirstIndexConflict",
        "SecondIndexConflict",
        expected_first.as_str(),
        expected_conflicting.as_str(),
    ] {
        assert_that!(message.as_str(), contains_substring(fragment));
    }
    Ok(())
}

#[test]
fn action_signature_index_reports_missing_selected_signature()
-> Result<(), Box<dyn std::error::Error>> {
    let docs = load_theorem_docs(&theorem_yaml("MissingIndexedAction", ""))?;
    let selected = vec!["account.deposit"];
    let index = ActionSignatureIndex::for_actions(&docs, &selected)?;

    let error = index
        .signature_for("account.deposit")
        .expect_err("missing selected signature should fail");

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::MissingActionSignature { .. }),
    );
    let message = error.to_string();
    assert_that!(
        message.as_str(),
        contains_substring("missing an Actions signature")
    );
    Ok(())
}

fn theorem_yaml(name: &str, actions: &str) -> String {
    format!(
        concat!(
            "Theorem: {name}\n",
            "About: Index test fixture\n",
            "{actions}",
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
        name = name,
        actions = actions,
    )
}
