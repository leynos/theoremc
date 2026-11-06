//! Locates where a build script wrote `theorem_suite.rs` inside a fixture.
//!
//! Cargo's classic layout keeps a build script's output at
//! `build/<package>-<hash>/out`; newer nightlies add a directory level,
//! `build/<package>/<hash>/out`. Both places are candidates.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::fs_utf8::Dir;

/// Lists where a build script may have written `theorem_suite.rs`.
///
/// # Errors
///
/// Returns an error when the build directory cannot be listed.
pub fn generated_suite_candidates(dir: &Dir) -> Result<Vec<Utf8PathBuf>, String> {
    let build_dir = Utf8Path::new("target/debug/build");
    let mut candidates = Vec::new();
    for entry_result in dir.read_dir(build_dir).map_err(|error| error.to_string())? {
        let entry = entry_result.map_err(|error| error.to_string())?;
        let package_dir = build_dir.join(entry.file_name().map_err(|error| error.to_string())?);
        candidates.push(package_dir.join("out/theorem_suite.rs"));
        candidates.extend(nested_suite_candidates(dir, &package_dir)?);
    }
    Ok(candidates)
}

/// Lists the per-hash output directories below one package directory.
fn nested_suite_candidates(dir: &Dir, package_dir: &Utf8Path) -> Result<Vec<Utf8PathBuf>, String> {
    let Ok(nested) = dir.read_dir(package_dir) else {
        return Ok(Vec::new());
    };
    nested
        .map(|entry| {
            let name = entry
                .and_then(|found| found.file_name())
                .map_err(|error| error.to_string())?;
            Ok(package_dir.join(name).join("out/theorem_suite.rs"))
        })
        .collect()
}

/// Whether a read failed because the path (or a parent) does not exist.
pub fn is_missing_path(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
    )
}

#[cfg(test)]
mod tests {
    //! Locates the generated suite in each build-directory layout Cargo uses.

    use camino::Utf8PathBuf;
    use cap_std::{ambient_authority, fs_utf8::Dir};

    use super::{generated_suite_candidates, is_missing_path};

    /// Asserts that exactly one candidate for a suite written under
    /// `build/<output_dir>` is readable.
    fn assert_one_readable_candidate(output_dir: &str) {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");
        let output = format!("target/debug/build/{output_dir}");
        dir.create_dir_all(&output)
            .expect("create the output directory");
        dir.write(format!("{output}/theorem_suite.rs"), "// suite")
            .expect("write the generated suite");

        let candidates = generated_suite_candidates(&dir).expect("list the candidates");

        let readable: Vec<_> = candidates
            .iter()
            .filter(|path| dir.read_to_string(path).is_ok())
            .collect();
        assert_eq!(readable.len(), 1, "candidates: {candidates:?}");
    }

    /// The classic layout: `build/<package>-<hash>/out`.
    #[test]
    fn a_readable_candidate_exists_for_the_direct_layout() {
        assert_one_readable_candidate("pkg-0123abcd/out");
    }

    /// The newer layout: `build/<package>/<hash>/out`.
    #[test]
    fn a_readable_candidate_exists_for_the_nested_layout() {
        assert_one_readable_candidate("pkg/0123abcd/out");
    }

    #[test]
    fn a_missing_path_is_reported_as_missing() {
        let scratch = tempfile::tempdir().expect("create a scratch directory");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_path_buf())
            .expect("the scratch directory path should be UTF-8");
        let dir =
            Dir::open_ambient_dir(&root, ambient_authority()).expect("open the scratch directory");

        let error = dir
            .read_to_string("target/debug/build/none/out/theorem_suite.rs")
            .expect_err("the suite does not exist");

        assert!(is_missing_path(&error));
    }
}
