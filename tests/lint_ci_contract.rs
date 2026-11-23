//! Contract tests for the CI side of the workspace-wide lint gate.
//!
//! The workflow must install a pinned Whitaker and then run `make lint`, the
//! same aggregate target contributors run locally. Every job that runs the
//! aggregate needs its own pinned install, because a second job would otherwise
//! run the gate without the suite it is meant to exercise. A lint step must
//! carry no arguments and no environment override that could narrow the gate,
//! and no `if:` or `continue-on-error:` may let it skip or fail softly. A
//! workflow-level `env:` reaches every step, so it is checked too.
//!
//! The tests read the workflow as text, so a gate cannot be provisioned
//! differently from the way it is documented or quietly widened.
//!
//! `tests/lint_dylint_contract.rs` holds the configuration side,
//! `tests/lint_makefile_contract.rs` holds the recipes, and
//! `tests/lint_gate_execution.rs` proves the recipes really run.

#[path = "lint_ci_contract_support/mod.rs"]
mod lint_ci_contract_support;

use lint_ci_contract_support::{CI, verify_lint_gate};

/// A pinned Whitaker install step, as the synthetic jobs spell it.
const INSTALL_STEP: &str = concat!(
    "      - name: Install Whitaker\n",
    "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
    "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
);

/// The aggregate lint step, as the synthetic jobs spell it.
const LINT_STEP: &str = "      - name: Lint\n        run: make lint\n";

/// Wraps job bodies in the workflow skeleton the synthetic cases share.
fn workflow_with(job_bodies: &[(&str, &str)]) -> String {
    job_bodies
        .iter()
        .fold(String::from("jobs:\n"), |mut text, (name, body)| {
            text.push_str("  ");
            text.push_str(name);
            text.push_str(":\n    runs-on: ubuntu-latest\n    steps:\n");
            text.push_str(body);
            text
        })
}

/// Verifies that the committed workflow satisfies the whole contract.
#[test]
fn ci_runs_the_aggregate_lint_with_a_pinned_installer() {
    if let Err(report) = verify_lint_gate(CI) {
        panic!("CI must hold the lint-gate contract: {report}");
    }
}

/// A job that never installs Whitaker cannot run the aggregate.
#[test]
fn a_missing_installer_is_rejected() {
    let workflow = workflow_with(&[("build-test", LINT_STEP)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a lint step without an installer must be rejected"
    );
}

/// An installer placed after the lint step is not a provisioning step.
#[test]
fn an_installer_after_the_lint_step_is_rejected() {
    let body = format!("{LINT_STEP}{INSTALL_STEP}");
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "an installer after the lint step must be rejected"
    );
}

/// A conditional lint step is a bypass even when the command is exact.
#[test]
fn a_bypass_key_on_the_lint_step_is_rejected() {
    let body =
        format!("{INSTALL_STEP}      - name: Lint\n        if: false\n        run: make lint\n");
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a step-level if: must be rejected"
    );
}

/// A non-fatal installer could hide a missing suite.
#[test]
fn a_bypass_key_on_the_installer_step_is_rejected() {
    let body = format!(
        "      - name: Install Whitaker\n        continue-on-error: true\n        uses: leynos/shared-actions/.github/actions/install-whitaker@0eabb6bc07148981de5da395e30cd78d83f5c7c2\n{LINT_STEP}"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a continue-on-error: on the installer must be rejected"
    );
}

/// A conditional job could skip the whole gate.
#[test]
fn a_bypass_key_on_the_job_is_rejected() {
    let mut workflow = String::from("jobs:\n  build-test:\n    if: false\n    steps:\n");
    workflow.push_str(INSTALL_STEP);
    workflow.push_str(LINT_STEP);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a job-level if: must be rejected"
    );
}

/// Every job that runs the aggregate needs its own pinned install.
#[test]
fn a_second_unprotected_lint_job_is_rejected() {
    let protected = format!("{INSTALL_STEP}{LINT_STEP}");
    let workflow = workflow_with(&[("build-test", &protected), ("second", LINT_STEP)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a second job running make lint without an installer must be rejected"
    );
}

/// A command-line variable would override the Makefile default.
#[test]
fn a_make_argument_override_is_rejected() {
    let body = format!(
        "{INSTALL_STEP}      - name: Lint\n        run: make lint LINT_PACKAGES=--package theoremc\n"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a make argument override must be rejected"
    );
}

/// A step-level environment override would narrow the gate.
#[test]
fn a_step_environment_override_is_rejected() {
    let body = format!(
        "{INSTALL_STEP}      - name: Lint\n        env:\n          CARGO_FLAGS: --package theoremc\n        run: make lint\n"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a step-level env override must be rejected"
    );
}

/// A blank line inside an `env:` mapping does not end the block.
#[test]
fn a_blank_line_does_not_hide_a_step_environment_override() {
    let body = format!(
        "{INSTALL_STEP}      - name: Lint\n        env:\n\n          CARGO_FLAGS: --package theoremc\n        run: make lint\n"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a blank line must not hide a step-level env override"
    );
}

/// A job-level environment override would narrow every step it covers.
#[test]
fn a_job_environment_override_is_rejected() {
    let workflow = format!(
        "jobs:\n  build-test:\n    env:\n      RUSTDOC_FLAGS: ''\n    steps:\n{INSTALL_STEP}{LINT_STEP}"
    );
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a job-level env override must be rejected"
    );
}

/// A workflow-level environment override reaches the aggregate as well.
#[test]
fn a_workflow_environment_override_is_rejected() {
    let workflow = format!(
        "env:\n  LINT_PACKAGES: --package theoremc\njobs:\n  build-test:\n    runs-on: ubuntu-latest\n    steps:\n{INSTALL_STEP}{LINT_STEP}"
    );
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a workflow-level env override must be rejected"
    );
}

/// The installer must name a full commit SHA, not a rolling reference.
#[test]
fn a_rolling_installer_reference_is_rejected() {
    let body = format!(
        "      - name: Install Whitaker\n        uses: leynos/shared-actions/.github/actions/install-whitaker@main\n{LINT_STEP}"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a rolling installer reference must be rejected"
    );
}

/// The inline `- uses:` form installs the suite just as the block form does.
#[test]
fn an_inline_uses_step_satisfies_the_installer_requirement() {
    let body = format!(
        "      - uses: leynos/shared-actions/.github/actions/install-whitaker@0eabb6bc07148981de5da395e30cd78d83f5c7c2\n{LINT_STEP}"
    );
    let workflow = workflow_with(&[("build-test", &body)]);
    if let Err(report) = verify_lint_gate(&workflow) {
        panic!("an inline uses step must satisfy the installer requirement: {report}");
    }
}

/// A job's own `steps:` mapping is found even after a matrix list.
#[test]
fn a_matrix_list_before_steps_does_not_hide_the_gate() {
    let workflow = format!(
        "jobs:\n  build-test:\n    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        include:\n          - os: ubuntu-latest\n    steps:\n{INSTALL_STEP}{LINT_STEP}"
    );
    if let Err(report) = verify_lint_gate(&workflow) {
        panic!("a matrix list before steps must not hide the lint gate: {report}");
    }
}
