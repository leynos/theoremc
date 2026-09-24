//! Drives the publisher's pinned group, triggers and upload wiring.
//!
//! Split from `publisher_cases.rs` under the repository's 400-line cap. Each
//! case varies one of these on an otherwise complying publisher.

use anyhow::{Result, ensure};
use rstest::rstest;

use super::{
    publisher_cases::{GUARD, NEVER_CANCEL, publisher},
    publisher_rules as rules,
    pull_request_cases::parse,
};

/// Scenario: the publisher's job declares a constant group beside a keyed
/// workflow group, or its trigger set gains or loses an event.
///
/// Invariant: each is named. A constant job group serializes the upload
/// across refs and events whatever the workflow group says, and the trigger
/// set is pinned because a lost dispatch or an added schedule changes when
/// the baseline is written without failing any other clause.
#[rstest]
#[case::constant_job_group(
    "  coverage:\n",
    "  coverage:\n    concurrency:\n      group: upload\n",
    "not exactly"
)]
#[case::schedule_added(
    "  workflow_dispatch:\n",
    "  workflow_dispatch:\n  schedule:\n    - cron: '0 0 * * *'\n",
    "not exactly"
)]
#[case::dispatch_dropped("  workflow_dispatch:\n", "", "not exactly")]
fn the_publisher_group_and_triggers_are_pinned(
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

/// Returns `text` with its one occurrence of `from` replaced by `to`.
///
/// A case whose anchor is absent changes nothing, and one whose anchor
/// occurs twice can pass on the change it does not name, so both are
/// refused before the rule is asked anything.
pub(super) fn mutate_once(text: &str, from: &str, to: &str) -> Result<String> {
    let count = text.matches(from).count();
    ensure!(
        count == 1,
        "{from:?} occurs {count} times; a case must change exactly one thing"
    );
    Ok(text.replacen(from, to, 1))
}

/// A publisher wired as this repository wires it: the upload reads what the
/// coverage step writes and passes the secret itself as its token.
const WIRED: &str = r"
on:
  push:
    branches: [main]
jobs:
  coverage:
    steps:
      - uses: leynos/shared-actions/.github/actions/generate-coverage@abc
        with:
          output-path: lcov.info
          format: lcov
      - uses: leynos/shared-actions/.github/actions/upload-codescene-coverage@abc
        with:
          path: lcov.info
          format: lcov
          access-token: ${{ secrets.CS_ACCESS_TOKEN }}
";

/// Scenario: the upload is rewired away from what was measured, or from the
/// token.
///
/// Invariant: each variation is named, and the wired publisher has none.
/// Every other publisher clause passes all five variations, because each
/// judges one step at a time.
#[rstest]
#[case::wired("", "", None)]
#[case::other_path(
    "          path: lcov.info",
    "          path: other.info",
    Some("which no coverage step writes")
)]
#[case::other_format(
    "          format: lcov\n          access",
    "          format: cobertura\n          access",
    Some("which no coverage step writes")
)]
#[case::no_token(
    "          access-token: ${{ secrets.CS_ACCESS_TOKEN }}\n",
    "",
    Some("not the secret itself")
)]
#[case::other_token(
    "${{ secrets.CS_ACCESS_TOKEN }}",
    "${{ secrets.OTHER }}",
    Some("not the secret itself")
)]
#[case::through_the_env(
    "${{ secrets.CS_ACCESS_TOKEN }}",
    "${{ env.CS_ACCESS_TOKEN }}",
    Some("not the secret itself")
)]
fn the_upload_sends_what_was_measured(
    #[case] from: &str,
    #[case] to: &str,
    #[case] expected: Option<&str>,
) -> Result<()> {
    let source = if from.is_empty() {
        WIRED.to_owned()
    } else {
        mutate_once(WIRED, from, to)?
    };
    let findings = rules::wiring_findings(&parse(&source)?);
    match expected {
        None => ensure!(findings.is_empty(), "unexpected findings: {findings:?}"),
        Some(clause) => ensure!(
            findings.len() == 1 && findings.iter().all(|f| f.contains(clause)),
            "expected one finding naming {clause:?}, saw {findings:?}"
        ),
    }
    Ok(())
}

/// Scenario: neither the upload nor the coverage step names the report.
///
/// Invariant: the upload is refused. Two absent inputs compare equal, and the
/// actions' defaults are not read here, so an empty reading must not pass.
#[test]
fn an_unnamed_report_is_refused() -> Result<()> {
    let unnamed = mutate_once(WIRED, "          path: lcov.info\n", "")?;
    let source = mutate_once(&unnamed, "          output-path: lcov.info\n", "")?;
    let findings = rules::wiring_findings(&parse(&source)?);
    ensure!(
        findings.len() == 1 && findings.iter().all(|f| f.contains("names no report")),
        "expected one finding naming the missing report, saw {findings:?}"
    );
    Ok(())
}

/// Scenario: a case's anchor is absent, or occurs more than once.
///
/// Invariant: the case is refused, since it would change nothing or could
/// pass on a change it does not name.
#[rstest]
#[case::absent("no such text")]
#[case::repeated("lcov.info")]
fn a_case_changes_exactly_one_place(#[case] from: &str) -> Result<()> {
    ensure!(
        mutate_once(WIRED, from, "other").is_err(),
        "{from:?} was accepted"
    );
    Ok(())
}

/// Scenario: the token check step is broken one way at a time, or the upload
/// stops reading its answer.
///
/// Invariant: each is named. A deleted check leaves the upload skipping
/// forever, a check behind an `if:` can be skipped, one with another shell or
/// allowed to fail may write nothing, one with no id cannot be read, a
/// changed command answers something else, and an upload guarded on the
/// environment or on another step reads no check.
#[rstest]
#[case::check_behind_an_if(
    "      - id: codescene-token\n",
    "      - id: codescene-token\n        if: always()\n",
    "carries an `if:`"
)]
#[case::check_with_a_shell(
    "      - id: codescene-token\n",
    "      - id: codescene-token\n        shell: bash -c 'exit 0; {0}'\n",
    "declares `shell`"
)]
#[case::check_continuing_on_error(
    "      - id: codescene-token\n",
    "      - id: codescene-token\n        continue-on-error: true\n",
    "declares `continue-on-error`"
)]
#[case::workflow_default_shell(
    "jobs:\n",
    "defaults:\n  run:\n    shell: bash -c 'exit 0; {0}'\njobs:\n",
    "the workflow sets `defaults.run`"
)]
#[case::job_default_shell(
    "  coverage:\n",
    "  coverage:\n    defaults:\n      run:\n        shell: bash -c 'exit 0; {0}'\n",
    "a job sets `defaults.run`"
)]
#[case::workflow_scope_granted(
    "permissions: {}\n",
    "permissions:\n  contents: write\n",
    "workflow permissions"
)]
#[case::workflow_scope_undeclared("permissions: {}\n", "", "workflow permissions")]
#[case::check_without_an_id(
    "      - id: codescene-token\n        run:",
    "      - run:",
    "has no id"
)]
#[case::check_command_changed(
    "secrets.CS_ACCESS_TOKEN != ''",
    "true",
    "exactly one token check step"
)]
#[case::check_deleted(
    concat!(
        "      - id: codescene-token\n        run: echo \"available=${{ secrets.CS_ACCESS_TOKEN != '' ",
        "}}\" >> \"$GITHUB_OUTPUT\"\n"
    ),
    "",
    "exactly one token check step"
)]
#[case::guard_on_the_environment(
    "steps.codescene-token.outputs.available == 'true'",
    "env.CS_ACCESS_TOKEN != ''",
    "not guarded"
)]
#[case::guard_on_another_step(
    "steps.codescene-token.outputs.available == 'true'",
    "steps.other.outputs.available == 'true'",
    "not guarded"
)]
fn the_token_check_is_exact(
    #[case] from: &str,
    #[case] to: &str,
    #[case] expected: &str,
) -> Result<()> {
    let complying = publisher(NEVER_CANCEL, GUARD, "");
    let source = mutate_once(&complying, from, to)?;
    let findings = rules::publisher_findings(&parse(&source)?);
    ensure!(
        findings.iter().any(|f| f.contains(expected)),
        "expected a finding naming {expected:?}, saw {findings:?}"
    );
    Ok(())
}
