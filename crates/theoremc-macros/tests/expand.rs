//! trybuild compile-error snapshot tests for the `theorem_file!` proc-macro.

use camino::Utf8PathBuf;
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};

fn stage_fixture(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let crate_root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = crate_root.join("tests/expand").join(name);
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|_| crate_root.join("../../target"));
    let relative_destination_dir = Utf8PathBuf::from("tests/trybuild/theoremc-macros/tests/expand");
    let destination = relative_destination_dir.join(name);
    let source_root = Utf8Dir::open_ambient_dir(
        source.parent().ok_or_else(|| {
            std::io::Error::other("staged trybuild fixture should have a source parent directory")
        })?,
        ambient_authority(),
    )?;
    let target_root = Utf8Dir::open_ambient_dir(&target_dir, ambient_authority())?;
    target_root.create_dir_all(&relative_destination_dir)?;
    let destination_root = target_root.open_dir(&relative_destination_dir)?;
    let source_name = source
        .file_name()
        .ok_or_else(|| std::io::Error::other("staged trybuild fixture should have a file name"))?;
    let destination_name = destination.file_name().ok_or_else(|| {
        std::io::Error::other("staged trybuild fixture should have a destination file name")
    })?;
    source_root.copy(source_name, &destination_root, destination_name)?;
    Ok(())
}

#[test]
fn compile_errors() -> Result<(), Box<dyn std::error::Error>> {
    stage_fixture("invalid_theorem.theorem")?;
    stage_fixture("valid_theorem.theorem")?;
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/valid_theorem.rs");
    t.compile_fail("tests/expand/invalid_theorem.rs");
    t.compile_fail("tests/expand/missing_theorem.rs");
    Ok(())
}
