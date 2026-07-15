//! Focused unit tests for the action-signature index.

use super::super::{ActionSignatureIndex, MacroExpansionError};
use googletest::prelude::*;
use pretty_assertions::assert_eq as pretty_assert_eq;
use theoremc_core::schema::load_theorem_docs;

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

#[test]
fn action_signature_index_rejects_conflicting_signatures() -> Result<(), Box<dyn std::error::Error>>
{
    let docs = load_theorem_docs(&format!(
        "{}---\n{}",
        theorem_yaml(
            "FirstConflictingAction",
            concat!(
                "Actions:\n",
                "  account.deposit:\n",
                "    params:\n",
                "      account: u64\n",
                "    returns: bool\n",
            ),
        ),
        theorem_yaml(
            "SecondConflictingAction",
            concat!(
                "Actions:\n",
                "  account.deposit:\n",
                "    params:\n",
                "      account: u32\n",
                "    returns: bool\n",
            ),
        ),
    ))?;
    let selected = vec!["account.deposit"];

    let error = ActionSignatureIndex::for_actions(&docs, &selected)
        .expect_err("conflicting selected signatures should fail");

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature { .. })
    );
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
