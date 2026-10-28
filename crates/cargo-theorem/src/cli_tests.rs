//! Parsing contracts for both Cargo and direct external-command entry.

use std::{
    ffi::OsString,
    io::{self, Write},
    process::ExitCode,
};

use super::{CargoCommand, CargoEntry, Commands};
use anyhow::{Result, ensure};
use clap::{CommandFactory, Parser};
use ortho_config::{MapEnv, OrthoConfigSubcommandDocs};

#[rstest::rstest]
#[case(["cargo-theorem", "theorem", "context", "--json"])]
#[case(["cargo", "theorem", "context", "--json"])]
fn both_entry_shapes_select_the_same_command(#[case] arguments: [&str; 4]) -> Result<()> {
    let parsed = CargoEntry::try_parse_from(arguments)?;
    let command = CargoEntry::command();
    ensure!(
        command.get_name() == "cargo",
        "the entry parser has the wrong root name"
    );
    let CargoEntry {
        command: super::CargoCommand::Theorem(theorem),
    } = parsed;
    ensure!(
        matches!(theorem.command, super::Commands::Context(_)),
        "the external command did not select context"
    );
    Ok(())
}

#[test]
fn help_and_parse_errors_keep_clap_exit_classes() -> Result<()> {
    let help_parse = CargoEntry::try_parse_from(["cargo", "theorem", "--help"]);
    let Err(help_error) = help_parse else {
        anyhow::bail!("help must be represented as a Clap display request");
    };
    ensure!(help_error.exit_code() == 0, "help must be successful");
    let unknown_parse = CargoEntry::try_parse_from(["cargo", "theorem", "unknown"]);
    let Err(parse_error) = unknown_parse else {
        anyhow::bail!("unknown command must be a parse error");
    };
    ensure!(
        parse_error.exit_code() == 2,
        "usage errors must use exit class 2"
    );
    Ok(())
}

#[test]
fn failed_help_write_returns_failure_even_when_fallback_succeeds() -> Result<()> {
    let help_parse = CargoEntry::try_parse_from(["cargo", "theorem", "--help"]);
    let Err(help_error) = help_parse else {
        anyhow::bail!("help must be represented as a Clap display request");
    };
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();

    let exit_code = super::report_clap_error(&help_error, &mut stdout, &mut stderr, false);

    ensure!(
        exit_code == ExitCode::FAILURE,
        "failed help output must return a failure status"
    );
    ensure!(
        stderr.starts_with(b"failed to write command-line diagnostic"),
        "the fallback diagnostic must be attempted"
    );
    Ok(())
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("intentional writer failure"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn generated_wrapper_metadata_matches_the_clap_tree() -> Result<()> {
    let command = CargoEntry::command();
    let clap_wrappers = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    let metadata_wrappers = CargoCommand::get_subcommand_doc_metadata();
    let metadata_names = metadata_wrappers
        .iter()
        .map(|entry| entry.app_name.as_str())
        .collect::<Vec<_>>();
    ensure!(
        clap_wrappers == metadata_names,
        "Cargo wrapper metadata differs from Clap: {clap_wrappers:?} vs {metadata_names:?}"
    );
    Ok(())
}

#[test]
fn generated_subcommand_metadata_matches_the_clap_tree() -> Result<()> {
    let command = CargoEntry::command();
    let theorem = command
        .find_subcommand("theorem")
        .ok_or_else(|| anyhow::anyhow!("theorem command is missing from Clap"))?;
    let clap_names = theorem
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect::<Vec<_>>();
    let metadata_names = Commands::get_subcommand_doc_metadata()
        .into_iter()
        .map(|entry| entry.app_name)
        .collect::<Vec<_>>();
    ensure!(
        clap_names == ["context"] && metadata_names == ["context"],
        "Clap and generated metadata differ: {clap_names:?} vs {metadata_names:?}"
    );
    Ok(())
}

#[test]
fn configuration_argument_builder_does_not_insert_absent_defaults() {
    let options = super::GlobalOptions::default();
    let args = super::config_arguments(&options, None);
    assert_eq!(args, [OsString::from("cargo-theorem")]);
}

#[test]
fn configuration_argument_builder_forwards_present_settings() {
    let options = super::GlobalOptions {
        color: Some(String::from("always")),
        width: Some(100),
        quiet: true,
        config_path: Some("settings.toml".into()),
        ..super::GlobalOptions::default()
    };
    assert_eq!(
        super::config_arguments(&options, Some("ci")),
        [
            "cargo-theorem",
            "--color",
            "always",
            "--width",
            "100",
            "--quiet",
            "--config",
            "settings.toml",
            "--profile",
            "ci",
        ]
        .map(OsString::from)
    );
}

#[test]
fn context_loads_only_an_explicit_configuration_file() -> Result<()> {
    let context = Commands::Context(super::ContextArgs { json: true });
    let options = super::GlobalOptions::default();
    let environment = MapEnv::new();
    ensure!(
        !super::should_load_configuration(&context, &options, &environment),
        "context must skip automatic configuration discovery"
    );

    let empty_environment = MapEnv::new().with_var("THEOREMC_CONFIG_PATH", "");
    ensure!(
        !super::should_load_configuration(&context, &options, &empty_environment),
        "an empty config path must not trigger discovery"
    );

    let cli_selected = super::GlobalOptions {
        config_path: Some("selected.toml".into()),
        ..super::GlobalOptions::default()
    };
    ensure!(
        super::should_load_configuration(&context, &cli_selected, &environment),
        "context must validate an explicitly selected CLI file"
    );

    let environment_selected = MapEnv::new().with_var("THEOREMC_CONFIG_PATH", "selected.toml");
    ensure!(
        super::should_load_configuration(&context, &options, &environment_selected),
        "context must validate an explicitly selected environment file"
    );
    Ok(())
}

#[path = "cli_tests/runner.rs"]
mod runner;
