//! Manifest readers and verifiers for the lint inheritance contract.
//!
//! `tests/lint_workspace_inheritance_contract.rs` states the contract and its
//! cases; this module holds the text readers and the verifiers they exercise,
//! so each file stays inside the repository's size limit.
//!
//! The manifests are read as text, following `test_helpers::toml_section` and a
//! line scan rather than adding a TOML parser dependency.

#[path = "walk.rs"]
mod walk;

pub(crate) use walk::{member_roots, package_directories};

use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;

/// The repository root, as the manifest directory.
pub(crate) const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The lint tables a member must inherit rather than define for itself.
const LINT_TABLES: [&str; 3] = ["lints.clippy", "lints.rust", "lints.rustdoc"];

/// The lint tables the root manifest must populate.
const WORKSPACE_LINT_TABLES: [&str; 3] = [
    "workspace.lints.clippy",
    "workspace.lints.rust",
    "workspace.lints.rustdoc",
];

/// Removes a TOML comment from a line, leaving string contents intact.
///
/// A `#` inside a string is data, so the scan tracks the open quote. TOML
/// accepts both basic and literal strings, so both are tracked.
pub(crate) fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    for (index, character) in line.char_indices() {
        match character {
            '"' | '\'' if quote == Some(character) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(character),
            '#' if quote.is_none() => return line.get(..index).unwrap_or(line),
            _ => {}
        }
    }
    line
}

/// Returns every TOML string in `text`, in order.
///
/// Reading the strings directly, rather than splitting on a delimiter, keeps a
/// quoted path inside a comment from being mistaken for an array item.
fn quoted_items(text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in text.chars() {
        match quote {
            Some(active) if character == active => {
                items.push(std::mem::take(&mut current));
                quote = None;
            }
            Some(_) => current.push(character),
            None if character == '"' || character == '\'' => quote = Some(character),
            None => {}
        }
    }
    items
}

/// Extracts the string items of a top-level TOML array.
///
/// The array is written across several lines in the root manifest, so the
/// body is joined before the quoted items are read. Comments are removed
/// first, because a commented path is not a member.
pub(crate) fn array_items(document: &str, key: &str) -> Option<Vec<String>> {
    let start = document.lines().position(|line| {
        line.split_once('=')
            .is_some_and(|(name, _)| name.trim().trim_matches(['"', '\'']) == key)
    })?;
    let mut body = String::new();
    let mut depth = 0usize;
    for (offset, raw) in document.lines().skip(start).enumerate() {
        let line = strip_comment(raw);
        // The key itself is not a value: a quoted key would otherwise be read
        // as the array's first item.
        let value = if offset == 0 {
            line.split_once('=').map_or("", |(_, value)| value)
        } else {
            line
        };
        depth += value.matches('[').count();
        depth = depth.saturating_sub(value.matches(']').count());
        body.push_str(value);
        body.push(' ');
        if depth == 0 {
            break;
        }
    }
    Some(quoted_items(&body))
}

/// Reports whether a manifest names a table header.
fn declares(document: &str, table: &str) -> bool {
    let header = format!("[{table}]");
    document
        .lines()
        .any(|line| line.trim_end() == header.as_str())
}

/// Provides read access to the repository tree.
pub(crate) struct Repository {
    /// A capability-scoped handle to the repository root.
    dir: Dir,
}

impl Repository {
    /// Opens the repository root.
    ///
    /// # Errors
    ///
    /// Returns the reason when the root cannot be opened.
    pub(crate) fn open() -> Result<Self, String> {
        let dir = Dir::open_ambient_dir(ROOT, ambient_authority())
            .map_err(|error| format!("opening {ROOT}: {error}"))?;
        Ok(Self { dir })
    }

    /// Reads a repository-relative file.
    ///
    /// # Errors
    ///
    /// Returns the reason when the file cannot be read.
    pub(crate) fn read(&self, path: &str) -> Result<String, String> {
        self.dir
            .read_to_string(path)
            .map_err(|error| format!("reading {path}: {error}"))
    }

    /// Returns the directories under `root` one level down, as repository
    /// paths.
    ///
    /// First-level packages live under `crates/` in this workspace, but a
    /// member pattern may name any tree, so the scan is rooted at the
    /// caller's choice.
    ///
    /// # Errors
    ///
    /// Returns the reason when `root` cannot be listed.
    pub(crate) fn subdirectories(&self, root: &str) -> Result<Vec<String>, String> {
        let entries = self
            .dir
            .read_dir(root)
            .map_err(|error| format!("listing {root}: {error}"))?;
        let mut paths = Vec::new();
        for entry in entries {
            let item = entry.map_err(|error| format!("reading a {root} entry: {error}"))?;
            let name = item
                .file_name()
                .map_err(|error| format!("reading a {root} entry name: {error}"))?;
            let is_directory = item
                .file_type()
                .map_err(|error| format!("reading the type of {root}/{name}: {error}"))?
                .is_dir();
            if is_directory {
                paths.push(format!("{root}/{name}"));
            }
        }
        Ok(paths)
    }
}

/// Returns the member paths the root manifest lists.
///
/// # Errors
///
/// Returns the reason when the manifest cannot be read or names no members.
pub(crate) fn workspace_members(repository: &Repository) -> Result<Vec<String>, String> {
    let manifest = repository.read("Cargo.toml")?;
    // `members` belongs to `[workspace]`; a same-named key elsewhere is a
    // different declaration and must not be read as the member list.
    let workspace = test_helpers::toml_section(&manifest, "workspace")
        .ok_or_else(|| "the root Cargo.toml must declare a [workspace] table".to_owned())?;
    let members = array_items(&workspace, "members")
        .ok_or_else(|| "the root Cargo.toml must declare a `members` array".to_owned())?;
    if members.is_empty() {
        return Err("the root Cargo.toml must list at least one workspace member".to_owned());
    }
    Ok(members)
}

/// Reports whether a `[lints]` section body is exactly the workspace opt-in.
///
/// TOML ignores whitespace around `=`, so the check normalizes each assignment
/// rather than comparing raw text, and rejects any other declaration in the
/// table. A trailing or whole-line comment is prose, not a declaration.
pub(crate) fn is_workspace_opt_in(body: &str) -> bool {
    let declarations: Vec<String> = body
        .lines()
        .map(strip_comment)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split_once('=').map_or_else(
                || line.to_owned(),
                |(key, value)| {
                    format!(
                        "{} = {}",
                        key.trim().trim_matches(['"', '\'']),
                        value.trim()
                    )
                },
            )
        })
        .collect();
    declarations == ["workspace = true"]
}

/// Fails unless a manifest opts into the workspace lints.
///
/// # Errors
///
/// Returns the missing opt-in.
fn verify_opt_in(path: &str, manifest: &str) -> Result<(), String> {
    let body = test_helpers::toml_section(manifest, "lints").unwrap_or_default();
    if is_workspace_opt_in(&body) {
        Ok(())
    } else {
        Err(format!(
            "{path} must declare `[lints]\\nworkspace = true` so the workspace policy applies"
        ))
    }
}

/// Fails unless the root manifest opts into and populates its lint tables.
///
/// # Errors
///
/// Returns the first missing opt-in or empty table.
pub(crate) fn verify_root_manifest(manifest: &str) -> Result<(), String> {
    verify_opt_in("the root Cargo.toml", manifest)?;
    for table in WORKSPACE_LINT_TABLES {
        let body = test_helpers::toml_section(manifest, table)
            .ok_or_else(|| format!("the root Cargo.toml must declare [{table}]"))?;
        let has_rule = body
            .lines()
            .any(|line| !line.trim().is_empty() && !line.trim().starts_with('#'));
        if !has_rule {
            return Err(format!("[{table}] must carry at least one rule"));
        }
    }
    Ok(())
}

/// Fails unless a member manifest opts into the workspace lints.
///
/// # Errors
///
/// Returns the first local lint table, or the missing opt-in.
pub(crate) fn verify_member_manifest(path: &str, manifest: &str) -> Result<(), String> {
    for table in LINT_TABLES {
        if declares(manifest, table) {
            return Err(format!(
                "{path} must inherit lint rules from the workspace, not define [{table}]"
            ));
        }
    }
    verify_opt_in(path, manifest)
}

/// Reports whether a `members` pattern is one Cargo accepts.
///
/// Cargo supports a `*` only as a whole final path segment: `crates/*` and
/// `crates/group/*` are valid, while `crates/theorem-*` is not. A partial
/// segment would resolve differently under Cargo than under this checker, so
/// the caller reports it rather than guessing.
fn is_supported_member_pattern(pattern: &str) -> bool {
    let wanted = pattern.trim_end_matches('/');
    let segments: Vec<&str> = wanted.split('/').collect();
    segments.iter().enumerate().all(|(index, segment)| {
        if segment.contains(['?', '[']) {
            // Cargo accepts these glob characters; this checker does not, so
            // the pattern is reported rather than matched differently.
            return false;
        }
        !segment.contains('*') || (*segment == "*" && index + 1 == segments.len())
    })
}

/// Returns the member patterns this checker cannot interpret.
pub(crate) fn unsupported_member_patterns(members: &[String]) -> Vec<String> {
    members
        .iter()
        .filter(|pattern| !is_supported_member_pattern(pattern))
        .cloned()
        .collect()
}

/// Reports whether a `members` pattern names `path`.
///
/// A final `*` covers every directory one level below its parent, and a
/// pattern with no `*` must match exactly.
fn member_covers(pattern: &str, path: &str) -> bool {
    let wanted = pattern.trim_end_matches('/');
    let candidate = path.trim_end_matches('/');
    let Some(parent) = wanted.strip_suffix("/*") else {
        return wanted == candidate;
    };
    candidate
        .strip_prefix(parent)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
}

/// Returns the package directories the workspace does not list.
///
/// A package directory that is no member is compiled by no gate and linted by
/// none either, so the caller treats any entry here as a failure.
pub(crate) fn undeclared_crates(package_directories: &[String], members: &[String]) -> Vec<String> {
    package_directories
        .iter()
        .filter(|path| !members.iter().any(|entry| member_covers(entry, path)))
        .cloned()
        .collect()
}

/// Returns the member manifest paths, with every glob expanded to its matches.
pub(crate) fn resolved_members(members: &[String], package_directories: &[String]) -> Vec<String> {
    members
        .iter()
        .flat_map(|entry| {
            if entry.contains('*') {
                package_directories
                    .iter()
                    .filter(|path| member_covers(entry, path))
                    .cloned()
                    .collect()
            } else {
                vec![entry.clone()]
            }
        })
        .collect()
}

/// Returns the manifest path of a member directory.
pub(crate) fn member_manifest_path(member: &str) -> String {
    Utf8Path::new(member).join("Cargo.toml").into_string()
}
