//! Drives the publisher's conditions: the splitter beneath the ref guard, and
//! the conditions that could stop the publisher's required work.
//!
//! Split from `publisher_cases.rs` under the repository's 400-line cap. The
//! ref-guard clause is only as good as the split beneath it, so the quoting
//! and byte-offset rules it depends on are pinned here on their own.

use anyhow::{Result, ensure};
use rstest::rstest;

use super::{
    guard_cases::mutate_once,
    publisher_cases::{GUARD, NEVER_CANCEL, publisher},
    publisher_rules as rules,
    pull_request_cases::parse,
};

/// Scenario: conditions whose operators sit inside quoted literals.
///
/// Invariant: a `||` inside a string is not a disjunction, and an `&&`
/// inside one does not split a conjunct.
#[rstest]
#[case::quoted_or("${{ github.ref == 'refs/heads/main' && env.X != 'a||b' }}", Some(2))]
#[case::quoted_and("${{ github.ref == 'refs/heads/main' && env.X != 'a&&b' }}", Some(2))]
#[case::bare_or("github.ref == 'refs/heads/main' || true", None)]
#[case::negated_group(
    "${{ !(env.X != '' && github.ref == 'refs/heads/main' && true) }}",
    None
)]
#[case::quoted_parenthesis("${{ github.ref == 'refs/heads/main' && env.X != '(a)' }}", Some(2))]
fn quoted_operators_are_not_operators(#[case] condition: &str, #[case] expected: Option<usize>) {
    assert_eq!(
        rules::conjuncts(condition).map(|parts| parts.len()),
        expected,
        "for {condition}"
    );
}

/// Scenario: a quoted literal holds multi-byte characters before an operator.
///
/// Invariant: each conjunct is cut at its own operator. The blanked copy
/// that locates operators must keep the input's byte offsets, or the cut
/// lands inside a character and the ref conjunct reads as something else.
#[test]
fn a_multibyte_literal_does_not_shift_the_split() {
    assert_eq!(
        rules::conjuncts("${{ env.X != 'é&&ü' && github.ref == 'refs/heads/main' }}"),
        Some(vec![
            "env.X != 'é&&ü'".to_owned(),
            "github.ref == 'refs/heads/main'".to_owned(),
        ]),
    );
}

/// Scenario: the required work sits behind a condition that can stop it.
///
/// Invariant: a conditional ratcheted coverage step does not count as the
/// publisher generating coverage, and a conditional publisher job is refused,
/// since `if: false` on either leaves every other clause reading the step as
/// present.
#[rstest]
#[case::coverage_step_if(
    "        with:\n          with-ratchet: 'true'\n",
    "        if: github.actor == 'x'\n        with:\n          with-ratchet: 'true'\n",
    "generates no ratcheted coverage"
)]
#[case::job_if("  coverage:\n", "  coverage:\n    if: false\n", "carries an `if:`")]
fn conditional_required_work_is_refused(
    #[case] from: &str,
    #[case] to: &str,
    #[case] expected: &str,
) -> Result<()> {
    let complying = publisher(NEVER_CANCEL, GUARD, "");
    let source = mutate_once(&complying, from, to)?;
    let findings = rules::publisher_findings(&parse(&source)?);
    ensure!(
        findings.len() == 1 && findings.iter().all(|f| f.contains(expected)),
        "expected one finding naming {expected:?}, saw {findings:?}"
    );
    Ok(())
}
