//! A fixture build that blocks until the test releases it.
//!
//! A concurrency test needs the build to be provably in progress at a chosen
//! moment. Polling process state cannot promise that, because the child may exit
//! immediately after any observation. This module supplies a build script the
//! fixture runs instead: it writes a readiness file and then waits for a release
//! file, so an action taken between the two is concurrent with the build.

use camino::Utf8Path;

use super::fixture_crate::FixtureCrate;

/// Build script that waits for the test to release it.
///
/// The script writes `barrier-ready` in the fixture root and then waits for
/// `barrier-release`, with an internal deadline so a test that never releases it
/// fails the build rather than leaving a process behind. Once released it emits
/// exactly what the repository `build.rs` emits.
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

/// Installs the barrier build script in `fixture`.
///
/// # Errors
///
/// Returns an error when the script cannot be written.
pub fn install_barrier(fixture: &FixtureCrate) -> Result<(), String> {
    fixture.write(Utf8Path::new("build.rs"), BARRIER_BUILD_SCRIPT)
}

/// Runs the barrier build once with its release already present.
///
/// This compiles the barrier script and warms the fixture, so the measured
/// build reuses both and reaches the barrier without paying to compile it. The
/// release and readiness files are removed afterwards, and the caller must touch
/// a `rerun-if-changed` input so Cargo runs the script again.
///
/// # Errors
///
/// Returns the same errors as [`FixtureCrate::cargo_build`] and
/// [`FixtureCrate::write`].
pub fn precompile_barrier(fixture: &FixtureCrate) -> Result<(), String> {
    fixture.write(Utf8Path::new(BARRIER_RELEASE_FILE), "warm")?;
    fixture.cargo_build()?;
    fixture.remove_file(Utf8Path::new(BARRIER_RELEASE_FILE))?;
    fixture.remove_file(Utf8Path::new(BARRIER_READY_FILE))
}

/// Waits until `path` exists, polling `interval` for at most `timeout`.
///
/// # Errors
///
/// Returns an error naming the path when it has not appeared within `timeout`, so
/// a build that never reaches its barrier fails the test rather than hanging it.
pub fn await_path(
    fixture: &FixtureCrate,
    path: &Utf8Path,
    timeout: std::time::Duration,
    interval: std::time::Duration,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if fixture.has_file(path) {
            return Ok(());
        }
        std::thread::sleep(interval);
    }
    Err(format!(
        "waited {timeout:?} for {path} to appear, but the build never reached it",
    ))
}
