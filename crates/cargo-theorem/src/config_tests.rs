//! Source-injection tests for global and per-command precedence.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Result, ensure};
use cap_std::{ambient_authority, fs::Dir};
use ortho_config::MapEnv;
use ortho_config::SelectedSubcommandMerge;
use rstest::rstest;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{GlobalOptions, merge_namespace};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CommandSettings {
    level: Option<u8>,
}

fn temp_config(contents: &str) -> Result<(tempfile::TempDir, PathBuf)> {
    let directory = tempfile::tempdir()?;
    let capability = Dir::open_ambient_dir(directory.path(), ambient_authority())?;
    capability.write("settings.toml", contents.as_bytes())?;
    let path = directory.path().join("settings.toml");
    Ok((directory, path))
}

/// One row of the global colour-precedence cases.
///
/// The inputs travel together in every case, so naming them keeps the case
/// list from becoming a row of interchangeable positional arguments.
struct GlobalCase {
    contents: &'static str,
    arguments: &'static [&'static str],
    environment_colour: Option<&'static str>,
    expected: &'static str,
}

#[rstest]
#[case::file_over_default(GlobalCase {
    contents: "color = \"never\"\n",
    arguments: &[],
    environment_colour: None,
    expected: "never",
})]
#[case::profile_over_file(GlobalCase {
    contents: "color = \"always\"\n[profile.ci]\ncolor = \"never\"\n",
    arguments: &["--profile", "ci"],
    environment_colour: None,
    expected: "never",
})]
#[case::profile_and_environment(GlobalCase {
    contents: "color = \"always\"\n[profile.ci]\ncolor = \"never\"\n",
    arguments: &["--profile", "ci"],
    environment_colour: Some("always"),
    expected: "always",
})]
#[case::cli_wins(GlobalCase {
    contents: "color = \"never\"\n",
    arguments: &["--color", "auto"],
    environment_colour: Some("always"),
    expected: "auto",
})]
fn global_values_follow_defaults_files_profiles_environment_and_cli(
    #[case] case: GlobalCase,
) -> Result<()> {
    let GlobalCase {
        contents,
        arguments,
        environment_colour,
        expected,
    } = case;
    let (_directory, path) = temp_config(contents)?;
    let mut environment = MapEnv::new().with_var("THEOREMC_CONFIG_PATH", path.as_path());
    if let Some(colour) = environment_colour {
        environment = environment.with_var("THEOREMC_COLOR", colour);
    }
    let source = Arc::new(environment);
    let mut args = vec![
        "cargo-theorem",
        "--config",
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
    ];
    args.extend_from_slice(arguments);
    let loaded = GlobalOptions::load_with_sources(args, source.clone(), source.clone())?;
    ensure!(
        loaded.config().resolve()?.color == expected,
        "expected colour {expected}"
    );
    Ok(())
}

/// One row of the global-switch precedence cases.
struct SwitchCase {
    contents: &'static str,
    arguments: &'static [&'static str],
    expected: Option<bool>,
}

#[rstest]
#[case::absent_everywhere(SwitchCase {
    contents: "",
    arguments: &[],
    expected: None,
})]
#[case::file_value_survives_absent_cli(SwitchCase {
    contents: "quiet = true\n",
    arguments: &[],
    expected: Some(true),
})]
#[case::cli_flag_overrides_file(SwitchCase {
    contents: "quiet = false\n",
    arguments: &["--quiet"],
    expected: Some(true),
})]
fn global_switches_preserve_absence_and_cli_precedence(#[case] case: SwitchCase) -> Result<()> {
    let SwitchCase {
        contents,
        arguments,
        expected,
    } = case;
    let (_directory, path) = temp_config(contents)?;
    let source = Arc::new(MapEnv::new());
    let mut args = vec![
        "cargo-theorem",
        "--config",
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
    ];
    args.extend_from_slice(arguments);
    let loaded = GlobalOptions::load_with_sources(args, source.clone(), source.clone())?;
    let resolved = loaded.config().resolve()?;
    ensure!(
        resolved.quiet == expected,
        "expected quiet {expected:?}, got {:?}",
        resolved.quiet
    );
    Ok(())
}

#[test]
fn global_defaults_are_used_when_sources_omit_a_value() -> Result<()> {
    let source = Arc::new(MapEnv::new());
    let loaded = GlobalOptions::load_with_sources(["cargo-theorem"], source.clone(), source)?;
    ensure!(
        loaded.config().resolve()?.color == "auto",
        "omitted colour must resolve to auto"
    );
    Ok(())
}

#[test]
fn the_internal_config_path_is_consumed_without_becoming_a_setting() -> Result<()> {
    let options: GlobalOptions = serde_json::from_str(r#"{"config_path":"ignored.toml"}"#)?;
    ensure!(
        options.config_path.is_none(),
        "the loader's internal config selector must not become persisted state"
    );
    Ok(())
}

#[rstest]
#[case(None, "auto")]
#[case(Some("on"), "on")]
#[case(Some("off"), "off")]
fn accessibility_policy_uses_its_own_vocabulary(
    #[case] configured: Option<&str>,
    #[case] expected: &str,
) -> Result<()> {
    let options = GlobalOptions {
        accessibility: configured.map(String::from),
        ..GlobalOptions::default()
    };
    ensure!(
        options.resolve()?.accessibility == expected,
        "expected accessibility policy {expected}"
    );
    Ok(())
}

#[test]
fn selected_command_adapter_merges_its_command_namespace() -> Result<()> {
    let namespaces = [(String::from("context"), json!({}))].into_iter().collect();
    let command = crate::cli::Commands::Context(crate::cli::ContextArgs::default());
    let resolved = super::ResolvedCommand {
        command,
        settings: GlobalOptions {
            cmds: namespaces,
            ..GlobalOptions::default()
        },
    };
    let matches = clap::Command::new("unused").try_get_matches_from(["unused"])?;
    let merged = resolved.load_and_merge_selected(&matches)?;
    ensure!(
        matches!(merged.command, crate::cli::Commands::Context(_)),
        "the context command must remain selected after merging"
    );
    Ok(())
}

#[test]
fn selected_namespace_overlays_cli_without_leaking_sibling_values() -> Result<()> {
    let namespaces = [
        (String::from("context"), json!({ "level": 4 })),
        (String::from("run"), json!({ "level": 9 })),
    ]
    .into_iter()
    .collect();
    let context = merge_namespace(&namespaces, "context", CommandSettings { level: Some(7) })?;
    ensure!(context.level == Some(7), "CLI context value must win");
    let defaulted = merge_namespace(&namespaces, "context", CommandSettings::default())?;
    ensure!(
        defaulted.level == Some(4),
        "context file value must be kept"
    );
    Ok(())
}

#[test]
fn selected_namespace_obeys_file_environment_and_cli_precedence() -> Result<()> {
    let (_directory, path) = temp_config("[cmds.context]\nlevel = 4\n[cmds.run]\nlevel = 8\n")?;
    let source = Arc::new(
        MapEnv::new()
            .with_var("THEOREMC_CONFIG_PATH", path.as_path())
            .with_var("THEOREMC_CMDS__CONTEXT__LEVEL", "6"),
    );
    let loaded = GlobalOptions::load_with_sources(
        [
            "cargo-theorem",
            "--config",
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
        ],
        source.clone(),
        source,
    )?;
    let _resolved_settings = loaded.config().resolve()?;
    let file_and_environment =
        merge_namespace(&loaded.config().cmds, "context", CommandSettings::default())?;
    ensure!(
        file_and_environment.level == Some(6),
        "environment value must override the file"
    );
    let command_line = merge_namespace(
        &loaded.config().cmds,
        "context",
        CommandSettings { level: Some(7) },
    )?;
    ensure!(
        command_line.level == Some(7),
        "CLI value must override the environment"
    );
    Ok(())
}

#[test]
fn selected_namespace_rejects_unknown_keys() {
    let namespaces = [(String::from("context"), json!({ "unknown": true }))]
        .into_iter()
        .collect();
    let result = merge_namespace(&namespaces, "context", CommandSettings::default());
    assert!(result.is_err(), "unknown command settings must be rejected");
}

#[rstest]
#[case("color = \"sepia\"\n", "expected `auto`, `always`, or `never`")]
#[case("unknown = true\n", "unknown field")]
#[case("[cmds.nonexistent]\nlevel = 4\n", "unknown command namespace")]
#[case(
    "[profile.ci]\ncmds = {}\n",
    "must not define the forbidden key 'cmds'"
)]
fn invalid_configuration_is_rejected(
    #[case] contents: &str,
    #[case] expected_message: &str,
) -> Result<()> {
    let (_directory, path) = temp_config(contents)?;
    let source = Arc::new(MapEnv::new());
    let args = [
        "cargo-theorem",
        "--config",
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
    ];
    let loaded = GlobalOptions::load_with_sources(args, source.clone(), source);
    match loaded {
        Ok(config) => {
            let resolved = config.config().resolve();
            ensure!(
                matches!(resolved, Err(error) if error.to_string().contains(expected_message)),
                "expected validation message {expected_message}"
            );
        }
        Err(error) => ensure!(
            error.to_string().contains(expected_message),
            "unexpected config error: {error}"
        ),
    }
    Ok(())
}

#[test]
fn explicit_missing_file_is_a_configuration_error() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("missing.toml");
    let source = Arc::new(MapEnv::new());
    let loaded = GlobalOptions::load_with_sources(
        [
            "cargo-theorem",
            "--config",
            path.to_str()
                .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
        ],
        source.clone(),
        source,
    );
    ensure!(loaded.is_err(), "an explicit missing file must fail");
    Ok(())
}

#[test]
fn explicit_directory_is_reported_as_an_unreadable_configuration_file() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let source = Arc::new(MapEnv::new());
    let loaded = GlobalOptions::load_with_sources(
        [
            "cargo-theorem",
            "--config",
            directory
                .path()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("config path is not UTF-8"))?,
        ],
        source.clone(),
        source,
    );
    ensure!(loaded.is_err(), "an explicit directory must fail to load");
    Ok(())
}
