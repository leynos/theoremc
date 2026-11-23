//! `cargo` commands for fixture crates that own their build directories.
//!
//! A fixture build is read back from the target directory the test expects, so
//! the command must not inherit anything that moves it. Building the command in
//! one place keeps every fixture spawn on the same environment and lets a unit
//! test inspect it without running Cargo.

use std::process::Command;

use camino::Utf8Path;

/// Variables that would move a build's artefacts, or put its lock, somewhere
/// the fixture does not own.
const INHERITED_BUILD_LOCATIONS: [&str; 1] = ["CARGO_BUILD_TARGET_DIR"];

/// Variables that would retarget the build's platform or its build-script
/// output directory.
const INHERITED_BUILD_LAYOUT: [&str; 1] = ["CARGO_BUILD_TARGET"];

/// Returns a `cargo` command that runs in `manifest_dir` and builds into
/// `target_dir`, whatever the caller exports.
///
/// `CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` are both set explicitly, so
/// an exported or configured value cannot win. Pinning the build directory as
/// well as the target keeps build-script output at `<target_dir>/debug/build`,
/// which is exactly where [`super::fixture_build`] looks for the generated
/// suite; an inherited `build.build-dir` in a `config.toml` would otherwise
/// move it. `CARGO_BUILD_TARGET_DIR` and `CARGO_BUILD_TARGET` are removed for
/// the same reason: neither may split, share or retarget the build.
///
/// The command runs offline: a fixture's lock is seeded from the workspace's
/// and its dependencies are already in the registry cache, so it needs no
/// network and must not wait on the index.
///
/// # Errors
///
/// Returns an error when `target_dir` is relative. The child runs in
/// `manifest_dir`, so a relative target would name a different directory there
/// than it does in the test; rejecting it keeps every caller explicit.
///
/// # Examples
///
/// ```
/// let command = test_helpers::hermetic_cargo(
///     camino::Utf8Path::new("/fixture"),
///     camino::Utf8Path::new("/fixture/target"),
/// )
/// .expect("an absolute target is accepted");
/// assert_eq!(
///     command.get_current_dir(),
///     Some(std::path::Path::new("/fixture")),
/// );
/// ```
pub fn hermetic_cargo(manifest_dir: &Utf8Path, target_dir: &Utf8Path) -> Result<Command, String> {
    if target_dir.is_relative() {
        return Err(format!(
            "the fixture target directory {target_dir} must be absolute: the child cargo runs in {manifest_dir}",
        ));
    }
    let mut command = Command::new("cargo");
    for variable in INHERITED_BUILD_LOCATIONS
        .into_iter()
        .chain(INHERITED_BUILD_LAYOUT)
    {
        command.env_remove(variable);
    }
    // Offline is set after the removals so it is the last operation on the
    // command's environment.
    command
        .current_dir(manifest_dir)
        .env("CARGO_TARGET_DIR", target_dir.as_str())
        .env("CARGO_BUILD_BUILD_DIR", target_dir.as_str())
        .env("CARGO_NET_OFFLINE", "true");
    Ok(command)
}

#[cfg(test)]
#[path = "hermetic_cargo_tests.rs"]
mod tests;
