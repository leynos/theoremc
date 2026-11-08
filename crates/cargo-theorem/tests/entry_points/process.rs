//! Child-process setup shared by the entry-point integration tests.
//!
//! The tests run the real binary both directly and through Cargo, so each
//! child needs a cleared environment, a temporary command directory, and the
//! values that must survive the clearing: the executable path and the Rust
//! toolchain home. Keeping that plumbing here lets the test module state only
//! the behaviour each case exercises.

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;

use super::BINARY;

/// Install the current test binary under Cargo's external-command name.
pub(super) fn install_subcommand(bin_dir: &Path) -> Result<()> {
    let destination = bin_dir.join(binary_name("cargo-theorem"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(BINARY, destination)?;
    #[cfg(windows)]
    std::fs::copy(BINARY, destination)?;
    Ok(())
}

/// Build an isolated child environment with no inherited configuration.
pub(super) fn isolated_command(binary: impl AsRef<OsStr>, cwd: &Path, path: &OsString) -> Command {
    let mut command = Command::new(binary);
    command
        .env_clear()
        .env("PATH", path)
        .env("HOME", cwd)
        .env("XDG_CONFIG_HOME", cwd)
        .env("XDG_CONFIG_DIRS", "")
        .current_dir(cwd);
    if let Some(rustup_home) = rustup_home() {
        command.env("RUSTUP_HOME", rustup_home);
    }
    command
}

/// Keep the Cargo proxy pointed at the installed toolchains after isolating HOME.
#[expect(
    clippy::disallowed_methods,
    reason = "the child process inherits the host environment, so the test must read the ambient values it copies into the cleared child"
)]
fn rustup_home() -> Option<OsString> {
    env::var_os("RUSTUP_HOME").or_else(|| {
        env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(|home| PathBuf::from(home).join(".rustup").into_os_string())
    })
}

/// Put the temporary command directory ahead of the host's executable path.
#[expect(
    clippy::disallowed_methods,
    reason = "the child process inherits the host environment, so the test must read the ambient values it copies into the cleared child"
)]
pub(super) fn isolated_path(bin_dir: &Path) -> Result<OsString> {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(current) = env::var_os("PATH") {
        paths.extend(env::split_paths(&current));
    }
    Ok(env::join_paths(paths)?)
}

/// Find the absolute Cargo client before the test child clears its environment.
#[expect(
    clippy::disallowed_methods,
    reason = "the child process inherits the host environment, so the test must read the ambient values it copies into the cleared child"
)]
pub(super) fn cargo_binary() -> Result<PathBuf> {
    let current = env::var_os("PATH").ok_or_else(|| anyhow::anyhow!("PATH is unavailable"))?;
    for directory in env::split_paths(&current) {
        let candidate = directory.join(binary_name("cargo"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(anyhow::anyhow!("Cargo executable was not found on PATH"))
}

/// Return the platform-specific executable filename.
fn binary_name(name: &str) -> OsString {
    #[cfg(windows)]
    return OsString::from(format!("{name}.exe"));
    #[cfg(not(windows))]
    OsString::from(name)
}
