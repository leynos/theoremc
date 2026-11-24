//! Fixture builds and the lookup of the suite they generate.
//!
//! A fixture failure has three shapes and each needs its own message: a child
//! that never started, a child that ran and failed, and a successful build whose
//! generated suite cannot then be found. Keeping the three apart here means a
//! captured failure says which one happened.

use std::process::{Child, Command, Stdio};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};

use super::BuildLog;
use super::command_run;
use super::hermetic_cargo::hermetic_cargo;
use super::io_context::path_error;
use super::suite_layout::{package_directories, read_suite};

/// The directory Cargo writes build-script output under, relative to a target.
const BUILD_SUBDIRECTORY: &str = "debug/build";

/// Identifies the fixture whose build or lookup is being described.
#[derive(Clone)]
pub(super) struct FixtureIdentity {
    /// The rewritten `fx_` package name, which names the build directories.
    package_name: String,
    /// The fixture's temporary source root.
    root: Utf8PathBuf,
    /// The shared target every fixture builds into.
    target_dir: Utf8PathBuf,
}

impl FixtureIdentity {
    /// Captures the identity of a fixture at `root` building into `target_dir`.
    pub(super) fn new(package_name: &str, root: &Utf8Path, target_dir: &Utf8Path) -> Self {
        Self {
            package_name: package_name.to_owned(),
            root: root.to_path_buf(),
            target_dir: target_dir.to_path_buf(),
        }
    }

    /// Describes the fixture for interpolation into an error message.
    fn describe(&self) -> String {
        format!(
            "fixture package {} at {} (shared target {})",
            self.package_name, self.root, self.target_dir
        )
    }

    /// Returns the full path Cargo writes build-script output under.
    fn build_dir(&self) -> Utf8PathBuf {
        self.target_dir.join(BUILD_SUBDIRECTORY)
    }
}

/// Names of the files a fixture build redirects its output streams to.
///
/// The streams go to files rather than pipes: a piped stream that nobody drains
/// can fill and block Cargo, and a build that blocks never reaches a test's
/// readiness barrier. Files also survive the wait, so a readiness failure can
/// still report what Cargo said.
/// Suffix of the file a fixture build redirects its standard output to.
const BUILD_STDOUT_FILE: &str = "build-stdout.log";

/// Suffix of the file a fixture build redirects its standard error to.
const BUILD_STDERR_FILE: &str = "build-stderr.log";

/// A fixture build that has started but has not been waited for.
///
/// Keeping the child accessible lets a concurrency test prove that a sibling's
/// source root is removed while this build is still running, instead of hoping
/// that two threads happen to overlap.
pub struct RunningBuild {
    /// The prepared command, retained so a failure can name it.
    command: Command,
    /// The child process that is running it, until it is collected.
    child: Option<Child>,
    /// The fixture the build belongs to.
    identity: FixtureIdentity,
}

impl RunningBuild {
    /// Waits for the build to finish and returns its log.
    ///
    /// # Errors
    ///
    /// Returns the exit status with labelled streams when Cargo fails, and the
    /// underlying error when its output cannot be read.
    pub fn wait(mut self) -> Result<BuildLog, String> {
        let mut child = self.take_child()?;
        let status = child.wait().map_err(|error| {
            format!(
                "waiting for {}: {error}; {} could not be built",
                command_run::describe(&self.command),
                self.identity.describe(),
            )
        })?;
        // The streams went to files, so `wait_with_output` would return nothing;
        // read them here, and propagate a read failure rather than reporting an
        // empty log as if the build had said nothing.
        let output = std::process::Output {
            status,
            stdout: read_log(&self.stdout_path()).map_err(|error| {
                format!("{error}; {} could not be built", self.identity.describe())
            })?,
            stderr: read_log(&self.stderr_path()).map_err(|error| {
                format!("{error}; {} could not be built", self.identity.describe())
            })?,
        };
        self.finish(&output)
    }

    /// Kills the build, reaps it and reports its output as a failure.
    ///
    /// A concurrency test calls this when the build never reached its barrier:
    /// the exit status and streams explain why, where a bare timeout would not.
    ///
    /// # Errors
    ///
    /// Always returns an error; it carries the status and labelled streams when
    /// they can be read, and otherwise the reason they could not be.
    #[must_use]
    pub fn abort(mut self, reason: &str) -> String {
        let Some(mut child) = self.child.take() else {
            return format!(
                "{reason}; {} was already collected",
                self.identity.describe()
            );
        };
        drop(child.kill());
        match child.wait() {
            Ok(status) => {
                // A log that vanished is itself diagnostic, so name it instead
                // of presenting the stream as empty; whichever log survives is
                // still reported.
                let mut unreadable = Vec::new();
                let output = std::process::Output {
                    status,
                    stdout: self.read_log_or_record(&self.stdout_path(), &mut unreadable),
                    stderr: self.read_log_or_record(&self.stderr_path(), &mut unreadable),
                };
                let report = format!(
                    "{reason}\n{}",
                    command_run::completed_failure(&self.command, &output),
                );
                if unreadable.is_empty() {
                    report
                } else {
                    format!("{report}\nunreadable logs: {}", unreadable.join("; "))
                }
            }
            Err(error) => format!("{reason}; also could not reap the child: {error}"),
        }
    }

    /// Reads one build log, recording the failure instead of hiding it.
    fn read_log_or_record(&self, path: &Utf8Path, unreadable: &mut Vec<String>) -> Vec<u8> {
        read_log(path).unwrap_or_else(|error| {
            unreadable.push(error);
            Vec::new()
        })
    }

    /// Takes the child, or explains that it was already collected.
    fn take_child(&mut self) -> Result<Child, String> {
        self.child
            .take()
            .ok_or_else(|| format!("{} was already collected", self.identity.describe()))
    }

    /// Turns a successful or failed run into a log or a labelled error.
    fn finish(&self, output: &std::process::Output) -> Result<BuildLog, String> {
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        if output.status.success() {
            Ok(BuildLog(log))
        } else {
            Err(format!(
                "{}\n{}",
                command_run::completed_failure(&self.command, output),
                self.identity.describe(),
            ))
        }
    }

    /// Returns the file this build redirects its standard output to.
    ///
    /// The logs live in the shared target rather than the fixture root: the
    /// target always exists, so redirecting output cannot mask the spawn failure
    /// a test is trying to observe, and the root may be removed by the time a
    /// failed build is reported.
    fn stdout_path(&self) -> Utf8PathBuf {
        self.identity.target_dir.join(format!(
            "{}-{BUILD_STDOUT_FILE}",
            self.identity.package_name
        ))
    }

    /// Returns the file this build redirects its standard error to.
    fn stderr_path(&self) -> Utf8PathBuf {
        self.identity.target_dir.join(format!(
            "{}-{BUILD_STDERR_FILE}",
            self.identity.package_name
        ))
    }
}

impl Drop for RunningBuild {
    /// Stops and reaps a build the test never waited for.
    ///
    /// A test that returns early must not leave a child holding the shared
    /// build-directory lock, because every later fixture in the run would wait
    /// on it. Cleanup is best effort: the child may already have exited.
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            drop(child.kill());
            drop(child.wait());
        }
    }
}

/// Reads a build log written by the redirected child streams.
///
/// # Errors
///
/// Returns the operation, the path and the underlying error when the log cannot
/// be read, so a normal completion never reports an empty log as if the build
/// had produced no output.
fn read_log(path: &Utf8Path) -> Result<Vec<u8>, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("the build log {path} has no parent directory"))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("the build log {path} has no file name"))?;
    Dir::open_ambient_dir(parent, ambient_authority())
        .and_then(|dir| dir.read(name))
        .map_err(|error| path_error("reading the build log", path, &error))
}

/// Creates the file a build stream is redirected to, or reports the failure.
///
/// # Errors
///
/// Returns the operation, the path and the underlying error when the file
/// cannot be created. The caller converts the handle for `Stdio`, so this
/// module never names `std::fs` directly.
fn create_log(path: &Utf8Path) -> Result<cap_std::fs_utf8::File, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("the build log {path} has no parent directory"))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("the build log {path} has no file name"))?;
    Dir::open_ambient_dir(parent, ambient_authority())
        .and_then(|dir| dir.create(name))
        .map_err(|error| path_error("creating the build log", path, &error))
}

/// Starts `cargo build -vv --color never` in the fixture without waiting for it.
///
/// The child's streams are redirected to files in the fixture root, so nothing
/// has to drain them while the build runs and their content outlives a failed
/// readiness check.
///
/// # Errors
///
/// Returns a message naming the command and directory when Cargo cannot start,
/// or the log paths when they cannot be created.
pub(super) fn spawn_build(identity: &FixtureIdentity) -> Result<RunningBuild, String> {
    let mut command = hermetic_cargo(&identity.root, &identity.target_dir)?;
    command.args(["build", "-vv", "--color", "never"]);
    spawn_redirected(identity, command)
}

/// Starts `command`, redirecting both streams to the fixture's own log files.
///
/// This is the seam the log-reading test uses: it can supply a stub command and
/// still exercise the real redirection and collection path.
///
/// # Errors
///
/// Returns the log path when it cannot be created, or the command and directory
/// when the child cannot be started.
fn spawn_redirected(
    identity: &FixtureIdentity,
    mut command: Command,
) -> Result<RunningBuild, String> {
    let stdout_path = identity
        .target_dir
        .join(format!("{}-{BUILD_STDOUT_FILE}", identity.package_name));
    let stderr_path = identity
        .target_dir
        .join(format!("{}-{BUILD_STDERR_FILE}", identity.package_name));
    let stdout = create_log(&stdout_path)?.into_std();
    let stderr = create_log(&stderr_path)?.into_std();
    command
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let child = command.spawn().map_err(|error| {
        format!(
            "{}; {} could not be built",
            command_run::spawn_failure(&command, &error),
            identity.describe(),
        )
    })?;
    Ok(RunningBuild {
        command,
        child: Some(child),
        identity: identity.clone(),
    })
}

/// Runs `cargo build -vv --color never` in the fixture and returns its log.
///
/// # Errors
///
/// Returns a message naming the command and directory when Cargo cannot start,
/// the exit status with labelled streams when it fails, and an empty log on
/// success.
pub(super) fn build_log(identity: &FixtureIdentity) -> Result<BuildLog, String> {
    spawn_build(identity)?.wait()
}

/// Reads the theorem suite the fixture's build script generated.
///
/// # Errors
///
/// Returns a message naming the build directory when it cannot be opened, and
/// naming the package, target and every attempted candidate when no candidate
/// holds a suite.
pub(super) fn generated_suite_contents(identity: &FixtureIdentity) -> Result<String, String> {
    let build_dir_path = identity.build_dir();
    let build_dir =
        Dir::open_ambient_dir(&build_dir_path, ambient_authority()).map_err(|error| {
            format!(
                "{}; {} has not been built, or its build directory moved",
                path_error("opening the build directory", &build_dir_path, &error),
                identity.describe(),
            )
        })?;
    let candidates = package_directories(&build_dir, &identity.package_name)
        .map_err(|error| format!("{error} (under {build_dir_path}; {})", identity.describe()))?;
    for name in &candidates {
        if let Some(contents) = read_suite(&build_dir, name)
            .map_err(|error| format!("{error} (under {build_dir_path}; {})", identity.describe()))?
        {
            return Ok(contents);
        }
    }
    Err(format!(
        "no generated theorem_suite.rs was found for {}; attempted under {}: [{}]",
        identity.describe(),
        build_dir_path,
        candidates.join(", "),
    ))
}

#[cfg(test)]
#[path = "fixture_build_tests.rs"]
mod tests;
