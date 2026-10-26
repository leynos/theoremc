//! Source-injection tests for global and per-command precedence.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Result, ensure};
use cap_std::{ambient_authority, fs::Dir};
use ortho_config::MapEnv;
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

#[rstest]
#[case::file_over_default("color = \"never\"\n", &[], None, "never")]
#[case::profile_over_file(
    "color = \"always\"\n[profile.ci]\ncolor = \"never\"\n",
    &["--profile", "ci"],
    None,
    "never"
)]
#[case::profile_and_environment(
        "color = \"always\"\n[profile.ci]\ncolor = \"never\"\n",
        &["--profile", "ci"],
        Some("always"),
        "always"
    )]
#[case::cli_wins(
        "color = \"never\"\n",
        &["--color", "auto"],
        Some("always"),
        "auto"
    )]
fn global_values_follow_defaults_files_profiles_environment_and_cli(
    #[case] contents: &str,
    #[case] arguments: &[&str],
    #[case] environment_color: Option<&str>,
    #[case] expected: &str,
) -> Result<()> {
    let (_directory, path) = temp_config(contents)?;
    let mut environment = MapEnv::new().with_var("THEOREMC_CONFIG_PATH", path.as_path());
    if let Some(color) = environment_color {
        environment = environment.with_var("THEOREMC_COLOR", color);
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
        "expected color {expected}"
    );
    Ok(())
}

#[test]
fn global_defaults_are_used_when_sources_omit_a_value() -> Result<()> {
    let source = Arc::new(MapEnv::new());
    let loaded = GlobalOptions::load_with_sources(["cargo-theorem"], source.clone(), source)?;
    ensure!(
        loaded.config().resolve()?.color == "auto",
        "omitted color must resolve to auto"
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
