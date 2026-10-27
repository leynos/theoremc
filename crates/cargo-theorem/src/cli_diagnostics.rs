//! Parse diagnostics and process-boundary error reporting for the CLI.

use std::{ffi::OsString, io::Write, process::ExitCode};

use crate::error::ApplicationError;

use super::{CargoCommand, CargoEntry, Commands, ContextArgs, TheoremArgs};

/// Preserve Clap's success and usage-error output classes and streams.
pub(super) fn report_clap_error(
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
    let message = error.to_string();
    if !write_json_diagnostic(stderr, "cli.usage", "usage", &message) {
        return ExitCode::FAILURE;
    }
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}

/// Detect explicit JSON intent when Clap cannot produce complete matches.
pub(super) fn parse_error_requests_json(args: &[OsString]) -> bool {
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
        if argument_text == "--" {
            return None;
        }
        skip_value = consumes_next_value(args, index, argument_text);
        if argument_text == "context" {
            return Some(index);
        }
        if !argument_text.starts_with('-') {
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
        skip_value = consumes_next_value(args, index, argument_text);
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

/// Report whether a global option at `index` consumes the following token.
fn consumes_next_value(args: &[OsString], index: usize, argument: &str) -> bool {
    takes_global_value(argument)
        && args
            .get(index + 1)
            .and_then(|next| next.to_str())
            .is_some_and(|next| !next.starts_with('-'))
}

/// Write a versioned JSON error document to stderr.
fn write_json_diagnostic(stderr: &mut dyn Write, code: &str, class: &str, message: &str) -> bool {
    let diagnostic = serde_json::json!({
        "schema_version": "1",
        "kind": "cargo-theorem.diagnostic",
        "command": ["context"],
        "status": "error",
        "error": {
            "code": code,
            "class": class,
            "message_fallback": message,
        }
    });
    serde_json::to_writer(&mut *stderr, &diagnostic).is_ok() && writeln!(stderr).is_ok()
}

/// Write configuration diagnostics to stderr and keep their stable exit class.
pub(super) fn report_application_error(
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
        let message = error.to_string();
        if !write_json_diagnostic(stderr, diagnostic_code, class, &message) {
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
pub(super) const fn entry_wants_json(entry: &CargoEntry) -> bool {
    matches!(
        &entry.command,
        CargoCommand::Theorem(TheoremArgs {
            command: Commands::Context(ContextArgs { json: true }),
            ..
        })
    )
}
