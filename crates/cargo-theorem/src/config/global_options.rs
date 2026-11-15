//! The `GlobalOptions` settings struct and its `OrthoConfig` derive.
//!
//! The derive is the only code in this crate that reads the process
//! environment, so the lint expectation lives here, in the one module that
//! holds nothing else: a later direct read elsewhere in `config` stays denied.

// The `OrthoConfig` derive expands to a process-environment read, and Clippy
// reports that read at the derive on `GlobalOptions`. An expectation on the
// struct does not reach the derive expansion, so it is declared at module
// scope, which this module keeps to the derive alone. Callers that need
// isolation inject sources through `GlobalOptions::load_with_sources` instead
// of relying on this read.
#![expect(
    clippy::disallowed_methods,
    reason = "the OrthoConfig derive expands to a process-environment read; callers that need isolation inject sources through load_with_sources"
)]

use std::collections::BTreeMap;

use clap::{ArgAction, Args};
use ortho_config::OrthoConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Consume `OrthoConfig`'s internal config selector without treating it as a
/// persisted setting.
fn ignore_config_path<'de, D>(deserializer: D) -> Result<Option<std::path::PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let _ignored = serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(None)
}

/// Global options and selected-command namespaces for the application.
///
/// `None` fields keep Clap defaults out of the source merge. Effective renderer
/// defaults are applied only after files, profiles, environment, and CLI have
/// been combined. The switches are `Option<bool>` for the same reason: `None`
/// means "not selected here", so a later source or the renderer can still
/// choose the behaviour.
#[derive(Debug, Clone, Default, Args, Deserialize, Serialize, OrthoConfig)]
#[serde(default, deny_unknown_fields)]
#[ortho_config(
    prefix = "THEOREMC_",
    discovery(
        app_name = "theoremc",
        config_file_name = "theoremc.toml",
        dotfile_name = ".theoremc.toml",
        project_file_name = "theoremc.toml",
        config_cli_long = "config",
        config_cli_visible = true,
        automatic_mode = "stack_scopes",
        scope_order = ["system", "user", "project"],
    ),
    profiles
)]
pub struct GlobalOptions {
    /// Preferred colour policy: `auto`, `always`, or `never`.
    #[arg(long, global = true, value_name = "WHEN")]
    pub color: Option<String>,
    /// Preferred emoji policy: `auto`, `always`, or `never`.
    #[arg(long, global = true, value_name = "WHEN")]
    pub emoji: Option<String>,
    /// Preferred progress display: `auto`, `always`, or `never`.
    #[arg(long, global = true, value_name = "WHEN")]
    pub progress: Option<String>,
    /// Preferred accessibility policy: `auto`, `on`, or `off`.
    #[arg(long, global = true, value_name = "WHEN")]
    pub accessibility: Option<String>,
    /// Preferred terminal width; absent values use the renderer default.
    #[arg(long, global = true, value_name = "COLUMNS")]
    pub width: Option<usize>,
    /// Locale requested for human-readable command output.
    #[arg(long, global = true, value_name = "LANGUAGE")]
    pub locale: Option<String>,
    /// Explicit config path, kept out of the merged application settings.
    #[arg(long = "config", global = true, value_name = "PATH")]
    #[serde(
        default,
        deserialize_with = "ignore_config_path",
        skip_serializing_if = "Option::is_none"
    )]
    #[ortho_config(cli_long = "config")]
    pub config_path: Option<std::path::PathBuf>,
    /// Suppress non-essential human-readable output.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    #[ortho_config(cli_default_as_absent)]
    pub quiet: Option<bool>,
    /// Increase diagnostic detail for human-readable output.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    #[ortho_config(cli_default_as_absent)]
    pub verbose: Option<bool>,
    /// Disable prompts for commands that support interaction.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    #[ortho_config(cli_default_as_absent)]
    pub no_input: Option<bool>,
    /// Disable decorative human-renderer output.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    #[ortho_config(cli_default_as_absent)]
    pub plain: Option<bool>,
    /// Disable paging for human-readable output.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    #[ortho_config(cli_default_as_absent)]
    pub no_pager: Option<bool>,
    /// Settings for registered commands; only the selected namespace is read.
    #[arg(skip)]
    #[serde(default)]
    #[ortho_config(skip_cli)]
    pub cmds: BTreeMap<String, Value>,
}
