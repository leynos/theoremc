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
pub(super) fn environment_keys<'a>(lines: &[Line<'a>], indent: usize) -> Vec<&'a str> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.indent() == indent || line.indent() + 2 == indent)
        .filter_map(|(index, line)| {
            let trimmed = line.trimmed();
            let entry = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            let rest = entry.strip_prefix("env:")?.trim();
            Some((index, rest))
        })
        .flat_map(|(index, rest)| {
            let mut keys = env_block_keys(lines, index, rest);
            keys.extend(multiline_flow_keys(lines, index));
            keys
        })
        .collect()
}

/// Returns the keys a single `env:` entry declares.
///
/// A flow mapping is read from the `env:` line itself; a block mapping is read
/// from the entries that follow, down to the first content line at or above
/// the key's own indentation. Blank lines and comments never end the block.
fn env_block_keys<'a>(lines: &[Line<'a>], env_index: usize, rest: &'a str) -> Vec<&'a str> {
    if let Some(flow) = flow_mapping(rest) {
        return flow.split(',').filter_map(entry_key).collect();
    }
    let header = rest
        .split_once('#')
        .map_or(rest, |(before, _)| before)
        .trim();
    if !header.is_empty() {
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

/// Returns the body of a flow mapping, ignoring a trailing comment.
///
/// A flow mapping is `{ … }`; anything after the closing brace is a comment or
/// trailing whitespace.
fn flow_mapping(rest: &str) -> Option<&str> {
    let body = rest.trim().strip_prefix('{')?;
    body.get(..body.find('}')?)
}

/// Returns the keys of a flow mapping that spans several lines.
///
/// The opening brace is followed by the entries and a closing brace on a later
/// line, so the span is gathered before the entries are read. A brace inside a
/// quoted value does not close the mapping.
fn multiline_flow_keys<'a>(lines: &[Line<'a>], env_index: usize) -> Vec<&'a str> {
    let Some(header) = lines.get(env_index) else {
        return Vec::new();
    };
    let Some(declared) = header.trimmed().strip_prefix("env:") else {
        return Vec::new();
    };
    if declared.trim() != "{" {
        return Vec::new();
    }
    lines
        .iter()
        .skip(env_index + 1)
        .take_while(|line| !line.trimmed().starts_with('}'))
        .filter_map(|line| entry_key(line.trimmed()))
        .collect()
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
