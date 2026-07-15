//! Unit tests for generated compile-time action binding probes.

use super::MacroExpansionError;
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

    assert!(
        expanded.contains(concat!(
            "const_:fn(u64,u32)->bool=crate::theorem_actions::",
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
    let error = expansion
        .downcast_ref::<MacroExpansionError>()
        .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature { .. })
    );
    Ok(())
}

fn assert_conflicting_action_signature(
    fixture_path: &str,
    theorem_source: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(theorem_source);

    let expansion = expand_fixture(Utf8Path::new(fixture_path), &theorem)
        .expect_err("conflicting action signature should fail expansion");
    let error = expansion
        .downcast_ref::<MacroExpansionError>()
        .ok_or_else(|| std::io::Error::other("expected macro expansion error"))?;

    assert_that!(
        error,
        matches_pattern!(MacroExpansionError::ConflictingActionSignature { .. })
    );
    Ok(())
}

#[test]
fn expansion_rejects_stale_unreferenced_conflicting_action_signature()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem_source = concat!(
        "Theorem: ReferencedActionProbe\n",
        "About: Referenced action with stale declaration\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: u64\n",
        "    returns: bool\n",
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
        "Theorem: StaleActionDeclaration\n",
        "About: Stale conflicting declaration\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: u32\n",
        "    returns: bool\n",
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
    .to_owned();

    assert_conflicting_action_signature("theorems/stale-conflict.theorem", theorem_source)
}

#[test]
fn expansion_rejects_conflicting_unreferenced_action_signature()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem_source = concat!(
        "Theorem: ReferencedActionProbe\n",
        "About: Referenced action with stale unreferenced declaration\n",
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: u64\n",
        "    returns: bool\n",
        "  inventory.reserve:\n",
        "    params:\n",
        "      sku: u64\n",
        "    returns: bool\n",
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
        "Theorem: StaleUnreferencedActionDeclaration\n",
        "About: Stale unreferenced conflicting declaration\n",
        "Actions:\n",
        "  inventory.reserve:\n",
        "    params:\n",
        "      sku: String\n",
        "    returns: bool\n",
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
    .to_owned();

    assert_conflicting_action_signature("theorems/unreferenced-conflict.theorem", theorem_source)
}

#[test]
fn whitespace_only_signature_drift_does_not_conflict() -> Result<(), Box<dyn std::error::Error>> {
    // Two theorems declaring the same action with whitespace-only differences
    // (e.g. `Vec<u8>` vs `Vec <u8>`) describe the same Rust signature and must
    // not be reported as conflicting at macro expansion time.
    let theorem = TheoremFixture(
        concat!(
            "Theorem: FirstActionProbe\n",
            "About: First probe with compact type\n",
            "Actions:\n",
            "  payload.write:\n",
            "    params:\n",
            "      buffer: Vec<u8>\n",
            "    returns: u64\n",
            "Do:\n",
            "  - call:\n",
            "      action: payload.write\n",
            "      args:\n",
            "        buffer: [0]\n",
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
            "About: Second probe with spaced type\n",
            "Actions:\n",
            "  payload.write:\n",
            "    params:\n",
            "      buffer: \"Vec <u8>\"\n",
            "    returns: \"u64 \"\n",
            "Do:\n",
            "  - call:\n",
            "      action: payload.write\n",
            "      args:\n",
            "        buffer: [0]\n",
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

    let expanded = expand_fixture(Utf8Path::new("theorems/whitespace-drift.theorem"), &theorem)?;
    assert!(
        !expanded.contains("conflicting Actions signatures"),
        "whitespace-only differences must not trigger conflict, got: {expanded}",
    );
    Ok(())
}

#[path = "action_probe_tests/action_signature_index.rs"]
mod action_signature_index;
