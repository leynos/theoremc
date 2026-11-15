//! `OrthoConfig` settings, validation, and selected-command resolution.

// The `OrthoConfig` derive expands to a process-environment read, and Clippy
// reports that read at the derive on `GlobalOptions`. An expectation on the
// struct does not reach the derive expansion, so the struct sits alone in an
// inline-scoped module and the expectation is declared on that module item.
// Nothing else in this file is covered, so a direct environment read added
// here stays denied. Callers that need isolation inject sources through
// `GlobalOptions::load_with_sources` instead of relying on the derive's read.
#[expect(
    clippy::disallowed_methods,
    reason = "the OrthoConfig derive expands to a process-environment read; callers that need isolation inject sources through load_with_sources"
)]
mod global_options;

pub use global_options::GlobalOptions;

use std::{collections::BTreeMap, ffi::OsString, path::Path, sync::Arc};

use ortho_config::{
    EnvSource, OrthoError, OrthoResult, ProfileLoadOutcome, SelectedSubcommandMerge,
    SelectedSubcommandMergeError, SharedEnvSource, SharedScanEnvSource, SubcommandFileContext,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::cli::Commands;

/// Consume `OrthoConfig`'s internal config selector without treating it as a
/// persisted setting.
///
/// It lives here, outside the module that carries the derive's lint
/// expectation, because it reads nothing from the environment.
fn ignore_config_path<'de, D>(deserializer: D) -> Result<Option<std::path::PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let _ignored = serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(None)
}

// Keep namespaces for deferred top-level commands so documented future
// settings remain usable, while rejecting misspellings before dispatch. This
// name-only allowlist does not define parser arguments or command metadata.
/// Command names accepted in configuration even when not yet implemented.
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
/// Accepted values for policy-style settings.
const POLICY_VALUES: &[&str] = &["auto", "always", "never"];
/// Accepted values for accessibility settings.
const ACCESSIBILITY_POLICY_VALUES: &[&str] = &["auto", "on", "off"];

/// Effective values after the application-owned policy defaults are applied.
///
/// Policy fields are concrete because their vocabulary has an application
/// default. The switches stay optional because the renderer owns their defaults
/// and neither a file nor the command line need select them.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Whether quiet mode was selected.
    pub quiet: Option<bool>,
    /// Whether verbose mode was selected.
    pub verbose: Option<bool>,
    /// Whether interactive prompts were disabled.
    pub no_input: Option<bool>,
    /// Whether decorative human-renderer output was disabled.
    pub plain: Option<bool>,
    /// Whether paging was disabled.
    pub no_pager: Option<bool>,
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
    validate_policy(name, value, POLICY_VALUES, "`auto`, `always`, or `never`")
}

/// Validate the separate `auto|on|off` accessibility vocabulary.
fn accessibility_policy(value: Option<&str>) -> OrthoResult<String> {
    validate_policy(
        "accessibility",
        value,
        ACCESSIBILITY_POLICY_VALUES,
        "`auto`, `on`, or `off`",
    )
}

/// Validate one named policy while preserving its caller-facing vocabulary.
fn validate_policy(
    name: &str,
    value: Option<&str>,
    allowed: &[&str],
    expected: &str,
) -> OrthoResult<String> {
    let selected = value.unwrap_or("auto");
    if allowed.contains(&selected) {
        Ok(String::from(selected))
    } else {
        Err(OrthoError::Validation {
            key: name.to_owned(),
            message: format!("expected {expected}; received `{selected}`"),
        }
        .into())
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
#[path = "../config_tests.rs"]
mod tests;
