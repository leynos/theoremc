//! Proc-macro expansion for compile-time theorem integration.

use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir as Utf8Dir};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{LitStr, parse_macro_input};
use theoremc_core::{
    mangle::{mangle_module_path, mangle_theorem_harness},
    schema::{SchemaError, SourceId, load_theorem_docs_with_source},
};

/// Expands a crate-relative `.theorem` file into a stable private module.
#[proc_macro]
pub fn theorem_file(input: TokenStream) -> TokenStream {
    let path_literal = parse_macro_input!(input as LitStr);
    match expand_theorem_file(&path_literal) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error(path_literal.span()).into(),
    }
}

fn expand_theorem_file(path_literal: &LitStr) -> Result<TokenStream2, MacroExpansionError> {
    let manifest_dir = manifest_dir_from_env()?;
    expand_theorem_file_at(&manifest_dir, path_literal)
}

fn manifest_dir_from_env() -> Result<Utf8PathBuf, MacroExpansionError> {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").map_err(|_| MacroExpansionError::MissingManifestDir)?;
    Utf8PathBuf::from_path_buf(manifest_dir.into())
        .map_err(|path| MacroExpansionError::NonUtf8ManifestDir(path.to_string_lossy().into()))
}

fn expand_theorem_file_at(
    manifest_dir: &Utf8Path,
    path_literal: &LitStr,
) -> Result<TokenStream2, MacroExpansionError> {
    let theorem_path = path_literal.value();
    let theorem_source = read_theorem_source(manifest_dir, &theorem_path)?;
    let source_id = SourceId::new(&theorem_path);
    let theorem_docs = load_theorem_docs_with_source(&source_id, &theorem_source)
        .map_err(|error| MacroExpansionError::from_schema(theorem_path.clone(), &error))?;

    Ok(render_expansion(path_literal, &theorem_path, &theorem_docs))
}

fn read_theorem_source(
    manifest_dir: &Utf8Path,
    theorem_path: &str,
) -> Result<String, MacroExpansionError> {
    let manifest_root =
        Utf8Dir::open_ambient_dir(manifest_dir, ambient_authority()).map_err(|source| {
            MacroExpansionError::OpenManifestDir {
                path: manifest_dir.to_path_buf(),
                source,
            }
        })?;
    manifest_root
        .read_to_string(theorem_path)
        .map_err(|source| MacroExpansionError::ReadTheoremFile {
            path: theorem_path.to_owned(),
            source,
        })
}

fn render_expansion(
    path_literal: &LitStr,
    theorem_path: &str,
    theorem_docs: &[theoremc_core::schema::TheoremDoc],
) -> TokenStream2 {
    let module_ident = identifier(mangle_module_path(theorem_path).module_name());
    let harness_idents: Vec<Ident> = theorem_docs
        .iter()
        .map(|doc| {
            identifier(mangle_theorem_harness(theorem_path, doc.theorem.as_str()).identifier())
        })
        .collect();
    let harness_count = syn::LitInt::new(&harness_idents.len().to_string(), Span::call_site());

    quote! {
        mod #module_ident {
            const _: &str =
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #path_literal));

            pub(super) mod kani {
                #(pub(crate) fn #harness_idents() {})*
            }

            const _: [fn(); #harness_count] = [#(kani::#harness_idents),*];
        }
    }
}

fn identifier(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

#[derive(Debug, thiserror::Error)]
enum MacroExpansionError {
    #[error("`CARGO_MANIFEST_DIR` is not set during theorem macro expansion")]
    MissingManifestDir,
    #[error("`CARGO_MANIFEST_DIR` is not valid UTF-8: {0}")]
    NonUtf8ManifestDir(String),
    #[error("failed to open manifest directory '{path}': {source}")]
    OpenManifestDir {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read theorem file '{path}': {source}")]
    ReadTheoremFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to load theorem file '{path}': {message}")]
    InvalidTheoremFile { path: String, message: String },
}

impl MacroExpansionError {
    fn to_compile_error(&self, span: Span) -> TokenStream2 {
        syn::Error::new(span, self.to_string()).to_compile_error()
    }
}

impl MacroExpansionError {
    fn from_schema(path: String, error: &SchemaError) -> Self {
        let message = error.diagnostic().map_or_else(
            || error.to_string(),
            theoremc_core::schema::SchemaDiagnostic::render,
        );
        Self::InvalidTheoremFile { path, message }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
