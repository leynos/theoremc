//! Build the compact agent context from the declared command metadata.

use crate::cli::command_metadata;
use cargo_orthohelp::{
    agent_context::bridge_ir_to_agent_context, schema::DocMetadata as BridgeDocMetadata,
};

/// Serialize metadata for the commands implemented by this foundation.
///
/// The context is generated from the same command payloads used by Clap, so
/// future command slices can add their paths without maintaining another
/// schema by hand.
pub(crate) fn to_json() -> Result<String, serde_json::Error> {
    let metadata = command_metadata();
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

#[cfg(test)]
mod tests {
    //! Checks that agent context exposes only registered foundation commands.

    use super::command_metadata;
    use anyhow::{Result, ensure};

    #[test]
    fn metadata_contains_context_and_no_planned_handlers() -> Result<()> {
        let metadata = command_metadata();
        ensure!(
            metadata.fields.is_empty(),
            "binary root must not expose theorem options"
        );
        let [theorem] = metadata.subcommands.as_slice() else {
            anyhow::bail!("binary must expose only the theorem wrapper");
        };
        ensure!(
            theorem.app_name == "theorem",
            "unexpected Cargo wrapper name"
        );
        let [context] = theorem.subcommands.as_slice() else {
            anyhow::bail!("theorem must expose only the context command");
        };
        ensure!(
            context.app_name == "context",
            "unexpected foundation command"
        );
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
                    vec![String::from("cargo-theorem"), String::from("theorem")],
                    vec![
                        String::from("cargo-theorem"),
                        String::from("theorem"),
                        String::from("context")
                    ]
                ],
            "unexpected agent-context commands: {paths:?}"
        );
        Ok(())
    }
}
