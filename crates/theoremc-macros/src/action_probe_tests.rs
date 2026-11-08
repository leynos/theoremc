//! Unit tests for generated compile-time action binding probes.

use super::error::MacroExpansionError;
use super::tests_support::{TheoremFixture, expand_fixture};
use camino::Utf8Path;
use googletest::prelude::*;
use rstest::rstest;

#[test]
fn expansion_emits_typed_action_probe_for_referenced_action()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(
        concat!(
            "Theorem: ActionProbe\n",
            "About: Probe generated action signatures\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: u64\n",
            "      amount: u32\n",
            "    returns: bool\n",
            "Let:\n",
            "  accepted:\n",
            "    call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        account: 1\n",
            "        amount: 10\n",
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
        )
        .to_owned(),
    );

    let expanded = expand_fixture(Utf8Path::new("theorems/action-probe.theorem"), &theorem)?;

    assert_that!(
        expanded.as_str(),
        contains_substring(concat!(
            "const_:fn(u64,u32)->bool=crate::theorem_actions::",
            "account__deposit__h05158894bfb4;"
        )),
    );
    Ok(())
}

/// Builds a two-document fixture whose `account.deposit` declarations differ.
///
/// Each document declares and calls the same action, so both declarations are
/// reachable and expansion must compare their signatures. The caller chooses
/// where the second document drifts: its parameter type or its return type.
fn shared_action_conflict_fixture(
    first_param: &str,
    first_return: &str,
    second_param: &str,
    second_return: &str,
) -> TheoremFixture {
    TheoremFixture(format!(
        concat!(
            "Theorem: FirstActionProbe\n",
            "About: First probe\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: {first_param}\n",
            "    returns: {first_return}\n",
            "Do:\n",
            "  - call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        account: 1\n",
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
            "Theorem: SecondActionProbe\n",
            "About: Second probe\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: {second_param}\n",
            "    returns: {second_return}\n",
            "Do:\n",
            "  - call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        account: 2\n",
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
        first_param = first_param,
        first_return = first_return,
        second_param = second_param,
        second_return = second_return,
    ))
}

#[rstest]
fn expansion_rejects_conflicting_signatures_for_shared_action(
    #[values("return", "parameter")] conflict: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (first_param, first_return, second_param, second_return) = match conflict {
        "return" => ("u64", "bool", "u64", "u32"),
        "parameter" => ("u64", "bool", "u32", "bool"),
        _ => {
            return Err(std::io::Error::other(format!(
                "unexpected action signature conflict case: {conflict:?}"
            ))
            .into());
        }
    };
    let theorem =
        shared_action_conflict_fixture(first_param, first_return, second_param, second_return);

    let expansion = expand_fixture(Utf8Path::new("theorems/conflicting.theorem"), &theorem)
        .expect_err("conflicting shared action signatures should fail expansion");
    let error = expansion
        .downcast_ref::<MacroExpansionError>()
        .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature { .. })
    );
    Ok(())
}

#[path = "action_probe_tests/conflicting_signatures.rs"]
mod conflicting_signatures;

#[path = "action_probe_tests/action_signature_index.rs"]
mod action_signature_index;

#[path = "action_probe_tests/reference_selection.rs"]
mod reference_selection;
