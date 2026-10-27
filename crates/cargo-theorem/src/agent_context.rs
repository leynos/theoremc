//! Build the compact agent context from the declared command metadata.

use cargo_orthohelp::{
    agent_context::bridge_ir_to_agent_context, schema::DocMetadata as BridgeDocMetadata,
};
use ortho_config::OrthoConfigSubcommandDocs;
use ortho_config::docs::OrthoConfigDocs;

use crate::{cli::Commands, config::GlobalOptions};

#[cfg(test)]
use ortho_config::docs::DocMetadata;

/// Serialize metadata for the commands implemented by this foundation.
///
/// The context is generated from the same command payloads used by Clap, so
/// future command slices can add their paths without maintaining another
/// schema by hand.
pub(crate) fn to_json() -> Result<String, serde_json::Error> {
    let mut metadata = GlobalOptions::get_doc_metadata();
    metadata.app_name = String::from("cargo-theorem");
    metadata.bin_name = Some(String::from("cargo-theorem"));
    metadata.subcommands = Commands::get_subcommand_doc_metadata();
    let bridge_metadata = bridge_metadata(&metadata)?;
    let context = bridge_ir_to_agent_context(&bridge_metadata, "cargo-theorem", None);
    serde_json::to_string(&context)
}

/// Convert `OrthoConfig`'s documentation IR to cargo-orthohelp's public schema
/// mirror while preserving the shared serialized IR contract.
fn bridge_metadata(
    metadata: &ortho_config::docs::DocMetadata,
) -> Result<BridgeDocMetadata, serde_json::Error> {
    serde_json::from_value(serde_json::to_value(metadata)?)
}

/// Return metadata for focused assertions without converting the schema.
#[cfg(test)]
fn command_metadata() -> DocMetadata {
    let mut metadata = GlobalOptions::get_doc_metadata();
    metadata.subcommands = Commands::get_subcommand_doc_metadata();
    metadata
}

#[cfg(test)]
mod tests {
    //! Checks that agent context exposes only registered foundation commands.

    use super::command_metadata;
    use anyhow::{Result, ensure};

    #[test]
    fn metadata_contains_context_and_no_planned_handlers() -> Result<()> {
        let metadata = command_metadata();
        let names = metadata
            .subcommands
            .iter()
            .map(|command| command.app_name.as_str())
            .collect::<Vec<_>>();
        ensure!(
            names == ["context"],
            "unexpected foundation commands: {names:?}"
        );
        let context = metadata
            .subcommands
            .first()
            .ok_or_else(|| anyhow::anyhow!("context metadata is missing"))?;
        ensure!(
            context.behaviour.is_some(),
            "the read-only context command must declare behaviour metadata"
        );
        Ok(())
    }

    #[test]
    fn generated_context_uses_the_registered_command_surface() -> Result<()> {
        let context: ortho_config::AgentContext = serde_json::from_str(&super::to_json()?)?;
        let paths = context
            .commands
            .iter()
            .map(|command| command.path.as_slice())
            .collect::<Vec<_>>();
        ensure!(
            paths
                == [
                    vec![String::from("cargo-theorem")],
                    vec![String::from("cargo-theorem"), String::from("context")]
                ],
            "unexpected agent-context commands: {paths:?}"
        );
        Ok(())
    }
}
