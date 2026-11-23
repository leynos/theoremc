//! Running a prepared child process with diagnostics that identify it.
//!
//! `Command::output` reports a spawn failure as a bare [`std::io::Error`], and a
//! completed failure as an exit status with two output streams. Both shapes are
//! useless without the command that produced them, so fixture builds and lookup
//! helpers route their spawns through [`run`] and [`completed_failure`].

use std::process::{Command, Output};

/// Runs `command` to completion, naming the command and its working directory
/// when the process cannot be started.
///
/// A spawn failure happens before the child runs. The common causes are a
/// missing program and a working directory that does not exist; both surface as
/// `No such file or directory (os error 2)`, so the message must distinguish
/// them by naming the command and the directory.
///
/// # Errors
///
/// Returns the command description and the underlying [`std::io::Error`] when
/// the child process cannot be started.
pub(super) fn run(command: &mut Command) -> Result<Output, String> {
    command
        .output()
        .map_err(|error| format!("running {}: {error}", describe(command)))
}

/// Formats a completed but unsuccessful run with labelled output streams.
///
/// The exit status and each stream are labelled separately, because Cargo
/// writes diagnostics to stderr and progress to stdout; concatenating them
/// hides which stream carried the failure.
pub(super) fn completed_failure(command: &Command, output: &Output) -> String {
    format!(
        "{} failed with status {}\nstdout:\n{}\nstderr:\n{}",
        describe(command),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

/// Describes `command` as `` `program arguments` in `directory` ``.
fn describe(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let directory = command.get_current_dir().map_or_else(
        || "<inherited working directory>".to_owned(),
        |path| format!("`{}`", path.display()),
    );
    if arguments.is_empty() {
        format!("`{program}` in {directory}")
    } else {
        format!("`{program} {arguments}` in {directory}")
    }
}

#[cfg(test)]
#[path = "command_run_tests.rs"]
mod tests;
