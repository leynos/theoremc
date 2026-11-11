//! Fixture inspection that separates absence from failed inspection.
//!
//! The fixture isolation contract asserts that one fixture cannot see another
//! fixture's sources or generated suite. An `is_err()` test cannot support that
//! assertion, because an absent path and an unreadable one both fail. These
//! helpers answer absence with `Ok(false)` or `Ok(None)` and report every other
//! failure, so an environmental fault cannot masquerade as the isolation the
//! contract is checking.

use std::io::ErrorKind;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::suite_layout::{package_directories, read_suite};

/// Reports whether `path` exists inside `dir`.
///
/// # Errors
///
/// Returns an error for any inspection failure other than a missing path, so
/// a caller cannot mistake an unreadable path for an absent one.
///
/// # Examples
///
/// ```rust,ignore
/// let present = path_is_present(&dir, Utf8Path::new("Cargo.toml"))?;
/// let absent = path_is_present(&dir, Utf8Path::new("missing.rs"))?;
/// assert!(present && !absent);
/// ```
pub fn path_is_present(dir: &Dir, path: &Utf8Path) -> Result<bool, String> {
    match dir.metadata(path.as_str()) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("cannot inspect {path}: {error}")),
    }
}

/// Reads the suite generated under `build_root` for `package_name`.
///
/// Returns `Ok(None)` when the fixture has not been built, or when its build
/// wrote no `theorem_suite.rs`; every other inspection failure is an error.
///
/// # Errors
///
/// Returns an error when the build output or one of its package directories
/// cannot be inspected.
pub fn find_suite_in_build_root(
    build_root: &Utf8Path,
    package_name: &str,
) -> Result<Option<String>, String> {
    let build_dir = match Dir::open_ambient_dir(build_root, ambient_authority()) {
        Ok(dir) => dir,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot inspect {build_root}: {error}")),
    };
    for name in package_directories(&build_dir, package_name)? {
        if let Some(contents) = read_suite(&build_dir, &name)? {
            return Ok(Some(contents));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    //! Fixture inspection over scratch directories: presence, absence, and the
    //! inspection failures that must not read as absence.

    use camino::Utf8PathBuf;
    use cap_std::{ambient_authority, fs_utf8::Dir};

    use super::{find_suite_in_build_root, path_is_present};

    /// Opens a scratch directory as a fixture root.
    fn scratch() -> (tempfile::TempDir, Utf8PathBuf, Dir) {
        let temp = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(temp.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");
        (temp, root, dir)
    }

    #[test]
    fn a_present_path_is_reported_present() {
        let (_temp, _root, dir) = scratch();
        dir.write("Cargo.toml", "[package]\n")
            .expect("write a fixture file");

        assert_eq!(
            path_is_present(&dir, camino::Utf8Path::new("Cargo.toml")),
            Ok(true)
        );
    }

    #[test]
    fn a_missing_path_is_reported_absent() {
        let (_temp, _root, dir) = scratch();

        assert_eq!(
            path_is_present(&dir, camino::Utf8Path::new("theorems/absent.theorem")),
            Ok(false)
        );
    }

    #[test]
    fn an_unbuilt_fixture_yields_no_suite() {
        let (_temp, root, _dir) = scratch();

        assert_eq!(
            find_suite_in_build_root(&root.join("debug/build"), "fx_ab"),
            Ok(None)
        );
    }

    #[test]
    fn a_build_root_that_is_not_a_directory_is_an_error() {
        let (_temp, root, dir) = scratch();
        dir.create_dir_all("debug")
            .expect("create the debug directory");
        dir.write("debug/build", "not a directory")
            .expect("write a file where the build root would be");

        let error = find_suite_in_build_root(&root.join("debug/build"), "fx_ab")
            .expect_err("a build root that is a file must not read as absent");

        assert!(
            error.contains("cannot inspect") && error.contains("debug/build"),
            "{error}"
        );
    }

    #[test]
    fn a_path_below_a_file_is_an_inspection_error() {
        let (_temp, _root, dir) = scratch();
        dir.write("blocker", "not a directory")
            .expect("write a file where a directory would be");

        let error = path_is_present(&dir, camino::Utf8Path::new("blocker/child"))
            .expect_err("a path below a file must not read as absent");

        assert!(error.contains("cannot inspect"), "{error}");
    }

    #[test]
    fn a_package_entry_that_is_a_file_is_an_inspection_error() {
        let (_temp, root, dir) = scratch();
        dir.create_dir_all("debug/build")
            .expect("create the build directory");
        dir.write("debug/build/fx_ab-111", "not a directory")
            .expect("write a file where a build directory would be");

        let error = find_suite_in_build_root(&root.join("debug/build"), "fx_ab")
            .expect_err("a package entry that is a file must not read as absent");

        assert!(error.contains("fx_ab-111"), "{error}");
    }

    #[test]
    fn a_built_fixture_yields_its_suite() {
        let (_temp, root, dir) = scratch();
        dir.create_dir_all("debug/build/fx_ab-111/out")
            .expect("create the build directory");
        dir.write("debug/build/fx_ab-111/out/theorem_suite.rs", "// suite")
            .expect("write the generated suite");

        assert_eq!(
            find_suite_in_build_root(&root.join("debug/build"), "fx_ab"),
            Ok(Some("// suite".to_owned()))
        );
    }
}
