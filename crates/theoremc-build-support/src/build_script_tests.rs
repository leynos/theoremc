//! Direct tests for Cargo build-script preparation.
//!
//! Every case supplies its inputs explicitly: a temporary manifest and output
//! directory, or a stub variable reader. Nothing here touches the process
//! environment, so the suite holds no environment lock and needs no
//! serialization.

use std::{env, error::Error, fmt::Debug, io};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};
use rstest::{fixture, rstest};

use super::{BuildScriptContext, BuildScriptError, BuildScriptOutput, prepare_build_script};

/// A temporary crate root with a sibling output directory.
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

    /// Builds a context from this fixture's own directories.
    fn context(&self) -> BuildScriptContext {
        BuildScriptContext::new(self.manifest_dir.clone(), self.out_dir.clone())
    }

    /// Builds a context through a stub reader seeded with this fixture.
    fn context_from_lookup(&self) -> Result<BuildScriptContext, BuildScriptError> {
        BuildScriptContext::from_lookup(|variable| self.read_cargo_variable(variable))
    }

    fn read_cargo_variable(&self, variable: &'static str) -> Result<String, env::VarError> {
        match variable {
            "CARGO_MANIFEST_DIR" => Ok(self.manifest_dir.to_string()),
            "OUT_DIR" => Ok(self.out_dir.to_string()),
            _ => Err(env::VarError::NotPresent),
        }
    }

    fn prepare(&self) -> Result<BuildScriptOutput, BuildScriptError> {
        prepare_build_script(&self.context())
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

/// Extract a context error, retaining unexpected success.
fn require_context_error(
    result: Result<BuildScriptContext, BuildScriptError>,
    context: &str,
) -> Result<BuildScriptError, Box<dyn Error>> {
    match result {
        Err(error) => Ok(error),
        Ok(built) => Err(io::Error::other(format!("{context}: got {built:?}")).into()),
    }
}

/// Extract the variable name from an environment error.
fn environment_variable(
    error: &BuildScriptError,
    context: &str,
) -> Result<&'static str, Box<dyn Error>> {
    let BuildScriptError::Environment { variable, .. } = error else {
        return Err(io::Error::other(format!(
            "{context}: expected an environment error, got {error:?}"
        ))
        .into());
    };
    Ok(variable)
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
fn lookup_reads_both_cargo_variables_into_the_context(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;

    let context = fixture.context_from_lookup()?;

    require_equal(
        &context.manifest_dir(),
        &fixture.manifest_dir.as_path(),
        "looked-up manifest directory",
    )?;
    require_equal(
        &context.out_dir(),
        &fixture.out_dir.as_path(),
        "looked-up output directory",
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
    let error = require_context_error(
        BuildScriptContext::from_lookup(|variable| {
            if variable == missing_variable {
                Err(env::VarError::NotPresent)
            } else {
                fixture.read_cargo_variable(variable)
            }
        }),
        "a required Cargo variable that is absent should fail",
    )?;

    require_equal(
        environment_variable(&error, "missing Cargo variable")?,
        missing_variable,
        "missing Cargo variable identity",
    )?;
    Ok(())
}

#[rstest]
#[case::manifest_dir("CARGO_MANIFEST_DIR")]
#[case::output_dir("OUT_DIR")]
fn non_unicode_cargo_environment_is_reported(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
    #[case] bad_variable: &'static str,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let error = require_context_error(
        BuildScriptContext::from_lookup(|variable| {
            if variable == bad_variable {
                Err(env::VarError::NotUnicode("not utf-8".into()))
            } else {
                fixture.read_cargo_variable(variable)
            }
        }),
        "a non-Unicode Cargo variable should fail",
    )?;

    let BuildScriptError::Environment { variable, source } = error else {
        return Err(
            io::Error::other(format!("expected an environment error, got {error:?}")).into(),
        );
    };
    require_equal(
        &variable,
        &bad_variable,
        "non-Unicode Cargo variable identity",
    )?;
    require_equal(
        &matches!(source, env::VarError::NotUnicode(_)),
        &true,
        "non-Unicode lookup failure variant",
    )?;
    Ok(())
}

/// Records the variables the lookup closure is asked for, in order.
#[rstest]
fn lookup_stops_at_the_first_missing_variable(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let mut asked = Vec::new();

    let error = require_context_error(
        BuildScriptContext::from_lookup(|variable| {
            asked.push(variable);
            if variable == "CARGO_MANIFEST_DIR" {
                Err(env::VarError::NotPresent)
            } else {
                fixture.read_cargo_variable(variable)
            }
        }),
        "an absent manifest directory should stop the lookup",
    )?;

    require_equal(
        environment_variable(&error, "short-circuited lookup")?,
        "CARGO_MANIFEST_DIR",
        "short-circuited lookup variable identity",
    )?;
    require_equal(
        &asked,
        &vec!["CARGO_MANIFEST_DIR"],
        "variables read before the first failure",
    )?;
    Ok(())
}

#[rstest]
fn explicit_context_selects_unrelated_directories(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    // Prepare the first fixture first, so its suite file exists and the final
    // assertion can show that preparing the second fixture left it alone.
    fixture.prepare()?;
    let other = BuildScriptFixture::new()?;
    other.dir.create_dir_all("theorems")?;
    other.dir.write("theorems/other.theorem", "fixture")?;

    let output = prepare_build_script(&other.context())?;

    require_equal(
        &output
            .rerun_paths()
            .map(|path| path.as_str())
            .collect::<Vec<_>>(),
        &vec!["theorems", "theorems/other.theorem"],
        "second fixture rerun paths",
    )?;
    require_equal(
        fixture.dir.read_to_string("out/theorem_suite.rs")?.as_str(),
        "\n",
        "first fixture suite must stay untouched",
    )?;
    Ok(())
}

/// Filesystem failures whose diagnostics must identify the preparation stage.
#[derive(Debug, Clone, Copy)]
enum PreparationFailure {
    /// Opening the nonexistent manifest directory must fail during discovery.
    MissingManifestDirectory,
    /// A directory at the generated suite path must prevent writing the suite.
    SuitePathIsDirectory,
}

#[rstest]
#[case::missing_manifest_directory(PreparationFailure::MissingManifestDirectory)]
#[case::suite_path_is_directory(PreparationFailure::SuitePathIsDirectory)]
fn preparation_preserves_filesystem_error_variants(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
    #[case] failure: PreparationFailure,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let (context, detail) = match failure {
        PreparationFailure::MissingManifestDirectory => (
            BuildScriptContext::new(
                fixture.manifest_dir.join("missing"),
                fixture.out_dir.clone(),
            ),
            "discovery",
        ),
        PreparationFailure::SuitePathIsDirectory => {
            fixture.dir.create_dir_all("out/theorem_suite.rs")?;
            (fixture.context(), "suite")
        }
    };
    let error = require_build_error(
        prepare_build_script(&context),
        "preparation should preserve the filesystem failure",
    )?;

    match (failure, error) {
        (PreparationFailure::MissingManifestDirectory, BuildScriptError::Discovery(_))
        | (PreparationFailure::SuitePathIsDirectory, BuildScriptError::Suite(_)) => Ok(()),
        (_, unexpected) => Err(io::Error::other(format!(
            "expected a {detail} error for {failure:?}, got {unexpected:?}"
        ))
        .into()),
    }
}

#[rstest]
fn missing_output_directory_is_reported_with_its_path(
    build_script_fixture: Result<BuildScriptFixture, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let fixture = build_script_fixture?;
    let missing_out_dir = fixture.manifest_dir.join("missing-out");
    let context = BuildScriptContext::new(fixture.manifest_dir.clone(), missing_out_dir.clone());

    let error = require_build_error(
        prepare_build_script(&context),
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
