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

impl From<ortho_config::SelectedSubcommandMergeError> for ApplicationError {
    fn from(error: ortho_config::SelectedSubcommandMergeError) -> Self {
        match error {
            ortho_config::SelectedSubcommandMergeError::Merge(inner) => Self::Configuration(inner),
            other => Self::Configuration(Arc::new(ortho_config::OrthoError::Validation {
                key: String::from("selected command"),
                message: other.to_string(),
            })),
        }
    }
}

impl From<ortho_config::LoadGlobalsAndSelectedSubcommandError<std::convert::Infallible>>
    for ApplicationError
{
    fn from(
        error: ortho_config::LoadGlobalsAndSelectedSubcommandError<std::convert::Infallible>,
    ) -> Self {
        match error {
            ortho_config::LoadGlobalsAndSelectedSubcommandError::Globals(never) => match never {},
            ortho_config::LoadGlobalsAndSelectedSubcommandError::Subcommand(merge_error) => {
                Self::from(merge_error)
            }
            other => Self::Configuration(Arc::new(ortho_config::OrthoError::Validation {
                key: String::from("selected command"),
                message: other.to_string(),
            })),
        }
    }
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

#[cfg(test)]
mod tests {
    //! Tests for the application's stable process exit classes.

    use std::{io, process::ExitCode, sync::Arc};

    use super::ApplicationError;

    #[test]
    fn application_errors_keep_their_stable_exit_classes() {
        let configuration =
            ApplicationError::Configuration(Arc::new(ortho_config::OrthoError::Validation {
                key: String::from("color"),
                message: String::from("invalid value"),
            }));
        let serialization = ApplicationError::Serialization(
            serde_json::from_str::<serde_json::Value>("{")
                .expect_err("incomplete JSON must fail to parse"),
        );
        let output = ApplicationError::Output(io::Error::other("closed stream"));

        assert_eq!(configuration.exit_code(), ExitCode::from(3));
        assert_eq!(serialization.exit_code(), ExitCode::from(10));
        assert_eq!(output.exit_code(), ExitCode::from(10));
    }

    #[test]
    fn selected_subcommand_merge_keeps_the_original_configuration_error() {
        let original = Arc::new(ortho_config::OrthoError::Validation {
            key: String::from("cmds.context"),
            message: String::from("invalid context setting"),
        });
        let expected = original.to_string();
        let error = ApplicationError::from(ortho_config::SelectedSubcommandMergeError::Merge(
            Arc::clone(&original),
        ));

        assert_eq!(error.to_string(), expected);
    }
}
