//! The verifier that holds a workflow to the lint-gate contract.
//!
//! The readers in the parent module turn text into steps, commands, and
//! environment keys; this module decides whether what they found satisfies the
//! contract, and names the first drift it can prove.

use super::environment::environment_keys;
use super::workflow_contract_support::has_sequence_entry;
use super::{
    COVERAGE_VARIABLES, LINT_COMMAND, Line, MAKEFLAGS_VARIABLES, WHITAKER_INSTALLER, Workflow,
    has_bypass_key, invokes_aggregate, jobs, key_indent, run_command, steps, uses_reference,
};

/// Fails when `keys` assign a variable that would narrow the lint gate.
///
/// # Errors
///
/// Returns the offending variable and the block that assigned it.
fn reject_coverage_override(scope: &str, keys: &[&str]) -> Result<(), String> {
    keys.iter()
        .find(|key| {
            COVERAGE_VARIABLES.contains(key) || MAKEFLAGS_VARIABLES.contains(key)
        })
        .map_or(Ok(()), |key| {
            Err(format!(
                "{scope} must not set `{key}`, because it would redirect the lint stage away from the workspace"
            ))
        })
}

/// Fails unless an installer reference pins a full commit SHA.
///
/// # Errors
///
/// Returns the reference and pin when the pin is not 40 hexadecimal digits.
fn check_installer_pin(job: &str, reference: &str) -> Result<(), String> {
    let pin = reference
        .strip_prefix(WHITAKER_INSTALLER)
        .ok_or_else(|| format!("job `{job}` must install Whitaker through the shared action"))?;
    let sha = pin
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("job `{job}` must pin the Whitaker action to a commit SHA"))?;
    let is_sha = sha.len() == 40 && sha.chars().all(|character| character.is_ascii_hexdigit());
    if is_sha {
        Ok(())
    } else {
        Err(format!(
            "the Whitaker action in job `{job}` must be pinned to a full commit SHA; found `{sha}`"
        ))
    }
}

/// A step that runs the aggregate lint target.
struct LintStep<'a, 'b> {
    /// The job that owns the step.
    job: &'a str,
    /// Where the step starts, so a reader can find the steps before it.
    start: usize,
    /// The step's own lines.
    lines: &'a [Line<'b>],
    /// The indentation of the step's own keys.
    indent: usize,
    /// The command the step's `run:` key carries.
    command: String,
}

/// Collects the steps of `job` that invoke the aggregate lint target.
///
/// # Errors
///
/// Returns the reason when a step's command cannot be read, so an unreadable
/// command cannot hide a second job's aggregate.
fn lint_steps<'a, 'b>(name: &'a str, job: &'a [Line<'b>]) -> Result<Vec<LintStep<'a, 'b>>, String> {
    let mut found = Vec::new();
    for (start, step) in steps(job) {
        let Some(indent) = key_indent(step) else {
            continue;
        };
        let Some(command) = run_command(step, indent)? else {
            continue;
        };
        if invokes_aggregate(&command) {
            found.push(LintStep {
                job: name,
                start,
                lines: step,
                indent,
                command,
            });
        }
    }
    Ok(found)
}

/// Fails unless the job carrying the lint step is unconditional.
///
/// # Errors
///
/// Returns the job-level bypass key or coverage override.
fn verify_lint_job(name: &str, job: &[Line<'_>], job_indent: usize) -> Result<(), String> {
    // A `- if:` at the job's own indentation belongs to an indentless step
    // list, and the step checks cover it, so the job scan skips it.
    if has_sequence_entry(job, job_indent) {
        let job_keys: Vec<Line<'_>> = job
            .iter()
            .filter(|line| line.indent() == job_indent && !line.trimmed().starts_with("- "))
            .copied()
            .collect();
        if has_bypass_key(&job_keys, job_indent) {
            return Err(format!(
                "job `{name}` must not make the lint gate conditional or non-fatal"
            ));
        }
    } else if has_bypass_key(job, job_indent) {
        return Err(format!(
            "job `{name}` must not make the lint gate conditional or non-fatal"
        ));
    }
    reject_coverage_override(
        &format!("job `{name}`"),
        &environment_keys(job, job_indent).map_err(|error| format!("job `{name}`: {error}"))?,
    )
}

/// Fails unless the installer before `lint` is pinned and unconditional.
///
/// # Errors
///
/// Returns the missing, out-of-order, unpinned, bypassed, or overridden
/// installer.
fn verify_installer(
    job_steps: &[(usize, &[Line<'_>])],
    lint: &LintStep<'_, '_>,
) -> Result<(), String> {
    let name = lint.job;
    let (installer, indent) = find_installer(job_steps, lint.start).ok_or_else(|| {
        format!("job `{name}` must install a pinned Whitaker before its lint step")
    })?;
    if has_bypass_key(installer, indent) {
        return Err(format!(
            "the Whitaker install in job `{name}` must not be conditional or non-fatal"
        ));
    }
    let keys = environment_keys(installer, indent)
        .map_err(|error| format!("the Whitaker install in job `{name}`: {error}"))?;
    reject_coverage_override(&format!("the Whitaker install in job `{name}`"), &keys)?;
    check_installer_pin(name, uses_reference(installer, indent).unwrap_or_default())
}

/// Fails unless one lint step is the argument-free, unbypassed aggregate.
///
/// # Errors
///
/// Returns the offending command, bypass key, or environment override.
fn verify_lint_step(
    job: &str,
    step: &[Line<'_>],
    indent: usize,
    command: &str,
) -> Result<(), String> {
    if command != LINT_COMMAND {
        return Err(format!(
            "the lint step in job `{job}` must run exactly `{LINT_COMMAND}`; found `{command}`"
        ));
    }
    if has_bypass_key(step, indent) {
        return Err(format!(
            "the lint step in job `{job}` must not be conditional or non-fatal"
        ));
    }
    let keys = environment_keys(step, indent)
        .map_err(|error| format!("the lint step in job `{job}`: {error}"))?;
    reject_coverage_override(&format!("the lint step in job `{job}`"), &keys)
}

/// Returns the Whitaker installer step that precedes `lint_start`.
fn find_installer<'a, 'b>(
    job_steps: &'a [(usize, &'a [Line<'b>])],
    lint_start: usize,
) -> Option<(&'a [Line<'b>], usize)> {
    job_steps
        .iter()
        .take_while(|(start, _)| *start < lint_start)
        .find_map(|(_, step)| {
            let indent = key_indent(step)?;
            uses_reference(step, indent)
                .is_some_and(|reference| reference.starts_with(WHITAKER_INSTALLER))
                .then_some((*step, indent))
        })
}

/// Fails unless `job` wires every lint step as the contract requires.
///
/// Returns the number of lint steps found, so the caller can report a
/// workflow that never invokes the aggregate.
///
/// # Errors
///
/// Returns the first drift found in the job.
fn verify_lint_job_steps(name: &str, job: &[Line<'_>]) -> Result<usize, String> {
    let found = lint_steps(name, job)?;
    if found.is_empty() {
        return Ok(0);
    }
    let indent = key_indent(job).ok_or_else(|| format!("job `{name}` must have a body"))?;
    verify_lint_job(name, job, indent)?;
    let job_steps = steps(job);
    for lint in &found {
        verify_lint_step(name, lint.lines, lint.indent, &lint.command)?;
        verify_installer(&job_steps, lint)?;
    }
    Ok(found.len())
}

/// Fails unless `workflow` wires the lint gate as the contract requires.
///
/// # Errors
///
/// Returns the first drift found: a workflow-level override, a missing
/// aggregate step, a second job without its own installer, an installer out of
/// order or unpinned, a bypass key, or an environment override.
pub(crate) fn verify_lint_gate(workflow: &str) -> Result<(), String> {
    let workflow_lines: Vec<Line<'_>> = workflow.lines().map(Line::new).collect();
    let workflow_env =
        environment_keys(&workflow_lines, 0).map_err(|error| format!("the workflow: {error}"))?;
    reject_coverage_override("the workflow", &workflow_env)?;
    let all_jobs = jobs(Workflow(workflow));
    if all_jobs.is_empty() {
        return Err("the workflow must define at least one job".to_owned());
    }
    let mut lint_job_count = 0;
    for (name, job) in &all_jobs {
        lint_job_count += verify_lint_job_steps(name, job)?;
    }
    if lint_job_count == 0 {
        return Err(format!(
            "CI must run `{LINT_COMMAND}`, the same aggregate target used locally"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod reader_contracts {
    //! The readers must fail closed on forms they cannot interpret.

    use super::super::{Line, single_job, verify_lint_gate};

    /// A pinned installer step, as these cases spell it.
    const INSTALL: &str = concat!(
        "      - name: Install Whitaker\n",
        "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
        "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
    );

    /// A workflow whose lint step runs `command`.
    fn job_running(command: &str) -> String {
        single_job(&format!(
            "{INSTALL}      - name: Lint\n        run: {command}\n"
        ))
    }

    /// A command the checker cannot interpret fails the contract rather than
    /// passing it, so an exotic form cannot hide an unprotected aggregate.
    #[test]
    fn an_unreadable_run_form_is_rejected() {
        for command in ["[make, lint]", "*aggregate"] {
            let report = verify_lint_gate(&job_running(command))
                .expect_err("an unreadable run form must be rejected");
            assert!(
                report.contains("cannot read"),
                "the report must say the form is unreadable; found {report:?}"
            );
        }
    }

    /// An `env:` header the checker cannot interpret fails the contract too.
    #[test]
    fn an_unreadable_env_header_is_rejected() {
        let workflow = single_job(&format!(
            "{INSTALL}      - name: Lint\n        env: |\n          A: b\n        run: make lint\n"
        ));
        let report =
            verify_lint_gate(&workflow).expect_err("an unreadable env header must be rejected");
        assert!(
            report.contains("cannot read"),
            "the report must say the header is unreadable; found {report:?}"
        );
    }

    /// A `- run:` step is the aggregate, not an unreadable form.
    #[test]
    fn a_sequence_opening_run_step_is_read() {
        let workflow = single_job(&format!("{INSTALL}      - run: make lint\n"));
        if let Err(report) = verify_lint_gate(&workflow) {
            panic!("a `- run: make lint` step must be accepted: {report}");
        }
    }

    /// A quoted scalar is decoded before the aggregate check.
    #[test]
    fn a_quoted_run_scalar_is_read() {
        let workflow = single_job(&format!(
            "{INSTALL}      - name: Lint\n        run: 'make lint'\n"
        ));
        if let Err(report) = verify_lint_gate(&workflow) {
            panic!("a quoted `make lint` scalar must be accepted: {report}");
        }
    }

    /// A shell separator starts a new command, so a later aggregate is read.
    #[test]
    fn a_later_command_is_read() {
        let workflow = job_running("true && make lint");
        let decision = verify_lint_gate(&workflow);
        if let Err(report) = decision {
            assert!(
                report.contains("must run exactly"),
                "`true && make lint` must be read as the aggregate; found {report:?}"
            );
        }
    }

    /// A quoted bypass key and a tab after the colon are both honoured.
    #[test]
    fn a_quoted_bypass_key_is_detected() {
        for key in ["\"if\": false", "'if': false", "if:\tfalse"] {
            let body =
                format!("{INSTALL}      - name: Lint\n        {key}\n        run: make lint\n");
            assert!(
                verify_lint_gate(&single_job(&body)).is_err(),
                "a bypass written as `{key}` must be rejected"
            );
        }
    }

    /// A leading assignment still reaches the aggregate checks.
    #[test]
    fn a_leading_assignment_is_read() {
        let report = verify_lint_gate(&job_running("FOO=1 make lint"))
            .expect_err("an assignment-prefixed aggregate must be rejected as inexact");
        assert!(
            report.contains("must run exactly"),
            "the command must be read as the aggregate; found {report:?}"
        );
    }

    /// `MAKEFLAGS` is an execution-control variable and is rejected.
    #[test]
    fn makeflags_is_rejected() {
        let body = format!(
            "{INSTALL}      - name: Lint\n        env:\n          MAKEFLAGS: -s\n        run: make lint\n"
        );
        assert!(
            verify_lint_gate(&single_job(&body)).is_err(),
            "MAKEFLAGS must be rejected like a coverage variable"
        );
    }

    /// A whitespace-separated colon is still a bypass key.
    #[test]
    fn a_spaced_colon_is_read() {
        let body =
            format!("{INSTALL}      - name: Lint\n        if : false\n        run: make lint\n");
        let report = verify_lint_gate(&single_job(&body))
            .expect_err("a conditional lint step must be rejected");
        assert!(
            report.contains("conditional or non-fatal"),
            "`if :` must be read as a bypass key; found {report:?}"
        );
    }

    /// A line reader sees a continuation `run:` at the step's key indent.
    #[test]
    fn line_indentation_is_measured() {
        let line = Line::new("        run: make lint");
        assert_eq!(line.indent(), 8, "the reader must measure indentation");
        assert_eq!(line.trimmed(), "run: make lint", "the reader must trim");
    }
}
