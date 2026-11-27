//! Direct tests for the child-process host boundary.
//!
//! Each case builds a [`HostEnv`] from explicit values and checks the resulting
//! child command or error, so the fallback rules are exercised without reading
//! or changing the real process environment.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use cap_std::{ambient_authority, fs::Dir};
use rstest::rstest;

use super::{HostEnv, binary_name, cargo_binary, isolated_command, isolated_path, rustup_home};

/// Reports mismatched values without panicking inside a fallible test.
fn require_equal<T: std::fmt::Debug + PartialEq + ?Sized>(
    actual: &T,
    expected: &T,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(
            std::io::Error::other(format!("{context}: expected {expected:?}, got {actual:?}"))
                .into(),
        )
    }
}

/// Converts a literal into the owned form the host description stores.
fn os(value: &str) -> OsString {
    OsString::from(value)
}

/// Builds a host description whose every field is set.
fn fully_populated_host() -> HostEnv {
    HostEnv::from_parts(
        Some(os("/host/bin")),
        Some(os("/explicit/rustup")),
        Some(os("/home/person")),
        Some(os("C:\\Users\\P")),
    )
}

/// Reports whether the command sets `name` to `value` in the child.
fn has_env(command: &std::process::Command, name: &str, value: &OsString) -> bool {
    let expected = std::ffi::OsStr::new(name);
    command
        .get_envs()
        .any(|(key, candidate)| key == expected && candidate == Some(value.as_os_str()))
}

/// Verifies the `RUSTUP_HOME` resolution order.
///
/// An explicit value wins; otherwise the home directory is turned into the
/// conventional `.rustup` path, with `USERPROFILE` as the Windows fallback.
/// Every case supplies its values directly, so no case reads the process
/// environment.
#[rstest]
#[case::explicit_value(
    Some(os("/explicit/rustup")),
    Some(os("/home/person")),
    Some(os("C:\\Users\\P")),
    Some(os("/explicit/rustup"))
)]
#[case::home_directory(
    None,
    Some(os("/home/person")),
    Some(os("C:\\Users\\P")),
    Some(PathBuf::from("/home/person").join(".rustup").into_os_string()),
)]
#[case::windows_profile(
    None,
    None,
    Some(os("C:\\Users\\P")),
    Some(PathBuf::from("C:\\Users\\P").join(".rustup").into_os_string()),
)]
#[case::nothing_set(None, None, None, None)]
fn rustup_home_follows_the_documented_fallback_order(
    #[case] rustup_home_value: Option<OsString>,
    #[case] home: Option<OsString>,
    #[case] user_profile: Option<OsString>,
    #[case] expected: Option<OsString>,
) {
    let host = HostEnv::from_parts(Some(os("/bin")), rustup_home_value, home, user_profile);

    assert_eq!(
        rustup_home(&host),
        expected,
        "the resolved RUSTUP_HOME must follow the documented fallback order"
    );
}

#[test]
fn an_isolated_child_receives_only_the_values_it_is_given() -> Result<(), Box<dyn std::error::Error>>
{
    let host = fully_populated_host();
    let cwd = Path::new("/tmp/work");
    let path = OsString::from("/child/bin:/host/bin");

    let command = isolated_command(&host, "cargo-theorem", cwd, &path);

    require_equal(&has_env(&command, "PATH", &path), &true, "child PATH")?;
    require_equal(
        &has_env(&command, "HOME", &os("/tmp/work")),
        &true,
        "child HOME",
    )?;
    require_equal(
        &has_env(&command, "RUSTUP_HOME", &os("/explicit/rustup")),
        &true,
        "child RUSTUP_HOME",
    )?;
    Ok(())
}

#[test]
fn isolated_path_puts_the_bin_directory_first() -> Result<(), Box<dyn std::error::Error>> {
    let host = HostEnv::from_parts(Some(os("/host/bin")), None, None, None);
    let bin_dir = Path::new("/child/bin");

    let path = isolated_path(&host, bin_dir)?;

    let mut entries = std::env::split_paths(&path);
    require_equal(
        &entries.next(),
        &Some(PathBuf::from(bin_dir)),
        "first search-path entry",
    )?;
    require_equal(
        &entries.next(),
        &Some(PathBuf::from("/host/bin")),
        "inherited search-path entry",
    )?;
    Ok(())
}

#[test]
fn isolated_path_without_a_host_path_keeps_only_the_bin_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let host = HostEnv::from_parts(None, None, None, None);

    let path = isolated_path(&host, Path::new("/child/bin"))?;

    require_equal(
        &std::env::split_paths(&path).collect::<Vec<_>>(),
        &vec![PathBuf::from("/child/bin")],
        "search path with no host value",
    )?;
    Ok(())
}

#[test]
fn cargo_binary_is_absent_without_a_search_path() {
    let host = HostEnv::from_parts(None, None, None, None);

    let error = cargo_binary(&host).expect_err("an absent PATH must fail the lookup");

    assert!(
        error.to_string().contains("PATH is unavailable"),
        "error must say PATH is unavailable, got {error}"
    );
}

#[test]
fn cargo_binary_is_found_in_the_first_directory_that_holds_one()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let empty = temp.path().join("empty");
    let with_cargo = temp.path().join("with-cargo");
    let capability = Dir::open_ambient_dir(temp.path(), ambient_authority())?;
    capability.create_dir_all("empty")?;
    capability.create_dir_all("with-cargo")?;
    let executable = binary_name("cargo");
    let executable_name = executable.to_string_lossy();
    capability.write(format!("with-cargo/{executable_name}"), b"#!/bin/sh\n")?;
    let host = HostEnv::from_parts(
        Some(std::env::join_paths([&empty, &with_cargo])?),
        None,
        None,
        None,
    );

    let found = cargo_binary(&host)?;

    require_equal(
        &found,
        &with_cargo.join(executable_name.as_ref()),
        "Cargo executable selected from the search path",
    )?;
    Ok(())
}

#[test]
fn cargo_binary_reports_an_absent_executable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let host = HostEnv::from_parts(Some(std::env::join_paths([temp.path()])?), None, None, None);

    let error = cargo_binary(&host).expect_err("an empty search path must fail the lookup");

    let message = error.to_string();
    if message.contains("Cargo executable was not found") {
        Ok(())
    } else {
        Err(
            std::io::Error::other(format!("error must say Cargo was not found, got {message}"))
                .into(),
        )
    }
}
