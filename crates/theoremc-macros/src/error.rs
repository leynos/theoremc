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
    /// The crate root cannot be resolved because `CARGO_MANIFEST_DIR` cannot
    /// be read (it is unset or not valid Unicode).
    #[error("`CARGO_MANIFEST_DIR` is not set during theorem macro expansion")]
    MissingManifestDir,
    /// A theorem omits the Kani evidence the harness needs.
    #[error("theorem `{theorem}` does not declare required `Evidence.kani` configuration")]
    MissingKaniEvidence {
        /// Name of the offending theorem.
        theorem: String,
    },
    /// A referenced action has no entry in the `Actions` table.
    #[error("referenced action `{action}` is missing an Actions signature entry")]
    MissingActionSignature {
        /// Canonical name of the action without a signature.
        action: String,
    },
    /// Two theorems declare one action with inequivalent signatures.
    #[error(
        "referenced action `{action}` has conflicting Actions signatures: theorem `{first_theorem}` declares `{first_signature}`, but theorem `{conflicting_theorem}` declares `{conflicting_signature}`"
    )]
    ConflictingActionSignature {
        /// Canonical name of the action.
        action: String,
        /// Theorem whose declaration was seen first.
        first_theorem: String,
        /// Theorem whose declaration conflicts with the first.
        conflicting_theorem: String,
        /// Rendered signature from the first theorem.
        first_signature: String,
        /// Rendered signature from the conflicting theorem.
        conflicting_signature: String,
    },
    /// An action signature cannot be rendered as Rust types.
    #[error("referenced action `{action}` has an invalid Actions signature: {message}")]
    InvalidActionSignature {
        /// Canonical name of the action.
        action: String,
        /// Parser explanation of what is invalid.
        message: String,
    },
    /// A referenced type is not valid Rust syntax.
    #[error("referenced type `{ty}` is invalid: {message}")]
    InvalidReferencedType {
        /// The type text as written in the theorem.
        ty: String,
        /// Parser explanation of what is invalid.
        message: String,
    },
    /// The theorem file failed to load; carries the rendered diagnostic.
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
