//! Readers and verifier for the CI side of the workspace-wide lint gate.
//!
//! `tests/lint_ci_contract.rs` states the contract and the synthetic cases;
//! this module holds the YAML readers and the verifier they exercise, so each
//! file stays inside the repository's size limit. The workflow primitives come
//! from `tests/workflow_contract_support/mod.rs`, which the spelling contract
//! shares.

#[path = "../workflow_contract_support/mod.rs"]
mod workflow_contract_support;

#[path = "environment.rs"]
mod environment;

use environment::environment_keys;

pub(crate) use workflow_contract_support::{
    Line, Workflow, has_bypass_key, jobs, key_indent, step_run_line, steps,
};

/// The CI workflow under contract.
pub(crate) const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));

/// The provisioning action CI must use for the Whitaker suite.
pub(crate) const WHITAKER_INSTALLER: &str =
    "leynos/shared-actions/.github/actions/install-whitaker@";

/// The aggregate target CI must invoke, exactly.
pub(crate) const LINT_COMMAND: &str = "make lint";

/// Variables that redirect a lint stage away from its workspace default.
///
/// A workflow-level, job-level, or step-level assignment of any of these would
/// let CI run a narrower gate than the one `make lint` defines, so the contract
/// rejects them outright.
const COVERAGE_VARIABLES: [&str; 8] = [
    "LINT_PACKAGES",
    "LINT_TARGET_FLAGS",
    "CARGO_FLAGS",
    "CLIPPY_FLAGS",
    "RUSTDOC_TARGET_FLAGS",
    "RUST_FLAGS",
    "RUSTDOC_FLAGS",
    "WHITAKER",
];

/// Returns a step's own `run:` command, without the key.
///
/// Only the step's own key is read, so a nested `env: { run: … }` is data. A
/// block scalar's `|` or `>` marker is replaced by the scalar's body, because
/// the command a step actually runs is the body, not the marker.
fn run_command(step: &[Line<'_>], indent: usize) -> Option<String> {
    let run_line = step_run_line(step, indent)?;
    let trimmed = run_line.trimmed();
    let raw = trimmed
        .strip_prefix("- run:")
        .or_else(|| trimmed.strip_prefix("run:"))?
        .trim();
    let value = unquote(raw);
    if !value.starts_with(['|', '>']) {
        return Some(value.to_owned());
    }
    let body = step
        .iter()
        .skip_while(|line| !std::ptr::eq(*line, run_line))
        .skip(1)
        .take_while(|line| line.trimmed().is_empty() || line.indent() > indent)
        .filter(|line| !line.trimmed().is_empty())
        .map(|line| line.trimmed())
        .collect::<Vec<_>>()
        .join("\n");
    Some(body)
}

/// Removes one layer of YAML quoting from a scalar value.
///
/// YAML permits a plain, single-quoted, or double-quoted scalar, and the
/// quotes are syntax rather than part of the command, so `'make lint'` runs the
/// same command as `make lint`.
fn unquote(value: &str) -> &str {
    let trimmed = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = trimmed
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    trimmed
}

/// Reports whether a command invokes the aggregate lint target.
///
/// Each line of a block-scalar body is checked, so a `run: |` step that calls
/// `make lint` is recognized and then rejected for not being the exact
/// command. The target name is matched as a whole token, so `make
/// lint-whitaker` is a different command.
fn invokes_aggregate(command: &str) -> bool {
    command
        .lines()
        .any(|line| make_target(line) == Some("lint"))
}

/// Returns the target Make is asked to build, ignoring option arguments.
///
/// Make accepts options before the target, and several of them take the next
/// token as their argument. Skipping both keeps `make -s lint` and
/// `make -f Makefile lint` classified by the word `lint`.
fn make_target(line: &str) -> Option<&str> {
    let mut tokens = line.split_whitespace();
    if tokens.next() != Some("make") {
        return None;
    }
    loop {
        let token = tokens.next()?;
        if !token.starts_with('-') {
            return Some(token);
        }
        if takes_argument(token) {
            tokens.next();
        }
    }
}

/// Reports whether a Make option consumes the following token.
fn takes_argument(option: &str) -> bool {
    matches!(option, "-f" | "-C" | "-I" | "-o" | "-W" | "-j" | "-l")
}

/// Returns the reference a step's own `uses:` key names.
///
/// Both the block form (`uses: owner/repo@ref`) and the sequence-item form
/// (`- uses: owner/repo@ref`) are recognized, because either can install the
/// suite.
fn uses_reference<'a>(step: &'a [Line<'_>], indent: usize) -> Option<&'a str> {
    step.iter().find_map(|line| {
        let trimmed = line.trimmed();
        let candidate = if line.indent() == indent {
            trimmed.strip_prefix("uses:")
        } else if line.indent() + 2 == indent {
            trimmed.strip_prefix("- uses:")
        } else {
            None
        };
        candidate.map(str::trim)
    })
}

/// Fails when `keys` assign a variable that would narrow the lint gate.
///
/// # Errors
///
/// Returns the offending variable and the block that assigned it.
fn reject_coverage_override(scope: &str, keys: &[&str]) -> Result<(), String> {
    keys.iter()
        .find(|key| COVERAGE_VARIABLES.contains(key))
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
fn lint_steps<'a, 'b>(name: &'a str, job: &'a [Line<'b>]) -> Vec<LintStep<'a, 'b>> {
    steps(job)
        .into_iter()
        .filter_map(|(start, step)| {
            let indent = key_indent(step)?;
            let command = run_command(step, indent)?;
            invokes_aggregate(&command).then_some(LintStep {
                job: name,
                start,
                lines: step,
                indent,
                command,
            })
        })
        .collect()
}

/// Fails unless the job carrying the lint step is unconditional.
///
/// # Errors
///
/// Returns the job-level bypass key or coverage override.
fn verify_lint_job(name: &str, job: &[Line<'_>], job_indent: usize) -> Result<(), String> {
    // A job's own keys sit at `job_indent`; a `- if:` at that indentation in
    // an indentless step list belongs to a step, and the step checks cover it.
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
    reject_coverage_override(&format!("job `{name}`"), &environment_keys(job, job_indent))
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
    reject_coverage_override(
        &format!("the Whitaker install in job `{name}`"),
        &environment_keys(installer, indent),
    )?;
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
    reject_coverage_override(
        &format!("the lint step in job `{job}`"),
        &environment_keys(step, indent),
    )
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
    let found = lint_steps(name, job);
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
    reject_coverage_override("the workflow", &environment_keys(&workflow_lines, 0))?;
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
