//! Workflow-text parsers shared by the workflow contract tests.
//!
//! The spelling and lint contracts are both expressed by GitHub Actions
//! workflows, so both read the YAML as text and locate a job, a step, or a key
//! by its structure rather than by a whole-file substring. Keeping the readers
//! here means the two contracts agree on how a job and a step are recognized
//! instead of drifting apart in two handwritten copies.
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
    pub(crate) fn new(text: &'a str) -> Self {
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
    if line.is_comment() {
        return None;
    }
    let rest = line.text.strip_prefix("  ")?;
    if rest.starts_with(' ') {
        return None;
    }
    let (key, suffix) = rest.split_once(':')?;
    let trimmed_suffix = suffix.trim();
    (trimmed_suffix.is_empty() || trimmed_suffix.starts_with('#')).then_some(key)
}

/// Returns the line range of the workflow's `jobs:` mapping.
///
/// Discovery starts after the top-level `jobs:` key and stops at the next
/// column-zero key, so `build-test:` text inside an earlier top-level block
/// scalar is data rather than a job.
fn jobs_range(lines: &[Line<'_>]) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|line| line.indent() == 0 && line.trimmed().starts_with("jobs:"))?
        + 1;
    let end = lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, line)| {
            line.indent() == 0 && !line.is_blank() && !line.is_comment() && !line.opens_step()
        })
        .map_or(lines.len(), |(index, _)| index);
    Some((start, end))
}

/// Returns every top-level job in file order, with the lines it owns.
///
/// A job runs until the next line that opens a job, so a contract can inspect
/// every job rather than only the first one that matches a command. Keeping
/// the name beside the lines lets a diagnostic say which job drifted.
pub(crate) fn jobs<'a>(workflow: Workflow<'a>) -> Vec<(&'a str, Vec<Line<'a>>)> {
    let lines: Vec<Line<'a>> = workflow.0.lines().map(Line::new).collect();
    let Some((first, last)) = jobs_range(&lines) else {
        return Vec::new();
    };
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .take(last)
        .skip(first)
        .filter(|(_, line)| job_key(**line).is_some())
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .enumerate()
        .filter_map(|(position, start)| {
            let name = job_key(*lines.get(*start)?)?;
            let end = starts.get(position + 1).copied().unwrap_or(last);
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

/// Returns the index of the job's own `steps:` key.
///
/// Only a key at the job's key indentation counts, so `steps:` text inside an
/// earlier block scalar — a `run: |` body, for example — is data and not a
/// mapping key. The key is matched with or without a trailing comment.
fn steps_key_index(job: &[Line<'_>]) -> Option<(usize, usize)> {
    let key_indent = key_indent(job)?;
    job.iter().enumerate().find_map(|(index, line)| {
        let trailing = line
            .trimmed()
            .strip_prefix("steps:")
            .map(str::trim)
            .filter(|trailing| trailing.is_empty() || trailing.starts_with('#'));
        (line.indent() == key_indent && trailing.is_some()).then_some((index, key_indent))
    })
}

/// Returns the indentation of the job's next key after `steps_line`.
///
/// A job-level key that follows the step list ends it: `name: |` or `env:` at
/// the job's key indentation is a sibling of `steps:`, so anything after it is
/// data. Returning the job's length when no sibling follows keeps the step
/// list open to the end of the job.
fn next_sibling(job: &[Line<'_>], steps_line: usize, key_indent: usize) -> usize {
    job.iter()
        .enumerate()
        .skip(steps_line + 1)
        .find(|(_, line)| {
            !line.is_blank()
                && !line.is_comment()
                && line.indent() <= key_indent
                && !line.opens_step()
        })
        .map_or(job.len(), |(index, _)| index)
}

/// Returns each step of a job with its start index, in file order.
///
/// The search starts at the job's own `steps:` key and ends at the next
/// sibling key, so a block sequence belonging to something else — a
/// `strategy.matrix.os` list before the key, or a `name: |` scalar after it —
/// cannot supply steps. The sequence indentation is read from the first item
/// after the key, because YAML allows a step list to start at the key's own
/// indentation as well as two spaces deeper. Only items at that indentation
/// count: a nested sequence under `with:` is data, not a step.
pub(crate) fn steps<'a, 'b>(job: &'a [Line<'b>]) -> Vec<(usize, &'a [Line<'b>])> {
    let Some((steps_line, key_indent)) = steps_key_index(job) else {
        return Vec::new();
    };
    let end = next_sibling(job, steps_line, key_indent);
    let Some((first, step_indent)) = job
        .iter()
        .enumerate()
        .take(end)
        .skip(steps_line + 1)
        .find(|(_, line)| line.opens_step() && line.indent() >= key_indent)
        .map(|(index, line)| (index, line.indent()))
    else {
        return Vec::new();
    };
    let starts: Vec<usize> = job
        .iter()
        .enumerate()
        .take(end)
        .skip(first)
        .filter(|(_, line)| line.opens_step() && line.indent() == step_indent)
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .filter_map(|start| {
            let stop = step_end(job, *start, step_indent).min(end);
            Some((*start, job.get(*start..stop)?))
        })
        .collect()
}

/// Returns the indentation of a block's own keys.
///
/// A block's first line opens it (a job key or a step's `- name:` line), and
/// its keys sit two spaces deeper. A step's keys sit two spaces past the `- `,
/// which is the same arithmetic and keeps one rule.
pub(crate) fn key_indent(block: &[Line<'_>]) -> Option<usize> {
    block.first().map(|line| line.indent() + 2)
}

/// Reports whether a block carries a key that can silence a failure at its own
/// key indentation.
///
/// Only the block's own keys are inspected: a nested `if:` under `env:` or
/// `with:` is data, not a workflow condition. Both the sequence-opening form
/// (`- if: false`) and the continuation form (`  if: false`) count, and so
/// does a key whose value begins on the next line (`if:` alone), because
/// GitHub evaluates the expression either way.
pub(crate) fn has_bypass_key(lines: &[Line<'_>], indent: usize) -> bool {
    lines
        .iter()
        .filter(|line| line.indent() == indent || line.indent() + 2 == indent)
        .any(|line| {
            let trimmed = line.trimmed();
            let entry = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            ["if:", "continue-on-error:"].iter().any(|key| {
                entry
                    .strip_prefix(*key)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
            })
        })
}

/// Returns the step's own `run:` line, ignoring nested keys such as `env:`.
///
/// The `run:` key must sit at the step's key indentation, so a nested
/// `env: { run: make spelling }` cannot satisfy the contract while the step
/// actually runs something else. The sequence-opening form (`- run: …`) is
/// read too, because a one-key step is valid YAML.
pub(crate) fn step_run_line<'a, 'b>(step: &'a [Line<'b>], indent: usize) -> Option<&'a Line<'b>> {
    step.iter().find(|line| {
        let trimmed = line.trimmed();
        (line.indent() == indent && trimmed.starts_with("run:"))
            || (line.indent() + 2 == indent && trimmed.starts_with("- run:"))
    })
}
