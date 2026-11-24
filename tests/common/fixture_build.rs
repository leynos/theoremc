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
    /// The child process that is running it, until `wait` collects it.
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
    /// underlying error when its output cannot be collected.
    pub fn wait(mut self) -> Result<BuildLog, String> {
        let Some(child) = self.child.take() else {
            return Err(format!(
                "{} was already collected",
                self.identity.describe(),
            ));
        };
        let output = child.wait_with_output().map_err(|error| {
            format!(
                "waiting for {}: {error}; {} could not be built",
                command_run::describe(&self.command),
                self.identity.describe(),
            )
        })?;
        if !output.status.success() {
            return Err(format!(
                "{}\n{}",
                command_run::completed_failure(&self.command, &output),
                self.identity.describe(),
            ));
        }
        Ok(BuildLog(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )))
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

/// A fixture build script that blocks until the test releases it.
///
/// The script writes `barrier-ready` in the fixture root and then waits for
/// `barrier-release`. That gives a test a synchronisation point controlled by
/// the build itself: once readiness is observed, the build cannot finish until
/// the test writes the release file, so an action taken between the two is
/// provably concurrent with the build. Process-state polling cannot promise
/// that, because the child may exit immediately after any observation.
///
/// The wait is bounded so a test that never releases it fails the build instead
/// of leaving a process behind.
pub const BARRIER_BUILD_SCRIPT: &str = concat!(
    "//! Fixture build script that waits for the test to release it.\n",
    "\n",
    "fn main() -> Result<(), theoremc_build_support::BuildScriptError> {\n",
    "    let root = std::path::PathBuf::from(\n",
    "        std::env::var_os(\"CARGO_MANIFEST_DIR\").expect(\"Cargo sets CARGO_MANIFEST_DIR\"),\n",
    "    );\n",
    "    let readiness = root.join(\"barrier-ready\");\n",
    "    let release = root.join(\"barrier-release\");\n",
    "    std::fs::write(&readiness, b\"ready\").expect(\"signal the barrier readiness\");\n",
    "    let deadline =\n",
    "        std::time::Instant::now() + std::time::Duration::from_secs(300);\n",
    "    while !release.exists() {\n",
    "        assert!(\n",
    "            std::time::Instant::now() < deadline,\n",
    "            \"the test never released the fixture build\",\n",
    "        );\n",
    "        std::thread::sleep(std::time::Duration::from_millis(10));\n",
    "    }\n",
    "    let build_output = theoremc_build_support::prepare_build_script()?;\n",
    "\n",
    "    println!(\"cargo::rustc-check-cfg=cfg(theoremc_has_theorems)\");\n",
    "    if build_output.has_theorems() {\n",
    "        println!(\"cargo::rustc-cfg=theoremc_has_theorems\");\n",
    "    }\n",
    "    for path in build_output.rerun_paths() {\n",
    "        println!(\"cargo::rerun-if-changed={}\", path.as_str());\n",
    "    }\n",
    "\n",
    "    Ok(())\n",
    "}\n",
);

/// Name of the readiness file the barrier build script creates.
pub const BARRIER_READY_FILE: &str = "barrier-ready";

/// Name of the release file the test creates to let the build finish.
pub const BARRIER_RELEASE_FILE: &str = "barrier-release";

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
