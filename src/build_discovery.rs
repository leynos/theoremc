//! Discovers theorem files for Cargo build invalidation.
//!
//! Step 3.1.1 stops at deterministic discovery plus rerun metadata. It does
//! not generate `OUT_DIR/theorem_suite.rs`; Step 3.1.2 will consume the
//! ordered theorem file list and own per-file code generation.

use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs_utf8::{Dir, DirEntry},
};
use thiserror::Error;

const THEOREMS_DIR: &str = "theorems";

/// Ordered theorem inputs plus the directories Cargo should watch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildDiscovery {
    theorem_files: Vec<Utf8PathBuf>,
    watched_directories: Vec<Utf8PathBuf>,
}

impl BuildDiscovery {
    /// Returns a discovery result that watches only the root `theorems`
    /// directory, with no discovered theorem files.
    fn root_only() -> Self {
        Self {
            theorem_files: Vec::new(),
            watched_directories: vec![Utf8PathBuf::from(THEOREMS_DIR)],
        }
    }

    /// Sorts both path vectors lexicographically and removes duplicates so
    /// emitted rerun lines and future suite inputs are deterministic.
    fn sort_and_dedup(&mut self) {
        self.theorem_files.sort();
        self.theorem_files.dedup();
        self.watched_directories.sort();
        self.watched_directories.dedup();
    }

    /// Returns the discovered theorem files in deterministic order.
    pub(crate) fn theorem_files(&self) -> impl Iterator<Item = &Utf8Path> {
        self.theorem_files.iter().map(Utf8PathBuf::as_path)
    }

    /// Returns the watched directories in deterministic order.
    pub(crate) fn watched_directories(&self) -> impl Iterator<Item = &Utf8Path> {
        self.watched_directories.iter().map(Utf8PathBuf::as_path)
    }

    /// Returns the exact rerun path order emitted by `build.rs`.
    pub(crate) fn rerun_paths(&self) -> impl Iterator<Item = &Utf8Path> {
        self.watched_directories().chain(self.theorem_files())
    }
}

/// Filesystem-traversal failures during build discovery.
#[derive(Debug, Error)]
pub(crate) enum BuildDiscoveryError {
    #[error("could not {operation} '{path}': {source}")]
    Io {
        operation: &'static str,
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("theorem root '{path}' exists but is not a directory")]
    TheoremRootNotDirectory { path: Utf8PathBuf },
}

/// Discovers theorem files below `CARGO_MANIFEST_DIR/theorems`.
///
/// Returned theorem paths and watched directories are crate-relative and
/// normalized to forward slashes so downstream name mangling stays stable
/// across host platforms.
///
/// # Errors
///
/// Returns [`BuildDiscoveryError`] when the crate root cannot be opened or the
/// theorem tree cannot be traversed. An absent `theorems/` directory is not an
/// error; it returns a root-only watch set.
pub(crate) fn discover_theorem_inputs(
    manifest_dir: &Utf8Path,
) -> Result<BuildDiscovery, BuildDiscoveryError> {
    let crate_root = Dir::open_ambient_dir(manifest_dir, ambient_authority())
        .map_err(|source| io_err("open crate root", manifest_dir, source))?;
    let theorem_root = Utf8Path::new(THEOREMS_DIR);
    let Some(theorem_dir) = open_theorem_root(&crate_root, theorem_root)? else {
        return Ok(BuildDiscovery::root_only());
    };
    let mut discovery = BuildDiscovery {
        theorem_files: Vec::new(),
        watched_directories: Vec::new(),
    };

    collect_theorem_inputs(&theorem_dir, theorem_root, &mut discovery)?;
    discovery.sort_and_dedup();
    Ok(discovery)
}

/// Recursively collects theorem files and watched directories from a single
/// directory level, appending results to `discovery`.
fn collect_theorem_inputs(
    directory: &Dir,
    relative_dir: &Utf8Path,
    discovery: &mut BuildDiscovery,
) -> Result<(), BuildDiscoveryError> {
    discovery
        .watched_directories
        .push(relative_dir.to_path_buf());

    let entries = directory
        .entries()
        .map_err(|source| io_err("read theorem directory", relative_dir, source))?;

    for entry_result in entries {
        let entry = entry_result
            .map_err(|source| io_err("read theorem directory entry", relative_dir, source))?;
        collect_entry(&entry, relative_dir, discovery)?;
    }

    Ok(())
}

/// Classifies a single directory entry: recurses into subdirectories and
/// appends `.theorem` files to the discovery result.
fn collect_entry(
    entry: &DirEntry,
    relative_dir: &Utf8Path,
    discovery: &mut BuildDiscovery,
) -> Result<(), BuildDiscoveryError> {
    let file_name = entry
        .file_name()
        .map_err(|source| io_err("read theorem entry name", relative_dir, source))?;
    let relative_path = relative_dir.join(&file_name);
    let file_type = entry
        .file_type()
        .map_err(|source| io_err("inspect theorem entry", &relative_path, source))?;

    if file_type.is_dir() {
        let child_dir = entry
            .open_dir()
            .map_err(|source| io_err("open theorem directory", &relative_path, source))?;
        return collect_theorem_inputs(&child_dir, &relative_path, discovery);
    }

    if file_type.is_file() && is_theorem_path(&relative_path) {
        discovery.theorem_files.push(relative_path);
    }

    Ok(())
}

/// Opens the `theorems` directory if it exists and is a directory, returning
/// `None` for a missing directory and an error for a non-directory path.
fn open_theorem_root(
    crate_root: &Dir,
    theorem_root: &Utf8Path,
) -> Result<Option<Dir>, BuildDiscoveryError> {
    let metadata = match crate_root.metadata(theorem_root) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io_err("inspect theorem root", theorem_root, source)),
    };

    if !metadata.is_dir() {
        return Err(BuildDiscoveryError::TheoremRootNotDirectory {
            path: theorem_root.to_path_buf(),
        });
    }

    crate_root
        .open_dir(theorem_root)
        .map(Some)
        .map_err(|source| io_err("open theorem directory", theorem_root, source))
}

/// Returns `true` when the path has a `.theorem` file extension.
fn is_theorem_path(path: &Utf8Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "theorem")
}

/// Constructs a [`BuildDiscoveryError::Io`] with the given operation label,
/// path, and underlying IO error.
fn io_err(operation: &'static str, path: &Utf8Path, source: io::Error) -> BuildDiscoveryError {
    BuildDiscoveryError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
#[path = "build_discovery_tests.rs"]
mod tests;
