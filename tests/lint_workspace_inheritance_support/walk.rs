//! Package discovery for the lint inheritance contract.
//!
//! Cargo's `members` may name a directory anywhere in the repository, and a
//! pattern may reach a package nested below another. The walk here records
//! every directory that holds a `Cargo.toml`, so a member pattern can be
//! resolved against the real tree rather than a guess about its depth.

use super::Repository;

/// How far the package walk descends below `crates` and `tools`.
///
/// A workspace that nests packages deeper than this would not be reachable by
/// a sane member pattern, and bounding the walk keeps a stray tree from
/// stalling the contract test.
const MAX_WALK_DEPTH: usize = 4;

/// Returns every package directory the repository holds.
///
/// A package directory holds a `Cargo.toml`. The workspace's members live
/// under `crates/` and `tools/`, so both trees are walked; a literal member
/// outside them is still read, because it needs no scan to be resolved. The
/// walk descends through directories without a manifest, so a member pattern
/// such as `crates/group/*` is reachable. A tree that cannot be listed
/// contributes no packages, which the member checks then report rather than
/// silently accepting.
/// # Errors
///
/// Returns the reason when a tree cannot be listed, because a silent empty
/// result would let every member check pass.
pub(crate) fn package_directories(repository: &Repository) -> Result<Vec<String>, String> {
    let mut packages = Vec::new();
    for root in ["crates", "tools"] {
        collect_packages(repository, root, &mut packages, 0)?;
    }
    Ok(packages)
}

/// Walks `directory`, recording packages and descending through its children.
///
/// The walk continues below a package, because Cargo's `members` may name a
/// nested package and the parent's own manifest does not cover it. `depth`
/// bounds the walk so a deep tree cannot stall the test.
///
/// # Errors
///
/// Returns the reason when a directory cannot be listed.
fn collect_packages(
    repository: &Repository,
    directory: &str,
    packages: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_WALK_DEPTH {
        return Ok(());
    }
    for child in repository.subdirectories(directory)? {
        if repository.read(&format!("{child}/Cargo.toml")).is_ok() {
            packages.push(child.clone());
        }
        collect_packages(repository, &child, packages, depth + 1)?;
    }
    Ok(())
}
