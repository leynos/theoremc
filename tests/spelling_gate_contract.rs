//! Contract tests for the Concordat spelling gate.
//!
//! The gate is one pinned command in the Makefile, run by CI in the
//! `build-test` job after Markdown lint and before the Whitaker install. These
//! tests read the wiring as text, so a change to the pin, the scope, or the
//! job-scoped ordering fails here rather than in a later review.

mod spelling_gate_contract_support;

#[path = "spelling_gate_contract/property.rs"]
mod property;

use spelling_gate_contract_support::{
    CI, JobName, MAKEFILE, Makefile, PINNED_RECIPE, StepName, TargetName, VariableName, Workflow,
    build_test_job, build_test_step, default_assignment, has_bypass_key, job_lines, key_indent,
    recipe_lines, runs_exact_command, spelling_runs_between_markdown_lint_and_whitaker, step_index,
    step_run_line, variable_definition,
};

const TYPOS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/typos.toml"));
const LOCAL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/typos.local.toml"));

/// The `build-test` job header used by the synthetic workflows.
///
/// The shared fragments live here so each case states only the lines that
/// distinguish it; repeating whole YAML blocks would itself be duplication.
const BUILD_TEST_HEADER: &str = "jobs:\n  build-test:\n    steps:\n";

/// The Spelling step's `- name:` line.
const SPELLING_NAME: &str = "      - name: Spelling\n";

/// The Spelling step's exact `run:` line.
const SPELLING_RUN: &str = "        run: make spelling\n";

/// The Markdown lint step, as the ordering cases spell it.
const MARKDOWN_LINT_STEP: &str = "      - name: Markdown lint\n        run: make markdownlint\n";

/// The Whitaker install step, as the ordering cases spell it.
const WHITAKER_STEP: &str = "      - name: Install Whitaker\n        run: make install-whitaker\n";

/// Returns the `build-test` job's ordering verdict.
fn ordering_holds(workflow: &str) -> Option<bool> {
    let job = build_test_job(Workflow(workflow))?;
    Some(spelling_runs_between_markdown_lint_and_whitaker(&job))
}

/// Reports whether the `build-test` job carries a job-level bypass key.
fn job_bypass_key(workflow: &str) -> Option<bool> {
    let job = build_test_job(Workflow(workflow))?;
    let indent = key_indent(&job)?;
    Some(has_bypass_key(&job, indent))
}

/// Reports whether the Spelling step carries a step-level bypass key.
fn step_bypass_key(workflow: &str) -> Option<bool> {
    let (step, indent) = build_test_step(Workflow(workflow), StepName("Spelling"))?;
    Some(has_bypass_key(&step, indent))
}

/// Returns the Spelling step's own `run:` command.
fn step_run_command(workflow: &str) -> Option<&str> {
    let (step, indent) = build_test_step(Workflow(workflow), StepName("Spelling"))?;
    step_run_line(&step, indent).map(|line| line.trimmed())
}

#[test]
fn spelling_target_runs_the_pinned_gate() {
    let command = variable_definition(Makefile(MAKEFILE), VariableName("SPELLING_COMMAND"))
        .expect("SPELLING_COMMAND must be defined");
    for fragment in [
        "$(SPELLING_BUILDER)",
        "@$(SPELLING_BUILDER_REF)",
        "--python 3.14",
        "typos-config-builder gate",
        // Quoting keeps a `UV` override containing a space a single argument.
        "\"$(UV)\"",
        // Quoting keeps an override containing a space a single argument.
        "--repository \"$(SPELLING_REPOSITORY)\"",
        "--scope $(SPELLING_SCOPE)",
    ] {
        assert!(
            command.contains(fragment),
            "SPELLING_COMMAND must contain {fragment:?}, got {command:?}"
        );
    }
    assert_eq!(
        default_assignment(Makefile(MAKEFILE), VariableName("SPELLING_BUILDER")),
        Some("git+https://github.com/leynos/typos-config-builder.git"),
        "the builder must default to the pinned HTTPS URL"
    );
    assert_eq!(
        default_assignment(Makefile(MAKEFILE), VariableName("SPELLING_BUILDER_REF")),
        Some("v0.1.3"),
        "the builder revision must default to the adopted pin"
    );
    assert_eq!(
        default_assignment(Makefile(MAKEFILE), VariableName("SPELLING_REPOSITORY")),
        Some("."),
        "the spelling gate must default to this repository"
    );
    assert_eq!(
        default_assignment(Makefile(MAKEFILE), VariableName("SPELLING_SCOPE")),
        Some("all"),
        "the spelling gate must default to the full scope"
    );
    assert_eq!(
        recipe_lines(Makefile(MAKEFILE), TargetName("spelling")),
        vec![PINNED_RECIPE],
        "the spelling target must run SPELLING_COMMAND"
    );
}

/// The spelling gate runs after Markdown lint and before the Whitaker install.
///
/// Spelling is the cheap gate: it should fail fast, before CI spends minutes
/// installing Whitaker. Both it and Markdown lint sit in the `build-test` job.
#[test]
fn ci_runs_the_spelling_gate_between_markdown_lint_and_whitaker() {
    let job = build_test_job(Workflow(CI)).expect("CI must define the build-test job");
    assert!(
        spelling_runs_between_markdown_lint_and_whitaker(&job),
        "the spelling gate must run after Markdown lint and before the Whitaker install"
    );

    let (step, step_indent) = build_test_step(Workflow(CI), StepName("Spelling"))
        .expect("the Spelling step must have a body");
    assert!(
        runs_exact_command(&step, step_indent),
        "the Spelling step must run exactly `make spelling`"
    );
    assert!(
        !has_bypass_key(&step, step_indent),
        "the Spelling step must not be conditional or non-fatal"
    );
    let job_indent = key_indent(&job).expect("the build-test job must have a body");
    assert!(
        !has_bypass_key(&job, job_indent),
        "the build-test job must not be conditional or non-fatal"
    );
}

/// Job extraction stops at the next job and ignores that job's text.
///
/// The synthetic job line carries a trailing comment and the body holds a
/// column-zero comment, both of which must be tolerated.
#[test]
fn job_lines_stop_at_the_next_job() {
    let workflow = format!(
        "jobs:\n  build-test:  # the job under contract\n    steps:\n# a column-zero comment does not end the job\n\n{SPELLING_NAME}{SPELLING_RUN}\n  other:\n    steps:\n{SPELLING_NAME}{SPELLING_RUN}"
    );
    assert_eq!(
        step_run_command(&workflow).expect("the build-test job must define the Spelling step"),
        "run: make spelling",
        "job extraction must find the step inside its own job"
    );
    assert!(
        job_lines(Workflow(&workflow), JobName("missing")).is_none(),
        "an absent job must not be extracted"
    );
}

/// A conditional or non-fatal step is detected before it can silence the gate.
#[test]
fn bypass_keys_are_detected_in_a_step() {
    let workflow = format!(
        "{BUILD_TEST_HEADER}{SPELLING_NAME}        continue-on-error: true\n{SPELLING_RUN}"
    );
    assert!(
        step_bypass_key(&workflow).expect("the synthetic job must define the Spelling step"),
        "continue-on-error must be reported as a bypass key"
    );
}

/// A job-level bypass key is found even when it follows the `steps:` mapping.
#[test]
fn job_bypass_keys_are_found_at_any_position() {
    let after_steps = format!("{BUILD_TEST_HEADER}{SPELLING_NAME}{SPELLING_RUN}    if: false\n");
    assert!(
        job_bypass_key(&after_steps).expect("the synthetic workflow must define the job"),
        "a job-level if: after steps: must be detected"
    );

    let before_steps =
        format!("jobs:\n  build-test:\n    if: false\n    steps:\n{SPELLING_NAME}{SPELLING_RUN}");
    assert!(
        job_bypass_key(&before_steps).expect("the synthetic workflow must define the job"),
        "a job-level if: before steps: must be detected"
    );
}

/// A nested `run:` key does not satisfy the step's own command contract.
#[test]
fn nested_run_keys_do_not_satisfy_the_step_command() {
    let workflow = format!(
        "{BUILD_TEST_HEADER}{SPELLING_NAME}        env:\n          run: make spelling\n        run: echo skipped\n"
    );
    assert_eq!(
        step_run_command(&workflow).expect("the synthetic job must define the Spelling step"),
        "run: echo skipped",
        "the step's own run key must be the one inspected"
    );
}

/// A conditional Spelling step is a bypass even when the command is exact.
#[test]
fn if_on_the_spelling_step_is_a_bypass() {
    let workflow = format!("{BUILD_TEST_HEADER}{SPELLING_NAME}        if: false\n{SPELLING_RUN}");
    assert!(
        step_bypass_key(&workflow).expect("the synthetic job must define the Spelling step"),
        "a step-level if: must be reported as a bypass key"
    );
}

/// Ordinary keys, including a nested `if:`, are not bypass keys.
#[test]
fn ordinary_step_keys_are_not_bypasses() {
    let workflow = format!(
        "{BUILD_TEST_HEADER}{SPELLING_NAME}{SPELLING_RUN}        with:\n          if: false\n        env:\n          continue-on-error: true\n"
    );
    assert!(
        !step_bypass_key(&workflow).expect("the synthetic job must define the Spelling step"),
        "only if: and continue-on-error: at the step's own indentation are bypasses"
    );
}

/// Only the exact command satisfies the contract.
#[test]
fn only_the_exact_command_satisfies_the_contract() {
    let variants = [
        ("run: make spelling", true),
        ("run: make spelling --dry-run", false),
        ("run: make spelling || true", false),
        ("# run: make spelling", false),
        ("run: make spelling # a comment", false),
        ("run:  make spelling", false),
    ];
    for (command, expected) in variants {
        let workflow = format!("{BUILD_TEST_HEADER}{SPELLING_NAME}        {command}\n");
        let (step, indent) = build_test_step(Workflow(&workflow), StepName("Spelling"))
            .expect("the synthetic job must define the Spelling step");
        assert_eq!(
            runs_exact_command(&step, indent),
            expected,
            "the contract must treat {command:?} as expected={expected}"
        );
    }
}

/// The ordering predicate rejects a reversed or incomplete job.
#[test]
fn spelling_must_follow_markdown_lint_and_precede_whitaker() {
    let reversed = format!(
        "{BUILD_TEST_HEADER}{WHITAKER_STEP}{SPELLING_NAME}{SPELLING_RUN}{MARKDOWN_LINT_STEP}"
    );
    assert!(
        !ordering_holds(&reversed).expect("the synthetic workflow must define the build-test job"),
        "a reversed order must not satisfy the contract"
    );

    let missing_whitaker =
        format!("{BUILD_TEST_HEADER}{MARKDOWN_LINT_STEP}{SPELLING_NAME}{SPELLING_RUN}");
    assert!(
        !ordering_holds(&missing_whitaker)
            .expect("the synthetic workflow must define the build-test job"),
        "a missing Whitaker install must not satisfy the contract"
    );

    let other_job_has_the_order = format!(
        "jobs:\n  other:\n    steps:\n{MARKDOWN_LINT_STEP}{SPELLING_NAME}{SPELLING_RUN}{WHITAKER_STEP}  build-test:\n    steps:\n{WHITAKER_STEP}{SPELLING_NAME}{SPELLING_RUN}{MARKDOWN_LINT_STEP}"
    );
    assert!(
        !ordering_holds(&other_job_has_the_order)
            .expect("the synthetic workflow must define the build-test job"),
        "a correctly ordered step list in another job must not satisfy build-test"
    );
}

/// A duplicated step name is rejected rather than silently resolved.
#[test]
#[should_panic(expected = "exactly one")]
fn duplicate_step_names_are_rejected() {
    let workflow =
        format!("{BUILD_TEST_HEADER}{SPELLING_NAME}{SPELLING_RUN}{SPELLING_NAME}{SPELLING_RUN}");
    let job = build_test_job(Workflow(&workflow))
        .expect("the synthetic workflow must define the build-test job");
    let _ = step_index(&job, StepName("Spelling"));
}

#[test]
fn generated_configuration_is_committed() {
    assert!(
        TYPOS.starts_with("# Generated from the shared en-GB-oxendict dictionary."),
        "typos.toml must be the generated file, not a handwritten one"
    );
    assert!(
        LOCAL.contains("schema = 1"),
        "typos.local.toml must declare the overlay schema"
    );
}
