//! Locates where a build script wrote `theorem_suite.rs` inside a fixture.
//!
//! Cargo's classic layout keeps a build script's output at
//! `build/<package>-<hash>/out`; newer nightlies add a directory level,
//! `build/<package>/<hash>/out`. Both places are searched.

use cap_std::fs_utf8::Dir;

/// Lists the build directories that belong to one package, relative to the
/// `build` directory, in both layouts.
///
/// # Errors
///
/// Returns an error when the `build` directory cannot be listed, and when an
/// entry named after the package is a file rather than a directory.
pub fn package_directories(build_dir: &Dir, package_name: &str) -> Result<Vec<String>, String> {
    let prefix = format!("{package_name}-");
    let mut names = Vec::new();
    for listing in build_dir.entries().map_err(|error| error.to_string())? {
        let name = listing
            .and_then(|found| found.file_name())
            .map_err(|error| error.to_string())?;
        if name.starts_with(&prefix) {
            names.push(name);
        } else if name == package_name {
            names.extend(hash_directories(build_dir, &name)?);
        }
    }
    names.sort();
    Ok(names)
}

/// Lists the per-hash directories below a package directory (newer layout).
fn hash_directories(build_dir: &Dir, package_dir: &str) -> Result<Vec<String>, String> {
    let children = match build_dir.read_dir(package_dir) {
        Ok(children) => children,
        // A missing package directory is simply absent. A package entry that
        // is a file is a malformed build layout, not an unbuilt fixture, so it
        // must not read as "no children".
        Err(error) if is_not_found(&error) => return Ok(Vec::new()),
        Err(error) => return Err(format!("read {package_dir}: {error}")),
    };
    children
        .map(|entry| {
            let name = entry
                .and_then(|found| found.file_name())
                .map_err(|error| error.to_string())?;
            Ok(format!("{package_dir}/{name}"))
        })
        .collect()
}

/// Reads a build directory's generated suite, or `None` when it has none.
///
/// # Errors
///
/// Returns an error when the suite exists but cannot be read, and when a path
/// component that should be a directory is a file.
pub fn read_suite(build_dir: &Dir, name: &str) -> Result<Option<String>, String> {
    let suite_path = format!("{name}/out/theorem_suite.rs");
    match build_dir.read_to_string(&suite_path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(format!("read {suite_path}: {error}")),
    }
}

/// Whether a read failed because the path does not exist.
///
/// `NotADirectory` is deliberately excluded. A build entry that is a file
/// where a directory is expected is a malformed layout, and reporting it as
/// absence would let a broken fixture pass as an unbuilt one.
fn is_not_found(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
}

#[cfg(test)]
mod tests {
    //! Finds the generated suite in each build-directory layout Cargo uses.

    use camino::Utf8PathBuf;
    use cap_std::{ambient_authority, fs_utf8::Dir};

    use super::{package_directories, read_suite};

    /// Writes a suite under `<output_dir>/out` and asserts that exactly one
    /// directory of the package yields it.
    fn assert_suite_found(output_dir: &str) {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let build_dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");
        let out = format!("{output_dir}/out");
        build_dir
            .create_dir_all(&out)
            .expect("create the output directory");
        build_dir
            .write(format!("{out}/theorem_suite.rs"), "// suite")
            .expect("write the generated suite");

        let names = package_directories(&build_dir, "pkg").expect("list the package directories");

        let found: Vec<String> = names
            .iter()
            .filter_map(|name| read_suite(&build_dir, name).expect("read the suite"))
            .collect();
        assert_eq!(found, ["// suite"], "directories: {names:?}");
    }

    /// The classic layout: `build/<package>-<hash>/out`.
    #[test]
    fn the_direct_layout_yields_its_suite() {
        assert_suite_found("pkg-0123abcd");
    }

    /// The newer layout: `build/<package>/<hash>/out`.
    #[test]
    fn the_nested_layout_yields_its_suite() {
        assert_suite_found("pkg/0123abcd");
    }

    /// A package entry that is a file is a malformed layout, not an absent
    /// package, so listing its children must fail rather than report none.
    #[test]
    fn a_package_directory_that_is_a_file_is_an_error() {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let build_dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");
        build_dir
            .write("pkg", "not a directory")
            .expect("write a file");

        let error = package_directories(&build_dir, "pkg")
            .expect_err("a package entry that is a file must not read as absent");

        assert!(error.contains("read pkg"), "{error}");
    }

    /// A build directory that is a file cannot hold a suite, and that is an
    /// inspection failure rather than an unbuilt fixture.
    #[test]
    fn a_build_directory_that_is_a_file_is_an_error() {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let build_dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");
        build_dir
            .write("pkg-0123abcd", "not a directory")
            .expect("write a file");

        let error = read_suite(&build_dir, "pkg-0123abcd")
            .expect_err("a build directory that is a file must not read as absent");

        assert!(error.contains("pkg-0123abcd"), "{error}");
    }

    /// A package with no output reads as having no suite.
    #[test]
    fn a_missing_suite_reads_as_none() {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let build_dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");

        assert_eq!(read_suite(&build_dir, "none-0123abcd").expect("read"), None);
    }
}
