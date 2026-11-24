//! A minimal reader for the `[[package]]` entries of a Cargo lock file.
//!
//! The fixture lock contract compares three fields of each entry and nothing
//! else, so the reader stays small and dependency-free: the workspace has no
//! direct TOML parser, and adding one to read a file Cargo writes in a fixed
//! shape would be disproportionate.

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
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            if let Some(complete) = entry.take() {
                packages.push(complete.finish()?);
            }
            entry = Some(Entry::default());
            continue;
        }
        if let Some(open) = entry.as_mut() {
            open.read(line);
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

#[cfg(test)]
#[path = "lock_file_tests.rs"]
mod tests;
