//! Reference-selection tests for the conflicting action-signature diagnostic.
//!
//! The diagnostic names the first declaration in document order as the
//! reference. These tests pin that choice against three ways it could silently
//! drift: the position of the *reference* inside the later document, the number
//! of equivalent re-declarations that precede a conflict, and the parameter
//! order the signature summary is rendered in.
//!
//! The error path and the success path are not equally sensitive to depth, and
//! the tests are aimed accordingly. `ActionSignatureIndex::for_actions` scans
//! every declaration in `doc.actions` and checks for conflicts there, and it
//! consults the selected reference set only afterwards, when filtering the
//! result. Conflict detection is therefore independent of reference *selection*,
//! and the *diagnostic* cannot observe where a reference sits; those cases guard
//! against a future index change that starts consulting position. The *success*
//! path is the live one: the generated probe anchor exists only if
//! `collect_step_actions` reached the reference through `Step::Maybe`. Seeding
//! that traversal fault leaves the conflict cases green and fails the nested
//! equivalent cases, which is the evidence that the latter are not vacuous.

use super::super::MacroExpansionError;
use super::super::tests_support::{TheoremFixture, ensure_condition, expand_fixture};
use camino::Utf8Path;
use rstest::rstest;
use theoremc_core::mangle::mangle_action_name;

#[path = "reference_provenance.rs"]
mod reference_provenance;
use reference_provenance::Provenance;

/// Renders one theorem document whose `Do` section is supplied verbatim.
///
/// Every document carries the same `About` prose. The schema requires it to be
/// non-blank, but no assertion here reads it, so taking it as a parameter would
/// only push identical boilerplate through every call site.
///
/// Passing an empty `do_block` omits the section entirely, which is how a
/// document can declare an action without referencing it.
fn document(name: &str, actions: &str, do_block: &str) -> String {
    format!(
        concat!(
            "Theorem: {name}\n",
            "About: Shared fixture document\n",
            "{actions}",
            "{do_block}",
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
        do_block = do_block,
    )
}

/// Renders a `Do` section whose only step calls `account.deposit` with `args`,
/// wrapped in `depth` nested `maybe:` blocks.
///
/// A step at nesting level `level` starts its `- ` dash at column
/// `2 + 6 * level`: each `maybe:` wrapper indents its `because` and `do` keys
/// four columns below its own dash, and the `do` list one level below that.
fn do_with_deposit_reference(depth: usize, args: &[(&str, &str)]) -> String {
    let indent = |level: usize| " ".repeat(2 + 6 * level);
    let pad = indent(depth);
    let mut arg_lines = String::new();
    for (name, value) in args {
        arg_lines.push_str(&pad);
        arg_lines.push_str("      ");
        arg_lines.push_str(name);
        arg_lines.push_str(": ");
        arg_lines.push_str(value);
        arg_lines.push('\n');
    }

    let mut steps = format!(
        concat!(
            "{pad}- call:\n",
            "{pad}    action: account.deposit\n",
            "{pad}    args:\n",
            "{arg_lines}",
        ),
        pad = pad,
        arg_lines = arg_lines,
    );

    for level in (0..depth).rev() {
        let nested_pad = indent(level);
        steps = format!(
            concat!(
                "{pad}- maybe:\n",
                "{pad}    because: optional nesting level\n",
                "{pad}    do:\n",
                "{steps}",
            ),
            pad = nested_pad,
            steps = steps,
        );
    }

    format!("Do:\n{steps}")
}

/// Declares `account.deposit` with `account: u64` and `returns: bool`.
const BASELINE_DECLARATION: &str = concat!(
    "Actions:\n",
    "  account.deposit:\n",
    "    params:\n",
    "      account: u64\n",
    "    returns: bool\n",
);

/// The document pair a nested-reference fixture is built from.
///
/// Grouping the four spellings keeps the fixture builder's signature short and
/// lets each caller state its two theorem names once, beside the types that
/// decide whether the pair conflicts.
struct NestedPair<'a> {
    first_theorem: &'a str,
    later_theorem: &'a str,
    later_param: &'a str,
    later_return: &'a str,
}

/// A two-document fixture whose later document references `account.deposit`
/// only inside `depth` nested `maybe:` blocks.
///
/// The first document always declares the [`BASELINE_DECLARATION`], so
/// [`NestedPair::later_param`] and [`NestedPair::later_return`] alone decide
/// whether the pair conflicts (`u32`, `bool`) or is merely spelled differently
/// (`"u64 "`, `"bool "`).
fn nested_reference_fixture(pair: &NestedPair<'_>, depth: usize) -> TheoremFixture {
    let later_actions = format!(
        concat!(
            "Actions:\n",
            "  account.deposit:\n",
            "    params:\n",
            "      account: {later_param}\n",
            "    returns: {later_return}\n",
        ),
        later_param = pair.later_param,
        later_return = pair.later_return,
    );

    TheoremFixture(format!(
        "{first}---\n{second}",
        first = document(pair.first_theorem, BASELINE_DECLARATION, ""),
        second = document(
            pair.later_theorem,
            &later_actions,
            &do_with_deposit_reference(depth, &[("account", "1")]),
        ),
    ))
}

/// Two documents that disagree on `account.deposit`.
const CONFLICT_PAIR: NestedPair<'static> = NestedPair {
    first_theorem: "NestedReferenceFirst",
    later_theorem: "NestedReferenceSecond",
    later_param: "u32",
    later_return: "bool",
};

/// Two documents that agree on `account.deposit` once tokenized, but spell its
/// types differently.
const EQUIVALENT_PAIR: NestedPair<'static> = NestedPair {
    first_theorem: "NestedEquivalentFirst",
    later_theorem: "NestedEquivalentSecond",
    later_param: "\"u64 \"",
    later_return: "\"bool \"",
};

#[rstest]
#[case::top_level_reference(0)]
#[case::reference_inside_one_maybe(1)]
#[case::reference_inside_two_maybes(2)]
fn conflict_diagnostic_is_independent_of_reference_depth(
    #[case] depth: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let provenance = Provenance {
        action: "account.deposit",
        first_theorem: "NestedReferenceFirst",
        conflicting_theorem: "NestedReferenceSecond",
        first_signature: "fn(account: u64) -> bool",
        conflicting_signature: "fn(account: u32) -> bool",
    };

    let message = provenance.assert_reported(
        &format!("theorems/nested-conflict-depth-{depth}.theorem"),
        &nested_reference_fixture(&CONFLICT_PAIR, depth),
    )?;

    // One expectation above serves every arrangement, which is the claim: only
    // the reference *position* varies across these cases. Depth zero is the
    // baseline, so compare against it only once the reference has moved.
    if depth > 0 {
        let baseline = provenance.assert_reported(
            "theorems/nested-conflict-depth-0.theorem",
            &nested_reference_fixture(&CONFLICT_PAIR, 0),
        )?;
        ensure_condition(
            message == baseline,
            format!("moving the reference {depth} level(s) deep changed the diagnostic"),
        )?;
    }
    Ok(())
}

#[rstest]
#[case::top_level_reference(0)]
#[case::reference_inside_one_maybe(1)]
#[case::reference_inside_two_maybes(2)]
fn equivalent_signature_reference_expands_at_any_depth(
    #[case] depth: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = nested_reference_fixture(&EQUIVALENT_PAIR, depth);
    let path = format!("theorems/nested-equivalent-depth-{depth}.theorem");

    // The `?` is the negative control: nesting the reference must not turn an
    // equivalent pair into a conflict. What remains to check is that accepting
    // the pair still generated the probe for the agreed signature.
    let expanded = expand_fixture(Utf8Path::new(&path), &fixture)?;
    let mangled_action = mangle_action_name("account.deposit");
    let probe_identifier = mangled_action.identifier();
    ensure_condition(
        expanded.contains(&format!("crate::theorem_actions::{probe_identifier}")),
        format!("expected the account.deposit probe at nesting depth {depth}, got: {expanded}"),
    )?;
    Ok(())
}

#[test]
fn first_declaration_supplies_the_reference_signature_spelling()
-> Result<(), Box<dyn std::error::Error>> {
    // Semantically identical to `BASELINE_DECLARATION` but spelled differently.
    // The index must keep the first declaration rather than adopt this
    // spelling, so the expected summary below is the unscored one.
    let equivalent_actions = concat!(
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: \"u64 \"\n",
        "    returns: \"bool \"\n",
    );
    let conflicting_actions = concat!(
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      account: u32\n",
        "    returns: bool\n",
    );

    let fixture = TheoremFixture(format!(
        "{first}---\n{second}---\n{third}",
        first = document("FirstOrderingThesis", BASELINE_DECLARATION, ""),
        second = document("SecondOrderingThesis", equivalent_actions, ""),
        third = document(
            "ThirdOrderingThesis",
            conflicting_actions,
            &do_with_deposit_reference(0, &[("account", "1")]),
        ),
    ));

    // `fn(account: u64) -> bool` is the *first* document's spelling. Had the
    // equivalent re-declaration replaced the reference, the summary would read
    // `fn(account: u64 ) -> bool ` and the conflict would name
    // `SecondOrderingThesis` instead of the third document.
    Provenance {
        action: "account.deposit",
        first_theorem: "FirstOrderingThesis",
        conflicting_theorem: "ThirdOrderingThesis",
        first_signature: "fn(account: u64) -> bool",
        conflicting_signature: "fn(account: u32) -> bool",
    }
    .assert_reported("theorems/first-declaration-order.theorem", &fixture)?;
    Ok(())
}

#[test]
fn conflict_summary_preserves_declaration_parameter_order() -> Result<(), Box<dyn std::error::Error>>
{
    // `zeta` precedes `alpha` in the first declaration, and the reverse is true
    // in the second. The first order is non-alphabetical, so a summary rendered
    // in sorted order cannot satisfy the expectation below; the second
    // declaration happens to be alphabetical, which is why the non-alphabetical
    // one must be the reference.
    let first_actions = concat!(
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      zeta: u64\n",
        "      alpha: u32\n",
        "    returns: bool\n",
    );
    let reordered_actions = concat!(
        "Actions:\n",
        "  account.deposit:\n",
        "    params:\n",
        "      alpha: u32\n",
        "      zeta: u64\n",
        "    returns: bool\n",
    );

    let fixture = TheoremFixture(format!(
        "{first}---\n{second}",
        first = document(
            "FirstParameterOrderThesis",
            first_actions,
            &do_with_deposit_reference(0, &[("zeta", "1"), ("alpha", "2")]),
        ),
        second = document("SecondParameterOrderThesis", reordered_actions, ""),
    ));

    Provenance {
        action: "account.deposit",
        first_theorem: "FirstParameterOrderThesis",
        conflicting_theorem: "SecondParameterOrderThesis",
        first_signature: "fn(zeta: u64, alpha: u32) -> bool",
        conflicting_signature: "fn(alpha: u32, zeta: u64) -> bool",
    }
    .assert_reported("theorems/parameter-order.theorem", &fixture)?;
    Ok(())
}
