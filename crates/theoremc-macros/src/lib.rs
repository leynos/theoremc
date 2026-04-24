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
///
/// # Input
///
/// The macro accepts a single string-literal argument: a path to a `.theorem`
/// file relative to the consuming crate's manifest directory
/// (`CARGO_MANIFEST_DIR`). The path must be relative, must not begin with a
/// drive prefix (e.g. `C:`), and must not contain `..` components.
///
/// ```ignore
/// theorem_file!("theorems/my_theorem.theorem");
/// ```
///
/// # Generated output
///
/// Each invocation emits a deterministic, private module whose name is derived
/// from the theorem file path via [`theoremc_core::mangle::mangle_module_path`].
/// Inside that module:
///
/// - A `const _: &str = include_str!(…)` anchors the theorem source to
///   `CARGO_MANIFEST_DIR` so the file is tracked as a compile-time dependency.
/// - A `pub(super) mod kani` sub-module contains one zero-sized
///   `pub(crate) fn` per theorem document, named via
///   [`theoremc_core::mangle::mangle_theorem_harness`].
/// - A const array of `fn()` pointers sized to the harness count anchors all
///   generated symbols.
///
/// Document order is preserved: the first theorem document in the file
/// produces the first harness function.
///
/// # Errors
///
/// All failures are reported as `compile_error!` at the macro call site:
///
/// | Cause | Diagnostic |
/// | --- | --- |
/// | `CARGO_MANIFEST_DIR` is not set | macro configuration error message |
/// | Path is absolute or contains `..` or a drive prefix | `InvalidTheoremPath` message |
/// | Theorem file cannot be read | `ReadTheoremFile` message with IO code |
/// | File contains no theorem documents | `EmptyTheoremFile` message |
/// | Schema parsing or validation fails | rendered `SchemaDiagnostic` (includes source location) |
///
/// # Panics
///
/// This macro does not panic. All error conditions are converted to
/// `compile_error!` invocations so failures surface as ordinary Rust compiler
/// diagnostics.
///
/// # Example
///
/// Given a file `theorems/my_theorem.theorem` containing:
///
/// ```text
/// Schema: 1
/// Theorem: MyLemma
/// About: A simple proof obligation
/// Prove:
///   - assert: "true"
///     because: trivial
/// Evidence:
///   kani:
///     unwind: 1
///     expect: SUCCESS
/// ```
///
/// The invocation:
///
/// ```ignore
/// theorem_file!("theorems/my_theorem.theorem");
/// ```
///
/// expands to (approximately):
///
/// ```ignore
/// mod theorems__my_theorem__h<hash> {
///     const _: &str = include_str!(
///         concat!(env!("CARGO_MANIFEST_DIR"), "/", "theorems/my_theorem.theorem")
///     );
///     pub(super) mod kani {
///         pub(crate) fn theorem__my_lemma__h<hash>() {}
///     }
///     const _: [fn(); 1] = [kani::theorem__my_lemma__h<hash>];
/// }
/// ```
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
    let canonical_path = path_literal.value().replace('\\', "/");
    let canonical_path_literal = LitStr::new(&canonical_path, path_literal.span());
    let theorem_path = Utf8PathBuf::from(&canonical_path);
    let theorem_docs = load_theorem_file_from_manifest_dir(manifest_dir, &theorem_path)
        .map_err(|error| MacroExpansionError::from_load(&error))?;

    Ok(render_expansion(
        &canonical_path_literal,
        &canonical_path,
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
