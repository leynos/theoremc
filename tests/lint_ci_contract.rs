//! Contract tests for the CI and Dylint sides of the workspace-wide lint gate.
//!
//! The workflow must install a pinned Whitaker and then run `make lint`, the
//! same aggregate target contributors run locally. Every job that runs the
//! aggregate needs its own pinned install, because a second job would otherwise
//! run the gate without the suite it is meant to exercise. A lint step must
//! carry no arguments and no environment override that could narrow the gate,
//! and no `if:` or `continue-on-error:` may let it skip or fail softly.
//!
//! The root `dylint.toml` must not exempt a whole crate, and every
//! `excluded_paths` entry must carry an adjacent comment saying why.
//!
//! The tests read both files as text, so a gate cannot be provisioned
//! differently from the way it is documented or quietly widened.
//!
//! `tests/lint_makefile_contract.rs` holds the same contract for the recipes,
//! and `tests/lint_gate_execution.rs` proves the recipes really run.

#[path = "workflow_contract_support/mod.rs"]
mod workflow_contract_support;

use workflow_contract_support::{
    Line, Workflow, has_bypass_key, jobs, key_indent, step_run_line, steps,
};

const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const DYLINT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dylint.toml"));

/// The provisioning action CI must use for the Whitaker suite.
const WHITAKER_INSTALLER: &str = "leynos/shared-actions/.github/actions/install-whitaker@";

/// The aggregate target CI must invoke, exactly.
const LINT_COMMAND: &str = "make lint";

/// Variables that redirect a lint stage away from its workspace default.
///
/// A step-level or job-level assignment of any of these would let CI run a
/// narrower gate than the one `make lint` defines, so the contract rejects
/// them outright.
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
/// from the entries that follow, down to the first line at or above the key's
/// own indentation. Comments and blank lines never end the block.
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
        .take_while(|line| line.indent() > env_line.indent())
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

/// Fails unless the job carrying `lint` is unconditional and unoverridden.
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
/// Returns the first drift found: a missing aggregate step, a second job
/// without its own installer, an installer out of order or unpinned, a bypass
/// key, or an environment override.
fn verify_lint_gate(workflow: &str) -> Result<(), String> {
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

/// Fails unless one lint step is the argument-free, unbypassed aggregate.
///
/// # Errors
///
/// Returns the offending command or bypass key.
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

/// Reports the active lines of a Dylint configuration.
///
/// Comment lines are removed so the prose that explains the exclusion policy
/// cannot read as the policy itself.
fn active_lines(document: &str) -> Vec<&str> {
    document
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Reports whether the line before `index` is a justification comment.
fn has_justification(lines: &[&str], index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|previous| lines.get(previous))
        .is_some_and(|line| line.trim().starts_with('#'))
}

/// Reports whether a value line is an `excluded_paths` entry needing a reason.
///
/// A quoted entry inside an `excluded_paths` value is the only shape that
/// needs justification; every other line is policy, structure, or prose.
fn is_exclusion_entry(value: &str) -> bool {
    value.starts_with('"') || value.contains('"')
}

/// Returns the active `excluded_paths` entries that carry no adjacent reason.
///
/// Entries are read from the `excluded_paths` value itself and from the list
/// items below it, down to the closing bracket.
fn unjustified_exclusions(document: &str) -> Vec<String> {
    let lines: Vec<&str> = document.lines().collect();
    let mut offences = Vec::new();
    let mut inside_list = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if inside_list && trimmed.starts_with(']') {
            inside_list = false;
            continue;
        }
        let value = if inside_list {
            trimmed
        } else {
            let Some(value) = excluded_paths_value(trimmed) else {
                continue;
            };
            if value.starts_with('[') && !value.contains(']') {
                inside_list = true;
                continue;
            }
            value
        };
        if is_exclusion_entry(value) && !has_justification(&lines, index) {
            offences.push((*line).to_owned());
        }
    }
    offences
}

/// Returns the value of an `excluded_paths = …` assignment.
fn excluded_paths_value(trimmed: &str) -> Option<&str> {
    trimmed
        .strip_prefix("excluded_paths")?
        .trim_start()
        .strip_prefix('=')
        .map(str::trim)
}

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

/// A pinned Whitaker install step, as the synthetic jobs spell it.
const INSTALL_STEP: &str = concat!(
    "      - name: Install Whitaker\n",
    "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
    "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
);

/// The aggregate lint step, as the synthetic jobs spell it.
const LINT_STEP: &str = "      - name: Lint\n        run: make lint\n";

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

/// Verifies that `dylint.toml` declares no whole-crate exclusion.
#[test]
fn the_dylint_configuration_exempts_no_whole_crate() {
    assert!(
        !active_lines(DYLINT)
            .iter()
            .any(|line| line.starts_with("excluded_crates")),
        "dylint.toml must not exempt a whole crate; prefer `excluded_paths` with a recorded reason"
    );
}

/// Verifies that every committed exclusion carries an adjacent reason.
#[test]
fn every_committed_exclusion_is_justified() {
    let offences = unjustified_exclusions(DYLINT);
    assert!(
        offences.is_empty(),
        "every excluded_paths entry needs a comment immediately above it; found {offences:?}"
    );
}

/// An exclusion without an adjacent comment is rejected.
#[test]
fn an_unjustified_exclusion_is_rejected() {
    let inline = "excluded_paths = [\"crate::legacy_io\"]\n";
    assert!(
        !unjustified_exclusions(inline).is_empty(),
        "an inline entry without a preceding comment must be rejected"
    );
    let list = "excluded_paths = [\n  \"crate::legacy_io\",\n]\n";
    assert!(
        !unjustified_exclusions(list).is_empty(),
        "a list item without a preceding comment must be rejected"
    );
}

/// A documented exclusion, inline or itemized, is accepted.
#[test]
fn a_justified_exclusion_is_accepted() {
    let inline = "# The ambient probe cannot be capability-scoped.\nexcluded_paths = [\"crate::legacy_io\"]\n";
    assert!(
        unjustified_exclusions(inline).is_empty(),
        "an inline entry with a preceding comment must be accepted"
    );
    let list = "excluded_paths = [\n  # The ambient probe cannot be capability-scoped.\n  \"crate::legacy_io\",\n]\n";
    assert!(
        unjustified_exclusions(list).is_empty(),
        "a list item with a preceding comment must be accepted"
    );
}
