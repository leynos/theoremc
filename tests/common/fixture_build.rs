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

/// A fixture build that has started but has not been waited for.
///
/// Keeping the child accessible lets a concurrency test prove that a sibling's
/// source root is removed while this build is still running, instead of hoping
/// that two threads happen to overlap.
pub struct RunningBuild {
    /// The prepared command, retained so a failure can name it.
    command: Command,
    /// The child process that is running it.
    child: Child,
    /// The fixture the build belongs to.
    identity: FixtureIdentity,
}

impl RunningBuild {
    /// Returns whether the child process is still running.
    ///
    /// A build that has already exited is reported as not running; a process
    /// that could not be queried is an error, not an absence.
    ///
    /// # Errors
    ///
    /// Returns the command and the underlying error when the child status
    /// cannot be read.
    pub fn is_running(&mut self) -> Result<bool, String> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|error| {
                format!(
                    "checking {}: {error}; {} could not be queried",
                    command_run::describe(&self.command),
                    self.identity.describe(),
                )
            })
    }

    /// Waits for the build to finish and returns its log.
    ///
    /// # Errors
    ///
    /// Returns the exit status with labelled streams when Cargo fails, and the
    /// underlying error when its output cannot be collected.
    pub fn wait(self) -> Result<BuildLog, String> {
        let Self {
            command,
            child,
            identity,
        } = self;
        let output = child.wait_with_output().map_err(|error| {
            format!(
                "waiting for {}: {error}; {} could not be built",
                command_run::describe(&command),
                identity.describe(),
            )
        })?;
        if !output.status.success() {
            return Err(format!(
                "{}\n{}",
                command_run::completed_failure(&command, &output),
                identity.describe(),
            ));
        }
        Ok(BuildLog(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )))
    }
}

/// Starts `cargo build -vv --color never` in the fixture without waiting for it.
///
/// # Errors
///
/// Returns a message naming the command and directory when Cargo cannot start,
/// or when the child's output streams cannot be configured.
pub(super) fn spawn_build(identity: &FixtureIdentity) -> Result<RunningBuild, String> {
    let mut command = hermetic_cargo(&identity.root, &identity.target_dir)?;
    command
        .args(["build", "-vv", "--color", "never"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().map_err(|error| {
        format!(
            "{}; {} could not be built",
            command_run::spawn_failure(&command, &error),
            identity.describe(),
        )
    })?;
    Ok(RunningBuild {
        command,
        child,
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
