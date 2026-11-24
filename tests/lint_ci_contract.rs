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
use rstest::rstest;

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

/// A single-job workflow whose body is `body`.
fn single_job(body: &str) -> String {
    workflow_with(&[("build-test", body)])
}

/// Verifies that the committed workflow satisfies the whole contract.
#[test]
fn ci_runs_the_aggregate_lint_with_a_pinned_installer() {
    if let Err(report) = verify_lint_gate(CI) {
        panic!("CI must hold the lint-gate contract: {report}");
    }
}

/// Every way a lint job can be provisioned or narrowed wrongly is rejected.
///
/// Each case is a whole workflow, so the verifier sees exactly what a reviewer
/// would: the shape that must fail, with no fixture hiding it.
#[rstest]
#[case::no_installer(LINT_STEP)]
#[case::installer_after_lint(
    concat!(
        "      - name: Lint\n        run: make lint\n",
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
    )
)]
#[case::lint_step_if(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        if: false\n        run: make lint\n",
    )
)]
#[case::installer_continue_on_error(
    concat!(
        "      - name: Install Whitaker\n",
        "        continue-on-error: true\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        run: make lint\n",
    )
)]
#[case::make_argument(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        run: make lint LINT_PACKAGES=--package theoremc\n",
    )
)]
#[case::step_env(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        env:\n          CARGO_FLAGS: --package theoremc\n",
        "        run: make lint\n",
    )
)]
#[case::step_env_after_blank_line(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        env:\n\n          CARGO_FLAGS: --package theoremc\n",
        "        run: make lint\n",
    )
)]
#[case::step_env_annotated_header(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        env:  # lint settings\n",
        "          LINT_PACKAGES: --package theoremc\n        run: make lint\n",
    )
)]
#[case::step_env_quoted_key(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
        "      - name: Lint\n        env:\n          'LINT_PACKAGES': --package theoremc\n",
        "        run: make lint\n",
    )
)]
#[case::rolling_installer(
    concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@main\n",
        "      - name: Lint\n        run: make lint\n",
    )
)]
#[case::block_scalar_lint("      - name: Lint\n        run: |\n          make lint\n")]
fn an_invalid_lint_job_is_rejected(#[case] body: &str) {
    let workflow = single_job(body);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "this job must be rejected:\n{body}"
    );
}

/// A job-level bypass key is rejected wherever it appears.
#[rstest]
#[case::before_steps("jobs:\n  build-test:\n    if: false\n    steps:\n", "")]
#[case::after_steps("jobs:\n  build-test:\n    steps:\n", "    if: false\n")]
fn a_bypass_key_on_the_job_is_rejected(#[case] skeleton: &str, #[case] trailer: &str) {
    let workflow = format!("{skeleton}{INSTALL_STEP}{LINT_STEP}{trailer}");
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a job-level if: must be rejected"
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

/// A normal lint job cannot mask a second job that runs the aggregate in a
/// block scalar without an installer.
#[test]
fn a_block_scalar_lint_job_cannot_hide_behind_a_normal_one() {
    let protected = format!("{INSTALL_STEP}{LINT_STEP}");
    let workflow = workflow_with(&[
        ("build-test", &protected),
        (
            "second",
            "      - name: Lint\n        run: |\n          make lint\n",
        ),
    ]);
    assert!(
        verify_lint_gate(&workflow).is_err(),
        "a block-scalar lint job without an installer must be rejected"
    );
}

/// The inline `- uses:` form installs the suite just as the block form does.
#[test]
fn an_inline_uses_step_satisfies_the_installer_requirement() {
    let body = format!(
        "      - uses: leynos/shared-actions/.github/actions/install-whitaker@0eabb6bc07148981de5da395e30cd78d83f5c7c2\n{LINT_STEP}"
    );
    let workflow = single_job(&body);
    if let Err(report) = verify_lint_gate(&workflow) {
        panic!("an inline uses step must satisfy the installer requirement: {report}");
    }
}

/// A job's own `steps:` mapping is found after a matrix list, and a step list
/// at the key's own indentation is read as readily as an indented one.
#[rstest]
#[case::matrix_before_steps(
    "jobs:\n  build-test:\n    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        include:\n          - os: ubuntu-latest\n    steps:\n"
)]
#[case::list_at_key_indentation("jobs:\n  build-test:\n    runs-on: ubuntu-latest\n    steps:\n")]
fn a_valid_lint_job_is_accepted(#[case] skeleton: &str) {
    let workflow = if skeleton.contains("matrix") {
        format!("{skeleton}{INSTALL_STEP}{LINT_STEP}")
    } else {
        format!(
            "{skeleton}    - name: Install Whitaker\n      uses: leynos/shared-actions/.github/actions/install-whitaker@0eabb6bc07148981de5da395e30cd78d83f5c7c2\n    - name: Lint\n      run: make lint\n"
        )
    };
    if let Err(report) = verify_lint_gate(&workflow) {
        panic!("a valid lint job must be accepted: {report}");
    }
}
