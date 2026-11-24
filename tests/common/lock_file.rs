//! A minimal reader and rewriter for the `[[package]]` entries of a Cargo lock file.
//!
//! The fixture lock contract compares three fields of each entry and nothing
//! else, so the reader stays small and dependency-free: the workspace has no
//! direct TOML parser, and adding one to read a file Cargo writes in a fixed
//! shape would be disproportionate.

use std::fmt::Write;

/// One `[[package]]` entry of a Cargo lock file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockPackage<'a> {
    /// The package name.
    pub name: &'a str,
    /// The locked version.
    pub version: &'a str,
    /// The registry or git source; `None` for a path package.
    pub source: Option<&'a str>,
}

/// Returns every `[[package]]` entry of `lock`, in file order.
///
/// Only `name`, `version` and `source` are read. A `dependencies` list, a
/// checksum and any other key are ignored, because the fixture contract asks
/// which packages a lock resolves and not what each entry records about them.
///
/// # Errors
///
/// Returns an error when an entry omits its name or version, which a lock Cargo
/// wrote cannot do; an unreadable lock must not read as an empty one.
pub fn lock_packages(lock: &str) -> Result<Vec<LockPackage<'_>>, String> {
    let mut packages = Vec::new();
    let mut entry: Option<Entry<'_>> = None;
    for raw in lock.lines() {
        let trimmed = raw.trim();
        if trimmed == "[[package]]" {
            if let Some(complete) = entry.take() {
                packages.push(complete.finish()?);
            }
            entry = Some(Entry::default());
            continue;
        }
        if let Some(open) = entry.as_mut() {
            open.read(trimmed);
        }
    }
    if let Some(complete) = entry {
        packages.push(complete.finish()?);
    }
    Ok(packages)
}

/// The fields of one entry as they are read, before the entry is complete.
#[derive(Default)]
struct Entry<'a> {
    /// The entry's `name`, once its line has been read.
    name: Option<&'a str>,
    /// The entry's `version`, once its line has been read.
    version: Option<&'a str>,
    /// The entry's `source`, once its line has been read.
    source: Option<&'a str>,
}

impl<'a> Entry<'a> {
    /// Records `line` when it assigns one of the three fields this crate reads.
    ///
    /// The first assignment wins, so a `dependencies` string that happens to
    /// contain an `=` cannot overwrite a field the entry already stated.
    fn read(&mut self, line: &'a str) {
        for (key, slot) in [
            ("name", &mut self.name),
            ("version", &mut self.version),
            ("source", &mut self.source),
        ] {
            if slot.is_none()
                && let Some(value) = assigned_string(line, key)
            {
                *slot = Some(value);
            }
        }
    }

    /// Returns the finished entry.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry names no package or no version.
    fn finish(self) -> Result<LockPackage<'a>, String> {
        let name = self
            .name
            .ok_or_else(|| "lock entry has no name".to_owned())?;
        let version = self
            .version
            .ok_or_else(|| format!("lock entry {name} has no version"))?;
        Ok(LockPackage {
            name,
            version,
            source: self.source,
        })
    }
}

/// Returns the value of a `key = "value"` line.
///
/// The fixture rename reads a template's `name` and `source` lines with it, so
/// it is shared rather than repeated.
pub(super) fn assigned_string<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let value = line
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    value.strip_prefix('"')?.strip_suffix('"')
}

/// Returns `lock` with the path entry named `from` renamed to `to`.
///
/// A template's root entry records no `source`, which is what distinguishes it
/// from a registry package of the same name. Requiring exactly one such entry
/// keeps the rename from rewriting a dependency.
///
/// # Errors
///
/// Returns an error when no entry names `from`, when more than one does, or
/// when the only candidate records a source.
pub(super) fn with_renamed_root(lock: &str, from: &str, to: &str) -> Result<String, String> {
    let lines: Vec<&str> = lock.lines().collect();
    let target = root_name_line(&lines, from)?;
    let mut rendered = String::with_capacity(lock.len());
    for (index, line) in lines.iter().enumerate() {
        match index {
            _ if index == target => {
                write!(rendered, "name = \"{to}\"")
                    .map_err(|error| format!("rendering the renamed root: {error}"))?;
            }
            _ => rendered.push_str(line),
        }
        rendered.push('\n');
    }
    Ok(rendered)
}

/// Returns the line index of the source-less entry naming `root`.
///
/// # Errors
///
/// Returns an error when no entry names the template root, or when more than
/// one does.
fn root_name_line(lines: &[&str], root: &str) -> Result<usize, String> {
    let mut found = None;
    for (start, end) in package_blocks(lines) {
        let block = lines
            .get(start..end)
            .ok_or_else(|| format!("fixture lock block {start}..{end} is out of range"))?;
        if block
            .iter()
            .any(|line| assigned_string(line.trim(), "source").is_some())
        {
            continue;
        }
        let Some(offset) = block
            .iter()
            .position(|line| assigned_string(line.trim(), "name") == Some(root))
        else {
            continue;
        };
        if found.is_some() {
            return Err(format!(
                "fixture lock names {root:?} as a root package more than once"
            ));
        }
        found = Some(start + offset);
    }
    found.ok_or_else(|| format!("fixture lock names no root package {root:?}"))
}

/// Returns the `[[package]]` blocks of a lock as `[start, end)` line ranges.
fn package_blocks(lines: &[&str]) -> Vec<(usize, usize)> {
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim() == "[[package]]")
        .map(|(index, _)| index)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(position, start)| {
            let end = starts.get(position + 1).copied().unwrap_or(lines.len());
            (*start, end)
        })
        .collect()
}

#[cfg(test)]
#[path = "lock_file_tests.rs"]
mod tests;
