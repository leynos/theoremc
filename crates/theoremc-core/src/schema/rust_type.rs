//! Shared parsing and canonicalisation for Rust type strings.

use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, BoundLifetimes, GenericArgument, GenericParam,
    ParenthesizedGenericArguments, PathArguments, ReturnType, Type, TypeArray, TypeFnPtr,
    TypeGroup, TypeImplTrait, TypeMacro, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice,
    TypeTraitObject, TypeTuple,
};

#[derive(Clone, Copy)]
struct LifetimeScope<'a>(&'a [String]);

impl LifetimeScope<'_> {
    const EMPTY: LifetimeScope<'static> = LifetimeScope(&[]);

    fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|bound| bound == name)
    }
}

/// Parses a theorem-owned Rust type string.
pub(crate) fn parse(ty: &str) -> Result<Type, syn::Error> {
    syn::parse_str(ty.trim())
}

/// Returns the canonical token stream for a valid Rust type string.
pub(crate) fn canonical_token_stream(ty: &str) -> Option<String> {
    parse(ty)
        .ok()
        .map(|parsed| parsed.to_token_stream().to_string())
}

/// Parses a Rust type and returns its first free named lifetime.
///
/// A lifetime is "free" when it is named, is neither `'static` nor `'_`, and
/// is not bound by a `for<..>` binder around the position it appears in.
/// Parsing failures are reported as [`syn::Error`]; a type that parses but
/// carries no free named lifetime yields `Ok(None)`.
pub(crate) fn parse_with_free_named_lifetime(ty: &str) -> Result<Option<String>, syn::Error> {
    let parsed = parse(ty)?;
    Ok(free_named_lifetime_in_type(&parsed, LifetimeScope::EMPTY))
}

/// Returns the first free named lifetime in a Rust type string.
#[cfg(test)]
pub(crate) fn free_named_lifetime(ty: &str) -> Option<String> {
    parse(ty)
        .ok()
        .and_then(|parsed| free_named_lifetime_in_type(&parsed, LifetimeScope::EMPTY))
}

/// Scans one parsed type for a free named lifetime.
///
/// The scan recurses through the composite `syn::Type` variants that can
/// contain a lifetime, including function pointers and `Fn`-family
/// parenthesized arguments.
fn free_named_lifetime_in_type(ty: &Type, scope: LifetimeScope<'_>) -> Option<String> {
    match ty {
        Type::Array(array) => free_named_lifetime_in_array(array, scope),
        Type::FnPtr(fn_ptr) => free_named_lifetime_in_fn_ptr(fn_ptr, scope),
        Type::Group(group) => free_named_lifetime_in_group(group, scope),
        Type::ImplTrait(impl_trait) => free_named_lifetime_in_impl_trait(impl_trait, scope),
        Type::Macro(type_macro) => free_named_lifetime_in_macro(type_macro, scope),
        Type::Paren(paren) => free_named_lifetime_in_paren(paren, scope),
        Type::Path(path) => free_named_lifetime_in_path(path, scope),
        Type::Ptr(ptr) => free_named_lifetime_in_ptr(ptr, scope),
        Type::Reference(reference) => free_named_lifetime_in_reference(reference, scope),
        Type::Slice(slice) => free_named_lifetime_in_slice(slice, scope),
        Type::TraitObject(trait_object) => free_named_lifetime_in_trait_object(trait_object, scope),
        Type::Tuple(tuple) => free_named_lifetime_in_tuple(tuple, scope),
        _ => None,
    }
}

fn free_named_lifetime_in_array(ty: &TypeArray, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, scope)
}

/// Scans a function-pointer type for a free named lifetime.
///
/// A `for<..>` binder written on the pointer introduces lifetimes that are
/// bound across its whole signature, so the binder is added to the scope
/// before the signature is scanned.
fn free_named_lifetime_in_fn_ptr(ty: &TypeFnPtr, scope: LifetimeScope<'_>) -> Option<String> {
    ty.lifetimes.as_ref().map_or_else(
        || free_named_lifetime_in_fn_ptr_signature(ty, scope),
        |lifetimes| {
            let scoped_lifetimes = scoped_lifetimes(scope, Some(lifetimes));
            free_named_lifetime_in_fn_ptr_signature(ty, LifetimeScope(&scoped_lifetimes))
        },
    )
}

/// Scans the inputs and output of a function-pointer signature.
///
/// Earlier `syn` releases exposed each input as a bare type; `syn 3` wraps it
/// in a [`NamedArg`], so the argument type is read from its `ty` field.
fn free_named_lifetime_in_fn_ptr_signature(
    ty: &TypeFnPtr,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    ty.inputs
        .iter()
        .find_map(|arg| free_named_lifetime_in_type(&arg.ty, scope))
        .or_else(|| free_named_lifetime_in_return_type(&ty.output, scope))
}

fn free_named_lifetime_in_group(ty: &TypeGroup, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, scope)
}

fn free_named_lifetime_in_impl_trait(
    ty: &TypeImplTrait,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, scope))
}

fn free_named_lifetime_in_macro(ty: &TypeMacro, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.mac.path.segments, scope)
}

fn free_named_lifetime_in_paren(ty: &TypeParen, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, scope)
}

fn free_named_lifetime_in_path(ty: &TypePath, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.path.segments, scope)
}

fn free_named_lifetime_in_ptr(ty: &TypePtr, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, scope)
}

fn free_named_lifetime_in_reference(
    ty: &TypeReference,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    ty.lifetime
        .as_ref()
        .and_then(|lifetime| free_named_lifetime_name(&lifetime.ident.to_string(), scope))
        .or_else(|| free_named_lifetime_in_type(&ty.elem, scope))
}

fn free_named_lifetime_in_slice(ty: &TypeSlice, scope: LifetimeScope<'_>) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, scope)
}

fn free_named_lifetime_in_trait_object(
    ty: &TypeTraitObject,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, scope))
}

fn free_named_lifetime_in_tuple(ty: &TypeTuple, scope: LifetimeScope<'_>) -> Option<String> {
    ty.elems
        .iter()
        .find_map(|elem| free_named_lifetime_in_type(elem, scope))
}

fn free_named_lifetime_in_return_type(
    output: &ReturnType,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => free_named_lifetime_in_type(ty, scope),
    }
}

fn free_named_lifetime_in_type_param_bound(
    bound: &syn::TypeParamBound,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    match bound {
        syn::TypeParamBound::Trait(trait_bound) => trait_bound.lifetimes.as_ref().map_or_else(
            || free_named_lifetime_in_path_arguments(&trait_bound.path.segments, scope),
            |lifetimes| {
                let scoped_lifetimes = scoped_lifetimes(scope, Some(lifetimes));
                free_named_lifetime_in_path_arguments(
                    &trait_bound.path.segments,
                    LifetimeScope(&scoped_lifetimes),
                )
            },
        ),
        syn::TypeParamBound::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string(), scope)
        }
        _ => None,
    }
}

fn free_named_lifetime_in_path_arguments(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    segments
        .iter()
        .find_map(|segment| free_named_lifetime_in_arguments(&segment.arguments, scope))
}

fn free_named_lifetime_in_arguments(
    arguments: &PathArguments,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    match arguments {
        PathArguments::None => None,
        PathArguments::AngleBracketed(angle_bracketed) => {
            free_named_lifetime_in_angle_bracketed_arguments(angle_bracketed, scope)
        }
        PathArguments::Parenthesized(parenthesized) => {
            free_named_lifetime_in_parenthesized_arguments(parenthesized, scope)
        }
    }
}

fn free_named_lifetime_in_angle_bracketed_arguments(
    arguments: &AngleBracketedGenericArguments,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string(), scope)
        }
        GenericArgument::Type(ty) => free_named_lifetime_in_type(ty, scope),
        GenericArgument::AssocType(assoc) => free_named_lifetime_in_type(&assoc.ty, scope),
        GenericArgument::Constraint(constraint) => constraint
            .bounds
            .iter()
            .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, scope)),
        _ => None,
    })
}

/// Scans the inputs and output of parenthesized generic arguments.
///
/// This is the `(A) -> B` form of a `Fn`-family bound or path, such as
/// `dyn Fn(&'a T)`. In `syn 3` each input is a [`NamedArg`] whose `ty` field
/// holds the argument type.
fn free_named_lifetime_in_parenthesized_arguments(
    arguments: &ParenthesizedGenericArguments,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    arguments
        .inputs
        .iter()
        .find_map(|arg| free_named_lifetime_in_type(&arg.ty, scope))
        .or_else(|| free_named_lifetime_in_return_type(&arguments.output, scope))
}

fn scoped_lifetimes(
    scope: LifetimeScope<'_>,
    new_lifetimes: Option<&BoundLifetimes>,
) -> Vec<String> {
    let mut scoped = scope.0.to_vec();
    if let Some(lifetimes) = new_lifetimes {
        scoped.extend(lifetimes.lifetimes.iter().filter_map(|param| match param {
            GenericParam::Lifetime(lifetime) => Some(lifetime.lifetime.ident.to_string()),
            _ => None,
        }));
    }
    scoped
}

fn free_named_lifetime_name(name: &str, scope: LifetimeScope<'_>) -> Option<String> {
    if is_reserved_lifetime_name(name) || is_bound_lifetime(name, scope) {
        None
    } else {
        Some(name.to_owned())
    }
}

fn is_reserved_lifetime_name(name: &str) -> bool {
    name == "static" || name == "_"
}

fn is_bound_lifetime(name: &str, scope: LifetimeScope<'_>) -> bool {
    scope.contains(name)
}

#[cfg(test)]
#[path = "rust_type_property_tests.rs"]
mod property_tests;

#[cfg(test)]
mod tests {
    //! Unit tests for Rust type lifetime detection.

    use super::free_named_lifetime;
    use rstest::rstest;

    // `dyn for<'a> Fn(&'a crate::Account)` covers the parenthesized-input
    // branch of the function-trait scan with an input binder, the mirror of
    // the function-pointer binder above.
    #[rstest]
    #[case("for<'a> fn(&'a crate::Account)")]
    #[case("dyn for<'a> Trait<&'a crate::Account>")]
    #[case("dyn for<'a> Fn(&'a crate::Account)")]
    fn bound_hrtb_lifetimes_are_not_free(#[case] ty: &str) {
        assert_eq!(free_named_lifetime(ty), None);
    }

    #[test]
    fn plain_named_reference_lifetime_is_free() {
        assert_eq!(
            free_named_lifetime("&'a crate::Account"),
            Some("a".to_owned())
        );
    }

    #[rstest]
    #[case("&'static crate::Account")]
    #[case("&'_ crate::Account")]
    fn reserved_lifetime_names_are_not_free(#[case] ty: &str) {
        assert_eq!(free_named_lifetime(ty), None);
    }

    #[rstest]
    #[case("[&'a crate::Account; 1]")]
    #[case("fn() -> &'a crate::Account")]
    #[case("impl Trait<&'a crate::Account>")]
    #[case("(&'a crate::Account)")]
    #[case("*const &'a crate::Account")]
    #[case("[&'a crate::Account]")]
    #[case("dyn Trait<'a>")]
    #[case("dyn Fn(crate::Account) -> &'a crate::Account")]
    #[case("dyn Trait<Item = &'a crate::Account>")]
    #[case("dyn Trait<Assoc: Other<'a>>")]
    fn nested_named_lifetimes_are_free(#[case] ty: &str) {
        assert_eq!(free_named_lifetime(ty), Some("a".to_owned()));
    }

    // Regression guard for the `syn 3` input-scanning paths: both the function
    // pointer and the `Fn`-family parenthesized arguments now store their
    // inputs as `NamedArg` rather than a bare `Type`, so a free lifetime in an
    // input must still be reported. An implementation that dropped either input
    // walk would pass every return-type case above and fail here.
    #[rstest]
    #[case("fn(&'a crate::Account)")]
    #[case("dyn Fn(&'a crate::Account)")]
    fn free_lifetime_in_callable_inputs_is_reported(#[case] ty: &str) {
        assert_eq!(free_named_lifetime(ty), Some("a".to_owned()));
    }

    #[test]
    fn macro_types_have_no_scanned_lifetimes() {
        assert_eq!(free_named_lifetime("type_macro!()"), None);
    }
}
