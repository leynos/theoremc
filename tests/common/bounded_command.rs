//! Runs a fixture command to completion within a deadline.
//!
//! A fixture `cargo` build, or a `cargo kani` verification, can stall: Cargo
//! waits on the shared package-cache lock, a build script blocks on compiler
//! admission, or a verifier spins. Without a bound the stall waits for the whole
//! nextest run's `global-timeout`, holding the host's admission queue behind one
//! test. This module gives every fixture command a deadline and turns an overrun
//! into a test failure that names the command and carries what it printed.
//!
//! On Unix the child is placed in its own process group, so a timeout kills the
//! group rather than one process: `cargo` spawns `rustc`, and `cargo kani`
//! spawns a whole build, so killing only the direct child would leave the
//! compiler work running and still holding the resources the deadline exists to
//! release.

use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// How often a running child is checked against its deadline.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// How long the output readers may take to finish after the child is gone.
///
/// A child that has exited has at most one pipe buffer of output still in flight,
/// so this is generous for reading it. It exists for the other case: a
/// grandchild that inherited the pipes can keep them open indefinitely, and
/// waiting on such a pipe for ever would defeat the deadline it must not outlive.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// What a completed fixture command produced.
#[derive(Debug)]
pub struct CommandOutput {
    /// The child's exit status.
    pub status: ExitStatus,
    /// Everything the child wrote to stdout.
    pub stdout: String,
    /// Everything the child wrote to stderr.
    pub stderr: String,
}

impl CommandOutput {
    /// Returns whether the child exited successfully.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status.success()
    }

    /// Returns stdout and stderr concatenated, in that order.
    ///
    /// `cargo` splits a build's messages between the two streams, so callers
    /// that only ask whether a fragment appeared read the combined text.
    #[must_use]
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Why a fixture command did not produce a completed run.
#[derive(Debug)]
pub enum CommandFailure {
    /// The command could not be started, or waiting for it failed.
    Spawn(io::Error),
    /// The command was killed for outliving its deadline.
    Timeout {
        /// The deadline the command overran.
        deadline: Duration,
        /// How long the command actually ran before it was killed.
        elapsed: Duration,
        /// What the command printed before it was killed.
        output: String,
    },
}

impl std::fmt::Display for CommandFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "could not run the command: {error}"),
            Self::Timeout {
                deadline,
                elapsed,
                output,
            } => write!(
                formatter,
                "the command did not finish within {deadline:?} (ran for {elapsed:?}); \
                 it printed:\n{output}"
            ),
        }
    }
}

/// Runs `command`, killing its process group when it outlives `deadline`.
///
/// Both streams are piped and drained on their own threads, because a child that
/// fills a pipe buffer blocks in `write` and would then look like a hang to this
/// wait.
///
/// # Errors
///
/// Returns [`CommandFailure::Spawn`] when the process cannot be started, and
/// [`CommandFailure::Timeout`] when it outlives `deadline`. A timeout carries
/// whatever the command printed before it was killed.
pub fn run_within_deadline(
    mut command: Command,
    deadline: Duration,
) -> Result<CommandOutput, CommandFailure> {
    place_in_own_process_group(&mut command);
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CommandFailure::Spawn)?;
    let readers = match Readers::start(&mut child) {
        Ok(readers) => readers,
        Err(failure) => {
            kill_process_group(&mut child);
            return Err(failure);
        }
    };

    match wait_within_deadline(&mut child, deadline, started) {
        Wait::Exited(status) => {
            let [stdout, stderr] = readers.finish();
            Ok(CommandOutput {
                status,
                stdout,
                stderr,
            })
        }
        Wait::TimedOut => {
            kill_process_group(&mut child);
            let elapsed = started.elapsed();
            let [stdout, stderr] = readers.finish();
            Err(CommandFailure::Timeout {
                deadline,
                elapsed,
                output: format!("stdout:\n{stdout}\nstderr:\n{stderr}"),
            })
        }
        Wait::Failed(error) => {
            kill_process_group(&mut child);
            Err(CommandFailure::Spawn(error))
        }
    }
}

/// The threads draining a running child's two streams.
struct Readers {
    /// The reader for standard output.
    stdout: Reader,
    /// The reader for standard error.
    stderr: Reader,
}

impl Readers {
    /// Starts one reader per stream.
    fn start(child: &mut Child) -> Result<Self, CommandFailure> {
        Ok(Self {
            stdout: Reader::start(child.stdout.take(), "standard output")?,
            stderr: Reader::start(child.stderr.take(), "standard error")?,
        })
    }

    /// Collects both streams within [`DRAIN_GRACE`] and returns what arrived.
    ///
    /// A reader still receiving at the end of the grace period is abandoned: its
    /// thread is detached, and the bytes already forwarded are returned. That
    /// happens only when something other than the child holds the pipe open, and
    /// the partial text is exactly what a timeout report needs.
    fn finish(self) -> [String; 2] {
        [self.stdout.finish(), self.stderr.finish()]
    }
}

/// One stream's reader thread and the bytes it has forwarded so far.
///
/// The thread forwards each chunk as it reads it rather than buffering the whole
/// stream, so a descendant that keeps the pipe open cannot hide what the child
/// already wrote.
struct Reader {
    /// Chunks the thread has forwarded.
    chunks: Receiver<Vec<u8>>,
    /// The bytes collected from those chunks.
    collected: Vec<u8>,
}

impl Reader {
    /// Starts a thread that forwards `stream`'s chunks until the pipe closes.
    fn start(
        stream: Option<impl Read + Send + 'static>,
        name: &str,
    ) -> Result<Self, CommandFailure> {
        let source = stream.ok_or_else(|| {
            CommandFailure::Spawn(io::Error::other(format!("{name} was not captured")))
        })?;
        let (sender, chunks) = mpsc::channel();
        // A failed spawn must not leave the child running behind a released
        // lock, so it is reported through the same failure the caller already
        // handles by killing and reaping the child.
        std::thread::Builder::new()
            .name(format!("fixture-{name}-reader"))
            .spawn(move || forward_chunks(source, &sender))
            .map_err(|error| {
                CommandFailure::Spawn(io::Error::other(format!(
                    "could not start the {name} reader: {error}"
                )))
            })?;
        Ok(Self {
            chunks,
            collected: Vec::new(),
        })
    }

    /// Collects the forwarded chunks, waiting at most [`DRAIN_GRACE`] for more.
    ///
    /// The elapsed time is checked before each receive, because a channel with
    /// queued chunks answers even a zero timeout: a descendant that keeps the
    /// pipe busy would otherwise hold this loop, and the fixture lock with it,
    /// past the grace period.
    fn finish(mut self) -> String {
        let started = Instant::now();
        while started.elapsed() < DRAIN_GRACE {
            let left = DRAIN_GRACE.saturating_sub(started.elapsed());
            match self.chunks.recv_timeout(left) {
                Ok(chunk) => self.collected.extend_from_slice(&chunk),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        String::from_utf8_lossy(&self.collected).into_owned()
    }
}

/// Reads `source` to its end, forwarding each chunk to `sender`.
///
/// The loop ends when the pipe closes, when the reader is abandoned (the send
/// fails), or on a read error; an interrupted read is retried, because a signal
/// is not the end of the stream.
fn forward_chunks(mut source: impl Read, sender: &mpsc::Sender<Vec<u8>>) {
    let mut buffer = [0_u8; 8_192];
    while let Some(chunk) = read_chunk(&mut source, &mut buffer) {
        if sender.send(chunk).is_err() {
            return;
        }
    }
}

/// Reads the next non-empty chunk, retrying an interrupted read.
///
/// Returns `None` at end of stream or on a read error: the caller is reporting a
/// build outcome, and a diagnostic about the pipe would displace the compiler's
/// own message.
fn read_chunk(source: &mut impl Read, buffer: &mut [u8; 8_192]) -> Option<Vec<u8>> {
    loop {
        match source.read(buffer) {
            Ok(0) => return None,
            Ok(read) => return buffer.get(..read).map(<[u8]>::to_vec),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

/// How the wait for a child ended.
enum Wait {
    /// The child exited with this status.
    Exited(ExitStatus),
    /// The child outlived its deadline.
    TimedOut,
    /// Waiting for the child failed.
    Failed(io::Error),
}

/// Polls `child` until it exits, or reports a timeout once `deadline` passes.
fn wait_within_deadline(child: &mut Child, deadline: Duration, started: Instant) -> Wait {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Wait::Exited(status),
            Ok(None) => {}
            Err(error) => return Wait::Failed(error),
        }
        if started.elapsed() >= deadline {
            return Wait::TimedOut;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Puts the child in a new process group, so a timeout can kill its descendants.
#[cfg(unix)]
fn place_in_own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

/// Leaves the child in the caller's process group.
///
/// Without process groups a timeout can kill the direct child only; see
/// [`kill_process_group`] for the consequence.
#[cfg(not(unix))]
fn place_in_own_process_group(_command: &mut Command) {}

/// Kills the child and, on Unix, every process in its group, then reaps it.
///
/// The group is killed first so the compiler work `cargo` started dies with it;
/// reaping the direct child afterwards keeps the test from leaving a zombie.
#[cfg(unix)]
fn kill_process_group(child: &mut Child) {
    use rustix::process::{Pid, Signal, kill_process_group as kill_group};

    // A process id that does not fit an `i32`, or is not a usable `Pid`, simply
    // means there is no group to signal; the direct child is still killed below.
    let group = i32::try_from(child.id()).ok().and_then(Pid::from_raw);
    if let Some(target) = group {
        let _signal_result = kill_group(target, Signal::KILL);
    }
    let _kill_result = child.kill();
    let _reap_result = child.wait();
}

/// Kills the child, then reaps it.
///
/// Without process groups only the direct child dies. A `cargo` on a non-Unix
/// host can therefore leave the compiler it started running until that compiler
/// finishes on its own; the Unix path above is what this repository's gates run.
#[cfg(not(unix))]
fn kill_process_group(child: &mut Child) {
    let _kill_result = child.kill();
    let _reap_result = child.wait();
}

#[cfg(test)]
#[path = "bounded_command_tests.rs"]
mod tests;
