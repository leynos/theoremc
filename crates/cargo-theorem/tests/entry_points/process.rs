//! Child-process setup shared by the entry-point integration tests.
//!
//! The tests run the real binary both directly and through Cargo, so each
//! child needs a cleared environment, a temporary command directory, and the
//! values that must survive the clearing: the executable path and the Rust
//! toolchain home. Those values are captured once from the host in
//! [`HostEnv::capture`], and every helper below receives them explicitly, so
//! the test bodies never reach for the process environment themselves.

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;

use super::BINARY;

/// Host values a cleared child still needs, captured at the test boundary.
///
/// A child runs with `env_clear`, so anything it needs from the host must be
/// copied in deliberately. Capturing the four candidate variables once keeps
/// the ambient read in one place and lets the fallback rules be tested from
/// constructed values.
#[derive(Debug, Clone, Default)]
pub(super) struct HostEnv {
    /// The host's executable search path.
    path: Option<OsString>,
    /// An explicitly configured Rust toolchain home, if the host exported one.
    rustup_home: Option<OsString>,
    /// The host's home directory, used to derive a toolchain home when absent.
    home: Option<OsString>,
    /// The Windows home-directory fallback.
    user_profile: Option<OsString>,
}

impl HostEnv {
    /// Captures the host values a cleared child needs, reading each once.
    ///
    /// # Examples
    ///
    /// A test captures the host once, then builds every child from it:
    ///
    /// ```text
    /// let host = HostEnv::capture();
    /// let path = isolated_path(&host, bin_dir)?;
    /// let command = isolated_command(&host, binary, cwd, &path);
    /// ```
    #[expect(
        clippy::disallowed_methods,
        reason = "the child process inherits the host environment, so the test must read the ambient values it copies into the cleared child"
    )]
    pub(super) fn capture() -> Self {
        Self {
            path: env::var_os("PATH"),
            rustup_home: env::var_os("RUSTUP_HOME"),
            home: env::var_os("HOME"),
            user_profile: env::var_os("USERPROFILE"),
        }
    }

    /// Builds a host description from explicit values.
    #[cfg(test)]
    const fn from_parts(
        path: Option<OsString>,
        rustup_home: Option<OsString>,
        home: Option<OsString>,
        user_profile: Option<OsString>,
    ) -> Self {
        Self {
            path,
            rustup_home,
            home,
            user_profile,
        }
    }

    /// Returns the search path the host exported, if any.
    fn path(&self) -> Option<&OsStr> {
        self.path.as_deref()
    }
}

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
pub(super) fn isolated_command(
    host: &HostEnv,
    binary: impl AsRef<OsStr>,
    cwd: &Path,
    path: &OsString,
) -> Command {
    let mut command = Command::new(binary);
    command
        .env_clear()
        .env("PATH", path)
        .env("HOME", cwd)
        .env("XDG_CONFIG_HOME", cwd)
        .env("XDG_CONFIG_DIRS", "")
        .current_dir(cwd);
    if let Some(rustup_home) = rustup_home(host) {
        command.env("RUSTUP_HOME", rustup_home);
    }
    command
}

/// Keep the Cargo proxy pointed at the installed toolchains after isolating
/// `HOME`.
///
/// An explicit `RUSTUP_HOME` wins; otherwise a home directory is turned into
/// the conventional `.rustup` path. Windows spells the home directory
/// differently, hence the `USERPROFILE` fallback.
fn rustup_home(host: &HostEnv) -> Option<OsString> {
    host.rustup_home.clone().or_else(|| {
        host.home
            .as_ref()
            .or(host.user_profile.as_ref())
            .map(|home| PathBuf::from(home).join(".rustup").into_os_string())
    })
}

/// Put the temporary command directory ahead of the host's executable path.
///
/// # Errors
///
/// Returns an error when the joined path is not a valid search path.
pub(super) fn isolated_path(host: &HostEnv, bin_dir: &Path) -> Result<OsString> {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(current) = host.path() {
        paths.extend(env::split_paths(current));
    }
    Ok(env::join_paths(paths)?)
}

/// Find the absolute Cargo client before the test child clears its environment.
///
/// # Errors
///
/// Returns an error when the host exported no search path, or when no directory
/// on it holds a Cargo executable.
pub(super) fn cargo_binary(host: &HostEnv) -> Result<PathBuf> {
    let current = host
        .path()
        .ok_or_else(|| anyhow::anyhow!("PATH is unavailable"))?;
    for directory in env::split_paths(current) {
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

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
