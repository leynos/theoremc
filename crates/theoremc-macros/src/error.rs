//! Macro expansion errors and their compiler diagnostics.
//!
//! Every expansion failure is converted into a `compile_error!` invocation at
//! the macro call site. The messages deliberately stay short enough to read in
//! a compiler diagnostic while still identifying the theorem, action, or type
//! that failed.

use proc_macro2::{Span, TokenStream as TokenStream2};
use theoremc_core::{TheoremFileLoadError, schema::SchemaDiagnostic};

/// Failures raised while planning or rendering a theorem macro expansion.
#[derive(Debug, thiserror::Error)]
pub(super) enum MacroExpansionError {
    #[error("`CARGO_MANIFEST_DIR` is not set during theorem macro expansion")]
    MissingManifestDir,
    #[error("theorem `{theorem}` does not declare required `Evidence.kani` configuration")]
    MissingKaniEvidence { theorem: String },
    #[error("referenced action `{action}` is missing an Actions signature entry")]
    MissingActionSignature { action: String },
    #[error(
        "referenced action `{action}` has conflicting Actions signatures: theorem `{first_theorem}` declares `{first_signature}`, but theorem `{conflicting_theorem}` declares `{conflicting_signature}`"
    )]
    ConflictingActionSignature {
        action: String,
        first_theorem: String,
        conflicting_theorem: String,
        first_signature: String,
        conflicting_signature: String,
    },
    #[error("referenced action `{action}` has an invalid Actions signature: {message}")]
    InvalidActionSignature { action: String, message: String },
    #[error("referenced type `{ty}` is invalid: {message}")]
    InvalidReferencedType { ty: String, message: String },
    #[error("{0}")]
    LoadTheoremFile(String),
}

impl MacroExpansionError {
    /// Renders this failure as a `compile_error!` at `span`.
    pub(super) fn to_compile_error(&self, span: Span) -> TokenStream2 {
        syn::Error::new(span, self.to_string()).to_compile_error()
    }

    /// Converts a theorem-file load failure, preferring its structured
    /// diagnostic so the macro reports the same source location as the loader.
    pub(super) fn from_load(error: &TheoremFileLoadError) -> Self {
        let message = match &error {
            TheoremFileLoadError::InvalidTheoremFile { source, .. } => source
                .diagnostic()
                .map_or_else(|| error.to_string(), SchemaDiagnostic::render),
            _ => error.to_string(),
        };
        Self::LoadTheoremFile(message)
    }
}
