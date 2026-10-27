//! In-process command execution tests with injected environment sources.

use std::{ffi::OsString, io, process::ExitCode, sync::Arc};

use super::{CargoEntry, Commands, FailingWriter};
use anyhow::{Result, ensure};
use cap_std::{ambient_authority, fs::Dir};
use clap::Parser;
use ortho_config::MapEnv;

use crate::cli as application;

#[test]
fn in_process_runner_preserves_success_and_parse_error_streams() -> Result<()> {
    let (context_status, context_stdout, context_stderr) =
        run_with_map_env(["cargo-theorem", "theorem", "context", "--json"]);
    ensure!(context_status == ExitCode::SUCCESS, "context must succeed");
    ensure!(
        serde_json::from_slice::<serde_json::Value>(&context_stdout).is_ok(),
        "context must emit JSON"
    );
    ensure!(
        context_stderr.is_empty(),
        "successful context must not use stderr"
    );

    let (help_status, help_stdout, help_stderr) =
        run_with_map_env(["cargo-theorem", "theorem", "--help"]);
    ensure!(help_status == ExitCode::SUCCESS, "help must be successful");
    ensure!(
        help_stdout
            .windows(b"Usage:".len())
            .any(|part| part == b"Usage:"),
        "help must use stdout"
    );
    ensure!(help_stderr.is_empty(), "help must not use stderr");

    let (parse_status, parse_stdout, parse_stderr) =
        run_with_map_env(["cargo-theorem", "theorem", "unknown"]);
    ensure!(
        parse_status == ExitCode::from(2),
        "usage failures must use exit 2"
    );
    ensure!(
        parse_stdout.is_empty(),
        "usage failures must not use stdout"
    );
    ensure!(
        parse_stderr.starts_with(b"error:"),
        "usage failures must use stderr"
    );
    Ok(())
}

#[test]
fn in_process_runner_reports_configuration_failures_as_json() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.write("invalid.toml", b"color = [\n")?;
    let path = directory.path().join("invalid.toml");
    let (status, stdout, stderr) = run_with_map_env([
        "cargo-theorem",
        "theorem",
        "--config",
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("path is not UTF-8"))?,
        "context",
        "--json",
    ]);
    ensure!(
        status == ExitCode::from(3),
        "configuration failures use exit 3"
    );
    ensure!(
        stdout.is_empty(),
        "configuration failures must not use stdout"
    );
    let diagnostic: serde_json::Value = serde_json::from_slice(&stderr)?;
    ensure!(
        diagnostic
            .pointer("/error/class")
            .and_then(serde_json::Value::as_str)
            == Some("configuration"),
        "configuration errors must have the configuration class"
    );
    Ok(())
}

#[test]
fn in_process_runner_reports_output_failures_as_internal_json() -> Result<()> {
    let source = Arc::new(MapEnv::new());
    let sources = application::ConfigurationSources {
        discovery: source.clone(),
        merge: source,
    };
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();
    let status = application::run_with_sources_and_io(
        ["cargo-theorem", "theorem", "context", "--json"],
        &mut stdout,
        &mut stderr,
        &sources,
    );
    ensure!(status == ExitCode::from(10), "output failures use exit 10");
    let diagnostic: serde_json::Value = serde_json::from_slice(&stderr)?;
    ensure!(
        diagnostic
            .pointer("/error/class")
            .and_then(serde_json::Value::as_str)
            == Some("internal"),
        "output failures must have the internal class"
    );
    Ok(())
}

#[test]
fn json_parse_diagnostic_keeps_the_usage_exit_class() -> Result<()> {
    let (status, stdout, stderr) =
        run_with_map_env(["cargo-theorem", "theorem", "context", "--json", "--unknown"]);
    ensure!(
        status == ExitCode::from(2),
        "JSON usage failures use exit 2"
    );
    ensure!(stdout.is_empty(), "JSON usage failures must not use stdout");
    let diagnostic: serde_json::Value = serde_json::from_slice(&stderr)?;
    ensure!(
        diagnostic
            .pointer("/error/code")
            .and_then(serde_json::Value::as_str)
            == Some("cli.usage"),
        "JSON usage failures must keep the stable code"
    );
    Ok(())
}

#[rstest::rstest]
#[case(&["cargo-theorem", "theorem", "context", "--json"], true)]
#[case(&["cargo-theorem", "theorem", "--locale", "en", "context", "--json"], true)]
#[case(&["cargo-theorem", "theorem", "context", "--locale", "en", "--json"], true)]
#[case(&["cargo-theorem", "theorem", "context", "--", "--json"], false)]
#[case(&["cargo-theorem", "theorem", "--", "context", "--json"], false)]
#[case(&["cargo-theorem", "theorem", "unknown", "--json"], false)]
#[case(&["cargo-theorem", "theorem", "--config", "--json"], false)]
#[case(&["cargo-theorem"], false)]
#[case(&["cargo-theorem", "wrong", "context", "--json"], false)]
fn parse_error_json_detection_matches_command_shapes(
    #[case] arguments: &[&str],
    #[case] expected: bool,
) {
    assert_eq!(parse_error_requests_json(arguments), expected);
}

#[rstest::rstest]
#[case("--color")]
#[case("--emoji")]
#[case("--progress")]
#[case("--accessibility")]
#[case("--width")]
#[case("--locale")]
#[case("--config")]
#[case("--profile")]
fn parse_error_json_detection_skips_global_option_values(#[case] option: &str) {
    let arguments = [
        "cargo-theorem",
        "theorem",
        option,
        "value",
        "context",
        "--json",
    ];
    assert!(
        parse_error_requests_json(&arguments),
        "{option} value must be skipped"
    );
}

fn parse_error_requests_json(arguments: &[&str]) -> bool {
    let os_arguments = arguments.iter().map(OsString::from).collect::<Vec<_>>();
    application::parse_error_requests_json(&os_arguments)
}

#[test]
fn operands_after_end_of_options_do_not_select_json_diagnostics() -> Result<()> {
    let (status, stdout, stderr) = run_with_map_env([
        "cargo-theorem",
        "theorem",
        "--",
        "context",
        "--json",
        "--unknown",
    ]);
    ensure!(status == ExitCode::from(2), "usage errors keep exit 2");
    ensure!(stdout.is_empty(), "usage errors must not use stdout");
    ensure!(
        stderr.starts_with(b"error:"),
        "usage errors must stay human-readable"
    );
    ensure!(
        serde_json::from_slice::<serde_json::Value>(&stderr).is_err(),
        "an operand after `--` must not select JSON diagnostics"
    );
    Ok(())
}

#[test]
fn process_source_wrapper_keeps_help_independent_of_configuration() {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = application::run_with_io(
        ["cargo-theorem", "theorem", "--help"],
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(status, ExitCode::SUCCESS);
    assert!(
        stdout
            .windows(b"Usage:".len())
            .any(|part| part == b"Usage:")
    );
    assert!(stderr.is_empty());
}

#[test]
fn entry_json_detection_tracks_the_selected_context_mode() -> Result<()> {
    let json_entry = CargoEntry::try_parse_from(["cargo-theorem", "theorem", "context", "--json"])?;
    ensure!(
        application::entry_wants_json(&json_entry),
        "JSON context must be detected"
    );

    let text_entry = CargoEntry {
        command: application::CargoCommand::Theorem(application::TheoremArgs {
            options: application::GlobalOptions::default(),
            profile: None,
            command: Commands::Context(application::ContextArgs::default()),
        }),
    };
    ensure!(
        !application::entry_wants_json(&text_entry),
        "text context must not be treated as JSON"
    );
    Ok(())
}

#[test]
fn clap_json_diagnostic_and_application_text_errors_keep_their_exit_classes() -> Result<()> {
    let Err(parse_error) =
        CargoEntry::try_parse_from(["cargo-theorem", "theorem", "context", "--json", "--unknown"])
    else {
        anyhow::bail!("unknown argument must fail parsing");
    };
    let mut json_stderr = Vec::new();
    let json_status =
        application::report_clap_error(&parse_error, &mut Vec::new(), &mut json_stderr, true);
    ensure!(
        json_status == ExitCode::from(2),
        "JSON usage errors keep exit 2"
    );
    ensure!(
        serde_json::from_slice::<serde_json::Value>(&json_stderr).is_ok(),
        "diagnostic must be JSON"
    );

    let error = application::ApplicationError::Output(io::Error::other("closed stream"));
    let mut human_stderr = Vec::new();
    let human_status = application::report_application_error(&error, &mut human_stderr, false);
    ensure!(
        human_status == ExitCode::from(10),
        "output failures use exit 10"
    );
    ensure!(
        human_stderr.starts_with(b"cargo theorem error:"),
        "human errors use stderr"
    );
    Ok(())
}

#[test]
fn failed_json_diagnostic_write_returns_failure() {
    let error = application::ApplicationError::Output(io::Error::other("closed stream"));
    let status = application::report_application_error(&error, &mut FailingWriter, true);
    assert_eq!(status, ExitCode::FAILURE);
}

fn run_with_map_env<const N: usize>(arguments: [&str; N]) -> (ExitCode, Vec<u8>, Vec<u8>) {
    let source = Arc::new(MapEnv::new());
    let sources = application::ConfigurationSources {
        discovery: source.clone(),
        merge: source,
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status =
        application::run_with_sources_and_io(arguments, &mut stdout, &mut stderr, &sources);
    (status, stdout, stderr)
}
