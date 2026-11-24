//! Readers for the `env:` mappings a workflow step, job, or workflow declares.
//!
//! A lint step must not set a variable that narrows the gate, and an `env:`
//! can be written as a block mapping, a flow mapping on one line, or a flow
//! mapping across several lines. This module reads all three so the coverage
//! check sees the same keys GitHub would apply.

use super::Line;

/// Returns the environment keys a block assigns through its own `env:`.
///
/// Both the mapping form and a one-line flow mapping are read. Only the
/// block's own `env:` at `indent` counts, so a nested `env:` under `with:` in
/// another step cannot be mistaken for the caller's environment.
/// # Errors
///
/// Returns the reason when an `env:` header uses a form the reader cannot
/// interpret. Failing closed is the point: an unreadable mapping must not be
/// treated as one without coverage variables.
pub(super) fn environment_keys<'a>(
    lines: &[Line<'a>],
    indent: usize,
) -> Result<Vec<&'a str>, String> {
    let mut keys = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.indent() != indent && line.indent() + 2 != indent {
            continue;
        }
        let trimmed = line.trimmed();
        let Some(rest) = env_header_value(trimmed) else {
            continue;
        };
        keys.extend(env_block_keys(lines, index, rest)?);
    }
    Ok(keys)
}

/// Returns the value an `env:` header introduces, quoted key or not.
///
/// The key may be written `env:`, `"env":`, or `'env':`, and the value may
/// follow the colon directly or on the next line.
fn env_header_value(trimmed: &str) -> Option<&str> {
    let entry = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let after_quotes = entry.trim_start_matches(['"', '\'']);
    let after_key = after_quotes.strip_prefix("env")?;
    let closing = after_key.trim_start_matches(['"', '\'']).trim_start();
    closing.strip_prefix(':').map(str::trim)
}

/// Returns the keys a single `env:` entry declares.
///
/// A flow mapping is read from the `env:` line itself; a block mapping is read
/// from the entries that follow, down to the first content line at or above
/// the key's own indentation. Blank lines and comments never end the block.
/// # Errors
///
/// Returns the reason when the header is neither a complete flow mapping, the
/// opening brace of one, nor an empty header introducing a block mapping.
fn env_block_keys<'a>(
    lines: &[Line<'a>],
    env_index: usize,
    rest: &'a str,
) -> Result<Vec<&'a str>, String> {
    if let Some(flow) = flow_mapping(rest) {
        return Ok(flow.split(',').filter_map(entry_key).collect());
    }
    if rest.trim_start().starts_with('{') {
        return multiline_flow_keys(lines, env_index);
    }
    let header = rest
        .split_once('#')
        .map_or(rest, |(before, _)| before)
        .trim();
    if !header.is_empty() {
        return Err(format!(
            "an `env:` header here is `{rest}`, which this contract cannot read; use a block or flow mapping"
        ));
    }
    let Some(env_line) = lines.get(env_index) else {
        return Ok(Vec::new());
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
        return Ok(Vec::new());
    };
    Ok(block
        .iter()
        .filter(|line| line.indent() == entry_indent)
        .filter_map(|line| entry_key(line.trimmed()))
        .collect())
}

/// Returns the body of a flow mapping, ignoring a trailing comment.
///
/// A flow mapping is `{ … }`; anything after the closing brace is a comment or
/// trailing whitespace.
fn flow_mapping(rest: &str) -> Option<&str> {
    let body = rest.trim().strip_prefix('{')?;
    body.get(..closing_brace(body)?)
}

/// Returns the index of the brace closing a flow mapping.
///
/// A `}` inside a quoted value does not close the mapping, so the scan tracks
/// the open quote before accepting a brace.
fn closing_brace(body: &str) -> Option<usize> {
    let mut state = Scan::default();
    for (index, character) in body.char_indices() {
        if state.step(character) {
            return Some(index);
        }
    }
    None
}

/// The lexical state while scanning a flow mapping for its closing brace.
///
/// The scan tracks quotes, escapes, and comments, because a `}` inside any of
/// them is data rather than the end of the mapping.
#[derive(Default)]
struct Scan {
    /// The open quote character, when inside a string.
    quote: Option<char>,
    /// Whether the previous character escaped the next one.
    escaped: bool,
    /// Whether the rest of the line is a comment.
    comment: bool,
}

impl Scan {
    /// Advances the scan by one character, reporting a closing brace.
    fn step(&mut self, character: char) -> bool {
        if self.escaped {
            self.escaped = false;
            return false;
        }
        if self.comment {
            self.comment = character != '\n';
            return false;
        }
        match character {
            '#' if self.quote.is_none() => self.comment = true,
            '\\' if self.quote == Some('"') => self.escaped = true,
            '"' | '\'' if self.quote == Some(character) => self.quote = None,
            '"' | '\'' if self.quote.is_none() => self.quote = Some(character),
            '}' if self.quote.is_none() => return true,
            _ => {}
        }
        false
    }
}

/// Returns the keys of a flow mapping that spans several lines.
///
/// The opening brace is followed by the entries and a closing brace on a later
/// line, so the span is gathered before the entries are read. A brace inside a
/// quoted value does not close the mapping.
fn multiline_flow_keys<'a>(lines: &[Line<'a>], env_index: usize) -> Result<Vec<&'a str>, String> {
    let Some(header) = lines.get(env_index) else {
        return Ok(Vec::new());
    };
    let Some(declared) = env_header_value(header.trimmed()) else {
        return Ok(Vec::new());
    };
    let Some(body) = declared.strip_prefix('{') else {
        return Ok(Vec::new());
    };
    // A mapping that closes on its own line is already read by the flow path.
    if closing_brace(body).is_some() {
        return Ok(Vec::new());
    }
    let mut keys: Vec<&'a str> = body.split(',').filter_map(entry_key).collect();
    for line in lines.iter().skip(env_index + 1) {
        let text = line.trimmed();
        let closing = closing_brace(text);
        let segment = closing.map_or(text, |end| text.get(..end).unwrap_or_default());
        keys.extend(segment.split(',').filter_map(entry_key));
        if closing.is_some() {
            return Ok(keys);
        }
    }
    Err("an `env:` flow mapping is not closed, so this contract cannot read it".to_owned())
}

/// Splits a `NAME: value` entry into its key, without quotes.
///
/// YAML permits a quoted key, and the quotes are syntax rather than part of the
/// name, so they are removed before the key is compared with the policy list.
fn entry_key(entry: &str) -> Option<&str> {
    entry
        .split_once(':')
        .map(|(key, _)| key.trim().trim_matches(['"', '\'']))
}
