//! Unit tests for the macro expansion render phase.

use proc_macro2::{Ident, Span};
use syn::LitStr;

use super::*;

fn empty_plan() -> ExpansionPlan {
    ExpansionPlan {
        module_ident: Ident::new("theorems__empty__habcdef012345", Span::call_site()),
        path_literal: LitStr::new("theorems/empty.theorem", Span::call_site()),
        harnesses: Vec::new(),
        action_probes: Vec::new(),
        referenced_types: Vec::new(),
    }
}

#[test]
fn empty_probe_lists_render_no_tokens() {
    assert_eq!(render_action_probes(&[]).to_string(), "");
    assert_eq!(render_referenced_type_probes(&[]).to_string(), "");
}

#[test]
fn render_expansion_anchors_the_source_path() {
    let tokens = render_expansion(&empty_plan()).to_string();

    assert!(
        tokens.contains("mod theorems__empty__habcdef012345"),
        "module ident should appear in the expansion: {tokens}"
    );
    assert!(
        tokens.contains("theorems/empty.theorem"),
        "canonical path literal should appear in the expansion: {tokens}"
    );
    assert!(
        tokens.contains("include_str"),
        "the expansion should anchor the theorem source: {tokens}"
    );
}

#[test]
fn render_expansion_omits_probes_when_none_are_referenced() {
    let tokens = render_expansion(&empty_plan()).to_string();

    assert!(
        !tokens.contains("crate::theorem_actions"),
        "no action probes should be rendered: {tokens}"
    );
    assert!(
        !tokens.contains("__theoremc_assert_referenced"),
        "no referenced-type probes should be rendered: {tokens}"
    );
}

#[test]
fn render_expansion_allows_rather_than_expects_unexpected_cfgs() {
    // An `expect` would be unfulfilled in a consumer that registers
    // `cfg(kani)`, and `-D warnings` turns that into a build failure. Only
    // `allow` is correct for generated code, so pin the keyword here rather
    // than relying solely on the slower consumer builds.
    let tokens = render_expansion(&empty_plan()).to_string();

    assert!(
        tokens.contains("allow"),
        "the generated gate should use `allow`: {tokens}"
    );
    assert!(
        !tokens.contains("# [expect"),
        "the generated gate must not emit an `expect` attribute: {tokens}"
    );
    assert!(
        tokens.contains("unexpected_cfgs"),
        "the generated gate should name `unexpected_cfgs`: {tokens}"
    );
    assert!(
        tokens.contains("register cfg(kani)"),
        "the generated gate should carry a reason naming its assumption: {tokens}"
    );
}
