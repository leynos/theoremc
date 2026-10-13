//! Property tests for free named lifetime detection in Rust type strings.
//!
//! [`free_named_lifetime`] recurses through nested type forms and must report
//! the first free named lifetime while ignoring reserved (`'static`, `'_`) and
//! `for<..>`-bound lifetimes. The `rstest` tables in [`super`] pin individual
//! forms; these properties generalise them over generated callable and nested
//! shapes, so a regression in the recursion or in the `syn 3` argument wrappers
//! cannot hide behind hand-picked examples.
//!
//! Each generator draws every sub-term from one family, which makes the oracle
//! exact rather than approximate:
//!
//! - the *free* family plants the name `'free` in leaf positions and uses only
//!   non-binding wrappers, so every generated type contains at least one free
//!   lifetime and the scanner must report `"free"`;
//! - the *closed* family uses plain types, `'static`, and `'bound` under a
//!   `for<'bound>` binder that always encloses it, so no generated type
//!   contains a free lifetime and the scanner must report `None`.

use proptest::prelude::*;

use super::{free_named_lifetime, parse};

/// A plain type with no lifetime component.
const PLAIN: &str = "crate::Account";

/// A reference carrying the free lifetime `'free`.
const FREE_REF: &str = "&'free crate::Account";

/// A reference carrying the reserved lifetime `'static`.
const STATIC_REF: &str = "&'static crate::Account";

/// A function pointer binding its own input lifetime with `for<'bound>`.
const BOUND_FN_PTR: &str = "for<'bound> fn(&'bound crate::Account)";

/// A `Fn`-family bound binding its own input lifetime with `for<'bound>`.
const BOUND_FN_TRAIT: &str = "dyn for<'bound> Fn(&'bound crate::Account)";

/// Wraps a type in the non-binding shapes shared by both families.
///
/// These wrappers introduce no `for<..>` binder, so they preserve whatever
/// lifetime status the wrapped type already had.
fn wrap_non_binding(inner: BoxedStrategy<String>) -> BoxedStrategy<String> {
    let wrapped = prop_oneof![
        inner.clone().prop_map(|ty| format!("Box<{ty}>")),
        inner.clone().prop_map(|ty| format!("dyn Fn({ty})")),
        inner.clone().prop_map(|ty| format!("fn({ty})")),
        inner.clone().prop_map(|ty| format!("*const {ty}")),
        prop::collection::vec(inner, 1..3).prop_map(|types| format!("({})", types.join(", "))),
    ];
    wrapped.boxed()
}

/// Generates types in which `'free` appears at least once and is never bound.
///
/// Every leaf uses the free lifetime and every wrapper is non-binding or a
/// `for<'bound>` binder that constrains a different name, so `'free` remains
/// free in every generated position.
fn free_lifetime_types() -> BoxedStrategy<String> {
    let leaves = prop_oneof![
        Just(FREE_REF.to_owned()),
        Just("dyn for<'bound> Fn(&'bound crate::Account) -> &'free crate::Account".to_owned()),
        Just("for<'bound> fn(&'bound crate::Account) -> &'free crate::Account".to_owned()),
    ]
    .boxed();

    leaves
        .prop_recursive(3, 32, 3, move |inner| {
            let wrapped = prop_oneof![
                wrap_non_binding(inner.clone()),
                inner.prop_map(|ty| format!("for<'bound> fn({ty})")),
            ];
            wrapped.boxed()
        })
        .boxed()
}

/// Generates types whose every lifetime is reserved or `for<'bound>`-bound.
///
/// The only binder that introduces `'bound` also encloses the positions in
/// which `'bound` appears, so no generated type has a free lifetime.
fn closed_lifetime_types() -> BoxedStrategy<String> {
    let leaves = prop_oneof![
        Just(PLAIN.to_owned()),
        Just(STATIC_REF.to_owned()),
        Just(BOUND_FN_PTR.to_owned()),
        Just(BOUND_FN_TRAIT.to_owned()),
    ]
    .boxed();

    leaves
        .prop_recursive(3, 32, 3, move |inner| {
            let wrapped = prop_oneof![
                wrap_non_binding(inner.clone()),
                inner.clone().prop_map(|ty| format!("for<'bound> fn({ty})")),
                inner.prop_map(|ty| format!("dyn for<'bound> Fn({ty})")),
            ];
            wrapped.boxed()
        })
        .boxed()
}

proptest! {
    /// Every generated type parses, so a failure below is about lifetime
    /// detection rather than a malformed generator.
    #[test]
    fn generated_types_are_valid_rust_types(ty in free_lifetime_types()) {
        let parsed = parse(&ty);
        prop_assert!(
            parsed.is_ok(),
            "generated type '{}' should parse: {:?}",
            ty,
            parsed.err()
        );
    }

    /// A type built entirely from free-lifetime leaves reports `"free"`,
    /// however deeply the callable and container wrappers nest.
    #[test]
    fn free_lifetime_is_reported(ty in free_lifetime_types()) {
        let found = free_named_lifetime(&ty);
        prop_assert_eq!(
            found.as_deref(),
            Some("free"),
            "type '{}' contains a free lifetime",
            ty
        );
    }

    /// A type built from plain, reserved, and correctly bound lifetimes
    /// reports none, so a binder or reserved name is never mistaken for a
    /// free lifetime.
    #[test]
    fn bound_and_reserved_lifetimes_are_not_reported(ty in closed_lifetime_types()) {
        prop_assert_eq!(
            free_named_lifetime(&ty),
            None,
            "type '{}' contains no free lifetime",
            ty
        );
    }
}
