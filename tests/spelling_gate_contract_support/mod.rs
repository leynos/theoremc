//! Text parsers and composition for the spelling gate contract tests.
//!
//! The contract is expressed by a Makefile target and a GitHub Actions
//! workflow, so the tests read both files as text and locate the gate by its
//! structure rather than by a whole-file substring. The workflow primitives
//! live in `tests/workflow_contract_support/mod.rs`, which the lint contract
//! shares; the spelling-specific composition and the Makefile readers stay
//! here so each contract test states only its own rules.

/// The Makefile under contract.
pub(crate) const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));

/// The CI workflow under contract.
pub(crate) const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));

/// The recipe the `spelling` target must run, exactly as the baseline pins it.
pub(crate) const PINNED_RECIPE: &str = "$(SPELLING_COMMAND)";

/// The exact `run:` line the CI Spelling step must carry.
///
/// Anything else, including `--dry-run`, `|| true`, or a commented-out copy,
/// is a different line and must not satisfy the contract. The whole line is
/// compared, so `run:  make spelling` with a second space is rejected too.
pub(crate) const SPELLING_COMMAND: &str = "run: make spelling";

#[path = "../workflow_contract_support/mod.rs"]
mod workflow;

pub(crate) use workflow::{Line, Workflow, has_bypass_key, key_indent, step_run_line};
use workflow::{jobs, steps};

/// A Makefile to read as text.
#[derive(Clone, Copy)]
pub(crate) struct Makefile<'a>(pub(crate) &'a str);

/// A top-level job name.
#[derive(Clone, Copy)]
pub(crate) struct JobName<'a>(pub(crate) &'a str);

/// A step name within a job.
#[derive(Clone, Copy)]
pub(crate) struct StepName<'a>(pub(crate) &'a str);

/// A Makefile target name.
#[derive(Clone, Copy)]
pub(crate) struct TargetName<'a>(pub(crate) &'a str);

/// A Makefile variable name.
#[derive(Clone, Copy)]
pub(crate) struct VariableName<'a>(pub(crate) &'a str);

/// Returns the lines of a named top-level job.
///
/// The job opens at the two-space-indented `<name>:` key and runs until the
/// next job. Scoping the search to the job stops a same-named marker in
/// another job from satisfying an ordering assertion by accident.
pub(crate) fn job_lines<'a>(workflow: Workflow<'a>, name: JobName<'_>) -> Option<Vec<Line<'a>>> {
    jobs(workflow)
        .into_iter()
        .find(|(job_name, _)| *job_name == name.0)
        .map(|(_, lines)| lines)
}

/// Returns each step of a job, in order, as its own slice of lines.
///
/// Only sequence items at the indentation of the job's first step count: a
/// nested sequence under `with:` is data, not a second step.
fn job_steps<'a, 'b>(job: &'a [Line<'b>]) -> Vec<&'a [Line<'b>]> {
    steps(job).into_iter().map(|(_, step)| step).collect()
}

/// Returns the index of the only step named `name` in a job.
///
/// A duplicate `- name:` line would let the wrong step satisfy an ordering
/// assertion, so the count is checked before the index is used.
pub(crate) fn step_index(job: &[Line<'_>], name: StepName<'_>) -> Option<usize> {
    let marker = format!("- name: {}", name.0);
    let mut found = steps(job)
        .into_iter()
        .filter(|(_, step)| step.first().is_some_and(|line| line.trimmed() == marker));
    let first = found.next()?;
    assert!(
        found.next().is_none(),
        "the job must define exactly one step named {:?}",
        name.0
    );
    Some(first.0)
}

/// Returns the index of a step only when the job names it exactly once.
///
/// Unlike [`step_index`], a duplicate is treated as absent. Ordering
/// predicates use this so they can report `false` instead of panicking while a
/// contract test keeps the stricter duplicate check.
fn unique_step_index(job: &[Line<'_>], name: StepName<'_>) -> Option<usize> {
    let marker = format!("- name: {}", name.0);
    let mut found = steps(job)
        .into_iter()
        .filter(|(_, step)| step.first().is_some_and(|line| line.trimmed() == marker));
    let first = found.next()?;
    found.next().is_none().then_some(first.0)
}

/// Returns the lines of a named step, from its `- name:` line up to the next
/// step or the end of the job.
///
/// Blank lines and comments never end a step, so a note inside a step does not
/// hide the lines after it.
pub(crate) fn step_lines<'a, 'b>(
    job: &'a [Line<'b>],
    name: StepName<'_>,
) -> Option<&'a [Line<'b>]> {
    let marker = format!("- name: {}", name.0);
    job_steps(job)
        .into_iter()
        .find(|step| step.first().is_some_and(|line| line.trimmed() == marker))
}

/// Returns the `build-test` job of a workflow.
///
/// The contract tests all target the same job, so naming it once here keeps
/// each synthetic case focused on the shape it exercises.
pub(crate) fn build_test_job(workflow: Workflow<'_>) -> Option<Vec<Line<'_>>> {
    job_lines(workflow, JobName("build-test"))
}

/// Returns a named step of the `build-test` job together with its key
/// indentation.
///
/// Copying the step's lines lets a caller hold the result without borrowing
/// the job it came from, which keeps each synthetic case to a single parse.
pub(crate) fn build_test_step<'a>(
    workflow: Workflow<'a>,
    name: StepName<'_>,
) -> Option<(Vec<Line<'a>>, usize)> {
    let job = build_test_job(workflow)?;
    let step = step_lines(&job, name)?.to_vec();
    let indent = key_indent(&step)?;
    Some((step, indent))
}

/// Reports whether a step's own `run:` key is exactly the spelling command.
///
/// The comparison is exact, so a dry run, a shell fallback, or a commented
/// copy is rejected:
///
/// ```text
/// run: make spelling              -> true
/// run: make spelling --dry-run    -> false
/// run: make spelling || true      -> false
/// # run: make spelling            -> false
/// ```
pub(crate) fn runs_exact_command(step: &[Line<'_>], indent: usize) -> bool {
    step_run_line(step, indent).is_some_and(|line| {
        let trimmed = line.trimmed();
        trimmed.strip_prefix("- ").unwrap_or(trimmed) == SPELLING_COMMAND
    })
}

/// Reports whether a job orders Markdown lint, Spelling, and the Whitaker
/// install as required.
///
/// All three steps must be present exactly once, and the spelling gate must be
/// the cheap failure point between the other two. A missing or duplicated step
/// makes this return `false` rather than accidentally passing:
///
/// ```text
/// Markdown lint < Spelling < Install Whitaker  -> true
/// Install Whitaker < Spelling < Markdown lint  -> false
/// ```
pub(crate) fn spelling_runs_between_markdown_lint_and_whitaker(job: &[Line<'_>]) -> bool {
    let (Some(markdown_lint), Some(spelling), Some(install_whitaker)) = (
        unique_step_index(job, StepName("Markdown lint")),
        unique_step_index(job, StepName("Spelling")),
        unique_step_index(job, StepName("Install Whitaker")),
    ) else {
        return false;
    };
    markdown_lint < spelling && spelling < install_whitaker
}

/// Returns the recipe lines of a Makefile target, without the target line.
///
/// The leading tab is stripped so a caller compares the command text alone.
/// An absent target yields an empty list, which fails the equality assertion
/// rather than panicking.
pub(crate) fn recipe_lines<'a>(makefile: Makefile<'a>, target: TargetName<'_>) -> Vec<&'a str> {
    let header = format!("{}:", target.0);
    let mut lines = makefile.0.lines();
    if !lines.any(|line| line.starts_with(&header)) {
        return Vec::new();
    }
    lines
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// Returns the default value of a `NAME ?= value` assignment, trimmed.
///
/// Returns `None` when the assignment is absent, so a missing override fails
/// a comparison instead of silently matching.
pub(crate) fn default_assignment<'a>(
    makefile: Makefile<'a>,
    name: VariableName<'_>,
) -> Option<&'a str> {
    let prefix = format!("{} ?= ", name.0);
    makefile
        .0
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(str::trim)
}

/// Returns a Makefile variable definition with its continued lines joined.
///
/// A backslash at the end of a line continues the definition on the next
/// line; the fragments are joined with single spaces so a caller can search
/// the whole command without depending on the physical line breaks.
pub(crate) fn variable_definition(
    makefile: Makefile<'_>,
    name: VariableName<'_>,
) -> Option<String> {
    let plain = format!("{} =", name.0);
    let default = format!("{} ?=", name.0);
    let specifies = |line: &str| {
        line.strip_prefix(plain.as_str())
            .is_some_and(|rest| rest.starts_with(' '))
            || line
                .strip_prefix(default.as_str())
                .is_some_and(|rest| rest.starts_with(' '))
    };
    let lines: Vec<&str> = makefile.0.lines().collect();
    let start = lines.iter().position(|line| specifies(line))?;
    let mut definition = String::new();
    for line in lines.get(start..)? {
        let is_continued = line.ends_with('\\');
        if !definition.is_empty() {
            definition.push(' ');
        }
        definition.push_str(line.trim_end_matches('\\').trim());
        if !is_continued {
            break;
        }
    }
    Some(definition)
}
