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
/// Returns an error when the `build` directory cannot be listed.
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
    let Ok(children) = build_dir.read_dir(package_dir) else {
        return Ok(Vec::new());
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
/// Returns an error when the suite exists but cannot be read.
pub fn read_suite(build_dir: &Dir, name: &str) -> Result<Option<String>, String> {
    match build_dir.read_to_string(format!("{name}/out/theorem_suite.rs")) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if is_missing_path(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Whether a read failed because the path (or a parent) does not exist.
fn is_missing_path(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
    )
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
