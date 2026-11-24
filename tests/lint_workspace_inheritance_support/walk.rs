//! Package discovery for the lint inheritance contract.
//!
//! Cargo's `members` may name a directory anywhere in the repository, and a
//! pattern may reach a package nested below another. The walk here records
//! every directory that holds a `Cargo.toml`, so a member pattern can be
//! resolved against the real tree rather than a guess about its depth.

use super::Repository;

/// Directory names that hold a crate's own sources, not further packages.
///
/// A package's `src`, `tests`, `benches`, `examples`, and build output never
/// contain a workspace member, so descending into them would only slow the
/// walk and risk a depth error on an ordinary crate.
const NON_PACKAGE_DIRECTORIES: [&str; 5] = ["src", "target", "node_modules", ".git", ".cargo"];

/// How far the package walk descends below a member root.
///
/// A package nested deeper than this would be unusual, and the bound keeps a
/// stray tree from stalling the contract. The caller reports a walk that
/// reaches it, rather than silently returning a short inventory.
const MAX_WALK_DEPTH: usize = 4;

/// Returns every package directory reachable from `roots`.
///
/// A package directory holds a `Cargo.toml`. The roots are the first segments
/// of the `members` patterns, so a workspace that names `tools/*` or `libs/*`
/// is scanned where its members actually live. A literal member outside every
/// root is still read, because it needs no scan to be resolved.
///
/// # Errors
///
/// Returns the reason when a root cannot be listed, because a silent empty
/// result would let every member check pass.
pub(crate) fn package_directories(
    repository: &Repository,
    roots: &[String],
) -> Result<Vec<String>, String> {
    let mut packages = Vec::new();
    for root in roots {
        collect_packages(repository, root, &mut packages, 0)?;
    }
    Ok(packages)
}

/// Walks `directory`, recording packages and descending through its children.
///
/// The walk continues below a package, because Cargo's `members` may name a
/// nested package and the parent's own manifest does not cover it.
///
/// # Errors
///
/// Returns the reason when a directory cannot be listed, or when the walk
/// reaches its depth limit.
fn collect_packages(
    repository: &Repository,
    directory: &str,
    packages: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_WALK_DEPTH {
        return Err(format!(
            "{directory} is nested deeper than {MAX_WALK_DEPTH} levels, so package discovery cannot reach it"
        ));
    }
    for child in repository.subdirectories(directory)? {
        if is_crate_source(&child) {
            continue;
        }
        match repository.read(&format!("{child}/Cargo.toml")) {
            Ok(_) => packages.push(child.clone()),
            // A child without a manifest is a group directory, not an error.
            Err(error) if error.contains("No such file") || error.contains("Not a directory") => {}
            Err(error) => return Err(error),
        }
        collect_packages(repository, &child, packages, depth + 1)?;
    }
    Ok(())
}

/// Reports whether a path names a directory that holds crate sources.
fn is_crate_source(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| NON_PACKAGE_DIRECTORIES.contains(&name))
}

/// Returns the first path segment of each `members` pattern.
///
/// `crates/*` and `crates/group/*` both root at `crates`, so scanning the
/// first segment covers every pattern below it. Duplicates are removed so a
/// tree is walked once.
pub(crate) fn member_roots(members: &[String]) -> Vec<String> {
    let mut roots: Vec<String> = Vec::new();
    for member in members {
        let Some(root) = member.split('/').next() else {
            continue;
        };
        if root.is_empty() || root.contains('*') {
            continue;
        }
        if !roots.iter().any(|known| known == root) {
            roots.push(root.to_owned());
        }
    }
    roots
}
