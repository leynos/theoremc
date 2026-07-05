//! Decode-failure source-location lookup for schema loader diagnostics.
//!
//! Raw argument decoding errors preserve typed context such as the `Let`
//! binding name or `Do` step index. This module maps that context back into
//! the original YAML text so loader diagnostics can point at the failing
//! argument field instead of the theorem header.

/// Newtype representing a YAML key used for decode-location matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YamlKey<'a>(&'a str);

impl<'a> YamlKey<'a> {
    pub(crate) const fn new(key: &'a str) -> Self {
        Self(key)
    }

    pub(crate) const fn as_str(self) -> &'a str {
        self.0
    }
}

use super::raw::{RawDocDecodeError, RawTheoremDoc};

pub(crate) fn locate_decode_failure(
    input: &str,
    raw_doc: &RawTheoremDoc,
    error: &RawDocDecodeError,
) -> Option<(usize, usize)> {
    let argument_name = YamlKey::new(terminal_argument_name(error.param()));
    let start_line = usize::try_from(raw_doc.theorem_location().line()).ok()?;
    let start_index = start_line.saturating_sub(1);

    if let Some(binding_name) = error.let_binding_name() {
        return locate_let_binding_argument(
            input,
            start_index,
            YamlKey::new(binding_name),
            argument_name,
        );
    }
    if let Some(step_index) = error.do_step_index() {
        return locate_do_step_argument(input, start_index, step_index, argument_name);
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingLineOutcome {
    Found(usize, usize),
    ExitScope,
    Continue,
}

fn scan_binding_line(
    index: usize,
    line: &str,
    indent: usize,
    argument_name: &str,
) -> BindingLineOutcome {
    if exits_scope(line, indent) {
        return BindingLineOutcome::ExitScope;
    }
    match locate_argument_line(index, line, argument_name) {
        Some((row, column)) => BindingLineOutcome::Found(row, column),
        None => BindingLineOutcome::Continue,
    }
}

fn locate_let_binding_argument(
    input: &str,
    start_index: usize,
    binding_name: YamlKey<'_>,
    argument_name: YamlKey<'_>,
) -> Option<(usize, usize)> {
    let mut is_in_let = false;
    let mut binding_indent = None;

    for (index, line) in document_lines(input, start_index) {
        if !is_in_let {
            is_in_let = section_line(line, "Let");
            continue;
        }
        if is_top_level_section(line) {
            break;
        }

        match binding_indent {
            Some(indent) => match scan_binding_line(index, line, indent, argument_name.as_str()) {
                BindingLineOutcome::Found(row, column) => return Some((row, column)),
                BindingLineOutcome::ExitScope => break,
                BindingLineOutcome::Continue => {}
            },
            None => {
                if mapping_key_line(line, binding_name.as_str()) {
                    binding_indent = Some(indent_width(line));
                }
            }
        }
    }

    None
}

fn locate_do_step_argument(
    input: &str,
    start_index: usize,
    step_index: usize,
    argument_name: YamlKey<'_>,
) -> Option<(usize, usize)> {
    let mut is_in_do = false;
    let mut current_step = 0;
    let mut step_indent = None;

    for (index, line) in document_lines(input, start_index) {
        if !is_in_do {
            is_in_do = section_line(line, "Do");
            continue;
        }
        if is_top_level_section(line) {
            break;
        }

        let scope = update_step_scope(line, step_index, &mut current_step, &mut step_indent);
        if scope == StepScope::PastSelected {
            break;
        }

        if let Some(indent) = step_indent {
            if scope != StepScope::SelectedStart && exits_scope(line, indent) {
                break;
            }
            if let Some(location) = locate_argument_line(index, line, argument_name.as_str()) {
                return Some(location);
            }
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepScope {
    OutsideSelected,
    SelectedStart,
    InsideSelected,
    PastSelected,
}

fn update_step_scope(
    line: &str,
    target_step: usize,
    current_step: &mut usize,
    selected_indent: &mut Option<usize>,
) -> StepScope {
    if !is_list_item(line) {
        return selected_indent.map_or(StepScope::OutsideSelected, |_| StepScope::InsideSelected);
    }

    *current_step += 1;
    if *current_step == target_step {
        *selected_indent = Some(indent_width(line));
        StepScope::SelectedStart
    } else if selected_indent.is_some() {
        StepScope::PastSelected
    } else {
        StepScope::OutsideSelected
    }
}

fn document_lines(input: &str, start_index: usize) -> impl Iterator<Item = (usize, &str)> {
    input
        .lines()
        .enumerate()
        .skip(start_index)
        .take_while(move |(index, line)| *index == start_index || !is_theorem_header(line))
}

fn terminal_argument_name(param: &str) -> &str {
    param
        .rsplit_once(": ")
        .map_or(param, |(_, argument_name)| argument_name)
}

fn locate_argument_line(index: usize, line: &str, argument_name: &str) -> Option<(usize, usize)> {
    let trimmed = line.trim_start();
    let suffix = trimmed.strip_prefix(argument_name)?;
    if !suffix.starts_with(':') {
        return None;
    }
    let column = line.len() - trimmed.len() + 1;
    Some((index + 1, column))
}

fn section_line(line: &str, section: &str) -> bool {
    line.trim_start() == format!("{section}:")
}

fn is_top_level_section(line: &str) -> bool {
    !line.starts_with(' ') && line.trim_end().ends_with(':')
}

fn is_theorem_header(line: &str) -> bool {
    matches!(line.trim_start(), "Theorem:" | "theorem:")
}

fn mapping_key_line(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix(key)
        .is_some_and(|suffix| suffix.starts_with(':'))
}

fn is_list_item(line: &str) -> bool {
    line.trim_start().starts_with("- ")
}

fn exits_scope(line: &str, scope_indent: usize) -> bool {
    !line.trim().is_empty() && indent_width(line) <= scope_indent
}

fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
