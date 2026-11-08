//! Expansion pipeline behind the crate's `theorem_file` macro.
//!
//! The proc-macro entry point in the crate root only parses its argument and
//! turns the result, or a [`MacroExpansionError`], into compiler diagnostics.
//! Everything that reads a `.theorem` file and renders Rust lives here, so the
//! entry point stays a thin adapter and the pipeline is reachable from the
//! crate's unit tests without going through the macro interface.

use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::LitStr;
use theoremc_core::{
    TheoremFileLoadError,
    collision::{referenced_actions, referenced_types},
    load_theorem_file_from_manifest_dir,
    mangle::{mangle_action_name, mangle_module_path, mangle_theorem_harness},
    path_format::normalize_path_separators,
    schema::{ActionSignature, SchemaDiagnostic, TheoremDoc},
};

use crate::action_signature_index::ActionSignatureIndex;

/// Resolves the crate root from the environment and expands the referenced
/// theorem file.
pub(crate) fn expand_theorem_file(
    path_literal: &LitStr,
) -> Result<TokenStream2, MacroExpansionError> {
    let manifest_dir = manifest_dir_from_env()?;
    expand_theorem_file_at(&manifest_dir, path_literal)
}

/// Reads `CARGO_MANIFEST_DIR` through the injectable reader seam.
#[expect(
    clippy::disallowed_methods,
    reason = "a proc-macro can only learn its crate root from Cargo's environment, so this single read is the seam whose result is injected into the pure reader"
)]
fn manifest_dir_from_env() -> Result<Utf8PathBuf, MacroExpansionError> {
    manifest_dir_from_reader(env::var)
}

/// Parses the manifest directory returned by `read_variable`.
pub(crate) fn manifest_dir_from_reader(
    read_variable: impl FnOnce(&'static str) -> Result<String, env::VarError>,
) -> Result<Utf8PathBuf, MacroExpansionError> {
    read_variable("CARGO_MANIFEST_DIR")
        .map(Utf8PathBuf::from)
        .map_err(|_| MacroExpansionError::MissingManifestDir)
}

/// Loads and validates the theorem file beneath `manifest_dir` and renders its
/// expansion.
pub(crate) fn expand_theorem_file_at(
    manifest_dir: &Utf8Path,
    path_literal: &LitStr,
) -> Result<TokenStream2, MacroExpansionError> {
    let path_value = path_literal.value();
    let canonical_path = normalize_path_separators(&path_value);
    let canonical_path_literal = LitStr::new(&canonical_path, path_literal.span());
    let theorem_path = Utf8PathBuf::from(&canonical_path);
    let theorem_docs = load_theorem_file_from_manifest_dir(manifest_dir, &theorem_path)
        .map_err(|error| MacroExpansionError::from_load(&error))?;

    render_expansion(&canonical_path_literal, &canonical_path, &theorem_docs)
}

/// Builds the generated module from the loaded theorem documents.
fn render_expansion(
    path_literal: &LitStr,
    theorem_path: &str,
    theorem_docs: &[TheoremDoc],
) -> Result<TokenStream2, MacroExpansionError> {
    let module_ident = identifier(mangle_module_path(theorem_path).module_name());
    let harnesses = generated_harnesses(theorem_path, theorem_docs)?;
    let action_probes = generated_action_probes(theorem_docs)?;
    let action_probe_tokens = render_action_probes(&action_probes);
    let type_probes = generated_referenced_type_probes(theorem_docs)?;
    let type_probe_tokens = render_referenced_type_probes(&type_probes);
    let harness_idents: Vec<&Ident> = harnesses.iter().map(|harness| &harness.ident).collect();
    let unwind_literals: Vec<&syn::LitInt> = harnesses
        .iter()
        .map(|harness| &harness.unwind_literal)
        .collect();
    let harness_count = syn::LitInt::new(&harness_idents.len().to_string(), Span::call_site());

    Ok(quote! {
        #[expect(
            unexpected_cfgs,
            reason = "Kani sets cfg(kani) when compiling proof harnesses"
        )]
        mod #module_ident {
            const _: &str =
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #path_literal));

            #action_probe_tokens
            #type_probe_tokens

            #[cfg(kani)]
            pub(super) mod kani {
                #(
                    #[kani::proof]
                    #[kani::unwind(#unwind_literals)]
                    pub(crate) fn #harness_idents() {}
                )*
            }

            #[cfg(kani)]
            const _: [fn(); #harness_count] = [#(kani::#harness_idents),*];
        }
    })
}

/// Parses every referenced type into syntax types for the compile-time probes.
fn generated_referenced_type_probes(
    theorem_docs: &[TheoremDoc],
) -> Result<Vec<syn::Type>, MacroExpansionError> {
    referenced_types(theorem_docs)
        .into_iter()
        .map(parse_referenced_type)
        .collect()
}

/// Parses one referenced type, reporting the offending spelling on failure.
fn parse_referenced_type(ty: &str) -> Result<syn::Type, MacroExpansionError> {
    syn::parse_str(ty).map_err(|source| MacroExpansionError::InvalidReferencedType {
        ty: ty.to_owned(),
        message: source.to_string(),
    })
}
/// Creates one harness ident and unwind literal per theorem document.
pub(crate) fn generated_harnesses(
    theorem_path: &str,
    theorem_docs: &[TheoremDoc],
) -> Result<Vec<GeneratedHarness>, MacroExpansionError> {
    theorem_docs
        .iter()
        .map(|doc| {
            let kani = doc.evidence.kani.as_ref().ok_or_else(|| {
                MacroExpansionError::MissingKaniEvidence {
                    theorem: doc.theorem.as_str().to_owned(),
                }
            })?;
            Ok(GeneratedHarness {
                ident: identifier(
                    mangle_theorem_harness(theorem_path, doc.theorem.as_str()).identifier(),
                ),
                unwind_literal: syn::LitInt::new(&kani.unwind.to_string(), Span::call_site()),
            })
        })
        .collect()
}

/// Creates a signature-checking probe for every referenced action.
fn generated_action_probes(
    theorem_docs: &[TheoremDoc],
) -> Result<Vec<GeneratedActionProbe>, MacroExpansionError> {
    let referenced = referenced_actions(theorem_docs);
    let signature_index = ActionSignatureIndex::for_actions(theorem_docs, &referenced)?;
    referenced
        .iter()
        .map(|canonical| {
            let signature = signature_index.signature_for(canonical)?;
            action_probe(canonical, signature)
        })
        .collect()
}

/// Builds the probe ident and parsed types for one action signature.
fn action_probe(
    canonical: &str,
    signature: &ActionSignature,
) -> Result<GeneratedActionProbe, MacroExpansionError> {
    let param_types = signature
        .params
        .values()
        .map(|param| parse_action_type(canonical, param))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = parse_action_type(canonical, &signature.returns)?;

    Ok(GeneratedActionProbe {
        ident: identifier(mangle_action_name(canonical).identifier()),
        param_types,
        return_type,
    })
}

/// Parses one action parameter or return type, naming the action on failure.
fn parse_action_type(canonical: &str, ty: &str) -> Result<syn::Type, MacroExpansionError> {
    syn::parse_str(ty).map_err(|source| MacroExpansionError::InvalidActionSignature {
        action: canonical.to_owned(),
        message: source.to_string(),
    })
}

/// Renders the anonymous const items that anchor each action signature.
fn render_action_probes(action_probes: &[GeneratedActionProbe]) -> TokenStream2 {
    if action_probes.is_empty() {
        return TokenStream2::new();
    }

    let probe_idents = action_probes.iter().map(|probe| &probe.ident);
    let probe_param_types = action_probes.iter().map(|probe| &probe.param_types);
    let probe_return_types = action_probes.iter().map(|probe| &probe.return_type);

    // Each `const _: fn(...) -> ... = crate::theorem_actions::...;` anchors the
    // referenced symbol at compile time. Anonymous `_` items bypass dead-code
    // checks without an `#[allow]`, so a signature mismatch surfaces as a
    // normal type error rather than a silenced lint.
    quote! {
        #(
            const _: fn(#(#probe_param_types),*) -> #probe_return_types =
                crate::theorem_actions::#probe_idents;
        )*
    }
}

/// Renders the anonymous const item that anchors each referenced type.
fn render_referenced_type_probes(type_probes: &[syn::Type]) -> TokenStream2 {
    if type_probes.is_empty() {
        return TokenStream2::new();
    }

    quote! {
        const _: () = {
            fn __theoremc_assert_referenced<T: ?Sized>() {}
            #(
                let _ = __theoremc_assert_referenced::<#type_probes>;
            )*
        };
    }
}
/// Creates a call-site identifier from a mangled name.
fn identifier(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

/// Ident and unwind bound for one generated Kani harness.
pub(crate) struct GeneratedHarness {
    /// Call-site identifier of the generated harness function.
    pub(crate) ident: Ident,
    /// Unwind bound taken from the theorem's `Evidence.kani` block.
    pub(crate) unwind_literal: syn::LitInt,
}

/// Ident and parsed types for one compile-time action signature probe.
struct GeneratedActionProbe {
    /// Call-site identifier used as the action's `theorem_actions` symbol.
    ident: Ident,
    /// Parsed parameter types used to rebuild the expected `fn` type.
    param_types: Vec<syn::Type>,
    /// Parsed return type used to rebuild the expected `fn` type.
    return_type: syn::Type,
}

/// Failure modes reported as `compile_error!` at the macro call site.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MacroExpansionError {
    /// The macro could not learn its crate root from Cargo.
    #[error("`CARGO_MANIFEST_DIR` is not set during theorem macro expansion")]
    MissingManifestDir,
    /// A theorem document omits the Kani evidence the macro requires.
    #[error("theorem `{theorem}` does not declare required `Evidence.kani` configuration")]
    MissingKaniEvidence {
        /// Theorem that omitted its Kani evidence.
        theorem: String,
    },
    /// A referenced action has no declared signature.
    #[error("referenced action `{action}` is missing an Actions signature entry")]
    MissingActionSignature {
        /// Canonical action name that was referenced.
        action: String,
    },
    /// Two theorem documents declare the same action differently.
    #[error(
        "referenced action `{action}` has conflicting Actions signatures: theorem `{first_theorem}` declares `{first_signature}`, but theorem `{conflicting_theorem}` declares `{conflicting_signature}`"
    )]
    ConflictingActionSignature {
        /// Canonical action name under conflict.
        action: String,
        /// Theorem whose declaration is treated as the reference.
        first_theorem: String,
        /// Theorem that disagrees with the reference declaration.
        conflicting_theorem: String,
        /// Signature summary declared by `first_theorem`.
        first_signature: String,
        /// Signature summary declared by `conflicting_theorem`.
        conflicting_signature: String,
    },
    /// A declared action signature is not valid Rust syntax.
    #[error("referenced action `{action}` has an invalid Actions signature: {message}")]
    InvalidActionSignature {
        /// Canonical action name whose signature failed to parse.
        action: String,
        /// Parser diagnostic for the offending signature.
        message: String,
    },
    /// A referenced type is not valid Rust syntax.
    #[error("referenced type `{ty}` is invalid: {message}")]
    InvalidReferencedType {
        /// Type spelling that failed to parse.
        ty: String,
        /// Parser diagnostic for the offending type.
        message: String,
    },
    /// Loading or validating the theorem file failed.
    #[error("{0}")]
    LoadTheoremFile(String),
}

impl MacroExpansionError {
    /// Converts the error into a `compile_error!` invocation at `span`.
    pub(crate) fn to_compile_error(&self, span: Span) -> TokenStream2 {
        syn::Error::new(span, self.to_string()).to_compile_error()
    }

    /// Maps a theorem file load failure to a rendered diagnostic message.
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
