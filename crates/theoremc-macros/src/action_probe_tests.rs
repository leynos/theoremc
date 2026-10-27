//! Unit tests for generated compile-time action binding probes.

use super::MacroExpansionError;
use super::tests_support::{TheoremFixture, ensure_condition, expand_fixture};
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

    ensure_condition(
        expanded.contains(concat!(
            "const_:fn(u64,u32)->bool=crate::theorem_actions::",
            "account__deposit__h05158894bfb4;"
        )),
        format!("expected typed action probe in expansion, got: {expanded}"),
    )?;
    Ok(())
}

#[rstest]
#[case::return_type("u64", "bool", "u64", "u32")]
#[case::parameter_type("u64", "bool", "u32", "bool")]
fn expansion_rejects_conflicting_signatures_for_shared_action(
    #[case] first_param: &str,
    #[case] first_return: &str,
    #[case] second_param: &str,
    #[case] second_return: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(format!(
        "{}---\n{}",
        action_probe_theorem("FirstActionProbe", first_param, first_return, 1),
        action_probe_theorem("SecondActionProbe", second_param, second_return, 2),
    ));

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

fn action_probe_theorem(name: &str, parameter: &str, returns: &str, value: u8) -> String {
    format!(
        concat!(
            "Theorem: {name}\n",
            "About: Shared action signature probe\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: {parameter}\n",
            "    returns: {returns}\n",
            "Do:\n",
            "  - call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        account: {value}\n",
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
        parameter = parameter,
        returns = returns,
        value = value
    )
}

#[path = "action_probe_tests/conflicting_signatures.rs"]
mod conflicting_signatures;

#[path = "action_probe_tests/action_signature_index.rs"]
mod action_signature_index;

#[path = "action_probe_tests/reference_selection.rs"]
mod reference_selection;
