//! Token rendering for theorem-file macro expansion.
//!
//! This module turns an [`ExpansionPlan`](super::query::ExpansionPlan) into
//! the `TokenStream2` emitted by `theorem_file!`. It performs no I/O and makes
//! no semantic decisions: every input has already been validated by the query
//! phase, so failures cannot be introduced here.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

use super::query::{ExpansionPlan, GeneratedActionProbe};

/// Renders one expanded theorem module from a validated expansion plan.
pub(super) fn render_expansion(plan: &ExpansionPlan) -> TokenStream2 {
    let module_ident = &plan.module_ident;
    let path_literal = &plan.path_literal;
    let action_probe_tokens = render_action_probes(&plan.action_probes);
    let type_probe_tokens = render_referenced_type_probes(&plan.referenced_types);
    let harness_idents: Vec<&proc_macro2::Ident> =
        plan.harnesses.iter().map(|harness| &harness.ident).collect();
    let unwind_literals: Vec<&syn::LitInt> = plan
        .harnesses
        .iter()
        .map(|harness| &harness.unwind_literal)
        .collect();
    let harness_count = syn::LitInt::new(&harness_idents.len().to_string(), Span::call_site());

    quote! {
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
    }
}

/// Renders compile-time action-signature probes.
pub(super) fn render_action_probes(action_probes: &[GeneratedActionProbe]) -> TokenStream2 {
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

/// Renders compile-time referenced-type probes.
pub(super) fn render_referenced_type_probes(type_probes: &[syn::Type]) -> TokenStream2 {
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

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;