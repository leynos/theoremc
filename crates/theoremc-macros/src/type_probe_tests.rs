//! Unit tests for generated compile-time referenced-type probes.

use super::tests_support::{TheoremFixture, expand_fixture};
use camino::Utf8Path;
use rstest::{fixture, rstest};

const THEOREM_TRAILER: &str = concat!(
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
);

#[derive(Clone, Copy)]
enum ProbeFixture {
    ForallAndActions,
    Primitives,
    WhitespaceEquivalent,
    NoTypes,
}

struct ProbeCase {
    fixture: ProbeFixture,
    path: &'static str,
    expected_probes: &'static [&'static str],
}

#[fixture]
fn theorem_trailer() -> &'static str {
    THEOREM_TRAILER
}

fn theorem_fixture(fixture: ProbeFixture, trailer: &str) -> TheoremFixture {
    let source = match fixture {
        ProbeFixture::ForallAndActions => format!(
            concat!(
                "Theorem: ReferencedTypes\n",
                "About: Probe all declared Rust types\n",
                "Forall:\n",
                "  account: crate::Account\n",
                "Actions:\n",
                "  account.deposit:\n",
                "    params:\n",
                "      command: crate::DepositCommand\n",
                "      account: \"&mut crate::Account\"\n",
                "      profile: \"&crate::Profile\"\n",
                "    returns: crate::DepositOutcome\n",
                "Do:\n",
                "  - call:\n",
                "      action: account.deposit\n",
                "      args:\n",
                "        command:\n",
                "          amount: 10\n",
                "      as: outcome\n",
                "{trailer}",
            ),
            trailer = trailer
        ),
        ProbeFixture::Primitives => format!(
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
                "{trailer}",
            ),
            trailer = trailer
        ),
        ProbeFixture::WhitespaceEquivalent => {
            let compact = format!(
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
                    "      as: compact_len\n",
                    "{trailer}",
                ),
                trailer = trailer
            );
            let spaced = format!(
                concat!(
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
                    "      as: spaced_len\n",
                    "{trailer}",
                ),
                trailer = trailer
            );
            return TheoremFixture(format!("{compact}---\n{spaced}"));
        }
        ProbeFixture::NoTypes => format!(
            concat!(
                "Theorem: NoReferencedTypes\n",
                "About: No Forall entries and no Actions map\n",
                "{trailer}",
            ),
            trailer = trailer
        ),
    };
    TheoremFixture(source)
}

#[rstest]
#[case(ProbeCase {
    fixture: ProbeFixture::ForallAndActions,
    path: "theorems/referenced-types.theorem",
    expected_probes: &[
        "fn__theoremc_assert_referenced<T:?Sized>(){}",
        "let_=__theoremc_assert_referenced::<crate::Account>;",
        "let_=__theoremc_assert_referenced::<crate::DepositCommand>;",
        "let_=__theoremc_assert_referenced::<&mutcrate::Account>;",
        "let_=__theoremc_assert_referenced::<&crate::Profile>;",
        "let_=__theoremc_assert_referenced::<crate::DepositOutcome>;",
    ],
})]
#[case(ProbeCase {
    fixture: ProbeFixture::Primitives,
    path: "theorems/primitive-types.theorem",
    expected_probes: &[
        "let_=__theoremc_assert_referenced::<u64>;",
        "let_=__theoremc_assert_referenced::<bool>;",
        "let_=__theoremc_assert_referenced::<()>;",
    ],
})]
#[case(ProbeCase {
    fixture: ProbeFixture::WhitespaceEquivalent,
    path: "theorems/equivalent-types.theorem",
    expected_probes: &["let_=__theoremc_assert_referenced::<Vec<u8>>;"],
})]
#[case(ProbeCase {
    fixture: ProbeFixture::NoTypes,
    path: "theorems/no-types.theorem",
    expected_probes: &[],
})]
fn expansion_emits_referenced_type_probes(
    theorem_trailer: &str,
    #[case] case: ProbeCase,
) -> Result<(), Box<dyn std::error::Error>> {
    let theorem = theorem_fixture(case.fixture, theorem_trailer);
    let expanded = expand_fixture(Utf8Path::new(case.path), &theorem)?;

    if case.expected_probes.is_empty() {
        assert!(
            !expanded.contains("__theoremc_assert_referenced"),
            "expected no referenced-type probe block, got: {expanded}",
        );
    } else {
        for expected_probe in case.expected_probes {
            assert!(
                expanded.contains(expected_probe),
                "expected probe {expected_probe:?} in expansion, got: {expanded}",
            );
        }
    }

    if matches!(case.fixture, ProbeFixture::WhitespaceEquivalent) {
        assert_eq!(
            expanded
                .match_indices("let_=__theoremc_assert_referenced::<Vec<u8>>;")
                .count(),
            1,
            "expected one canonical Vec<u8> probe, got: {expanded}",
        );
    }
    Ok(())
}
