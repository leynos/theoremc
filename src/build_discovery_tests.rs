//! Direct tests for build-time theorem discovery.

use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::{BuildDiscovery, discover_theorem_inputs};

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
    discovery
        .theorem_files
        .iter()
        .map(|path| path.as_str())
        .collect()
}

fn watched_directories(discovery: &BuildDiscovery) -> Vec<&str> {
    discovery
        .watched_directories
        .iter()
        .map(|path| path.as_str())
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
