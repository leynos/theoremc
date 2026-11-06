//! Check agent-context invocation paths against the executable boundary.

use std::{ffi::OsStr, process::Output};

use anyhow::{Result, ensure};
use ortho_config::AgentContext;
use rstest::rstest;

use super::{BINARY, CargoSubcommandEnvironment, cargo_subcommand_environment, isolated_command};

#[rstest]
fn emitted_context_invocation_is_executable(
    cargo_subcommand_environment: Result<CargoSubcommandEnvironment>,
) -> Result<()> {
    let environment = cargo_subcommand_environment?;
    let original = invoke_json(
        &environment,
        &[BINARY.into(), "theorem".into(), "context".into()],
    )?;
    assert_success_json(&original)?;
    let document: AgentContext = serde_json::from_slice(&original.stdout)?;
    let command = document
        .commands
        .iter()
        .find(|command| command.path.last().map(String::as_str) == Some("context"))
        .ok_or_else(|| anyhow::anyhow!("agent context is missing its context command"))?;
    let repeated = invoke_json(&environment, &command.path)?;
    assert_success_json(&repeated)?;
    ensure!(
        repeated.stdout == original.stdout,
        "following the emitted invocation must reproduce the agent context"
    );
    Ok(())
}

/// Execute the supplied invocation path with the context command's required flag.
fn invoke_json(environment: &CargoSubcommandEnvironment, path: &[String]) -> Result<Output> {
    let (binary, arguments) = path
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("context invocation path is empty"))?;
    Ok(isolated_command(
        OsStr::new(binary),
        environment.directory.path(),
        &environment.path,
    )
    .args(arguments)
    .arg("--json")
    .output()?)
}

/// Check the shared JSON success stream contract.
pub(super) fn assert_success_json(output: &Output) -> Result<()> {
    ensure!(output.status.success(), "command must exit successfully");
    ensure!(
        output.stderr.is_empty(),
        "successful JSON must not use stderr"
    );
    let document: AgentContext = serde_json::from_slice(&output.stdout)?;
    ensure!(
        !document.schema_version.is_empty(),
        "agent context must include a schema version"
    );
    let paths = document
        .commands
        .iter()
        .map(|command| command.path.iter().map(String::as_str).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    ensure!(
        paths
            == [
                vec!["cargo-theorem"],
                vec!["cargo-theorem", "theorem"],
                vec!["cargo-theorem", "theorem", "context"],
            ],
        "agent context must include the declared Cargo wrapper: {paths:?}"
    );
    Ok(())
}
