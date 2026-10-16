//! Direct tests for Cargo build-script preparation.

use std::{env, error::Error, io};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};
use rstest::{fixture, rstest};

use super::{
    BuildScriptError, BuildScriptOutput, prepare_build_script_at, prepare_build_script_with_env,
};

struct BuildScriptFixture {
    _temp_dir: tempfile::TempDir,
    manifest_dir: Utf8PathBuf,
    out_dir: Utf8PathBuf,
    dir: Utf8Dir,
}

impl BuildScriptFixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let temp_dir = tempfile::tempdir()?;
        let manifest_dir = Utf8Path::from_path(temp_dir.path())
            .ok_or_else(|| io::Error::other("temp dir path is not valid UTF-8"))?
            .to_path_buf();
        let out_dir = manifest_dir.join("out");
        let dir = Utf8Dir::open_ambient_dir(&manifest_dir, ambient_authority())?;
        dir.create_dir_all("out")?;

        Ok(Self {
            _temp_dir: temp_dir,
            manifest_dir,
            out_dir,
            dir,
        })
    }

    fn read_cargo_variable(&self, variable: &'static str) -> Result<String, env::VarError> {
        match variable {
            "CARGO_MANIFEST_DIR" => Ok(self.manifest_dir.to_string()),
            "OUT_DIR" => Ok(self.out_dir.to_string()),
            _ => Err(env::VarError::NotPresent),
        }
    }

    fn prepare(&self) -> Result<BuildScriptOutput, BuildScriptError> {
        prepare_build_script_with_env(|variable| self.read_cargo_variable(variable))
    }
}

#[fixture]
fn build_script_fixture() -> Result<BuildScriptFixture, Box<dyn Error>> {
    BuildScriptFixture::new()
}

#[rstest]
fn preparation_generates_an_empty_suite_and_root_watch_path(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;

    let output = fixture.prepare()?;

    assert!(!output.has_theorems());
    assert_eq!(
        output
            .rerun_paths()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec!["theorems"]
    );
    assert_eq!(fixture.dir.read_to_string("out/theorem_suite.rs")?, "\n");
    Ok(())
}

#[rstest]
fn preparation_generates_theorem_suite_and_rerun_paths(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    fixture.dir.create_dir_all("theorems/nested")?;
    fixture
        .dir
        .write("theorems/nested/example.theorem", "fixture")?;

    let output = fixture.prepare()?;

    assert!(output.has_theorems());
    assert_eq!(
        output
            .rerun_paths()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "theorems",
            "theorems/nested",
            "theorems/nested/example.theorem"
        ]
    );
    assert_eq!(
        fixture.dir.read_to_string("out/theorem_suite.rs")?,
        "theorem_file!(\"theorems/nested/example.theorem\");\n"
    );
    Ok(())
}

#[rstest]
#[case::manifest_dir("CARGO_MANIFEST_DIR")]
#[case::output_dir("OUT_DIR")]
fn missing_cargo_environment_is_reported(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
    #[case] missing_variable: &'static str,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let error = prepare_build_script_with_env(|variable| {
        if variable == missing_variable {
            Err(env::VarError::NotPresent)
        } else {
            fixture.read_cargo_variable(variable)
        }
    })
    .expect_err("a required Cargo variable that is absent should fail");

    let BuildScriptError::Environment { variable, .. } = error else {
        panic!("expected an environment error, got {error:?}");
    };
    assert_eq!(variable, missing_variable);
    Ok(())
}

#[rstest]
fn missing_manifest_directory_is_reported_as_a_discovery_error(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let missing_manifest_dir = fixture.manifest_dir.join("missing");

    let error = prepare_build_script_at(&missing_manifest_dir, &fixture.out_dir)
        .expect_err("a missing manifest directory should fail discovery");

    assert!(matches!(error, BuildScriptError::Discovery(_)));
    Ok(())
}

#[rstest]
fn missing_output_directory_is_reported_with_its_path(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let missing_out_dir = fixture.manifest_dir.join("missing-out");

    let error = prepare_build_script_at(&fixture.manifest_dir, &missing_out_dir)
        .expect_err("a missing output directory should fail to open");

    let BuildScriptError::OpenOutputDirectory { path, .. } = error else {
        panic!("expected an output-directory error, got {error:?}");
    };
    assert_eq!(path, missing_out_dir);
    Ok(())
}

#[rstest]
fn an_unwritable_suite_target_is_reported_as_a_suite_error(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    fixture.dir.create_dir_all("out/theorem_suite.rs")?;

    let error = prepare_build_script_at(&fixture.manifest_dir, &fixture.out_dir)
        .expect_err("a directory at the suite-file path should fail to write");

    assert!(matches!(error, BuildScriptError::Suite(_)));
    Ok(())
}
