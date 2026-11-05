//! Property tests for the workflow text parsers.
//!
//! The contract tests pin the committed workflow. These properties exercise
//! the parsers over generated job boundaries, comments, blank lines, step
//! permutations, and bypass-key placements, so a parser cannot be weakened
//! while the committed workflow still passes.

use proptest::prelude::{any, prop_assert, prop_assert_eq, prop_assume, proptest};

use super::spelling_gate_contract_support::{
    JobName, StepName, Workflow, has_bypass_key, job_lines, key_indent,
    spelling_runs_between_markdown_lint_and_whitaker, step_index, step_lines, step_run_line,
};

/// Builds a job name from a generated index.
fn job_name(index: u8) -> String {
    format!("job-{index}")
}

/// Builds a step name from a generated index.
fn step_name(index: u8) -> String {
    format!("step-{index}")
}

/// Returns one of the six orderings of the three contract steps.
const fn permutation(index: u8) -> [&'static str; 3] {
    match index {
        0 => ["Markdown lint", "Spelling", "Install Whitaker"],
        1 => ["Markdown lint", "Install Whitaker", "Spelling"],
        2 => ["Spelling", "Markdown lint", "Install Whitaker"],
        3 => ["Spelling", "Install Whitaker", "Markdown lint"],
        4 => ["Install Whitaker", "Markdown lint", "Spelling"],
        _ => ["Install Whitaker", "Spelling", "Markdown lint"],
    }
}

/// The shape of a two-job workflow to generate.
#[derive(Clone, Copy)]
struct TwoJobs {
    first: u8,
    second: u8,
    first_step: u8,
    second_step: u8,
    blanks: u8,
    comment: bool,
}

/// Builds a workflow with two jobs, each holding one differently named step.
fn two_jobs(spec: TwoJobs) -> String {
    let first_job = job_name(spec.first);
    let second_job = job_name(spec.second);
    let first_step = step_name(spec.first_step);
    let second_step = step_name(spec.second_step);
    let blanks = "\n".repeat(usize::from(spec.blanks));
    let comment = if spec.comment {
        "# a column-zero comment\n"
    } else {
        ""
    };
    format!(
        "jobs:\n  {first_job}:\n    steps:\n{blanks}{comment}      - name: {first_step}\n        run: make spelling\n  {second_job}:\n    steps:\n      - name: {second_step}\n        run: make spelling\n"
    )
}

proptest! {
    /// Job extraction returns only the named job's lines.
    #[test]
    fn job_extraction_is_scoped_to_the_named_job(
        first in 0u8..8,
        second in 0u8..8,
        first_step in 0u8..8,
        second_step in 0u8..8,
        blanks in 0u8..3,
        comment in any::<bool>(),
    ) {
        prop_assume!(first != second);
        prop_assume!(first_step != second_step);
        let workflow = two_jobs(TwoJobs {
            first,
            second,
            first_step,
            second_step,
            blanks,
            comment,
        });
        let first_job = job_name(first);
        let second_job = job_name(second);
        let first_step_name = step_name(first_step);
        let second_step_name = step_name(second_step);

        let job = job_lines(Workflow(&workflow), JobName(&first_job))
            .expect("the first job must be extracted");
        prop_assert!(step_index(&job, StepName(&first_step_name)).is_some());
        prop_assert!(step_index(&job, StepName(&second_step_name)).is_none());
        prop_assert!(job_lines(Workflow(&workflow), JobName(&second_job)).is_some());
        prop_assert!(job_lines(Workflow(&workflow), JobName("absent")).is_none());
    }

    /// A bypass key is detected only at the indentation that owns it.
    #[test]
    fn bypass_keys_are_detected_only_at_their_own_indentation(
        key_index in 0u8..2,
        placement in 0u8..3,
        value_index in 0u8..3,
    ) {
        let bypass_key = match key_index {
            0 => "if:",
            _ => "continue-on-error:",
        };
        let bypass_value = match value_index {
            0 => "true",
            1 => "false",
            _ => "${{ false }}",
        };
        let base =
            "jobs:\n  build-test:\n    steps:\n      - name: Spelling\n        run: make spelling\n";
        let suffix = match placement {
            0 => format!("        {bypass_key} {bypass_value}\n"),
            1 => format!("    {bypass_key} {bypass_value}\n"),
            _ => format!("        env:\n          {bypass_key} {bypass_value}\n"),
        };
        let workflow = format!("{base}{suffix}");

        let job = job_lines(Workflow(&workflow), JobName("build-test"))
            .expect("the synthetic workflow must define the build-test job");
        let step = step_lines(&job, StepName("Spelling"))
            .expect("the synthetic job must define the Spelling step");
        let job_indent = key_indent(&job).expect("the build-test job must have a body");
        let step_indent = key_indent(step).expect("the Spelling step must have a body");

        prop_assert_eq!(has_bypass_key(step, step_indent), placement == 0);
        prop_assert_eq!(has_bypass_key(&job, job_indent), placement == 1);
    }

    /// Step extraction stops at the next step, tolerating comments and blanks.
    #[test]
    fn step_extraction_stops_at_the_next_step(
        first_index in 0u8..8,
        second_index in 0u8..8,
        blanks in 0u8..3,
        comment in any::<bool>(),
    ) {
        prop_assume!(first_index != second_index);
        let first_name = step_name(first_index);
        let second_name = step_name(second_index);
        let blank_lines = "\n".repeat(usize::from(blanks));
        let comment_line = if comment {
            "# a comment inside the first step\n"
        } else {
            ""
        };
        let workflow = format!(
            "jobs:\n  build-test:\n    steps:\n      - name: {first_name}\n        run: make spelling\n{blank_lines}{comment_line}      - name: {second_name}\n        run: make other\n"
        );

        let job = job_lines(Workflow(&workflow), JobName("build-test"))
            .expect("the synthetic workflow must define the build-test job");
        let first_step = step_lines(&job, StepName(&first_name))
            .expect("the first step must be extracted");
        let indent = key_indent(first_step).expect("the first step must have a body");
        let run = step_run_line(first_step, indent).expect("the first step must define run");
        prop_assert_eq!(run.trimmed(), "run: make spelling");
        prop_assert!(step_lines(&job, StepName(&second_name)).is_some());
    }

    /// The ordering predicate accepts exactly the required permutation.
    #[test]
    fn ordering_accepts_only_the_required_permutation(index in 0u8..6) {
        let [first, second, third] = permutation(index);
        let workflow = format!(
            "jobs:\n  build-test:\n    steps:\n      - name: {first}\n        run: make first\n      - name: {second}\n        run: make second\n      - name: {third}\n        run: make third\n"
        );
        let job = job_lines(Workflow(&workflow), JobName("build-test"))
            .expect("the synthetic workflow must define the build-test job");
        let expected = first == "Markdown lint"
            && second == "Spelling"
            && third == "Install Whitaker";
        prop_assert_eq!(
            spelling_runs_between_markdown_lint_and_whitaker(&job),
            expected
        );
    }

    /// A duplicated contract step is rejected by the ordering predicate.
    #[test]
    fn duplicated_contract_steps_are_rejected(
        blanks in 0u8..3,
        comment in any::<bool>(),
    ) {
        let blank_lines = "\n".repeat(usize::from(blanks));
        let comment_line = if comment {
            "# a duplicate Spelling step\n"
        } else {
            ""
        };
        let workflow = format!(
            "jobs:\n  build-test:\n    steps:\n      - name: Markdown lint\n        run: make markdownlint\n      - name: Spelling\n        run: make spelling\n{blank_lines}{comment_line}      - name: Spelling\n        run: make spelling\n      - name: Install Whitaker\n        run: make install-whitaker\n"
        );
        let job = job_lines(Workflow(&workflow), JobName("build-test"))
            .expect("the synthetic workflow must define the build-test job");
        prop_assert!(!spelling_runs_between_markdown_lint_and_whitaker(&job));
    }
}
