//! Hermetic contract test for the spelling recipe's command line.
//!
//! A stub `uv` records the arguments and environment Make passes to it, so the
//! test proves the recipe forwards the pinned builder, revision, repository,
//! and scope exactly. It never touches the network and never installs a real
//! tool.

// The stub is a `/bin/sh` script made executable through Unix permission bits,
// and `cap_std` exposes its permission API only on Unix. Guarding the whole
// file keeps the suite buildable on the other supported targets.
#![cfg(unix)]

use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};
use rstest::{fixture, rstest};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The stub `uv` script: records argv and the uv directory overrides, then
/// exits with the caller's chosen status.
const STUB_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    "printf '%s\\n' \"$@\" > \"$STUB_UV_ARG_LOG\"\n",
    "printf 'UV_CACHE_DIR=%s\\n' \"$UV_CACHE_DIR\" > \"$STUB_UV_ENV_LOG\"\n",
    "printf 'UV_TOOL_DIR=%s\\n' \"$UV_TOOL_DIR\" >> \"$STUB_UV_ENV_LOG\"\n",
    "exit \"${STUB_UV_EXIT:-0}\"\n",
);

/// Returns the package root, which holds the Makefile under test.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Writes an executable stub named `uv` into `directory`.
fn write_stub(directory: &Dir) -> Result<(), String> {
    directory
        .write("uv", STUB_SCRIPT)
        .map_err(|error| format!("write stub: {error}"))?;
    directory
        .set_permissions("uv", Permissions::from_mode(0o755))
        .map_err(|error| format!("make stub executable: {error}"))
}

/// A temporary directory holding an executable stub `uv`.
///
/// The directory must outlive the test that consumes it, so the fixture owns
/// both the temporary directory and the capability handle opened on it.
struct StubUv {
    temp: tempfile::TempDir,
    dir: Dir,
}

impl StubUv {
    /// Returns the stub directory as an ambient path for `make`.
    fn path(&self) -> &Path {
        self.temp.path()
    }
}

/// Prepares an isolated stub `uv` and returns it, failing with the reason the
/// temporary directory could not be created or written.
#[fixture]
fn stub_uv() -> Result<StubUv, String> {
    let temp = tempfile::tempdir().map_err(|error| format!("temp dir: {error}"))?;
    let dir = Dir::open_ambient_dir(temp.path(), ambient_authority())
        .map_err(|error| format!("open temp dir: {error}"))?;
    write_stub(&dir)?;
    Ok(StubUv { temp, dir })
}

/// Runs the `spelling` target with the stub as `UV` and `repository` as the
/// path handed to the builder.
///
/// The stub writes its argument log to `arguments.log` and its environment log
/// to `environment.log` beside the stub; `stub_exit` selects the status it
/// reports.
fn run_spelling(root: &Path, stub_exit: u32, repository: &str) -> Result<Output, String> {
    let makefile = repository_root().join("Makefile");
    let mut uv_assignment = OsString::from("UV=");
    uv_assignment.push(root.join("uv"));
    Command::new("make")
        .arg("--no-print-directory")
        .arg("-f")
        .arg(makefile)
        .arg("spelling")
        .arg(uv_assignment)
        .arg(format!("SPELLING_REPOSITORY={repository}"))
        .current_dir(repository_root())
        .env("STUB_UV_ARG_LOG", root.join("arguments.log"))
        .env("STUB_UV_ENV_LOG", root.join("environment.log"))
        .env("STUB_UV_EXIT", stub_exit.to_string())
        .output()
        .map_err(|error| format!("run make: {error}"))
}

/// Reads a log written by the stub, one recorded value per line.
fn read_lines(directory: &Dir, name: &str) -> Result<Vec<String>, String> {
    let text = directory
        .read_to_string(name)
        .map_err(|error| format!("read {name}: {error}"))?;
    Ok(text.lines().map(str::to_owned).collect())
}

/// Describes a failed make invocation for the assertion message.
fn make_failure(output: &Output) -> String {
    format!(
        "make failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

/// Compares the recorded arguments with the pinned invocation, which hands
/// `repository` to the gate unchanged.
fn assert_pinned_arguments(recorded: &[String], repository: &str) -> Result<(), String> {
    let expected: Vec<String> = [
        "tool",
        "run",
        "--python",
        "3.14",
        "--from",
        "git+https://github.com/leynos/typos-config-builder.git@v0.1.3",
        "typos-config-builder",
        "gate",
        "--repository",
        repository,
        "--scope",
        "all",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if recorded == expected.as_slice() {
        return Ok(());
    }
    Err(format!(
        "argv mismatch\nexpected: {expected:?}\nactual: {recorded:?}"
    ))
}

#[rstest]
fn recipe_passes_the_pinned_arguments_to_uv(stub_uv: Result<StubUv, String>) -> Result<(), String> {
    let stub = stub_uv?;
    let output = run_spelling(stub.path(), 0, ".")?;
    if !output.status.success() {
        return Err(make_failure(&output));
    }
    assert_pinned_arguments(&read_lines(&stub.dir, "arguments.log")?, ".")?;
    let recorded_environment = read_lines(&stub.dir, "environment.log")?;
    for expected in ["UV_CACHE_DIR=.uv-cache", "UV_TOOL_DIR=.uv-tools"] {
        if !recorded_environment.iter().any(|line| line == expected) {
            return Err(format!(
                "the recipe must export {expected}, got {recorded_environment:?}"
            ));
        }
    }
    Ok(())
}

#[rstest]
fn a_repository_path_with_spaces_stays_one_argument(
    stub_uv: Result<StubUv, String>,
) -> Result<(), String> {
    let stub = stub_uv?;
    stub.dir
        .create_dir("spelling checkout")
        .map_err(|error| format!("create checkout: {error}"))?;
    let spaced_repository = stub.path().join("spelling checkout");
    let repository = spaced_repository
        .to_str()
        .ok_or_else(|| "the temporary path must be valid UTF-8".to_owned())?;
    let output = run_spelling(stub.path(), 0, repository)?;
    if !output.status.success() {
        return Err(make_failure(&output));
    }
    assert_pinned_arguments(&read_lines(&stub.dir, "arguments.log")?, repository)
}

#[rstest]
fn a_failing_stub_fails_the_make_target(stub_uv: Result<StubUv, String>) -> Result<(), String> {
    let stub = stub_uv?;
    let output = run_spelling(stub.path(), 3, ".")?;
    if output.status.success() {
        return Err("make must fail when the spelling command fails".to_owned());
    }
    Ok(())
}
