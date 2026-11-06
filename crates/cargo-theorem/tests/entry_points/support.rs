//! Shared Cargo subcommand setup for entry-point integration tests.

use std::ffi::OsString;

use anyhow::Result;
use cap_std::{ambient_authority, fs::Dir};
use rstest::fixture;

use super::{install_subcommand, isolated_path};

pub(super) struct CargoSubcommandEnvironment {
    pub(super) directory: tempfile::TempDir,
    pub(super) path: OsString,
}

#[fixture]
pub(super) fn cargo_subcommand_environment() -> Result<CargoSubcommandEnvironment> {
    let directory = tempfile::tempdir()?;
    let bin_dir = directory.path().join("bin");
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.create_dir("bin")?;
    install_subcommand(&bin_dir)?;
    let path = isolated_path(&bin_dir)?;
    Ok(CargoSubcommandEnvironment { directory, path })
}
