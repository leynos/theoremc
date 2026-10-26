//! Application errors and their stable process exit classes.

use std::{io, process::ExitCode, sync::Arc};

use thiserror::Error;

/// Failure while resolving configuration or emitting application output.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Configuration could not be discovered, parsed, or validated.
    #[error("{0}")]
    Configuration(#[from] Arc<ortho_config::OrthoError>),
    /// Generated machine-readable output could not be encoded.
    #[error("could not encode agent context: {0}")]
    Serialization(#[from] serde_json::Error),
    /// The selected output stream rejected a write.
    #[error("could not write command output: {0}")]
    Output(#[from] io::Error),
}

impl ApplicationError {
    /// Return the stable application exit class for this failure.
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Configuration(_) => ExitCode::from(3),
            Self::Serialization(_) | Self::Output(_) => ExitCode::from(10),
        }
    }
}
