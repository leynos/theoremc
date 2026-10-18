//! Direct tests for Cargo build-script preparation.

use std::{env, error::Error, fmt::Debug, io};

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

/// Report mismatched values without introducing a panic into an I/O test.
fn require_equal<T: Debug + PartialEq + ?Sized>(
    actual: &T,
    expected: &T,
    context: &str,
) -> Result<(), Box<dyn Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::other(format!("{context}: expected {expected:?}, got {actual:?}")).into())
    }
}

/// Extract a preparation error while retaining unexpected successful output.
fn require_build_error(
    result: Result<BuildScriptOutput, BuildScriptError>,
    context: &str,
) -> Result<BuildScriptError, Box<dyn Error>> {
    match result {
        Err(error) => Ok(error),
        Ok(output) => Err(io::Error::other(format!("{context}: got {output:?}")).into()),
    }
}

#[rstest]
fn preparation_generates_an_empty_suite_and_root_watch_path(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;

    let output = fixture.prepare()?;

    require_equal(
        &output.has_theorems(),
        &false,
        "empty suite theorem presence",
    )?;
    require_equal(
        &output
            .rerun_paths()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        &vec!["theorems"],
        "empty suite rerun paths",
    )?;
    require_equal(
        fixture.dir.read_to_string("out/theorem_suite.rs")?.as_str(),
        "\n",
        "empty suite bytes",
    )?;
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

    require_equal(
        &output.has_theorems(),
        &true,
        "populated suite theorem presence",
    )?;
    require_equal(
        &output
            .rerun_paths()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        &vec![
            "theorems",
            "theorems/nested",
            "theorems/nested/example.theorem",
        ],
        "populated suite rerun paths in discovery order",
    )?;
    require_equal(
        fixture.dir.read_to_string("out/theorem_suite.rs")?.as_str(),
        "theorem_file!(\"theorems/nested/example.theorem\");\n",
        "populated suite bytes",
    )?;
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
    let error = require_build_error(
        prepare_build_script_with_env(|variable| {
            if variable == missing_variable {
                Err(env::VarError::NotPresent)
            } else {
                fixture.read_cargo_variable(variable)
            }
        }),
        "a required Cargo variable that is absent should fail",
    )?;

    let BuildScriptError::Environment { variable, .. } = error else {
        return Err(
            io::Error::other(format!("expected an environment error, got {error:?}")).into(),
        );
    };
    require_equal(
        variable,
        missing_variable,
        "missing Cargo variable identity",
    )?;
    Ok(())
}

#[rstest]
fn missing_manifest_directory_is_reported_as_a_discovery_error(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let missing_manifest_dir = fixture.manifest_dir.join("missing");

    let error = require_build_error(
        prepare_build_script_at(&missing_manifest_dir, &fixture.out_dir),
        "a missing manifest directory should fail discovery",
    )?;

    match error {
        BuildScriptError::Discovery(_) => Ok(()),
        unexpected => {
            Err(io::Error::other(format!("expected a discovery error, got {unexpected:?}")).into())
        }
    }
}

#[rstest]
fn missing_output_directory_is_reported_with_its_path(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let missing_out_dir = fixture.manifest_dir.join("missing-out");

    let error = require_build_error(
        prepare_build_script_at(&fixture.manifest_dir, &missing_out_dir),
        "a missing output directory should fail to open",
    )?;

    let BuildScriptError::OpenOutputDirectory { path, .. } = error else {
        return Err(
            io::Error::other(format!("expected an output-directory error, got {error:?}")).into(),
        );
    };
    require_equal(&path, &missing_out_dir, "missing output directory path")?;
    Ok(())
}

#[rstest]
fn an_unwritable_suite_target_is_reported_as_a_suite_error(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    fixture.dir.create_dir_all("out/theorem_suite.rs")?;

    let error = require_build_error(
        prepare_build_script_at(&fixture.manifest_dir, &fixture.out_dir),
        "a directory at the suite-file path should fail to write",
    )?;

    match error {
        BuildScriptError::Suite(_) => Ok(()),
        unexpected => {
            Err(io::Error::other(format!("expected a suite error, got {unexpected:?}")).into())
        }
    }
}
