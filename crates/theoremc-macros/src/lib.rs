//! Proc-macro expansion for compile-time theorem integration.

use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{LitStr, parse_macro_input};
use theoremc_core::{
    TheoremFileLoadError, load_theorem_file_from_manifest_dir,
    mangle::{mangle_module_path, mangle_theorem_harness},
    schema::SchemaDiagnostic,
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
    Ok(Utf8PathBuf::from(manifest_dir))
}

fn expand_theorem_file_at(
    manifest_dir: &Utf8Path,
    path_literal: &LitStr,
) -> Result<TokenStream2, MacroExpansionError> {
    let theorem_path = Utf8PathBuf::from(path_literal.value());
    let theorem_docs = load_theorem_file_from_manifest_dir(manifest_dir, &theorem_path)
        .map_err(|error| MacroExpansionError::from_load(&error))?;

    Ok(render_expansion(
        path_literal,
        theorem_path.as_str(),
        &theorem_docs,
    ))
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
    #[error("{0}")]
    LoadTheoremFile(String),
}

impl MacroExpansionError {
    fn to_compile_error(&self, span: Span) -> TokenStream2 {
        syn::Error::new(span, self.to_string()).to_compile_error()
    }

    fn from_load(error: &TheoremFileLoadError) -> Self {
        let message = match &error {
            TheoremFileLoadError::InvalidTheoremFile { source, .. } => source
                .diagnostic()
                .map_or_else(|| error.to_string(), SchemaDiagnostic::render),
            _ => error.to_string(),
        };
        Self::LoadTheoremFile(message)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
