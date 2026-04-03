//! Direct tests for build-time theorem discovery.

use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{BuildDiscovery, BuildDiscoveryError, discover_theorem_inputs};

struct DiscoveryFixture {
    _temp_dir: tempfile::TempDir,
    manifest_dir: Utf8PathBuf,
    dir: Dir,
}

impl DiscoveryFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| io::Error::other("temp dir path is not valid UTF-8"))?
            .to_path_buf();
        let dir = Dir::open_ambient_dir(&manifest_dir, ambient_authority())?;

        Ok(Self {
            _temp_dir: temp_dir,
            manifest_dir,
            dir,
        })
    }

    fn create_dir_all(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.dir.create_dir_all(path)?;
        Ok(())
    }

    fn write(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = Utf8Path::new(path).parent() {
            if !parent.as_str().is_empty() {
                self.dir.create_dir_all(parent)?;
            }
        }
        self.dir.write(path, "fixture")?;
        Ok(())
    }

    fn discover(&self) -> BuildDiscovery {
        discover_theorem_inputs(&self.manifest_dir).expect("fixture discovery should succeed")
    }
}

fn discovered_paths(discovery: &BuildDiscovery) -> Vec<&str> {
    discovery.theorem_files().map(Utf8Path::as_str).collect()
}

fn watched_directories(discovery: &BuildDiscovery) -> Vec<&str> {
    discovery
        .watched_directories()
        .map(Utf8Path::as_str)
        .collect()
}

fn rerun_paths(discovery: &BuildDiscovery) -> Vec<&str> {
    discovery.rerun_paths().map(Utf8Path::as_str).collect()
}

fn assert_root_watch_only(discovery: &BuildDiscovery) {
    assert!(discovered_paths(discovery).is_empty());
    assert_eq!(watched_directories(discovery), vec!["theorems"]);
    assert_eq!(rerun_paths(discovery), vec!["theorems"]);
}

#[test]
fn missing_theorems_directory_returns_root_watch_only() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    assert_root_watch_only(&fixture.discover());
}

#[test]
fn empty_theorems_directory_returns_root_watch_only() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .create_dir_all("theorems")
        .expect("theorem root should be created");
    assert_root_watch_only(&fixture.discover());
}

#[test]
fn theorem_root_file_returns_not_directory_error() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .write("theorems")
        .expect("theorem root file should be written");

    let error = discover_theorem_inputs(&fixture.manifest_dir)
        .expect_err("file theorem root should fail discovery");
    let message = error.to_string();

    match &error {
        BuildDiscoveryError::TheoremRootNotDirectory { path } => {
            assert_eq!(path.as_str(), "theorems");
        }
        other @ BuildDiscoveryError::Io { .. } => {
            panic!("expected TheoremRootNotDirectory error, got {other:?}");
        }
    }

    assert_eq!(
        message,
        "theorem root 'theorems' exists but is not a directory"
    );
}

#[test]
fn discovers_nested_theorem_files_and_nested_watch_directories() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .write("theorems/root.theorem")
        .expect("root theorem should be written");
    fixture
        .write("theorems/nested/alpha.theorem")
        .expect("nested theorem should be written");
    fixture
        .write("theorems/nested/deeper/beta.theorem")
        .expect("deeper theorem should be written");

    let discovery = fixture.discover();

    assert_eq!(
        discovered_paths(&discovery),
        vec![
            "theorems/nested/alpha.theorem",
            "theorems/nested/deeper/beta.theorem",
            "theorems/root.theorem",
        ]
    );
    assert_eq!(
        watched_directories(&discovery),
        vec!["theorems", "theorems/nested", "theorems/nested/deeper",]
    );
    assert_eq!(
        rerun_paths(&discovery),
        vec![
            "theorems",
            "theorems/nested",
            "theorems/nested/deeper",
            "theorems/nested/alpha.theorem",
            "theorems/nested/deeper/beta.theorem",
            "theorems/root.theorem",
        ]
    );
}

#[test]
fn ignores_non_theorem_files() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .write("theorems/notes.txt")
        .expect("ignored note should be written");
    fixture
        .write("theorems/config.yaml")
        .expect("ignored yaml should be written");
    fixture
        .write("theorems/temp.theorem.bak")
        .expect("ignored backup should be written");
    fixture
        .write("theorems/kept.theorem")
        .expect("theorem should be written");

    let discovery = fixture.discover();

    assert_eq!(discovered_paths(&discovery), vec!["theorems/kept.theorem"]);
    assert_eq!(
        rerun_paths(&discovery),
        vec!["theorems", "theorems/kept.theorem"]
    );
}

#[test]
fn sorts_theorem_files_deterministically_regardless_of_creation_order() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .write("theorems/zeta.theorem")
        .expect("zeta theorem should be written");
    fixture
        .write("theorems/alpha.theorem")
        .expect("alpha theorem should be written");
    fixture
        .write("theorems/middle/theta.theorem")
        .expect("theta theorem should be written");

    let discovery = fixture.discover();

    assert_eq!(
        discovered_paths(&discovery),
        vec![
            "theorems/alpha.theorem",
            "theorems/middle/theta.theorem",
            "theorems/zeta.theorem",
        ]
    );
}

#[test]
fn returned_paths_use_forward_slashes() {
    let fixture = DiscoveryFixture::new().expect("temp fixture should be created");
    fixture
        .write("theorems/windows/style.theorem")
        .expect("nested theorem should be written");

    let discovery = fixture.discover();

    assert_eq!(
        discovered_paths(&discovery),
        vec!["theorems/windows/style.theorem"]
    );
    assert_eq!(
        watched_directories(&discovery),
        vec!["theorems", "theorems/windows"]
    );
}

// --- IO error path tests ---

#[test]
fn nonexistent_manifest_dir_returns_io_error() {
    let result = discover_theorem_inputs(Utf8Path::new("/nonexistent/manifest/dir"));
    let error = result.expect_err("nonexistent manifest dir should fail");

    match &error {
        BuildDiscoveryError::Io {
            operation, path, ..
        } => {
            assert_eq!(*operation, "open crate root");
            assert_eq!(path.as_str(), "/nonexistent/manifest/dir");
        }
        other @ BuildDiscoveryError::TheoremRootNotDirectory { .. } => {
            panic!("expected Io error, got {other:?}");
        }
    }

    assert!(
        error.to_string().contains("open crate root"),
        "Display should include the operation"
    );
}

#[test]
fn io_error_read_theorem_directory_display() {
    let source = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let error = BuildDiscoveryError::Io {
        operation: "read theorem directory",
        path: Utf8PathBuf::from("theorems"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("read theorem directory"),
        "should include operation"
    );
    assert!(display.contains("theorems"), "should include path");
}

#[test]
fn io_error_open_theorem_directory_display() {
    let source = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let error = BuildDiscoveryError::Io {
        operation: "open theorem directory",
        path: Utf8PathBuf::from("theorems/nested"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("open theorem directory"),
        "should include operation"
    );
    assert!(display.contains("theorems/nested"), "should include path");
}

#[test]
fn io_error_read_theorem_directory_entry_display() {
    let source = io::Error::other("entry iteration failed");
    let error = BuildDiscoveryError::Io {
        operation: "read theorem directory entry",
        path: Utf8PathBuf::from("theorems"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("read theorem directory entry"),
        "should include operation"
    );
}

#[test]
fn io_error_read_theorem_entry_name_display() {
    let source = io::Error::other("name retrieval failed");
    let error = BuildDiscoveryError::Io {
        operation: "read theorem entry name",
        path: Utf8PathBuf::from("theorems"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("read theorem entry name"),
        "should include operation"
    );
}

#[test]
fn io_error_inspect_theorem_entry_display() {
    let source = io::Error::other("file type inspection failed");
    let error = BuildDiscoveryError::Io {
        operation: "inspect theorem entry",
        path: Utf8PathBuf::from("theorems/example.theorem"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("inspect theorem entry"),
        "should include operation"
    );
    assert!(
        display.contains("theorems/example.theorem"),
        "should include path"
    );
}

#[test]
fn io_error_inspect_theorem_root_display() {
    let source = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let error = BuildDiscoveryError::Io {
        operation: "inspect theorem root",
        path: Utf8PathBuf::from("theorems"),
        source,
    };

    let display = error.to_string();
    assert!(
        display.contains("inspect theorem root"),
        "should include operation"
    );
}

#[test]
fn io_error_source_chain_is_accessible() {
    let inner = io::Error::new(io::ErrorKind::NotFound, "not found");
    let error = BuildDiscoveryError::Io {
        operation: "inspect theorem root",
        path: Utf8PathBuf::from("theorems"),
        source: inner,
    };

    let source = std::error::Error::source(&error).expect("Io variant should expose source");
    assert!(
        source.to_string().contains("not found"),
        "source chain should include the original IO error"
    );
}

#[test]
fn not_directory_error_display() {
    let error = BuildDiscoveryError::TheoremRootNotDirectory {
        path: Utf8PathBuf::from("theorems"),
    };
    assert_eq!(
        error.to_string(),
        "theorem root 'theorems' exists but is not a directory"
    );
    assert!(
        std::error::Error::source(&error).is_none(),
        "TheoremRootNotDirectory has no source"
    );
}
