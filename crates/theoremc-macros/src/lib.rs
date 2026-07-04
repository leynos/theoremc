//! Proc-macro expansion for compile-time theorem integration.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
};

use camino::{Utf8Path, Utf8PathBuf};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{LitStr, parse_macro_input};
use theoremc_core::{
    TheoremFileLoadError,
    collision::referenced_actions,
    load_theorem_file_from_manifest_dir,
    mangle::{mangle_action_name, mangle_module_path, mangle_theorem_harness},
    path_format::normalize_path_separators,
    schema::{ActionSignature, SchemaDiagnostic},
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
/// - A `#[cfg(kani)] pub(super) mod kani` sub-module contains one
///   `#[kani::proof]` and `#[kani::unwind(n)]` `pub(crate) fn` per theorem
///   document, named via
///   [`theoremc_core::mangle::mangle_theorem_harness`].
/// - A cfg-gated const array of `fn()` pointers sized to the harness count
///   anchors all generated symbols when Kani is compiling the crate.
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
/// | A theorem omits `Evidence.kani` | theorem `<name>` does not declare required `Evidence.kani` configuration |
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
///     #[expect(unexpected_cfgs, reason = "Kani sets cfg(kani) when compiling proof harnesses")]
///     #[cfg(kani)]
///     pub(super) mod kani {
///         #[kani::proof]
///         #[kani::unwind(1)]
///         pub(crate) fn theorem__my_lemma__h<hash>() {}
///     }
///     #[cfg(kani)]
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
    let path_value = path_literal.value();
    let canonical_path = normalize_path_separators(&path_value);
    let canonical_path_literal = LitStr::new(&canonical_path, path_literal.span());
    let theorem_path = Utf8PathBuf::from(&canonical_path);
    let theorem_docs = load_theorem_file_from_manifest_dir(manifest_dir, &theorem_path)
        .map_err(|error| MacroExpansionError::from_load(&error))?;

    render_expansion(&canonical_path_literal, &canonical_path, &theorem_docs)
}

fn render_expansion(
    path_literal: &LitStr,
    theorem_path: &str,
    theorem_docs: &[theoremc_core::schema::TheoremDoc],
) -> Result<TokenStream2, MacroExpansionError> {
    let module_ident = identifier(mangle_module_path(theorem_path).module_name());
    let harnesses = generated_harnesses(theorem_path, theorem_docs)?;
    let action_probes = generated_action_probes(theorem_docs)?;
    let action_probe_tokens = render_action_probes(&action_probes);
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

fn generated_harnesses(
    theorem_path: &str,
    theorem_docs: &[theoremc_core::schema::TheoremDoc],
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

fn generated_action_probes(
    theorem_docs: &[theoremc_core::schema::TheoremDoc],
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

#[derive(Debug)]
struct ActionSignatureIndex<'a> {
    signatures: BTreeMap<&'a str, &'a ActionSignature>,
}

impl<'a> ActionSignatureIndex<'a> {
    fn for_actions(
        theorem_docs: &'a [theoremc_core::schema::TheoremDoc],
        canonical_actions: &[&str],
    ) -> Result<Self, MacroExpansionError> {
        let selected = canonical_actions.iter().copied().collect::<BTreeSet<_>>();
        let mut declared_signatures: BTreeMap<&'a str, &'a ActionSignature> = BTreeMap::new();

        for doc in theorem_docs {
            for (action, signature) in &doc.actions {
                let canonical = action.as_str();
                Self::insert_signature(&mut declared_signatures, canonical, signature)?;
            }
        }

        let signatures = declared_signatures
            .into_iter()
            .filter(|(action, _)| selected.contains(action))
            .collect();

        Ok(Self { signatures })
    }

    fn insert_signature(
        signatures: &mut BTreeMap<&'a str, &'a ActionSignature>,
        canonical: &'a str,
        signature: &'a ActionSignature,
    ) -> Result<(), MacroExpansionError> {
        let Some(first) = signatures.get(canonical) else {
            signatures.insert(canonical, signature);
            return Ok(());
        };

        if signature.is_semantically_equivalent(first) {
            return Ok(());
        }

        Err(MacroExpansionError::ConflictingActionSignature {
            action: canonical.to_owned(),
        })
    }

    fn signature_for(&self, canonical: &str) -> Result<&'a ActionSignature, MacroExpansionError> {
        self.signatures.get(canonical).copied().ok_or_else(|| {
            MacroExpansionError::MissingActionSignature {
                action: canonical.to_owned(),
            }
        })
    }
}

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

fn parse_action_type(canonical: &str, ty: &str) -> Result<syn::Type, MacroExpansionError> {
    syn::parse_str(ty).map_err(|source| MacroExpansionError::InvalidActionSignature {
        action: canonical.to_owned(),
        message: source.to_string(),
    })
}

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

fn identifier(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

struct GeneratedHarness {
    ident: Ident,
    unwind_literal: syn::LitInt,
}

struct GeneratedActionProbe {
    ident: Ident,
    param_types: Vec<syn::Type>,
    return_type: syn::Type,
}

#[derive(Debug, thiserror::Error)]
enum MacroExpansionError {
    #[error("`CARGO_MANIFEST_DIR` is not set during theorem macro expansion")]
    MissingManifestDir,
    #[error("theorem `{theorem}` does not declare required `Evidence.kani` configuration")]
    MissingKaniEvidence { theorem: String },
    #[error("referenced action `{action}` is missing an Actions signature entry")]
    MissingActionSignature { action: String },
    #[error("referenced action `{action}` has conflicting Actions signatures")]
    ConflictingActionSignature { action: String },
    #[error("referenced action `{action}` has an invalid Actions signature: {message}")]
    InvalidActionSignature { action: String, message: String },
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

/// Fixture and assertion helpers consumed by the private macro expansion tests.
#[cfg(test)]
mod tests_support;

/// Private expansion tests that exercise this module through `tests_support`.
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Private expansion tests for compile-time action probe generation.
#[cfg(test)]
#[path = "action_probe_tests.rs"]
mod action_probe_tests;
