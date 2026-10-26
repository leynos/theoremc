//! `OrthoConfig` settings, validation, and selected-command resolution.

use std::{collections::BTreeMap, ffi::OsString, path::Path, sync::Arc};

use clap::Args;
use ortho_config::{
    EnvSource, OrthoConfig, OrthoError, OrthoResult, ProfileLoadOutcome, SelectedSubcommandMerge,
    SelectedSubcommandMergeError, SharedEnvSource, SharedScanEnvSource, SubcommandFileContext,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::cli::Commands;

// Keep namespaces for deferred top-level commands so documented future
// settings remain usable, while rejecting misspellings before dispatch. This
// name-only allowlist does not define parser arguments or command metadata.
const CONFIGURABLE_COMMAND_NAMES: &[&str] = &[
    "init",
    "list",
    "get",
    "create",
    "action",
    "check",
    "build-script",
    "generate",
    "run",
    "replay",
    "report",
    "backend",
    "jobs",
    "profile",
    "context",
    "feedback",
];

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
/// been combined.
#[derive(Debug, Clone, Default, Args, Deserialize, Serialize, OrthoConfig)]
#[serde(default, deny_unknown_fields)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent switches are the public CLI configuration surface."
)]
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
    #[arg(long, global = true)]
    #[ortho_config(cli_default_as_absent)]
    pub quiet: bool,
    /// Increase diagnostic detail for human-readable output.
    #[arg(long, global = true)]
    #[ortho_config(cli_default_as_absent)]
    pub verbose: bool,
    /// Disable prompts for commands that support interaction.
    #[arg(long, global = true)]
    #[ortho_config(cli_default_as_absent)]
    pub no_input: bool,
    /// Disable decorative human-renderer output.
    #[arg(long, global = true)]
    #[ortho_config(cli_default_as_absent)]
    pub plain: bool,
    /// Disable paging for human-readable output.
    #[arg(long, global = true)]
    #[ortho_config(cli_default_as_absent)]
    pub no_pager: bool,
    /// Settings for registered commands; only the selected namespace is read.
    #[arg(skip)]
    #[serde(default)]
    #[ortho_config(skip_cli)]
    pub cmds: BTreeMap<String, Value>,
}

/// Effective values after the application defaults are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "This value carries the independent renderer and interaction switches."
)]
pub struct ResolvedSettings {
    /// Colour policy used by human renderers.
    pub color: String,
    /// Emoji policy used by human renderers.
    pub emoji: String,
    /// Progress policy used by human renderers.
    pub progress: String,
    /// Accessibility policy used by human renderers.
    pub accessibility: String,
    /// Optional terminal width override.
    pub width: Option<usize>,
    /// Optional requested locale.
    pub locale: Option<String>,
    /// Whether quiet mode is active.
    pub quiet: bool,
    /// Whether verbose mode is active.
    pub verbose: bool,
    /// Whether interactive prompts are disabled.
    pub no_input: bool,
    /// Whether decorative human-renderer output is disabled.
    pub plain: bool,
    /// Whether paging is disabled.
    pub no_pager: bool,
}

impl GlobalOptions {
    /// Load from an argument iterator and injected environment sources.
    ///
    /// Explicit `--config` paths and profiles are handled by `OrthoConfig`'s
    /// derived loader, while `MapEnv` lets tests exercise the same precedence
    /// without mutating process-wide environment variables.
    ///
    /// # Errors
    ///
    /// Returns an `OrthoConfig` error when sources cannot be parsed or merged.
    pub fn load_with_sources<I, T>(
        args: I,
        discovery: SharedEnvSource,
        merge: SharedScanEnvSource,
    ) -> OrthoResult<ProfileLoadOutcome<Self>>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Self::load_with_profile_from_iter_with_sources(args, discovery, merge)
    }

    /// Apply application-owned defaults and validate enumerated policies.
    ///
    /// # Errors
    ///
    /// Returns a validation error for an unknown command namespace or policy.
    pub fn resolve(&self) -> OrthoResult<ResolvedSettings> {
        validate_command_namespaces(&self.cmds)?;
        Ok(ResolvedSettings {
            color: policy("color", self.color.as_deref())?,
            emoji: policy("emoji", self.emoji.as_deref())?,
            progress: policy("progress", self.progress.as_deref())?,
            accessibility: accessibility_policy(self.accessibility.as_deref())?,
            width: self.width,
            locale: self.locale.clone(),
            quiet: self.quiet,
            verbose: self.verbose,
            no_input: self.no_input,
            plain: self.plain,
            no_pager: self.no_pager,
        })
    }
}

/// Reject command namespace typos while retaining planned command settings.
fn validate_command_namespaces(namespaces: &BTreeMap<String, Value>) -> OrthoResult<()> {
    if let Some(unknown) = namespaces
        .keys()
        .find(|name| !CONFIGURABLE_COMMAND_NAMES.contains(&name.as_str()))
    {
        return Err(OrthoError::Validation {
            key: format!("cmds.{unknown}"),
            message: format!(
                "unknown command namespace; expected one of: {}",
                CONFIGURABLE_COMMAND_NAMES.join(", ")
            ),
        }
        .into());
    }
    Ok(())
}

/// Merge defaults, one selected namespace, and explicit command-line values.
///
/// `Option` fields preserve omission, so the CLI layer cannot accidentally
/// replace a configured value with a Clap default.
pub(crate) fn merge_namespace<T>(
    namespaces: &BTreeMap<String, Value>,
    name: &str,
    cli: T,
) -> Result<T, SelectedSubcommandMergeError>
where
    T: Default + Serialize + DeserializeOwned,
{
    let mut merged = serde_json::to_value(T::default()).map_err(|error| merge_error(&error))?;
    if let Some(configured) = namespaces.get(name) {
        overlay(&mut merged, configured.clone());
    }
    let cli_values = serde_json::to_value(cli).map_err(|error| merge_error(&error))?;
    overlay(&mut merged, cli_values);
    serde_json::from_value(merged).map_err(|error| merge_error(&error))
}

/// Pair a parsed command with its already-resolved `OrthoConfig` namespaces.
pub(crate) struct ResolvedCommand {
    /// Parsed application command.
    pub command: Commands,
    /// Global settings containing file and environment namespaces.
    pub settings: GlobalOptions,
}

impl SelectedSubcommandMerge for ResolvedCommand {
    fn load_and_merge_selected(
        self,
        _matches: &clap::ArgMatches,
    ) -> Result<Self, SelectedSubcommandMergeError> {
        self.merge_selected()
    }

    fn load_and_merge_selected_with_sources(
        self,
        _matches: &clap::ArgMatches,
        _files: SubcommandFileContext<'_>,
        _merge_source: SharedScanEnvSource,
    ) -> Result<Self, SelectedSubcommandMergeError> {
        // GlobalOptions already resolved the one authoritative file chain.
        // Re-discovering a prefixed dotfile here would split global and
        // command-specific policy across different files.
        self.merge_selected()
    }
}

impl ResolvedCommand {
    /// Merge only the namespace selected by the parsed command.
    fn merge_selected(mut self) -> Result<Self, SelectedSubcommandMergeError> {
        self.command = match self.command {
            Commands::Context(mut args) => {
                let json = args.json;
                args = merge_namespace(&self.settings.cmds, "context", args)?;
                args.json = json;
                Commands::Context(args)
            }
        };
        Ok(self)
    }
}

/// Source bundle used by the selected-command helper.
pub(crate) fn selected_sources<'a>(
    root: &'a Path,
    discovery: &'a dyn EnvSource,
    merge: SharedScanEnvSource,
) -> ortho_config::SelectedSubcommandSources<'a> {
    ortho_config::SelectedSubcommandSources::new(SubcommandFileContext::new(root, discovery), merge)
}

/// Return process-backed sources for the executable boundary.
pub(crate) fn process_sources() -> (SharedEnvSource, SharedScanEnvSource) {
    (
        ortho_config::process_env_source(),
        ortho_config::process_scan_env_source(),
    )
}

/// Convert an application policy into its resolved defaulted value.
fn policy(name: &str, value: Option<&str>) -> OrthoResult<String> {
    let selected = value.unwrap_or("auto");
    match selected {
        "auto" | "always" | "never" => Ok(String::from(selected)),
        _ => Err(OrthoError::Validation {
            key: name.to_owned(),
            message: format!("expected `auto`, `always`, or `never`; received `{selected}`"),
        }
        .into()),
    }
}

/// Validate the separate `auto|on|off` accessibility vocabulary.
fn accessibility_policy(value: Option<&str>) -> OrthoResult<String> {
    let selected = value.unwrap_or("auto");
    match selected {
        "auto" | "on" | "off" => Ok(String::from(selected)),
        _ => Err(OrthoError::Validation {
            key: String::from("accessibility"),
            message: format!("expected `auto`, `on`, or `off`; received `{selected}`"),
        }
        .into()),
    }
}

/// Recursively overlay object members while treating `null` as omission.
fn overlay(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target_fields), Value::Object(source_fields)) => {
            for (key, source_value) in source_fields {
                if source_value.is_null() {
                    continue;
                }
                match target_fields.get_mut(&key) {
                    Some(existing) => overlay(existing, source_value),
                    None => {
                        target_fields.insert(key, source_value);
                    }
                }
            }
        }
        (target_value, source_value) => *target_value = source_value,
    }
}

/// Convert a serialization or deserialization failure into `OrthoConfig`'s
/// structured merge error so the process boundary uses exit class 3.
fn merge_error(error: &serde_json::Error) -> SelectedSubcommandMergeError {
    SelectedSubcommandMergeError::Merge(Arc::new(OrthoError::Validation {
        key: String::from("selected command"),
        message: error.to_string(),
    }))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
