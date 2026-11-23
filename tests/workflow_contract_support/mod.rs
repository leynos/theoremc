//! Workflow-text parsers shared by the workflow contract tests.
//!
//! The spelling and lint contracts are both expressed by GitHub Actions
//! workflows, so both read the YAML as text and locate a job, a step, or a key
//! by its structure rather than by a whole-file substring. Keeping the readers
//! here means the two contracts agree on how a job and a step are recognised
//! instead of drifting apart in two hand-written copies.
//!
//! This module holds only the primitives both contracts use. Composition that
//! is specific to one contract stays with that contract, so including this
//! file adds no item a caller does not exercise.

/// A workflow document to read as text.
#[derive(Clone, Copy)]
pub(crate) struct Workflow<'a>(pub(crate) &'a str);

/// One line of a YAML workflow, with its indentation measured once.
///
/// CI YAML here is space-indented, so the leading-space count is the nesting
/// depth. Measuring it once keeps the extractors free of string arithmetic.
#[derive(Clone, Copy)]
pub(crate) struct Line<'a> {
    text: &'a str,
    indent: usize,
}

impl<'a> Line<'a> {
    /// Measures one line's leading spaces.
    fn new(text: &'a str) -> Self {
        Self {
            text,
            indent: text.len() - text.trim_start_matches(' ').len(),
        }
    }

    /// Returns the line without surrounding whitespace.
    pub(crate) fn trimmed(self) -> &'a str {
        self.text.trim()
    }

    /// Returns the line's leading-space count.
    pub(crate) fn indent(self) -> usize {
        self.indent
    }

    /// Reports whether the line carries no content.
    fn is_blank(self) -> bool {
        self.trimmed().is_empty()
    }

    /// Reports whether the line is a YAML comment, at any indentation.
    fn is_comment(self) -> bool {
        self.trimmed().starts_with('#')
    }

    /// Reports whether the line opens a sequence item, which in a job's
    /// `steps:` list marks the start of a step.
    fn opens_step(self) -> bool {
        self.trimmed().starts_with("- ")
    }
}

/// Returns a line's job key when the line opens a top-level job.
///
/// The key must be indented by exactly two spaces, as a job is a child of the
/// workflow's `jobs:` mapping. A trailing comment is allowed:
///
/// ```text
///   build-test:  # the only job          -> Some("build-test")
///   build-test-extra:                    -> Some("build-test-extra")
/// ```
fn job_key(line: Line<'_>) -> Option<&str> {
    let rest = line.text.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let (key, suffix) = rest.split_once(':')?;
    let trimmed_suffix = suffix.trim();
    (trimmed_suffix.is_empty() || trimmed_suffix.starts_with('#')).then_some(key)
}

/// Returns every top-level job in file order, with the lines it owns.
///
/// A job runs until the next line that opens a job, so a contract can inspect
/// every job rather than only the first one that matches a command. Keeping
/// the name beside the lines lets a diagnostic say which job drifted.
pub(crate) fn jobs<'a>(workflow: Workflow<'a>) -> Vec<(&'a str, Vec<Line<'a>>)> {
    let lines: Vec<Line<'a>> = workflow.0.lines().map(Line::new).collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| job_key(**line).is_some())
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .enumerate()
        .filter_map(|(position, start)| {
            let name = job_key(*lines.get(*start)?)?;
            let end = starts.get(position + 1).copied().unwrap_or(lines.len());
            Some((name, lines.get(*start..end)?.to_vec()))
        })
        .collect()
}

/// Returns the index just past the step that starts at `start`.
///
/// Blank lines and comments never end a step, and neither does a line that is
/// indented deeper than the step's own `- name:` line.
fn step_end(job: &[Line<'_>], start: usize, step_indent: usize) -> usize {
    job.iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| {
            !line.is_blank()
                && !line.is_comment()
                && (line.indent() < step_indent
                    || (line.opens_step() && line.indent() <= step_indent))
        })
        .map_or(job.len(), |(index, _)| index)
}

/// Returns each step of a job with its start index, in file order.
///
/// Only sequence items at the indentation of the job's first step count: a
/// nested sequence under `with:` is data, not a second step.
pub(crate) fn steps<'a, 'b>(job: &'a [Line<'b>]) -> Vec<(usize, &'a [Line<'b>])> {
    let Some(step_indent) = job
        .iter()
        .find(|line| line.opens_step())
        .map(|line| line.indent())
    else {
        return Vec::new();
    };
    let starts: Vec<usize> = job
        .iter()
        .enumerate()
        .filter(|(_, line)| line.opens_step() && line.indent() == step_indent)
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .filter_map(|start| Some((*start, job.get(*start..step_end(job, *start, step_indent))?)))
        .collect()
}

/// Returns the indentation of a block's own keys.
///
/// A block's first line opens it (a job key or a step's `- name:` line), and
/// its keys sit two spaces deeper.
pub(crate) fn key_indent(block: &[Line<'_>]) -> Option<usize> {
    block.first().map(|line| line.indent() + 2)
}

/// Reports whether a block carries a key that can silence a failure at its own
/// key indentation.
///
/// Only the block's own keys are inspected: a nested `if:` under `env:` or
/// `with:` is data, not a workflow condition.
pub(crate) fn has_bypass_key(lines: &[Line<'_>], indent: usize) -> bool {
    lines
        .iter()
        .filter(|line| line.indent() == indent)
        .any(|line| {
            let trimmed = line.trimmed();
            ["if:", "continue-on-error:"].iter().any(|key| {
                trimmed
                    .strip_prefix(*key)
                    .is_some_and(|rest| rest.starts_with(' '))
            })
        })
}

/// Returns the step's own `run:` line, ignoring nested keys such as `env:`.
///
/// The `run:` key must sit at the step's key indentation, so a nested
/// `env: { run: make spelling }` cannot satisfy the contract while the step
/// actually runs something else.
pub(crate) fn step_run_line<'a, 'b>(step: &'a [Line<'b>], indent: usize) -> Option<&'a Line<'b>> {
    step.iter()
        .find(|line| line.indent() == indent && line.trimmed().starts_with("run:"))
}
