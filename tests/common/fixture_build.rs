//! Fixture builds and the lookup of the suite they generate.
//!
//! A fixture failure has three shapes and each needs its own message: a child
//! that never started, a child that ran and failed, and a successful build whose
//! generated suite cannot then be found. Keeping the three apart here means a
//! captured failure says which one happened.

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
pub(super) struct FixtureIdentity<'a> {
    /// The rewritten `fx_` package name, which names the build directories.
    pub(super) package_name: &'a str,
    /// The fixture's temporary source root.
    pub(super) root: &'a Utf8Path,
    /// The shared target every fixture builds into.
    pub(super) target_dir: &'a Utf8Path,
}

impl FixtureIdentity<'_> {
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

/// Runs `cargo build -vv --color never` in the fixture and returns its log.
///
/// # Errors
///
/// Returns a message naming the command and directory when Cargo cannot start,
/// the exit status with labelled streams when it fails, and an empty log on
/// success.
pub(super) fn build_log(identity: &FixtureIdentity<'_>) -> Result<BuildLog, String> {
    let mut command = hermetic_cargo(identity.root, identity.target_dir)?;
    command.args(["build", "-vv", "--color", "never"]);
    let output = command_run::run(&mut command)
        .map_err(|error| format!("{error}; {} could not be built", identity.describe()))?;
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

/// Reads the theorem suite the fixture's build script generated.
///
/// # Errors
///
/// Returns a message naming the build directory when it cannot be opened, and
/// naming the package, target and every attempted candidate when no candidate
/// holds a suite.
pub(super) fn generated_suite_contents(identity: &FixtureIdentity<'_>) -> Result<String, String> {
    let build_dir_path = identity.build_dir();
    let build_dir =
        Dir::open_ambient_dir(&build_dir_path, ambient_authority()).map_err(|error| {
            format!(
                "{}; {} has not been built, or its build directory moved",
                path_error("opening the build directory", &build_dir_path, &error),
                identity.describe(),
            )
        })?;
    let candidates = package_directories(&build_dir, identity.package_name)
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
