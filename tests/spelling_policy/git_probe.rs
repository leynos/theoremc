//! Regression for the spelling recipe's Git probe.
//!
//! The recipe skips its freshness check only when Git reports that the path is
//! not a work tree. Every other probe failure must fail the gate instead of
//! silently skipping the check, so an unreadable repository cannot pass.

use super::{FIXTURE_PROSE, run_freshness_check, run_git, stage_fixture};
use std::process::Output;

/// The repository format version Git rejects.
///
/// A version past the one Git supports makes every later Git command fail with
/// an error other than "not a git repository", which is the probe result the
/// recipe must reject. The fresh fixture accepts the write before the probe
/// reads the unusable version back.
const UNSUPPORTED_REPOSITORY_FORMAT: &str = "99";

/// Creates a committed fixture repository that Git refuses to read.
fn stage_unreadable_repository() -> Result<tempfile::TempDir, String> {
    let directory = stage_fixture(FIXTURE_PROSE)?;
    run_git(
        directory.path(),
        &[
            "config",
            "core.repositoryformatversion",
            UNSUPPORTED_REPOSITORY_FORMAT,
        ],
    )?;
    Ok(directory)
}

/// Asserts that the gate failed on the probe and printed Git's own error.
///
/// A silent skip would let the gate pass, and a generic message would hide
/// which command failed, so both the status and the reported text matter.
fn check_probe_failure(output: &Output) -> Result<(), String> {
    if output.status.success() {
        return Err("the gate accepted a repository Git could not inspect".to_owned());
    }
    let reported = String::from_utf8_lossy(&output.stderr);
    if !reported.contains("git could not inspect the repository") {
        return Err(format!(
            "the gate failed without naming the Git probe error:\n{reported}"
        ));
    }
    if !reported.contains("Expected git repo version") {
        return Err(format!(
            "the gate did not print Git's probe error:\n{reported}"
        ));
    }
    Ok(())
}

/// A repository Git cannot read fails the probe instead of skipping the check.
#[test]
fn a_repository_git_cannot_read_fails_the_probe() -> Result<(), String> {
    let unreadable = stage_unreadable_repository()?;
    check_probe_failure(&run_freshness_check(unreadable.path())?)
}
