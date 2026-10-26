//! End-to-end checks for the Cargo and direct executable contracts.

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{Result, ensure};
use cap_std::{ambient_authority, fs::Dir};
use serde_json::Value;

const BINARY: &str = env!("CARGO_BIN_EXE_cargo-theorem");

#[test]
fn cargo_and_direct_invocations_emit_the_same_json_document() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let bin_dir = directory.path().join("bin");
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.create_dir("bin")?;
    install_subcommand(&bin_dir)?;
    let path = isolated_path(&bin_dir)?;
    let direct = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args(["theorem", "context", "--json"])
        .output()?;
    let cargo = cargo_binary()?;
    let via_cargo = isolated_command(cargo.as_os_str(), directory.path(), &path)
        .args(["theorem", "context", "--json"])
        .output()?;

    assert_success_json(&direct)?;
    assert_success_json(&via_cargo)?;
    ensure!(direct.stdout == via_cargo.stdout, "invocation forms differ");
    ensure!(
        direct.stderr.is_empty(),
        "direct invocation wrote to stderr"
    );
    ensure!(
        via_cargo.stderr.is_empty(),
        "Cargo invocation wrote to stderr"
    );
    Ok(())
}

#[test]
fn context_ignores_malformed_automatic_project_configuration() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.write("theoremc.toml", b"color = [\n")?;
    let path = isolated_path(directory.path())?;
    let output = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args(["theorem", "context", "--json"])
        .output()?;

    ensure!(
        output.status.success(),
        "context must skip malformed automatic project configuration"
    );
    assert_success_json(&output)?;
    Ok(())
}

#[test]
fn help_and_json_configuration_failures_use_the_documented_streams() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = isolated_path(directory.path())?;
    let help = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args(["theorem", "--help"])
        .output()?;
    ensure!(help.status.success(), "help must exit successfully");
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    ensure!(
        help_stdout.contains("Usage: cargo theorem"),
        "help must use stdout"
    );
    ensure!(help.stderr.is_empty(), "help must not use stderr");
    let version = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args(["theorem", "--version"])
        .output()?;
    ensure!(version.status.success(), "version must exit successfully");
    ensure!(
        version.stdout.starts_with(b"cargo-theorem "),
        "version must use the executable name"
    );
    ensure!(version.stderr.is_empty(), "version must not use stderr");

    let missing = directory.path().join("missing.toml");
    let invalid = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args([
            OsString::from("theorem"),
            OsString::from("--config"),
            missing.as_os_str().to_os_string(),
            OsString::from("context"),
            OsString::from("--json"),
        ])
        .output()?;
    ensure!(
        invalid.status.code() == Some(3),
        "config failure must use exit 3"
    );
    ensure!(
        invalid.stdout.is_empty(),
        "JSON failure must not use stdout"
    );
    let diagnostic: Value = serde_json::from_slice(&invalid.stderr)?;
    ensure!(
        diagnostic.get("status").and_then(Value::as_str) == Some("error"),
        "JSON failure must be marked as an error"
    );
    ensure!(
        diagnostic
            .get("error")
            .and_then(|error| error.get("class"))
            .and_then(Value::as_str)
            == Some("configuration"),
        "JSON failure must identify the configuration error class"
    );
    Ok(())
}

#[test]
fn command_parse_failure_keeps_usage_status_and_stderr() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = isolated_path(directory.path())?;
    let output = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args(["theorem", "unknown"])
        .output()?;
    ensure!(
        output.status.code() == Some(2),
        "parse failure must use exit 2"
    );
    ensure!(
        output.stdout.is_empty(),
        "parse failure must not use stdout"
    );
    ensure!(
        output.stderr.starts_with(b"error:"),
        "parse failure must use stderr"
    );
    Ok(())
}

#[test]
fn malformed_configuration_has_a_distinct_configuration_exit_class() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.write("broken.toml", b"color = [\n")?;
    let path = isolated_path(directory.path())?;
    let output = isolated_command(OsStr::new(BINARY), directory.path(), &path)
        .args([
            OsString::from("theorem"),
            OsString::from("--config"),
            directory.path().join("broken.toml").into_os_string(),
            OsString::from("context"),
            OsString::from("--json"),
        ])
        .output()?;
    ensure!(
        output.status.code() == Some(3),
        "malformed config must use exit 3"
    );
    ensure!(
        output.stdout.is_empty(),
        "config failure must not use stdout"
    );
    let diagnostic: Value = serde_json::from_slice(&output.stderr)?;
    ensure!(
        diagnostic
            .get("error")
            .and_then(|error| error.get("class"))
            .and_then(Value::as_str)
            == Some("configuration"),
        "malformed config must report the configuration error class"
    );
    Ok(())
}

/// Install the current test binary under Cargo's external-command name.
fn install_subcommand(bin_dir: &Path) -> Result<()> {
    let destination = bin_dir.join(binary_name("cargo-theorem"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(BINARY, destination)?;
    #[cfg(windows)]
    std::fs::copy(BINARY, destination)?;
    Ok(())
}

/// Build an isolated child environment with no inherited configuration.
fn isolated_command(binary: impl AsRef<OsStr>, cwd: &Path, path: &OsString) -> Command {
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
fn rustup_home() -> Option<OsString> {
    env::var_os("RUSTUP_HOME").or_else(|| {
        env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(|home| PathBuf::from(home).join(".rustup").into_os_string())
    })
}

/// Put the temporary command directory ahead of the host's executable path.
fn isolated_path(bin_dir: &Path) -> Result<OsString> {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(current) = env::var_os("PATH") {
        paths.extend(env::split_paths(&current));
    }
    Ok(env::join_paths(paths)?)
}

/// Find the absolute Cargo client before the test child clears its environment.
fn cargo_binary() -> Result<PathBuf> {
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

/// Check the shared JSON success stream contract.
fn assert_success_json(output: &Output) -> Result<()> {
    ensure!(output.status.success(), "command must exit successfully");
    ensure!(
        output.stderr.is_empty(),
        "successful JSON must not use stderr"
    );
    let document: Value = serde_json::from_slice(&output.stdout)?;
    ensure!(
        document.get("schema_version").is_some(),
        "agent context must include a schema version"
    );
    let commands = document
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("agent context is missing its commands array"))?;
    ensure!(
        commands.len() == 2,
        "expected binary root and context commands"
    );
    let binary_path = commands.first().and_then(|command| command.get("path"));
    let context_path = commands.get(1).and_then(|command| command.get("path"));
    ensure!(
        binary_path == Some(&serde_json::json!(["cargo-theorem"])),
        "agent context must include the binary root path"
    );
    ensure!(
        context_path == Some(&serde_json::json!(["cargo-theorem", "context"])),
        "agent context must include the registered context command"
    );
    Ok(())
}
