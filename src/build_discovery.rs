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
    fn root_only() -> Self {
        Self {
            theorem_files: Vec::new(),
            watched_directories: vec![Utf8PathBuf::from(THEOREMS_DIR)],
        }
    }

    fn sort_and_dedup(&mut self) {
        self.theorem_files.sort();
        self.theorem_files.dedup();
        self.watched_directories.sort();
        self.watched_directories.dedup();
    }

    /// Returns the exact rerun path order emitted by `build.rs`.
    pub(crate) fn rerun_paths(&self) -> impl Iterator<Item = &Utf8Path> {
        self.watched_directories
            .iter()
            .map(Utf8PathBuf::as_path)
            .chain(self.theorem_files.iter().map(Utf8PathBuf::as_path))
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
    let crate_root =
        Dir::open_ambient_dir(manifest_dir, ambient_authority()).map_err(|source| {
            BuildDiscoveryError::Io {
                operation: "open crate root",
                path: manifest_dir.to_path_buf(),
                source,
            }
        })?;
    let theorem_root = Utf8Path::new(THEOREMS_DIR);

    match crate_root.metadata(theorem_root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(BuildDiscoveryError::TheoremRootNotDirectory {
                path: theorem_root.to_path_buf(),
            });
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(BuildDiscovery::root_only());
        }
        Err(source) => {
            return Err(BuildDiscoveryError::Io {
                operation: "inspect theorem root",
                path: theorem_root.to_path_buf(),
                source,
            });
        }
    }

    let theorem_dir =
        crate_root
            .open_dir(theorem_root)
            .map_err(|source| BuildDiscoveryError::Io {
                operation: "open theorem directory",
                path: theorem_root.to_path_buf(),
                source,
            })?;
    let mut discovery = BuildDiscovery {
        theorem_files: Vec::new(),
        watched_directories: Vec::new(),
    };

    collect_theorem_inputs(&theorem_dir, theorem_root, &mut discovery)?;
    discovery.sort_and_dedup();
    Ok(discovery)
}

fn collect_theorem_inputs(
    directory: &Dir,
    relative_dir: &Utf8Path,
    discovery: &mut BuildDiscovery,
) -> Result<(), BuildDiscoveryError> {
    discovery
        .watched_directories
        .push(normalise_relative_path(relative_dir));

    let entries = directory
        .entries()
        .map_err(|source| BuildDiscoveryError::Io {
            operation: "read theorem directory",
            path: relative_dir.to_path_buf(),
            source,
        })?;

    for entry_result in entries {
        let entry = entry_result.map_err(|source| BuildDiscoveryError::Io {
            operation: "read theorem directory entry",
            path: relative_dir.to_path_buf(),
            source,
        })?;
        collect_entry(&entry, relative_dir, discovery)?;
    }

    Ok(())
}

fn collect_entry(
    entry: &DirEntry,
    relative_dir: &Utf8Path,
    discovery: &mut BuildDiscovery,
) -> Result<(), BuildDiscoveryError> {
    let file_name = entry
        .file_name()
        .map_err(|source| BuildDiscoveryError::Io {
            operation: "read theorem entry name",
            path: relative_dir.to_path_buf(),
            source,
        })?;
    let relative_path = relative_dir.join(&file_name);
    let file_type = entry
        .file_type()
        .map_err(|source| BuildDiscoveryError::Io {
            operation: "inspect theorem entry",
            path: relative_path.clone(),
            source,
        })?;

    if file_type.is_dir() {
        let child_dir = entry.open_dir().map_err(|source| BuildDiscoveryError::Io {
            operation: "open theorem directory",
            path: relative_path.clone(),
            source,
        })?;
        return collect_theorem_inputs(&child_dir, &relative_path, discovery);
    }

    if file_type.is_file() && is_theorem_path(&relative_path) {
        discovery
            .theorem_files
            .push(normalise_relative_path(&relative_path));
    }

    Ok(())
}

fn is_theorem_path(path: &Utf8Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "theorem")
}

fn normalise_relative_path(path: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(
        path.components()
            .map(|component| component.as_str())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

#[cfg(test)]
#[path = "build_discovery_tests.rs"]
mod tests;
