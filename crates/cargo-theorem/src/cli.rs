//! Cargo entry shape, application parsing, and process output.

use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use clap::{Args, CommandFactory, Parser, Subcommand};
use ortho_config::{
    LocalizeCmd, NoOpLocalizer, OrthoConfig, OrthoConfigSubcommandDocs, parse_localized_command,
};
use serde::{Deserialize, Serialize};

use crate::{
    agent_context,
    config::{GlobalOptions, ResolvedCommand, process_sources, selected_sources},
    error::ApplicationError,
};

/// Parse and execute one Cargo external-subcommand invocation.
///
/// Both Cargo dispatch and direct invocation pass the `theorem` token, so the
/// same parser accepts `cargo theorem ...` and `cargo-theorem theorem ...`.
pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    run_with_io(args, &mut stdout, &mut stderr)
}

/// Global configuration and command options passed to the application.
#[derive(Debug, Args)]
struct TheoremArgs {
    /// Options shared by the registered commands.
    #[command(flatten)]
    options: GlobalOptions,
    /// Select a named configuration profile.
    #[arg(long, global = true, value_name = "NAME")]
    profile: Option<String>,
    /// Registered command to execute.
    #[command(subcommand)]
    command: Commands,
}

/// Registered application commands for the current foundation milestone.
#[derive(Debug, Clone, Subcommand, Deserialize, Serialize, OrthoConfigSubcommandDocs)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Commands {
    /// Emit the compact, versioned agent command context.
    Context(ContextArgs),
}

/// Arguments for the read-only context command.
#[derive(Debug, Clone, Default, Args, Deserialize, Serialize, OrthoConfig)]
#[serde(deny_unknown_fields)]
#[ortho_config(
    prefix = "THEOREMC_CONTEXT_",
    behaviour(interaction = "non_interactive", mutation = "read_only")
)]
pub(crate) struct ContextArgs {
    /// Emit the machine-readable agent context document.
    #[arg(long, required = true)]
    #[serde(skip)]
    pub json: bool,
}

/// Outer wrapper matching Cargo's external-subcommand argument convention.
#[derive(Debug, Parser)]
#[command(
    name = "cargo",
    bin_name = "cargo",
    version,
    subcommand_required = true
)]
struct CargoEntry {
    /// The only external subcommand owned by this executable.
    #[command(subcommand)]
    command: CargoCommand,
}

/// Single-variant wrapper for `cargo theorem` dispatch.
#[derive(Debug, Subcommand)]
enum CargoCommand {
    /// Theoremc's Cargo-native command root.
    #[command(name = "theorem", version)]
    Theorem(TheoremArgs),
}

/// Parse, resolve configuration, and write the command result to its stream.
fn run_with_io<I, T>(input_args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let parsed_args = input_args.into_iter().map(Into::into).collect::<Vec<_>>();
    let localizer = NoOpLocalizer::new();
    let command = CargoEntry::command()
        .with_base("cargo-theorem")
        .localize(&localizer);
    let (entry, matches) =
        match parse_localized_command::<CargoEntry, _, _>(command, parsed_args, &localizer) {
            Ok(parsed) => parsed,
            Err(error) => return report_clap_error(&error, stdout, stderr),
        };
    let json_mode = entry_wants_json(&entry);
    match execute(entry, &matches, stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report_application_error(&error, stderr, json_mode),
    }
}

/// Resolve the selected command and dispatch it at the application boundary.
fn execute(
    entry: CargoEntry,
    matches: &clap::ArgMatches,
    stdout: &mut dyn Write,
) -> Result<(), ApplicationError> {
    let CargoCommand::Theorem(cli) = entry.command;
    let (discovery, merge) = process_sources();
    let config_args = config_arguments(&cli.options, cli.profile.as_deref());
    let loaded = GlobalOptions::load_with_sources(config_args, discovery.clone(), merge.clone())?;
    let settings = loaded.config().clone();
    let _effective = settings.resolve()?;

    let project_root = Path::new(".");
    let adapter = ResolvedCommand {
        command: cli.command,
        settings: settings.clone(),
    };
    let (_settings, selected) =
        ortho_config::load_globals_and_merge_selected_subcommand_with_sources(
            matches,
            adapter,
            selected_sources(project_root, discovery.as_ref(), merge),
            || Ok::<_, io::Error>(settings),
        )
        .map_err(|error| {
            ApplicationError::Configuration(std::sync::Arc::new(
                ortho_config::OrthoError::Validation {
                    key: String::from("selected command"),
                    message: error.to_string(),
                },
            ))
        })?;

    match selected.command {
        Commands::Context(context_args) if context_args.json => {
            writeln!(stdout, "{}", agent_context::to_json()?)?;
            Ok(())
        }
        Commands::Context(_) => Ok(()),
    }
}

/// Build a configuration-only argument list from explicitly parsed options.
fn config_arguments(options: &GlobalOptions, profile: Option<&str>) -> Vec<OsString> {
    let mut args = vec![OsString::from("cargo-theorem")];
    append_text(&mut args, "--color", options.color.as_deref());
    append_text(&mut args, "--emoji", options.emoji.as_deref());
    append_text(&mut args, "--progress", options.progress.as_deref());
    append_text(
        &mut args,
        "--accessibility",
        options.accessibility.as_deref(),
    );
    if let Some(width) = options.width {
        append_text(&mut args, "--width", Some(&width.to_string()));
    }
    append_text(&mut args, "--locale", options.locale.as_deref());
    append_bool(&mut args, "--quiet", options.quiet);
    append_bool(&mut args, "--verbose", options.verbose);
    append_bool(&mut args, "--no-input", options.no_input);
    append_bool(&mut args, "--plain", options.plain);
    append_bool(&mut args, "--no-pager", options.no_pager);
    if let Some(path) = options.config_path.as_deref() {
        args.push(OsString::from("--config"));
        args.push(path.as_os_str().to_os_string());
    }
    append_text(&mut args, "--profile", profile);
    args
}

/// Append one present option and its value without encoding an absent default.
fn append_text(args: &mut Vec<OsString>, name: &str, maybe_value: Option<&str>) {
    if let Some(selected_value) = maybe_value {
        args.push(OsString::from(name));
        args.push(OsString::from(selected_value));
    }
}

/// Append a true boolean flag while preserving absence as the default.
fn append_bool(args: &mut Vec<OsString>, name: &str, value: bool) {
    if value {
        args.push(OsString::from(name));
    }
}

/// Preserve Clap's success and usage-error output classes and streams.
fn report_clap_error(
    error: &clap::Error,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitCode {
    let code = error.exit_code();
    let rendered = error.render();
    let write_result = if error.use_stderr() {
        write!(stderr, "{rendered}")
    } else {
        write!(stdout, "{rendered}")
    };
    if write_result.is_err() && writeln!(stderr, "failed to write command-line diagnostic").is_err()
    {
        return ExitCode::FAILURE;
    }
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}

/// Write configuration diagnostics to stderr and keep their stable exit class.
fn report_application_error(
    error: &ApplicationError,
    stderr: &mut dyn Write,
    json_mode: bool,
) -> ExitCode {
    let code = error.exit_code();
    if json_mode {
        let (diagnostic_code, class) = match &error {
            ApplicationError::Configuration(_) => ("configuration.invalid", "configuration"),
            ApplicationError::Serialization(_) | ApplicationError::Output(_) => {
                ("application.internal", "internal")
            }
        };
        let diagnostic = serde_json::json!({
            "schema_version": "1",
            "kind": "cargo-theorem.diagnostic",
            "command": ["context"],
            "status": "error",
            "error": {
                "code": diagnostic_code,
                "class": class,
                "message_fallback": error.to_string(),
            }
        });
        if serde_json::to_writer(&mut *stderr, &diagnostic).is_err() || writeln!(stderr).is_err() {
            return ExitCode::FAILURE;
        }
        return code;
    }
    let label = if matches!(&error, ApplicationError::Configuration(_)) {
        "configuration error"
    } else {
        "cargo theorem error"
    };
    if writeln!(stderr, "{label}: {error}").is_err() {
        return ExitCode::FAILURE;
    }
    code
}

/// Identify machine-readable mode before configuration resolution can fail.
const fn entry_wants_json(entry: &CargoEntry) -> bool {
    matches!(
        &entry.command,
        CargoCommand::Theorem(TheoremArgs {
            command: Commands::Context(ContextArgs { json: true }),
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    //! Parsing contracts for both Cargo and direct external-command entry.

    use std::ffi::OsString;

    use super::{CargoEntry, Commands};
    use anyhow::{Result, ensure};
    use clap::{CommandFactory, Parser};
    use ortho_config::OrthoConfigSubcommandDocs;

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
}
