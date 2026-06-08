//! Unit tests for generated compile-time referenced-type probes.

use super::tests_support::{TheoremFixture, expand_fixture};
use camino::Utf8Path;

#[test]
fn expansion_emits_referenced_type_probes_for_forall_and_actions()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(
        concat!(
            "Theorem: ReferencedTypes\n",
            "About: Probe all declared Rust types\n",
            "Forall:\n",
            "  account: crate::Account\n",
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      command: crate::DepositCommand\n",
            "    returns: crate::DepositOutcome\n",
            "Do:\n",
            "  - call:\n",
            "      action: account.deposit\n",
            "      args:\n",
            "        command:\n",
            "          amount: 10\n",
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

    let expanded = expand_fixture(Utf8Path::new("theorems/referenced-types.theorem"), &theorem)?;

    assert!(
        expanded.contains("fn__theoremc_assert_referenced<T:?Sized>(){}"),
        "expected referenced-type helper in expansion, got: {expanded}",
    );
    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<crate::Account>;"),
        "expected Forall type probe in expansion, got: {expanded}",
    );
    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<crate::DepositCommand>;"),
        "expected action parameter type probe in expansion, got: {expanded}",
    );
    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<crate::DepositOutcome>;"),
        "expected action return type probe in expansion, got: {expanded}",
    );
    Ok(())
}

#[test]
fn expansion_emits_referenced_type_probes_for_primitive_types()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(
        concat!(
            "Theorem: PrimitiveTypes\n",
            "About: Probe primitive Rust types\n",
            "Forall:\n",
            "  n: u64\n",
            "Actions:\n",
            "  flag.check:\n",
            "    params:\n",
            "      flag: bool\n",
            "    returns: ()\n",
            "Do:\n",
            "  - call:\n",
            "      action: flag.check\n",
            "      args:\n",
            "        flag: true\n",
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

    let expanded = expand_fixture(Utf8Path::new("theorems/primitive-types.theorem"), &theorem)?;

    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<u64>;"),
        "expected primitive Forall type probe in expansion, got: {expanded}",
    );
    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<bool>;"),
        "expected primitive action parameter type probe in expansion, got: {expanded}",
    );
    assert!(
        expanded.contains("let_=__theoremc_assert_referenced::<()>;"),
        "expected unit return type probe in expansion, got: {expanded}",
    );
    Ok(())
}

#[test]
fn expansion_deduplicates_whitespace_equivalent_referenced_types()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(
        concat!(
            "Theorem: CompactType\n",
            "About: First reference uses compact spacing\n",
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
            "Theorem: SpacedType\n",
            "About: Second reference uses extra spacing\n",
            "Actions:\n",
            "  payload.write:\n",
            "    params:\n",
            "      buffer: \"Vec <u8>\"\n",
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
        )
        .to_owned(),
    );

    let expanded = expand_fixture(Utf8Path::new("theorems/equivalent-types.theorem"), &theorem)?;
    let probe_count = expanded
        .match_indices("let_=__theoremc_assert_referenced::<Vec<u8>>;")
        .count();

    assert_eq!(
        probe_count, 1,
        "expected one canonical Vec<u8> probe, got: {expanded}",
    );
    Ok(())
}

#[test]
fn expansion_omits_referenced_type_probe_block_when_no_types_are_referenced()
-> Result<(), Box<dyn std::error::Error>> {
    let theorem = TheoremFixture(
        concat!(
            "Theorem: NoReferencedTypes\n",
            "About: No Forall entries and no Actions map\n",
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

    let expanded = expand_fixture(Utf8Path::new("theorems/no-types.theorem"), &theorem)?;

    assert!(
        !expanded.contains("__theoremc_assert_referenced"),
        "expected no referenced-type probe block, got: {expanded}",
    );
    Ok(())
}
