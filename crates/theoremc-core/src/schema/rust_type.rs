//! Shared parsing and canonicalisation for Rust type strings.

use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, BoundLifetimes, GenericArgument, GenericParam,
    ParenthesizedGenericArguments, PathArguments, ReturnType, Type, TypeArray, TypeBareFn,
    TypeGroup, TypeImplTrait, TypeMacro, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice,
    TypeTraitObject, TypeTuple,
};

use super::SchemaError;

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

/// Validates a theorem-owned Rust type string with caller-owned diagnostics.
pub(crate) fn validate(
    ty: &str,
    context: impl FnOnce(syn::Error) -> SchemaError,
) -> Result<(), SchemaError> {
    parse(ty).map_err(context)?;
    Ok(())
}

/// Returns the first free named lifetime in a Rust type string.
pub(crate) fn free_named_lifetime(ty: &str) -> Option<String> {
    parse(ty)
        .ok()
        .and_then(|parsed| free_named_lifetime_in_type(&parsed, LifetimeScope::EMPTY))
}

fn free_named_lifetime_in_type(ty: &Type, scope: LifetimeScope<'_>) -> Option<String> {
    match ty {
        Type::Array(array) => free_named_lifetime_in_array(array, scope),
        Type::BareFn(bare_fn) => free_named_lifetime_in_bare_fn(bare_fn, scope),
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

fn free_named_lifetime_in_bare_fn(ty: &TypeBareFn, scope: LifetimeScope<'_>) -> Option<String> {
    let scoped_lifetimes = scoped_lifetimes(scope, ty.lifetimes.as_ref());
    let scoped_scope = LifetimeScope(&scoped_lifetimes);

    ty.inputs
        .iter()
        .find_map(|arg| free_named_lifetime_in_type(&arg.ty, scoped_scope))
        .or_else(|| free_named_lifetime_in_return_type(&ty.output, scoped_scope))
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
        syn::TypeParamBound::Trait(trait_bound) => {
            let scoped_lifetimes = scoped_lifetimes(scope, trait_bound.lifetimes.as_ref());
            free_named_lifetime_in_path_arguments(
                &trait_bound.path.segments,
                LifetimeScope(&scoped_lifetimes),
            )
        }
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

fn free_named_lifetime_in_parenthesized_arguments(
    arguments: &ParenthesizedGenericArguments,
    scope: LifetimeScope<'_>,
) -> Option<String> {
    arguments
        .inputs
        .iter()
        .find_map(|ty| free_named_lifetime_in_type(ty, scope))
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
    if name == "static" || name == "_" || scope.contains(name) {
        None
    } else {
        Some(name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for Rust type lifetime detection.

    use super::free_named_lifetime;
    use rstest::rstest;

    #[rstest]
    #[case("for<'a> fn(&'a crate::Account)")]
    #[case("dyn for<'a> Trait<&'a crate::Account>")]
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
}
