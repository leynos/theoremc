//! Cargo entry shape, application parsing, and process output.

use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use clap::{Args, CommandFactory, Parser, Subcommand};
use ortho_config::{
    EnvSource, LocalizeCmd, NoOpLocalizer, OrthoConfig, OrthoConfigSubcommandDocs,
    parse_localized_command,
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
    let parse_error_json_mode = parse_error_requests_json(&parsed_args);
    let localizer = NoOpLocalizer::new();
    let command = CargoEntry::command()
        .with_base("cargo-theorem")
        .localize(&localizer);
    let (entry, matches) =
        match parse_localized_command::<CargoEntry, _, _>(command, parsed_args, &localizer) {
            Ok(parsed) => parsed,
            Err(error) => {
                return report_clap_error(&error, stdout, stderr, parse_error_json_mode);
            }
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
    let settings = if should_load_configuration(&cli.command, &cli.options, discovery.as_ref()) {
        let config_args = config_arguments(&cli.options, cli.profile.as_deref());
        GlobalOptions::load_with_sources(config_args, discovery.clone(), merge.clone())?
            .config()
            .clone()
    } else {
        cli.options.clone()
    };
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

/// Keep generated context independent of implicit files and ordinary settings.
fn should_load_configuration(
    command: &Commands,
    options: &GlobalOptions,
    discovery: &dyn EnvSource,
) -> bool {
    !matches!(command, Commands::Context(_))
        || options.config_path.is_some()
        || discovery
            .get("THEOREMC_CONFIG_PATH")
            .is_some_and(|path| !path.is_empty())
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
    json_mode: bool,
) -> ExitCode {
    let code = error.exit_code();
    if json_mode && code != 0 {
        return report_json_clap_error(error, stderr, code);
    }
    let rendered = error.render();
    let write_result = if error.use_stderr() {
        write!(stderr, "{rendered}")
    } else {
        write!(stdout, "{rendered}")
    };
    if write_result.is_err() {
        // The exit status reports failure even if stderr cannot carry the fallback.
        writeln!(stderr, "failed to write command-line diagnostic").unwrap_or(());
        return ExitCode::FAILURE;
    }
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}

/// Emit a machine-readable usage diagnostic without changing Clap's exit code.
fn report_json_clap_error(error: &clap::Error, stderr: &mut dyn Write, code: i32) -> ExitCode {
    let diagnostic = serde_json::json!({
        "schema_version": "1",
        "kind": "cargo-theorem.diagnostic",
        "command": ["context"],
        "status": "error",
        "error": {
            "code": "cli.usage",
            "class": "usage",
            "message_fallback": error.to_string(),
        }
    });
    if serde_json::to_writer(&mut *stderr, &diagnostic).is_err() || writeln!(stderr).is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}

/// Detect explicit JSON intent when Clap cannot produce complete matches.
fn parse_error_requests_json(args: &[OsString]) -> bool {
    context_command_position(args).is_some_and(|position| has_json_flag_after(args, position))
}

/// Locate the context subcommand after the external Cargo entry token.
fn context_command_position(args: &[OsString]) -> Option<usize> {
    if args.get(1).is_none_or(|argument| argument != "theorem") {
        return None;
    }
    let mut skip_value = false;
    for (index, raw_argument) in args.iter().enumerate().skip(2) {
        if skip_value {
            skip_value = false;
            continue;
        }
        let Some(argument_text) = raw_argument.to_str() else {
            continue;
        };
        if takes_global_value(argument_text) {
            skip_value = args
                .get(index + 1)
                .and_then(|next| next.to_str())
                .is_some_and(|next| !next.starts_with('-'));
        } else if argument_text == "context" {
            return Some(index);
        } else if !argument_text.starts_with('-') {
            return None;
        }
    }
    None
}

/// Detect the JSON flag while ignoring global-option values.
fn has_json_flag_after(args: &[OsString], command_position: usize) -> bool {
    let mut skip_value = false;
    for (index, raw_argument) in args.iter().enumerate().skip(command_position + 1) {
        if skip_value {
            skip_value = false;
            continue;
        }
        let Some(argument_text) = raw_argument.to_str() else {
            continue;
        };
        if argument_text == "--json" {
            return true;
        }
        if argument_text == "--" {
            return false;
        }
        skip_value = takes_global_value(argument_text)
            && args
                .get(index + 1)
                .and_then(|next| next.to_str())
                .is_some_and(|next| !next.starts_with('-'));
    }
    false
}

/// Identify global options whose following token is a value, not a command.
fn takes_global_value(argument: &str) -> bool {
    matches!(
        argument,
        "--color"
            | "--emoji"
            | "--progress"
            | "--accessibility"
            | "--width"
            | "--locale"
            | "--config"
            | "--profile"
    )
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
#[path = "cli_tests.rs"]
mod tests;
