//! Decode-failure source-location lookup for schema loader diagnostics.
//!
//! Raw argument decoding errors preserve typed context such as the `Let`
//! binding name or `Do` step index. This module maps that context back into
//! the original YAML text so loader diagnostics can point at the failing
//! argument field instead of the theorem header.

use derive_more::{From, Into};
use the_newtype::Newtype;

use super::raw::{RawDocDecodeError, RawTheoremDoc};
use super::str_newtype::StrNewtype as _;

/// Newtype representing a YAML key used for decode-location matching.
///
/// The constructor and accessor come from [`super::str_newtype::StrNewtype`],
/// imported anonymously above so in-module call sites stay terse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Newtype, From, Into)]
pub(crate) struct YamlKey<'a>(&'a str);

/// Newtype representing one line of the source YAML document.
///
/// Every classifier in this module asks the same kind of question of a line:
/// how far it is indented, whether it opens a top-level section, or whether it
/// names a mapping key. Keeping those questions on the line type stops a bare
/// `&str` travelling through a dozen free functions, and lets the scanners
/// name what they are inspecting rather than how it is stored.
///
/// The constructor and accessor come from [`super::str_newtype::StrNewtype`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Newtype, From, Into)]
pub(crate) struct YamlLine<'a>(&'a str);

impl<'a> YamlLine<'a> {
    /// The line without its leading indentation.
    ///
    /// `YamlLine` is module-private, so this carries no doctest: doctests
    /// compile against the crate's public surface, which excludes it.
    #[must_use]
    fn trimmed(self) -> &'a str {
        self.as_str().trim_start()
    }

    /// Number of leading spaces, used to compare nesting depth.
    #[must_use]
    fn indent_width(self) -> usize {
        let raw = self.as_str();
        raw.len() - raw.trim_start().len()
    }

    /// True when the line carries content rather than only whitespace.
    #[must_use]
    fn has_content(self) -> bool {
        !self.as_str().trim().is_empty()
    }

    /// True when the line opens a YAML list item.
    #[must_use]
    fn is_list_item(self) -> bool {
        self.trimmed().starts_with("- ")
    }

    /// True when the line opens an unindented section, such as `Evidence:`.
    #[must_use]
    fn is_top_level_section(self) -> bool {
        !self.as_str().starts_with(' ') && self.as_str().trim_end().ends_with(':')
    }

    /// True when the line opens a new theorem document.
    #[must_use]
    fn is_theorem_header(self) -> bool {
        matches!(self.trimmed(), "Theorem:" | "theorem:")
    }

    /// True when the line is exactly the named section header.
    #[must_use]
    fn is_section_line(self, section: YamlKey<'a>) -> bool {
        self.trimmed() == format!("{}:", section.as_str())
    }

    /// True when the line begins a mapping entry for `key`.
    ///
    /// The key must be followed immediately by `:`, so `target_other:` does
    /// not match a search for `target`.
    #[must_use]
    fn opens_mapping_key(self, key: YamlKey<'a>) -> bool {
        self.trimmed()
            .strip_prefix(key.as_str())
            .is_some_and(|suffix| suffix.starts_with(':'))
    }

    /// True when the line is indented no deeper than the enclosing scope.
    ///
    /// Blank lines never close a scope; only content can do that.
    #[must_use]
    fn exits_scope(self, scope_indent: usize) -> bool {
        self.has_content() && self.indent_width() <= scope_indent
    }

    /// Locate `name` as a mapping key on this line.
    ///
    /// Returns the one-based row and column of the key, or `None` when the
    /// line does not open a mapping entry for that name.
    #[must_use]
    fn locate_argument(self, index: usize, name: YamlKey<'a>) -> Option<(usize, usize)> {
        let trimmed = self.trimmed();
        let suffix = trimmed.strip_prefix(name.as_str())?;
        if !suffix.starts_with(':') {
            return None;
        }
        let column = self.as_str().len() - trimmed.len() + 1;
        Some((index + 1, column))
    }

    /// Classify this line while scanning a `Let` binding for an argument.
    fn scan_binding(
        self,
        index: usize,
        indent: usize,
        argument_name: YamlKey<'a>,
    ) -> BindingLineOutcome {
        if self.exits_scope(indent) {
            return BindingLineOutcome::ExitScope;
        }
        match self.locate_argument(index, argument_name) {
            Some((row, column)) => BindingLineOutcome::Found(row, column),
            None => BindingLineOutcome::Continue,
        }
    }

    /// Classify this line while scanning a `Do` step for an argument.
    fn scan_do_step(self, index: usize, context: DoStepLineContext<'a>) -> DoStepLineOutcome {
        if context.scope != StepScope::SelectedStart && self.exits_scope(context.indent) {
            return DoStepLineOutcome::ExitScope;
        }
        match self.locate_argument(index, context.argument_name) {
            Some((row, column)) => DoStepLineOutcome::Found(row, column),
            None => DoStepLineOutcome::Continue,
        }
    }

    /// Advance the `Do` scan's position and report which step this line is
    /// part of.
    fn update_step_scope(
        self,
        target_step: usize,
        current_step: &mut usize,
        selected_indent: &mut Option<usize>,
    ) -> StepScope {
        if !self.is_list_item() {
            return selected_indent
                .map_or(StepScope::OutsideSelected, |_| StepScope::InsideSelected);
        }

        *current_step += 1;
        if *current_step == target_step {
            *selected_indent = Some(self.indent_width());
            StepScope::SelectedStart
        } else if selected_indent.is_some() {
            StepScope::PastSelected
        } else {
            StepScope::OutsideSelected
        }
    }
}

/// Find the source position of the argument named by a decode failure.
///
/// Returns the one-based `(row, column)` of the offending YAML key, or `None`
/// when the failure carries no locatable `Let` binding or `Do` step context.
pub(crate) fn locate_decode_failure(
    input: &str,
    raw_doc: &RawTheoremDoc,
    error: &RawDocDecodeError,
) -> Option<(usize, usize)> {
    let argument_name = terminal_argument_name(YamlKey::new(error.param()));
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

/// Scan a `Let` binding for the argument named in a decode failure.
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
            is_in_let = line.is_section_line(YamlKey::new("Let"));
            continue;
        }
        if line.is_top_level_section() {
            break;
        }

        match binding_indent {
            Some(indent) => match line.scan_binding(index, indent, argument_name) {
                BindingLineOutcome::Found(row, column) => return Some((row, column)),
                BindingLineOutcome::ExitScope => break,
                BindingLineOutcome::Continue => {}
            },
            None => {
                if line.opens_mapping_key(binding_name) {
                    binding_indent = Some(line.indent_width());
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoStepLineOutcome {
    Found(usize, usize),
    ExitScope,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DoStepLineContext<'a> {
    scope: StepScope,
    indent: usize,
    argument_name: YamlKey<'a>,
}

/// Scan the selected `Do` step for the argument named in a decode failure.
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
            is_in_do = line.is_section_line(YamlKey::new("Do"));
            continue;
        }
        if line.is_top_level_section() {
            break;
        }

        let scope = line.update_step_scope(step_index, &mut current_step, &mut step_indent);
        if scope == StepScope::PastSelected {
            break;
        }

        let Some(indent) = step_indent else {
            continue;
        };
        let context = DoStepLineContext {
            scope,
            indent,
            argument_name,
        };
        match line.scan_do_step(index, context) {
            DoStepLineOutcome::Found(row, column) => return Some((row, column)),
            DoStepLineOutcome::ExitScope => break,
            DoStepLineOutcome::Continue => {}
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

/// Iterate the document from `start_index`, stopping before the next theorem.
fn document_lines(input: &str, start_index: usize) -> impl Iterator<Item = (usize, YamlLine<'_>)> {
    input
        .lines()
        .enumerate()
        .skip(start_index)
        .take_while(move |(index, line)| {
            *index == start_index || !YamlLine::new(line).is_theorem_header()
        })
        .map(|(index, line)| (index, YamlLine::new(line)))
}

/// Reduce a dotted argument path to its final key.
fn terminal_argument_name(param: YamlKey<'_>) -> YamlKey<'_> {
    let raw = param.as_str();
    let terminal = raw.rsplit_once(": ").map_or(raw, |(_, name)| name);
    YamlKey::new(terminal)
}

#[cfg(test)]
#[path = "loader_decode_location_tests.rs"]
mod tests;
