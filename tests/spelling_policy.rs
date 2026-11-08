//! Non-vacuous policy test for the repository spelling overlay.
//!
//! The tests run the real pinned builder against temporary, tracked fixtures.
//! The first exercises every pattern in `typos.local.toml`, so a pattern that
//! stops matching makes the fixture fail. The second adds one ordinary
//! misspelling to prove the masked prose is still checked. The third commits a
//! `typos.toml` that the builder would regenerate differently and then stages
//! that regeneration, proving the gate's freshness check compares against
//! `HEAD` and so rejects a stale file whether it is committed or merely
//! staged.
//!
//! The fixture prose lives under `tests/fixtures/spelling/` and is excluded
//! from the repository gate, because it must contain the spellings the gate
//! rejects. The exclusion does not reach the temporary repository, which holds
//! the same prose at its root, so the patterns are still exercised.
//!
//! The test runs the same pinned command as `make spelling`, so it needs `uv`
//! and the builder package. `make spelling` fetches the package into the
//! repo-local uv cache on first use and later runs are offline; CI runs the
//! spelling gate before the test suite, so the package is already cached
//! there.

use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Output};

/// Natural prose that exercises every pattern in `typos.local.toml`.
///
/// It lives in a data file so the misspellings stay in readable prose instead
/// of escaped Rust literals, and so the repository gate can exclude the
/// fixture directory without losing coverage of this test's own source.
const FIXTURE_PROSE: &str = include_str!("fixtures/spelling/prose.txt");

/// One ordinary misspelling added to the passing fixture.
const INTRODUCED_TYPO: &str = include_str!("fixtures/spelling/introduced-typo.txt");

/// Returns the package root, which holds the Makefile and the pinned builder.
fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Runs one git command in a fixture and fails on a non-zero status.
fn run_git(directory: &Path, arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-c")
        .arg("user.name=Spelling Policy")
        .arg("-c")
        .arg("user.email=spelling@example.invalid")
        .arg("-c")
        .arg("commit.gpgsign=false")
        .args(arguments)
        .current_dir(directory)
        .output()
        .map_err(|error| format!("run git: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {arguments:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ))
}

/// Creates a committed fixture repository with the overlay and some prose.
fn stage_fixture(prose: &str) -> Result<tempfile::TempDir, String> {
    let directory = tempfile::tempdir().map_err(|error| format!("temp dir: {error}"))?;
    let repository = Dir::open_ambient_dir(repository_root(), ambient_authority())
        .map_err(|error| format!("open repository: {error}"))?;
    let fixture = Dir::open_ambient_dir(directory.path(), ambient_authority())
        .map_err(|error| format!("open temp dir: {error}"))?;
    repository
        .copy("typos.local.toml", &fixture, "typos.local.toml")
        .map_err(|error| format!("copy overlay: {error}"))?;
    fixture
        .write("prose.md", prose)
        .map_err(|error| format!("write prose: {error}"))?;
    run_git(directory.path(), &["init"])?;
    commit_fixture(directory.path(), "fixture")?;
    Ok(directory)
}

/// Runs the spelling gate against one fixture repository.
fn run_gate(fixture: &Path) -> Result<Output, String> {
    let mut repository = OsString::from("SPELLING_REPOSITORY=");
    repository.push(fixture);
    Command::new("make")
        .arg("--no-print-directory")
        .arg("-f")
        .arg(repository_root().join("Makefile"))
        .arg("spelling")
        .arg(repository)
        .current_dir(repository_root())
        .env("TYPOS_CONFIG_BUILDER_OFFLINE", "true")
        .output()
        .map_err(|error| format!("run make: {error}"))
}

/// Appends `line` to a fixture file, keeping the existing content.
///
/// The freshness case needs a committed `typos.toml` that differs from what
/// the builder writes, and a hand-appended line is the smallest such edit.
fn append_line(directory: &Path, name: &str, line: &str) -> Result<(), String> {
    let fixture = Dir::open_ambient_dir(directory, ambient_authority())
        .map_err(|error| format!("open fixture: {error}"))?;
    let existing = fixture
        .read_to_string(name)
        .map_err(|error| format!("read {name}: {error}"))?;
    fixture
        .write(name, format!("{existing}{line}"))
        .map_err(|error| format!("write {name}: {error}"))
}

/// Commits every change in a fixture repository.
fn commit_fixture(directory: &Path, message: &str) -> Result<(), String> {
    stage_fixture_changes(directory)?;
    run_git(directory, &["commit", "-m", message])
}

/// Stages every change in a fixture repository without committing it.
fn stage_fixture_changes(directory: &Path) -> Result<(), String> {
    run_git(directory, &["add", "-A"])
}

/// Renders a gate failure with both streams for the test assertion.
fn gate_failure(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("the gate rejected the passing fixture\nstdout:\n{stdout}\nstderr:\n{stderr}")
}

/// Renders both gate streams so a failing fixture can be searched.
fn gate_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

#[test]
fn overlay_patterns_pass_and_an_introduced_typo_fails() -> Result<(), String> {
    let passing = stage_fixture(FIXTURE_PROSE)?;
    let output = run_gate(passing.path())?;
    if !output.status.success() {
        return Err(gate_failure(&output));
    }

    let failing_prose = format!("{FIXTURE_PROSE}{INTRODUCED_TYPO}\n");
    let failing = stage_fixture(&failing_prose)?;
    let failing_output = run_gate(failing.path())?;
    if failing_output.status.success() {
        return Err("the gate accepted a fixture containing a misspelling".to_owned());
    }
    let rendered = gate_output(&failing_output);
    if !rendered.contains("should be") {
        return Err(format!(
            "the gate rejected the fixture without a correction hint:\n{rendered}"
        ));
    }
    Ok(())
}

/// Asserts that the gate rejected a stale `typos.toml` with its freshness hint.
fn check_freshness_failure(output: &Output, scenario: &str) -> Result<(), String> {
    if output.status.success() {
        return Err(format!(
            "the gate accepted a typos.toml its builder would not, when {scenario}"
        ));
    }
    let rendered = gate_output(output);
    if !rendered.contains("commit the regenerated file") {
        return Err(format!(
            "the gate rejected the stale file without the freshness hint, when \
             {scenario}:\n{rendered}"
        ));
    }
    Ok(())
}

#[test]
fn stale_typos_toml_fails_the_gate_when_committed_or_staged() -> Result<(), String> {
    let fixture = stage_fixture(FIXTURE_PROSE)?;
    let generated = run_gate(fixture.path())?;
    if !generated.status.success() {
        return Err(gate_failure(&generated));
    }
    commit_fixture(fixture.path(), "generated")?;
    let committed = run_gate(fixture.path())?;
    if !committed.status.success() {
        return Err(gate_failure(&committed));
    }

    append_line(fixture.path(), "typos.toml", "# stale hand edit\n")?;
    commit_fixture(fixture.path(), "stale")?;
    let stale = run_gate(fixture.path())?;
    check_freshness_failure(&stale, "the stale file is committed")?;

    // The failing gate regenerated the file, so staging that regeneration
    // leaves the index equal to the worktree while `HEAD` stays stale. A
    // worktree-to-index comparison would accept this; the check must name
    // `HEAD` to reject it.
    stage_fixture_changes(fixture.path())?;
    let staged = run_gate(fixture.path())?;
    check_freshness_failure(
        &staged,
        "the stale file is committed and the fix is only staged",
    )
}
