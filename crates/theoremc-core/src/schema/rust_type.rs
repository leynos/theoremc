//! Shared parsing and canonicalisation for Rust type strings.

use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, BoundLifetimes, GenericArgument, GenericParam,
    ParenthesizedGenericArguments, PathArguments, ReturnType, Type, TypeArray, TypeBareFn,
    TypeGroup, TypeImplTrait, TypeMacro, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice,
    TypeTraitObject, TypeTuple,
};

use super::SchemaError;

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
        .and_then(|parsed| free_named_lifetime_in_type(&parsed, &[]))
}

fn free_named_lifetime_in_type(ty: &Type, bound_lifetimes: &[String]) -> Option<String> {
    match ty {
        Type::Array(array) => free_named_lifetime_in_array(array, bound_lifetimes),
        Type::BareFn(bare_fn) => free_named_lifetime_in_bare_fn(bare_fn, bound_lifetimes),
        Type::Group(group) => free_named_lifetime_in_group(group, bound_lifetimes),
        Type::ImplTrait(impl_trait) => {
            free_named_lifetime_in_impl_trait(impl_trait, bound_lifetimes)
        }
        Type::Macro(type_macro) => free_named_lifetime_in_macro(type_macro, bound_lifetimes),
        Type::Paren(paren) => free_named_lifetime_in_paren(paren, bound_lifetimes),
        Type::Path(path) => free_named_lifetime_in_path(path, bound_lifetimes),
        Type::Ptr(ptr) => free_named_lifetime_in_ptr(ptr, bound_lifetimes),
        Type::Reference(reference) => free_named_lifetime_in_reference(reference, bound_lifetimes),
        Type::Slice(slice) => free_named_lifetime_in_slice(slice, bound_lifetimes),
        Type::TraitObject(trait_object) => {
            free_named_lifetime_in_trait_object(trait_object, bound_lifetimes)
        }
        Type::Tuple(tuple) => free_named_lifetime_in_tuple(tuple, bound_lifetimes),
        _ => None,
    }
}

fn free_named_lifetime_in_array(ty: &TypeArray, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, bound_lifetimes)
}

fn free_named_lifetime_in_bare_fn(ty: &TypeBareFn, bound_lifetimes: &[String]) -> Option<String> {
    let scoped_lifetimes = scoped_lifetimes(bound_lifetimes, ty.lifetimes.as_ref());

    ty.inputs
        .iter()
        .find_map(|arg| free_named_lifetime_in_type(&arg.ty, &scoped_lifetimes))
        .or_else(|| free_named_lifetime_in_return_type(&ty.output, &scoped_lifetimes))
}

fn free_named_lifetime_in_group(ty: &TypeGroup, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, bound_lifetimes)
}

fn free_named_lifetime_in_impl_trait(
    ty: &TypeImplTrait,
    bound_lifetimes: &[String],
) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, bound_lifetimes))
}

fn free_named_lifetime_in_macro(ty: &TypeMacro, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.mac.path.segments, bound_lifetimes)
}

fn free_named_lifetime_in_paren(ty: &TypeParen, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, bound_lifetimes)
}

fn free_named_lifetime_in_path(ty: &TypePath, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.path.segments, bound_lifetimes)
}

fn free_named_lifetime_in_ptr(ty: &TypePtr, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, bound_lifetimes)
}

fn free_named_lifetime_in_reference(
    ty: &TypeReference,
    bound_lifetimes: &[String],
) -> Option<String> {
    ty.lifetime
        .as_ref()
        .and_then(|lifetime| free_named_lifetime_name(&lifetime.ident.to_string(), bound_lifetimes))
        .or_else(|| free_named_lifetime_in_type(&ty.elem, bound_lifetimes))
}

fn free_named_lifetime_in_slice(ty: &TypeSlice, bound_lifetimes: &[String]) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem, bound_lifetimes)
}

fn free_named_lifetime_in_trait_object(
    ty: &TypeTraitObject,
    bound_lifetimes: &[String],
) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, bound_lifetimes))
}

fn free_named_lifetime_in_tuple(ty: &TypeTuple, bound_lifetimes: &[String]) -> Option<String> {
    ty.elems
        .iter()
        .find_map(|elem| free_named_lifetime_in_type(elem, bound_lifetimes))
}

fn free_named_lifetime_in_return_type(
    output: &ReturnType,
    bound_lifetimes: &[String],
) -> Option<String> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => free_named_lifetime_in_type(ty, bound_lifetimes),
    }
}

fn free_named_lifetime_in_type_param_bound(
    bound: &syn::TypeParamBound,
    bound_lifetimes: &[String],
) -> Option<String> {
    match bound {
        syn::TypeParamBound::Trait(trait_bound) => {
            let scoped_lifetimes =
                scoped_lifetimes(bound_lifetimes, trait_bound.lifetimes.as_ref());
            free_named_lifetime_in_path_arguments(&trait_bound.path.segments, &scoped_lifetimes)
        }
        syn::TypeParamBound::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string(), bound_lifetimes)
        }
        _ => None,
    }
}

fn free_named_lifetime_in_path_arguments(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
    bound_lifetimes: &[String],
) -> Option<String> {
    segments
        .iter()
        .find_map(|segment| free_named_lifetime_in_arguments(&segment.arguments, bound_lifetimes))
}

fn free_named_lifetime_in_arguments(
    arguments: &PathArguments,
    bound_lifetimes: &[String],
) -> Option<String> {
    match arguments {
        PathArguments::None => None,
        PathArguments::AngleBracketed(angle_bracketed) => {
            free_named_lifetime_in_angle_bracketed_arguments(angle_bracketed, bound_lifetimes)
        }
        PathArguments::Parenthesized(parenthesized) => {
            free_named_lifetime_in_parenthesized_arguments(parenthesized, bound_lifetimes)
        }
    }
}

fn free_named_lifetime_in_angle_bracketed_arguments(
    arguments: &AngleBracketedGenericArguments,
    bound_lifetimes: &[String],
) -> Option<String> {
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string(), bound_lifetimes)
        }
        GenericArgument::Type(ty) => free_named_lifetime_in_type(ty, bound_lifetimes),
        GenericArgument::AssocType(assoc) => {
            free_named_lifetime_in_type(&assoc.ty, bound_lifetimes)
        }
        GenericArgument::Constraint(constraint) => constraint
            .bounds
            .iter()
            .find_map(|bound| free_named_lifetime_in_type_param_bound(bound, bound_lifetimes)),
        _ => None,
    })
}

fn free_named_lifetime_in_parenthesized_arguments(
    arguments: &ParenthesizedGenericArguments,
    bound_lifetimes: &[String],
) -> Option<String> {
    arguments
        .inputs
        .iter()
        .find_map(|ty| free_named_lifetime_in_type(ty, bound_lifetimes))
        .or_else(|| free_named_lifetime_in_return_type(&arguments.output, bound_lifetimes))
}

fn scoped_lifetimes(
    bound_lifetimes: &[String],
    new_lifetimes: Option<&BoundLifetimes>,
) -> Vec<String> {
    let mut scoped = bound_lifetimes.to_vec();
    if let Some(lifetimes) = new_lifetimes {
        scoped.extend(lifetimes.lifetimes.iter().filter_map(|param| match param {
            GenericParam::Lifetime(lifetime) => Some(lifetime.lifetime.ident.to_string()),
            _ => None,
        }));
    }
    scoped
}

fn free_named_lifetime_name(name: &str, bound_lifetimes: &[String]) -> Option<String> {
    if name == "static" || name == "_" || bound_lifetimes.iter().any(|bound| bound == name) {
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
