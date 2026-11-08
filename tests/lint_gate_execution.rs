//! Executable contract test for the aggregate `lint` Makefile target.
//!
//! `tests/lint_makefile_contract.rs` reads the recipes as text, but a text copy
//! cannot show that `make lint` really runs each stage, in the documented
//! order, or that a failing stage stops the aggregate. This test puts stub
//! `cargo` and `whitaker` executables at the recipes' variables and records
//! what each invocation received.

#![cfg(unix)]

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};
use rstest::rstest;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

/// A stub `cargo`/`whitaker` that records its stage, environment, and arguments.
///
/// The stage is the executable name, except for `cargo`, where it is the first
/// argument (`doc` or `clippy`), so one script can stand in for every stage.
/// `STUB_FAIL_ON` names the stage that must exit non-zero.
const STUB: &str = concat!(
    "#!/bin/sh\n",
    "stage=$(basename \"$0\")\n",
    "if [ \"$stage\" = cargo ]; then stage=${1-}; fi\n",
    "{\n",
    "  printf 'STAGE=%s\\n' \"$stage\"\n",
    "  printf 'RUSTFLAGS=%s\\n' \"${RUSTFLAGS-<unset>}\"\n",
    "  printf 'RUSTDOCFLAGS=%s\\n' \"${RUSTDOCFLAGS-<unset>}\"\n",
    "  printf 'DYLINT_RUSTFLAGS=%s\\n' \"${DYLINT_RUSTFLAGS-<unset>}\"\n",
    "  for argument in \"$@\"; do printf 'ARG=%s\\n' \"$argument\"; done\n",
    "} >> \"$STUB_LOG\"\n",
    "if [ \"$stage\" = \"${STUB_FAIL_ON-}\" ]; then exit 5; fi\n",
    "exit 0\n",
);

/// One stage of the aggregate, as the stub records it.
///
/// Naming the stages as variants keeps a stage's recorded name and its expected
/// arguments together, so a test cannot check one stage against another's
/// arguments.
#[derive(Clone, Copy, Debug)]
enum Stage {
    /// `cargo doc`.
    Rustdoc,
    /// `cargo clippy`.
    Clippy,
    /// `whitaker`.
    Whitaker,
}

impl Stage {
    /// The stages in the order the aggregate must run them.
    const ORDER: [Self; 3] = [Self::Rustdoc, Self::Clippy, Self::Whitaker];

    /// The name the stub records for this stage.
    fn name(self) -> &'static str {
        match self {
            Self::Rustdoc => "doc",
            Self::Clippy => "clippy",
            Self::Whitaker => "whitaker",
        }
    }

    /// The arguments this stage must pass.
    ///
    /// `--workspace` is Whitaker's own flag and is forwarded to the `cargo
    /// check` it runs, so it must appear once, before `--`.
    fn arguments(self) -> &'static [&'static str] {
        match self {
            Self::Rustdoc => &["doc", "--workspace", "--all-features", "--no-deps"],
            Self::Clippy => &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
            Self::Whitaker => &[
                "--all",
                "--workspace",
                "--",
                "--all-targets",
                "--all-features",
            ],
        }
    }
}

/// One recorded stub invocation.
#[derive(Default)]
struct Invocation {
    stage: String,
    rustflags: String,
    rustdocflags: String,
    dylint_rustflags: String,
    arguments: Vec<String>,
}

impl Invocation {
    /// Fails unless this invocation passed exactly the arguments `expected`
    /// requires.
    ///
    /// The stage name comes from the invocation itself, so a caller cannot
    /// check one stage's arguments against another's name.
    ///
    /// # Errors
    ///
    /// Returns the mismatch, naming the stage and both argument lists.
    fn expect_arguments(&self, expected: Stage) -> Result<(), String> {
        let received: Vec<&str> = self.arguments.iter().map(String::as_str).collect();
        if received != expected.arguments() {
            return Err(format!(
                "the {} stage passed {received:?}, expected {:?}",
                self.stage,
                expected.arguments()
            ));
        }
        Ok(())
    }
}

/// An environment value a stage must pass, and the policy it enforces.
struct EnvironmentValue<'a> {
    /// The variable name.
    name: &'a str,
    /// The value the stub must have recorded.
    expected: &'a str,
    /// Why the stage must pass that value.
    reason: &'a str,
}

impl EnvironmentValue<'_> {
    /// Fails unless `actual` matches this expectation.
    ///
    /// # Errors
    ///
    /// Returns the mismatch, naming the variable and the value the stub saw.
    fn expect(&self, actual: &str) -> Result<(), String> {
        if actual != self.expected {
            return Err(format!(
                "{} must be `{}`, because {}; the stub saw `{actual}`",
                self.name, self.expected, self.reason
            ));
        }
        Ok(())
    }
}

/// Writes the stub under both executable names and makes them executable.
///
/// # Errors
///
/// Returns the reason when a script cannot be written or made executable.
fn write_stubs(directory: &Dir) -> Result<(), String> {
    for name in ["cargo", "whitaker"] {
        directory
            .write(name, STUB)
            .map_err(|error| format!("writing the stub {name}: {error}"))?;
        directory
            .set_permissions(name, Permissions::from_mode(0o755))
            .map_err(|error| format!("making the stub {name} executable: {error}"))?;
    }
    Ok(())
}

/// Variables a parent `make` or shell may export that would override the
/// Makefile defaults under test.
const INHERITED_VARIABLES: &[&str] = &[
    "MAKEFLAGS",
    "MAKELEVEL",
    "MFLAGS",
    "RUST_FLAGS",
    "RUSTDOC_FLAGS",
    "LINT_PACKAGES",
    "LINT_TARGET_FLAGS",
    "CARGO_FLAGS",
    "CLIPPY_FLAGS",
    "RUSTDOC_TARGET_FLAGS",
];

/// Runs the aggregate target against the stubs in `root`.
///
/// `fail_on` names the stage that must exit non-zero; `None` lets every stage
/// succeed. `RUSTFLAGS` is deliberately set, so the test can prove the Whitaker
/// stage clears it.
///
/// # Errors
///
/// Returns the reason when the stubs cannot be written or `make` cannot start.
fn run_lint_target(root: &Path, fail_on: Option<Stage>) -> Result<Output, String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    write_stubs(&directory)?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("make");
    command
        .arg("--no-print-directory")
        .arg("-f")
        .arg(manifest_dir.join("Makefile"))
        .arg("lint")
        .arg(format!("CARGO={}", root.join("cargo").display()))
        .arg(format!("WHITAKER={}", root.join("whitaker").display()))
        .current_dir(manifest_dir)
        .env("STUB_LOG", root.join("stub.log"))
        .env("STUB_FAIL_ON", fail_on.map(Stage::name).unwrap_or_default());
    for variable in INHERITED_VARIABLES {
        command.env_remove(variable);
    }
    command
        .env("RUSTFLAGS", "-Csentinel")
        .output()
        .map_err(|error| format!("running make lint: {error}"))
}

/// Reads the recorded invocations from the stub log.
///
/// Each invocation is one contiguous block that opens with its `STAGE=` line.
///
/// # Errors
///
/// Returns the reason when the log cannot be read.
fn read_invocations(root: &Path) -> Result<Vec<Invocation>, String> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    let recorded = directory
        .read_to_string("stub.log")
        .map_err(|error| format!("reading the stub log: {error}"))?;
    Ok(recorded.split("STAGE=").skip(1).map(parse_block).collect())
}

/// Parses one `STAGE=`-delimited block of the stub log.
fn parse_block(block: &str) -> Invocation {
    let mut lines = block.lines();
    let mut invocation = Invocation {
        stage: lines.next().unwrap_or_default().to_owned(),
        ..Invocation::default()
    };
    for line in lines {
        if let Some(value) = line.strip_prefix("RUSTFLAGS=") {
            invocation.rustflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("RUSTDOCFLAGS=") {
            invocation.rustdocflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("DYLINT_RUSTFLAGS=") {
            invocation.dylint_rustflags = value.to_owned();
        } else if let Some(value) = line.strip_prefix("ARG=") {
            invocation.arguments.push(value.to_owned());
        }
    }
    invocation
}

/// Returns the recorded invocation for `wanted`.
///
/// # Errors
///
/// Returns the reason when the stage never ran.
fn stage(invocations: &[Invocation], wanted: Stage) -> Result<&Invocation, String> {
    invocations
        .iter()
        .find(|invocation| invocation.stage == wanted.name())
        .ok_or_else(|| format!("make lint never ran the {} stage", wanted.name()))
}

/// Returns the names of the stages that ran, in order.
fn stage_names(invocations: &[Invocation]) -> Vec<&str> {
    invocations
        .iter()
        .map(|invocation| invocation.stage.as_str())
        .collect()
}

/// Returns the names of `stages`, in order.
fn names_of(stages: &[Stage]) -> Vec<&'static str> {
    stages.iter().copied().map(Stage::name).collect()
}

/// Verifies that `make lint` runs the three stages in order, each once, with the
/// arguments and warning denials it needs.
#[test]
fn aggregate_runs_rustdoc_then_clippy_then_whitaker() -> Result<(), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = run_lint_target(directory.path(), None)?;
    if !output.status.success() {
        return Err(format!(
            "make lint must succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let invocations = read_invocations(directory.path())?;
    let ran = stage_names(&invocations);
    let expected = names_of(&Stage::ORDER);
    if ran != expected {
        return Err(format!(
            "make lint must run rustdoc, then Clippy, then Whitaker; ran {ran:?}"
        ));
    }

    let doc = stage(&invocations, Stage::Rustdoc)?;
    doc.expect_arguments(Stage::Rustdoc)?;
    EnvironmentValue {
        name: "RUSTDOCFLAGS",
        expected: "-D warnings",
        reason: "the rustdoc stage must deny warnings",
    }
    .expect(&doc.rustdocflags)?;

    let clippy = stage(&invocations, Stage::Clippy)?;
    clippy.expect_arguments(Stage::Clippy)?;

    let whitaker = stage(&invocations, Stage::Whitaker)?;
    whitaker.expect_arguments(Stage::Whitaker)?;
    EnvironmentValue {
        name: "DYLINT_RUSTFLAGS",
        expected: "-D warnings",
        reason: "the Whitaker stage must deny lint warnings",
    }
    .expect(&whitaker.dylint_rustflags)?;
    EnvironmentValue {
        name: "RUSTFLAGS",
        expected: "",
        reason: "the Whitaker stage must clear it",
    }
    .expect(&whitaker.rustflags)?;
    Ok(())
}

/// Verifies that a failing stage stops the aggregate before the next one runs.
#[rstest]
#[case::rustdoc(Stage::Rustdoc, &[Stage::Rustdoc])]
#[case::clippy(Stage::Clippy, &[Stage::Rustdoc, Stage::Clippy])]
#[case::whitaker(Stage::Whitaker, &[Stage::Rustdoc, Stage::Clippy, Stage::Whitaker])]
fn a_failing_stage_stops_the_aggregate(
    #[case] failing: Stage,
    #[case] expected: &[Stage],
) -> Result<(), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let output = run_lint_target(directory.path(), Some(failing))?;
    if output.status.success() {
        return Err(format!(
            "make lint must fail when the {} stage fails",
            failing.name()
        ));
    }
    let invocations = read_invocations(directory.path())?;
    let ran = stage_names(&invocations);
    let expected_names = names_of(expected);
    if ran != expected_names {
        return Err(format!(
            "a failing {} stage must stop the aggregate after {expected_names:?}; ran {ran:?}",
            failing.name()
        ));
    }
    Ok(())
}
