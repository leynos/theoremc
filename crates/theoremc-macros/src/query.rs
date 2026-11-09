//! Query phase for theorem-file macro expansion.
//!
//! This module loads a `.theorem` file, checks the semantic inputs the
//! generated code depends on, and returns a typed [`ExpansionPlan`]. It never
//! builds `TokenStream2` output beyond the `LitStr` and `Ident` values that
//! must keep their call-site spans. Keeping the query phase separate from
//! rendering lets the semantic checks be tested without comparing token
//! streams, and keeps `render.rs` free of I/O and validation logic.

use std::env;

use camino::{Utf8Path, Utf8PathBuf};
use proc_macro2::{Ident, Span};
use syn::LitStr;
use theoremc_core::{
    canonical_action_name::CanonicalActionName,
    collision::{referenced_actions, referenced_types},
    load_theorem_file_from_manifest_dir,
    mangle::{mangle_action_name, mangle_module_path, mangle_theorem_harness},
    path_format::normalize_path_separators,
    schema::{ActionSignature, TheoremDoc},
};

/// First-seen action signature index with conflict provenance.
#[path = "action_signature_index.rs"]
pub(super) mod action_signature_index;

use action_signature_index::ActionSignatureIndex;

use super::error::MacroExpansionError;

/// Everything the render phase needs to emit one expanded theorem module.
pub(super) struct ExpansionPlan {
    /// Stable private module name derived from the theorem file path.
    pub(super) module_ident: Ident,
    /// Canonicalized theorem path literal, preserving the call-site span.
    pub(super) path_literal: LitStr,
    /// One generated Kani harness per theorem document, in document order.
    pub(super) harnesses: Vec<GeneratedHarness>,
    /// One typed action probe per distinct referenced action.
    pub(super) action_probes: Vec<GeneratedActionProbe>,
    /// One probe per distinct referenced Rust type.
    pub(super) referenced_types: Vec<syn::Type>,
}

/// A generated Kani proof harness.
pub(super) struct GeneratedHarness {
    pub(super) ident: Ident,
    pub(super) unwind_literal: syn::LitInt,
}

/// A generated compile-time action-signature probe.
pub(super) struct GeneratedActionProbe {
    pub(super) ident: Ident,
    pub(super) param_types: Vec<syn::Type>,
    pub(super) return_type: syn::Type,
}

/// Reads `CARGO_MANIFEST_DIR` for the crate invoking the macro.
pub(super) fn manifest_dir_from_env() -> Result<Utf8PathBuf, MacroExpansionError> {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").map_err(|_| MacroExpansionError::MissingManifestDir)?;
    Ok(Utf8PathBuf::from(manifest_dir))
}

/// Loads `path_literal` relative to `manifest_dir` and derives the plan.
pub(super) fn plan_expansion(
    manifest_dir: &Utf8Path,
    path_literal: &LitStr,
) -> Result<ExpansionPlan, MacroExpansionError> {
    let path_value = path_literal.value();
    let canonical_path = normalize_path_separators(&path_value);
    let canonical_path_literal = LitStr::new(&canonical_path, path_literal.span());
    let theorem_path = Utf8PathBuf::from(&canonical_path);
    let theorem_docs = load_theorem_file_from_manifest_dir(manifest_dir, &theorem_path)
        .map_err(|error| MacroExpansionError::from_load(&error))?;

    Ok(ExpansionPlan {
        module_ident: identifier(mangle_module_path(&canonical_path).module_name()),
        path_literal: canonical_path_literal,
        harnesses: generated_harnesses(&canonical_path, &theorem_docs)?,
        action_probes: generated_action_probes(&theorem_docs)?,
        referenced_types: generated_referenced_type_probes(&theorem_docs)?,
    })
}

fn generated_referenced_type_probes(
    theorem_docs: &[TheoremDoc],
) -> Result<Vec<syn::Type>, MacroExpansionError> {
    referenced_types(theorem_docs)
        .into_iter()
        .map(parse_referenced_type)
        .collect()
}

fn parse_referenced_type(ty: &str) -> Result<syn::Type, MacroExpansionError> {
    syn::parse_str(ty).map_err(|source| MacroExpansionError::InvalidReferencedType {
        ty: ty.to_owned(),
        message: source.to_string(),
    })
}

pub(super) fn generated_harnesses(
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

fn generated_action_probes(
    theorem_docs: &[TheoremDoc],
) -> Result<Vec<GeneratedActionProbe>, MacroExpansionError> {
    let referenced = referenced_actions(theorem_docs);
    let canonical_names: Vec<&str> = referenced.iter().map(|name| name.as_str()).collect();
    let signature_index = ActionSignatureIndex::for_actions(theorem_docs, &canonical_names)?;
    referenced
        .iter()
        .map(|canonical| {
            let signature = signature_index.signature_for(canonical.as_str())?;
            action_probe(canonical, signature)
        })
        .collect()
}

fn action_probe(
    canonical: &CanonicalActionName,
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

fn parse_action_type(
    canonical: &CanonicalActionName,
    ty: &str,
) -> Result<syn::Type, MacroExpansionError> {
    syn::parse_str(ty).map_err(|source| MacroExpansionError::InvalidActionSignature {
        action: canonical.as_str().to_owned(),
        message: source.to_string(),
    })
}

fn identifier(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
