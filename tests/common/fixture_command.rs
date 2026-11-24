//! The single entry point every fixture command goes through.
//!
//! A fixture command is a real `cargo` invocation, so it both races other
//! fixtures over the shared target directory and can stall. This module composes
//! the guards that make such a command safe to run inside a test suite: the
//! cross-process lock from [`super::fixture_lock`], held for the command's whole
//! lifetime; the deadline from [`super::bounded_command`]; and
//! [`TEST_FIXTURE_BUDGET`], which keeps a test's several commands inside the
//! per-test allowance the nextest group enforces.

use std::process::Command;
use std::time::{Duration, Instant};

use super::bounded_command::{CommandFailure, CommandOutput, run_within_deadline};
use super::fixture_lock::FixtureLock;

/// The environment variable that overrides the fixture command deadline.
pub const FIXTURE_TIMEOUT_ENV: &str = "THEOREMC_FIXTURE_COMMAND_TIMEOUT_SECS";

/// How long a fixture command may run before the test fails.
///
/// This is the default ceiling for **one** command. [`run_fixture_command`] also
/// clamps it to what remains of [`TEST_FIXTURE_BUDGET`], so a test that runs
/// several commands cannot spend more than one per-test allowance in total.
/// It sits deliberately below the `nested-cargo-builds` allowance
/// (twenty warning periods of 60 s in `.config/nextest.toml`), so an overrun is
/// reported by this code with the command and its output, rather than by nextest
/// killing the test with nothing but a timeout notice. The headroom covers the
/// wrapper's own work: taking the lock, spawning, and draining the output.
///
/// The default assumes a host that is not saturated. A cold fixture build — the
/// dependency graph compiled once into a target that was just removed — was
/// measured at more than 540 s while other agents held every compiler slot, so a
/// slow host raises the deadline through [`FIXTURE_TIMEOUT_ENV`]. The per-test
/// budget still clamps an override, so even a very large one cannot push a test
/// past the group's 1200 s allowance.
pub const DEFAULT_FIXTURE_COMMAND_TIMEOUT: Duration = Duration::from_secs(540);

/// The total time one test may spend on fixture commands.
///
/// Each command's own deadline bounds that command; this bounds their sum, so a
/// test that runs three builds cannot start its third at 1080 s and be killed by
/// nextest with the fixture diagnostic lost. It sits below the
/// `nested-cargo-builds` per-test allowance (1200 s), and
/// `tests/nextest_config_contract.rs` holds that relation.
pub const TEST_FIXTURE_BUDGET: Duration = Duration::from_secs(1_140);

/// What a fixture command did.
#[derive(Debug)]
pub enum FixtureOutcome {
    /// The command ran to completion; its status says whether it succeeded.
    Completed(CommandOutput),
    /// The command was killed for outliving its deadline.
    TimedOut {
        /// The deadline the command overran.
        deadline: Duration,
        /// How long the command actually ran before it was killed.
        elapsed: Duration,
        /// What the command printed before it was killed.
        output: String,
    },
}

/// Runs `command` while holding the fixture lock, failing on an overrun.
///
/// The lock is taken before the command starts and released when this function
/// returns, so the command's whole lifetime is guarded. The lock file lives in
/// the shared fixture target, which is the directory the commands contend over.
///
/// # Errors
///
/// Returns an error when the lock cannot be taken, when the command cannot be
/// started, or when the wait for the command fails. An overrun is *not* an error
/// here: it is reported as [`FixtureOutcome::TimedOut`], because a build timeout
/// and a build failure need different diagnostics.
pub fn run_fixture_command(command: Command) -> Result<FixtureOutcome, FixtureCommandError> {
    let deadline = command_deadline(fixture_command_timeout(), remaining_fixture_budget());
    run_fixture_command_within(command, deadline)
}

/// The deadline one command is given, given its configured ceiling and the
/// time left in the test's budget.
///
/// Split from both clocks so a test can prove the clamp: taking the larger of
/// the two, or ignoring the budget, would let a test's commands sum past the
/// group's per-test allowance.
fn command_deadline(configured: Duration, remaining: Duration) -> Duration {
    configured.min(remaining)
}

/// Returns the time left in this test's fixture budget.
///
/// The budget starts at the test's first fixture command and is spent by every
/// later one, including the gaps between them. It is thread-scoped, which is the
/// scope that *is* the test under both runners this repository uses: nextest
/// runs each test in its own process, and libtest — which `make kani` invokes
/// directly — runs each test on its own freshly spawned thread. A process-wide
/// clock would instead be shared by every test in a libtest binary.
///
/// Callers that apply a longer, purpose-specific bound, such as the Kani
/// command, clamp it with this value so one test's commands cannot sum past
/// nextest's per-test allowance.
#[must_use]
pub fn remaining_fixture_budget() -> Duration {
    thread_local! {
        /// When this test's first fixture command started.
        static STARTED: Instant = Instant::now();
    }
    STARTED
        .with(|started| TEST_FIXTURE_BUDGET.saturating_sub(started.elapsed()))
        .max(MINIMUM_FIXTURE_DEADLINE)
}

/// The least time a fixture command is ever given.
///
/// Once the test's budget is spent, the next command still gets this much: it
/// fails with the fixture's own diagnostic rather than starting with a
/// zero-length deadline that reports a timeout before `cargo` could run.
const MINIMUM_FIXTURE_DEADLINE: Duration = Duration::from_secs(5);

/// Runs `command` under the fixture lock with an explicit `deadline`.
///
/// Callers that are not fixture builds — such as the probe for a usable Kani —
/// use this to apply their own, shorter bound. The lock wait is bounded by the
/// same deadline, so a caller can never spend longer waiting for the lock than
/// the work it is about to do is allowed to take.
///
/// A busy lock is reported as [`FixtureCommandError::Lock`], not as a command
/// failure: a probe that could not start must not be mistaken for a missing
/// tool.
///
/// # Errors
///
/// Returns [`FixtureCommandError::Lock`] when the lock cannot be taken, and
/// [`FixtureCommandError::Command`] when the command cannot be started or
/// waiting for it fails.
pub fn run_fixture_command_within(
    command: Command,
    deadline: Duration,
) -> Result<FixtureOutcome, FixtureCommandError> {
    let started = Instant::now();
    let lock = FixtureLock::acquire_for(deadline).map_err(FixtureCommandError::Lock)?;
    let left = deadline.saturating_sub(started.elapsed());
    run_locked(&lock, command, left).map_err(FixtureCommandError::Command)
}

/// Why a fixture command did not produce an outcome.
#[derive(Debug)]
pub enum FixtureCommandError {
    /// The fixture lock could not be taken within the deadline.
    ///
    /// This is the caller's contention, not a property of the command, so a
    /// caller that is diagnosing a tool must never read it as absence.
    Lock(String),
    /// The command could not be started, or waiting for it failed.
    Command(String),
}

impl std::fmt::Display for FixtureCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lock(reason) | Self::Command(reason) => formatter.write_str(reason),
        }
    }
}

/// Runs `command` while `lock` is held, applying `deadline`.
///
/// The guard is borrowed so the caller keeps it alive for the command's whole
/// lifetime; it is released when the caller's binding drops.
///
/// This split exists so a unit test can supply a lock over a scratch directory
/// instead of taking the shared lock, which a concurrently running fixture suite
/// may hold for minutes.
///
/// # Errors
///
/// Returns an error when the command cannot be started or when waiting for it
/// fails. An overrun is reported as [`FixtureOutcome::TimedOut`].
pub(super) fn run_locked(
    _lock: &FixtureLock,
    command: Command,
    deadline: Duration,
) -> Result<FixtureOutcome, String> {
    Ok(match run_within_deadline(command, deadline) {
        Ok(output) => FixtureOutcome::Completed(output),
        Err(CommandFailure::Timeout {
            deadline: overran,
            elapsed,
            output,
        }) => FixtureOutcome::TimedOut {
            deadline: overran,
            elapsed,
            output,
        },
        Err(failure @ CommandFailure::Spawn(_)) => return Err(failure.to_string()),
    })
}

/// Returns the deadline a fixture command is given.
///
/// `THEOREMC_FIXTURE_COMMAND_TIMEOUT_SECS` overrides
/// [`DEFAULT_FIXTURE_COMMAND_TIMEOUT`] for a host whose fixture builds are slower
/// than the default allows. A value that is not a positive whole number of
/// seconds falls back to the default rather than failing the command, because a
/// malformed override should not be indistinguishable from a build failure.
#[must_use]
pub fn fixture_command_timeout() -> Duration {
    deadline_from_override(env_override().as_deref())
}

/// Reads the operator override once, at the fixture boundary.
#[expect(
    clippy::disallowed_methods,
    reason = "the deadline is an operator-facing override read once at the fixture boundary, which is the composition root for these helpers"
)]
fn env_override() -> Option<std::ffi::OsString> {
    std::env::var_os(FIXTURE_TIMEOUT_ENV)
}

/// Turns an override value into a deadline, falling back when it is unusable.
fn deadline_from_override(raw: Option<&std::ffi::OsStr>) -> Duration {
    raw.and_then(std::ffi::OsStr::to_str)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map_or(DEFAULT_FIXTURE_COMMAND_TIMEOUT, Duration::from_secs)
}

#[cfg(test)]
#[path = "fixture_command_tests.rs"]
mod tests;
