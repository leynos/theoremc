//! Shared Cargo subcommand setup for entry-point integration tests.

use std::ffi::OsString;

use anyhow::Result;
use cap_std::{ambient_authority, fs::Dir};
use rstest::fixture;

use super::process::{HostEnv, install_subcommand, isolated_path};

/// One installed `cargo-theorem` subcommand plus the host values it needs.
pub(super) struct CargoSubcommandEnvironment {
    /// Temporary command directory the child runs in.
    pub(super) directory: tempfile::TempDir,
    /// The child's search path, with the installed subcommand first.
    pub(super) path: OsString,
    /// Host values copied into the cleared child environment.
    pub(super) host: HostEnv,
}

#[fixture]
pub(super) fn cargo_subcommand_environment() -> Result<CargoSubcommandEnvironment> {
    let directory = tempfile::tempdir()?;
    let host = HostEnv::capture();
    let bin_dir = directory.path().join("bin");
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.create_dir("bin")?;
    install_subcommand(&bin_dir)?;
    let path = isolated_path(&host, &bin_dir)?;
    Ok(CargoSubcommandEnvironment {
        directory,
        path,
        host,
    })
}
