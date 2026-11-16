//! Non-vacuous policy test for the repository spelling overlay.
//!
//! The tests run the real pinned builder against temporary, tracked fixtures.
//! The first exercises every pattern in `typos.local.toml`, so a pattern that
//! stops matching makes the fixture fail, and adds one ordinary misspelling to
//! prove the masked prose is still checked. The second commits a `typos.toml`
//! that the builder would regenerate differently and then stages that
//! regeneration, proving the gate's freshness check compares against `HEAD`
//! and so rejects a stale file whether it is committed or merely staged. The
//! third proves the inverse: a stale file that only sits in the index passes,
//! because the run regenerates the worktree file to match `HEAD`. The fourth
//! shows the freshness check is skipped outside a Git work tree.
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

/// The generated configuration the freshness cases commit and then stale.
const TYPOS_FILE: &str = "typos.toml";

/// A hand edit the builder will not reproduce, which makes the file stale.
const STALE_EDIT: &str = "# stale hand edit\n";

/// The commits a fixture repository records while a case runs.
///
/// Naming them keeps the commit helper free of free text, so a caller cannot
/// pass a message that describes a state the case did not reach.
#[derive(Clone, Copy)]
enum FixtureCommit {
    Initial,
    Generated,
    Stale,
}

impl FixtureCommit {
    /// Returns the message this step records.
    const fn message(self) -> &'static str {
        match self {
            Self::Initial => "fixture",
            Self::Generated => "generated",
            Self::Stale => "stale",
        }
    }
}

/// How the stale configuration reached the repository when the gate ran.
#[derive(Clone, Copy)]
enum StaleState {
    Committed,
    Staged,
}

impl StaleState {
    /// Describes the state in the failure message of the assertion helper.
    const fn describe(self) -> &'static str {
        match self {
            Self::Committed => "the stale file is committed",
            Self::Staged => "the stale file is committed and the fix is only staged",
        }
    }
}

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

/// Writes the overlay and prose into a new, unversioned temporary directory.
///
/// Keeping this separate lets a case run the recipe outside a Git work tree,
/// where it never reaches the freshness check.
fn stage_fixture_files(prose: &str) -> Result<tempfile::TempDir, String> {
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
    Ok(directory)
}

/// Creates a committed fixture repository with the overlay and some prose.
///
/// The builder's generated `typos.toml` is deliberately absent from the
/// commit: the first gate run writes it, and the recipe then rejects it until
/// the case commits it.
fn stage_fixture(prose: &str) -> Result<tempfile::TempDir, String> {
    let directory = stage_fixture_files(prose)?;
    run_git(directory.path(), &["init"])?;
    commit_fixture(directory.path(), FixtureCommit::Initial)?;
    Ok(directory)
}

/// Runs the spelling gate against one fixture repository.
fn run_gate(fixture: &Path) -> Result<Output, String> {
    run_gate_with_overrides(fixture, &[])
}

/// Runs the spelling gate with extra `make` command-line variable overrides.
fn run_gate_with_overrides(fixture: &Path, overrides: &[&str]) -> Result<Output, String> {
    let mut repository = OsString::from("SPELLING_REPOSITORY=");
    repository.push(fixture);
    Command::new("make")
        .arg("--no-print-directory")
        .arg("-f")
        .arg(repository_root().join("Makefile"))
        .arg("spelling")
        .arg(repository)
        .args(overrides)
        .current_dir(repository_root())
        .env("TYPOS_CONFIG_BUILDER_OFFLINE", "true")
        .output()
        .map_err(|error| format!("run make: {error}"))
}

/// Runs only the recipe's freshness check by stubbing the builder command.
///
/// The pinned builder enumerates tracked files with git, so it cannot run in a
/// directory outside a work tree. Overriding `SPELLING_COMMAND` with `true`
/// isolates the recipe branch that decides whether the freshness check runs at
/// all, which is what the outside-a-work-tree case needs to prove.
fn run_freshness_check(fixture: &Path) -> Result<Output, String> {
    run_gate_with_overrides(fixture, &["SPELLING_COMMAND=true"])
}

/// Appends the stale hand edit to the fixture's tracked configuration.
///
/// The freshness cases need a `typos.toml` that differs from what the builder
/// writes, and a hand-appended line is the smallest such edit.
fn append_stale_edit(directory: &Path) -> Result<(), String> {
    let fixture = Dir::open_ambient_dir(directory, ambient_authority())
        .map_err(|error| format!("open fixture: {error}"))?;
    let existing = fixture
        .read_to_string(TYPOS_FILE)
        .map_err(|error| format!("read {TYPOS_FILE}: {error}"))?;
    fixture
        .write(TYPOS_FILE, format!("{existing}{STALE_EDIT}"))
        .map_err(|error| format!("write {TYPOS_FILE}: {error}"))
}

/// Commits every change in a fixture repository.
fn commit_fixture(directory: &Path, commit: FixtureCommit) -> Result<(), String> {
    stage_fixture_changes(directory)?;
    run_git(directory, &["commit", "-m", commit.message()])
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

/// Asserts that the gate rejected a generated `typos.toml` left untracked.
///
/// The recipe runs the builder first, so a fresh fixture always reaches the
/// checked-in-file test with a file the builder has just written.
fn check_untracked_failure(output: &Output, scenario: &str) -> Result<(), String> {
    if output.status.success() {
        return Err(format!(
            "the gate accepted an untracked typos.toml, when {scenario}"
        ));
    }
    let rendered = gate_output(output);
    if !rendered.contains("typos.toml is not tracked") {
        return Err(format!(
            "the gate rejected the fixture without the untracked hint, when {scenario}:\n{rendered}"
        ));
    }
    Ok(())
}

/// Asserts that the gate rejected a stale `typos.toml` with its freshness hint.
fn check_freshness_failure(output: &Output, state: StaleState) -> Result<(), String> {
    let scenario = state.describe();
    if output.status.success() {
        return Err(format!(
            "the gate accepted a typos.toml its builder would not, when {scenario}"
        ));
    }
    let rendered = gate_output(output);
    if !rendered.contains("differs from HEAD") {
        return Err(format!(
            "the gate rejected the stale file without the freshness hint, when \
             {scenario}:\n{rendered}"
        ));
    }
    if rendered.contains("typos.toml is not tracked") {
        return Err(format!(
            "the gate reported the file as untracked rather than stale, when \
             {scenario}:\n{rendered}"
        ));
    }
    Ok(())
}

/// Asserts that the gate rejected a fixture because of its spelling.
fn check_correction_hint(output: &Output, scenario: &str) -> Result<(), String> {
    if output.status.success() {
        return Err(format!(
            "the gate accepted a fixture containing a misspelling, when {scenario}"
        ));
    }
    let rendered = gate_output(output);
    if !rendered.contains("should be") {
        return Err(format!(
            "the gate rejected the fixture without a correction hint, when {scenario}:\n{rendered}"
        ));
    }
    Ok(())
}

/// Asserts that a gate run accepted its fixture.
fn check_gate_passes(output: &Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    Err(gate_failure(output))
}

/// Runs the gate against a fixture and asserts that it accepted the fixture.
fn check_fixture_passes(fixture: &Path) -> Result<(), String> {
    check_gate_passes(&run_gate(fixture)?)
}

/// Runs the gate against a fixture and asserts that it reported the fixture as
/// stale.
fn check_fixture_is_stale(fixture: &Path, state: StaleState) -> Result<(), String> {
    let output = run_gate(fixture)?;
    check_freshness_failure(&output, state)
}

/// Stages a fixture, proves its first gate run rejects the untracked generated
/// `typos.toml`, then commits that file.
///
/// The builder's generated configuration is absent from the initial commit, so
/// every case first proves the untracked-file rejection before it can exercise
/// a tracked run.
fn stage_fixture_and_commit_generated(
    prose: &str,
    scenario: &str,
) -> Result<tempfile::TempDir, String> {
    let fixture = stage_fixture(prose)?;
    let uncommitted = run_gate(fixture.path())?;
    check_untracked_failure(&uncommitted, scenario)?;
    commit_fixture(fixture.path(), FixtureCommit::Generated)?;
    Ok(fixture)
}

/// The overlay passes once its generated file is committed, and an introduced
/// typo fails with a correction hint.
///
/// The recipe runs the builder before the freshness check, so a fixture that
/// is misspelt reports the misspelling first and never reaches the untracked
/// test. The builder still writes the generated file, so the case commits it
/// before checking that the hint survives a tracked run.
#[test]
fn overlay_patterns_pass_and_an_introduced_typo_fails() -> Result<(), String> {
    let passing =
        stage_fixture_and_commit_generated(FIXTURE_PROSE, "the passing fixture is first gated")?;
    check_fixture_passes(passing.path())?;
    let failing = stage_fixture(&format!("{FIXTURE_PROSE}{INTRODUCED_TYPO}\n"))?;
    let first_run = run_gate(failing.path())?;
    check_correction_hint(&first_run, "the misspelling fixture is first gated")?;
    commit_fixture(failing.path(), FixtureCommit::Generated)?;
    let failing_output = run_gate(failing.path())?;
    check_correction_hint(&failing_output, "the generated typos.toml is committed")
}

/// A stale `typos.toml` fails whether it is committed or only staged.
#[test]
fn stale_typos_toml_fails_the_gate_when_committed_or_staged() -> Result<(), String> {
    let fixture = stage_fixture_and_commit_generated(FIXTURE_PROSE, "the fixture is first gated")?;
    check_fixture_passes(fixture.path())?;
    append_stale_edit(fixture.path())?;
    commit_fixture(fixture.path(), FixtureCommit::Stale)?;
    check_fixture_is_stale(fixture.path(), StaleState::Committed)?;

    // The failing gate regenerated the file, so staging that regeneration
    // leaves the index equal to the worktree while `HEAD` stays stale. A
    // worktree-to-index comparison would accept this; the check must name
    // `HEAD` to reject it.
    stage_fixture_changes(fixture.path())?;
    check_fixture_is_stale(fixture.path(), StaleState::Staged)
}

/// A stale file that is only staged passes, because the run rewrites the
/// worktree copy to match `HEAD` and the comparison ignores the index.
///
/// This is the inverse of the staged-regeneration case: committing the stale
/// file at `HEAD` is what fails, not staging it.
#[test]
fn staged_stale_typos_toml_passes_because_the_run_regenerates_it() -> Result<(), String> {
    let fixture = stage_fixture_and_commit_generated(FIXTURE_PROSE, "the fixture is first gated")?;
    append_stale_edit(fixture.path())?;
    stage_fixture_changes(fixture.path())?;
    check_fixture_passes(fixture.path())?;
    Ok(())
}

/// Outside a Git work tree the recipe skips its freshness check entirely.
///
/// The pinned builder itself enumerates tracked files with git, so a real gate
/// run fails before the recipe reaches the check. The recipe branch is
/// therefore isolated by stubbing the builder command with `true`: the same
/// overlay and prose, with no `git init`, then pass without any freshness
/// message.
#[test]
fn a_directory_outside_a_git_work_tree_skips_the_freshness_check() -> Result<(), String> {
    let unversioned = stage_fixture_files(FIXTURE_PROSE)?;

    // Pinning the limitation keeps the stub below honest: if the builder ever
    // learns to run without git, this assertion records that the isolation is
    // no longer needed.
    let builder_run = run_gate(unversioned.path())?;
    if builder_run.status.success() {
        return Err("the pinned builder unexpectedly ran outside a Git work tree".to_owned());
    }
    let builder_output = gate_output(&builder_run);
    if builder_output.contains("typos.toml is not tracked")
        || builder_output.contains("differs from HEAD")
    {
        return Err(format!(
            "the freshness check ran outside a Git work tree:\n{builder_output}"
        ));
    }

    let output = run_freshness_check(unversioned.path())?;
    check_gate_passes(&output)?;
    Ok(())
}
