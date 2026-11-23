//! Readers and verifier for the CI side of the workspace-wide lint gate.
//!
//! `tests/lint_ci_contract.rs` states the contract and the synthetic cases;
//! this module holds the YAML readers and the verifier they exercise, so each
//! file stays inside the repository's size limit. The workflow primitives come
//! from `tests/workflow_contract_support/mod.rs`, which the spelling contract
//! shares.

#[path = "../workflow_contract_support/mod.rs"]
mod workflow_contract_support;

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
/// Only the step's own key is read, so a nested `env: { run: … }` is data.
fn run_command<'a>(step: &'a [Line<'_>], indent: usize) -> Option<&'a str> {
    step_run_line(step, indent)?
        .trimmed()
        .strip_prefix("run:")
        .map(str::trim)
}

/// Reports whether a command invokes the aggregate lint target.
///
/// The target name is matched as a whole token, so `make lint-whitaker` is a
/// different command and does not satisfy the aggregate contract.
fn invokes_aggregate(command: &str) -> bool {
    let mut tokens = command.split_whitespace();
    tokens.next() == Some("make") && tokens.next() == Some("lint")
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

/// Returns the environment keys a block assigns through its own `env:`.
///
/// Both the mapping form and a one-line flow mapping are read. Only the
/// block's own `env:` at `indent` counts, so a nested `env:` under `with:` in
/// another step cannot be mistaken for the caller's environment.
fn environment_keys<'a>(lines: &[Line<'a>], indent: usize) -> Vec<&'a str> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.indent() == indent)
        .filter_map(|(index, line)| {
            let rest = line.trimmed().strip_prefix("env:")?.trim();
            Some((index, rest))
        })
        .flat_map(|(index, rest)| env_block_keys(lines, index, rest))
        .collect()
}

/// Returns the keys a single `env:` entry declares.
///
/// A flow mapping is read from the `env:` line itself; a block mapping is read
/// from the entries that follow, down to the first content line at or above
/// the key's own indentation. Blank lines and comments never end the block.
fn env_block_keys<'a>(lines: &[Line<'a>], env_index: usize, rest: &'a str) -> Vec<&'a str> {
    if let Some(flow) = rest
        .strip_prefix('{')
        .and_then(|body| body.strip_suffix('}'))
    {
        return flow.split(',').filter_map(entry_key).collect();
    }
    if !rest.is_empty() {
        return Vec::new();
    }
    let Some(env_line) = lines.get(env_index) else {
        return Vec::new();
    };
    let block: Vec<&Line<'a>> = lines
        .iter()
        .skip(env_index + 1)
        .take_while(|line| {
            let trimmed = line.trimmed();
            trimmed.is_empty() || trimmed.starts_with('#') || line.indent() > env_line.indent()
        })
        .filter(|line| {
            let trimmed = line.trimmed();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .collect();
    let Some(entry_indent) = block.first().map(|line| line.indent()) else {
        return Vec::new();
    };
    block
        .iter()
        .filter(|line| line.indent() == entry_indent)
        .filter_map(|line| entry_key(line.trimmed()))
        .collect()
}

/// Splits a `NAME: value` entry into its key.
fn entry_key(entry: &str) -> Option<&str> {
    entry.split_once(':').map(|(key, _)| key.trim())
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
    command: &'a str,
}

/// Collects the steps of `job` that invoke the aggregate lint target.
fn lint_steps<'a, 'b>(name: &'a str, job: &'a [Line<'b>]) -> Vec<LintStep<'a, 'b>> {
    steps(job)
        .into_iter()
        .filter_map(|(start, step)| {
            let indent = key_indent(step)?;
            let command = run_command(step, indent)?;
            invokes_aggregate(command).then_some(LintStep {
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
    if has_bypass_key(job, job_indent) {
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
        verify_lint_step(name, lint.lines, lint.indent, lint.command)?;
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
