//! Shared parsing and canonicalisation for Rust type strings.

use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, GenericArgument, ParenthesizedGenericArguments, PathArguments,
    ReturnType, Type, TypeArray, TypeBareFn, TypeGroup, TypeImplTrait, TypeMacro, TypeParen,
    TypePath, TypePtr, TypeReference, TypeSlice, TypeTraitObject, TypeTuple,
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
        .and_then(|parsed| free_named_lifetime_in_type(&parsed))
}

fn free_named_lifetime_in_type(ty: &Type) -> Option<String> {
    match ty {
        Type::Array(array) => free_named_lifetime_in_array(array),
        Type::BareFn(bare_fn) => free_named_lifetime_in_bare_fn(bare_fn),
        Type::Group(group) => free_named_lifetime_in_group(group),
        Type::ImplTrait(impl_trait) => free_named_lifetime_in_impl_trait(impl_trait),
        Type::Macro(type_macro) => free_named_lifetime_in_macro(type_macro),
        Type::Paren(paren) => free_named_lifetime_in_paren(paren),
        Type::Path(path) => free_named_lifetime_in_path(path),
        Type::Ptr(ptr) => free_named_lifetime_in_ptr(ptr),
        Type::Reference(reference) => free_named_lifetime_in_reference(reference),
        Type::Slice(slice) => free_named_lifetime_in_slice(slice),
        Type::TraitObject(trait_object) => free_named_lifetime_in_trait_object(trait_object),
        Type::Tuple(tuple) => free_named_lifetime_in_tuple(tuple),
        _ => None,
    }
}

fn free_named_lifetime_in_array(ty: &TypeArray) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem)
}

fn free_named_lifetime_in_bare_fn(ty: &TypeBareFn) -> Option<String> {
    ty.inputs
        .iter()
        .find_map(|arg| free_named_lifetime_in_type(&arg.ty))
        .or_else(|| free_named_lifetime_in_return_type(&ty.output))
}

fn free_named_lifetime_in_group(ty: &TypeGroup) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem)
}

fn free_named_lifetime_in_impl_trait(ty: &TypeImplTrait) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(free_named_lifetime_in_type_param_bound)
}

fn free_named_lifetime_in_macro(ty: &TypeMacro) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.mac.path.segments)
}

fn free_named_lifetime_in_paren(ty: &TypeParen) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem)
}

fn free_named_lifetime_in_path(ty: &TypePath) -> Option<String> {
    free_named_lifetime_in_path_arguments(&ty.path.segments)
}

fn free_named_lifetime_in_ptr(ty: &TypePtr) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem)
}

fn free_named_lifetime_in_reference(ty: &TypeReference) -> Option<String> {
    ty.lifetime
        .as_ref()
        .and_then(|lifetime| free_named_lifetime_name(&lifetime.ident.to_string()))
        .or_else(|| free_named_lifetime_in_type(&ty.elem))
}

fn free_named_lifetime_in_slice(ty: &TypeSlice) -> Option<String> {
    free_named_lifetime_in_type(&ty.elem)
}

fn free_named_lifetime_in_trait_object(ty: &TypeTraitObject) -> Option<String> {
    ty.bounds
        .iter()
        .find_map(free_named_lifetime_in_type_param_bound)
}

fn free_named_lifetime_in_tuple(ty: &TypeTuple) -> Option<String> {
    ty.elems.iter().find_map(free_named_lifetime_in_type)
}

fn free_named_lifetime_in_return_type(output: &ReturnType) -> Option<String> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => free_named_lifetime_in_type(ty),
    }
}

fn free_named_lifetime_in_type_param_bound(bound: &syn::TypeParamBound) -> Option<String> {
    match bound {
        syn::TypeParamBound::Trait(trait_bound) => {
            free_named_lifetime_in_path_arguments(&trait_bound.path.segments)
        }
        syn::TypeParamBound::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string())
        }
        _ => None,
    }
}

fn free_named_lifetime_in_path_arguments(
    segments: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
) -> Option<String> {
    segments
        .iter()
        .find_map(|segment| free_named_lifetime_in_arguments(&segment.arguments))
}

fn free_named_lifetime_in_arguments(arguments: &PathArguments) -> Option<String> {
    match arguments {
        PathArguments::None => None,
        PathArguments::AngleBracketed(angle_bracketed) => {
            free_named_lifetime_in_angle_bracketed_arguments(angle_bracketed)
        }
        PathArguments::Parenthesized(parenthesized) => {
            free_named_lifetime_in_parenthesized_arguments(parenthesized)
        }
    }
}

fn free_named_lifetime_in_angle_bracketed_arguments(
    arguments: &AngleBracketedGenericArguments,
) -> Option<String> {
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Lifetime(lifetime) => {
            free_named_lifetime_name(&lifetime.ident.to_string())
        }
        GenericArgument::Type(ty) => free_named_lifetime_in_type(ty),
        GenericArgument::AssocType(assoc) => free_named_lifetime_in_type(&assoc.ty),
        GenericArgument::Constraint(constraint) => constraint
            .bounds
            .iter()
            .find_map(free_named_lifetime_in_type_param_bound),
        _ => None,
    })
}

fn free_named_lifetime_in_parenthesized_arguments(
    arguments: &ParenthesizedGenericArguments,
) -> Option<String> {
    arguments
        .inputs
        .iter()
        .find_map(free_named_lifetime_in_type)
        .or_else(|| free_named_lifetime_in_return_type(&arguments.output))
}

fn free_named_lifetime_name(name: &str) -> Option<String> {
    if name == "static" || name == "_" {
        None
    } else {
        Some(name.to_owned())
    }
}
