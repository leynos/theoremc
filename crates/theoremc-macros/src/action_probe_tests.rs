//! Unit tests for generated compile-time action binding probes.

use super::tests_support::{TheoremFixture, expand_fixture};
use camino::Utf8Path;
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

    assert!(
        expanded.contains(concat!(
            "let_:fn(u64,u32)->bool=crate::theorem_actions::",
            "account__deposit__h05158894bfb4;"
        )),
        "expected typed action probe in expansion, got: {expanded}"
    );
    Ok(())
}

#[rstest]
fn expansion_rejects_conflicting_signatures_for_shared_action(
    #[values("return", "parameter")] conflict: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (first_param, first_return, second_param, second_return) = match conflict {
        "return" => ("u64", "bool", "u64", "u32"),
        "parameter" => ("u64", "bool", "u32", "bool"),
        _ => unreachable!("rstest provides known conflict cases"),
    };
    let theorem = TheoremFixture(format!(
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
    ));

    let expansion = expand_fixture(Utf8Path::new("theorems/conflicting.theorem"), &theorem)
        .expect_err("conflicting shared action signatures should fail expansion");
    assert!(
        expansion
            .to_string()
            .contains("conflicting Actions signatures"),
        "expected conflicting signature error, got: {expansion}"
    );
    Ok(())
}
