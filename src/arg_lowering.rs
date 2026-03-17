//! Argument-expression lowering for theorem action calls.
//!
//! This module converts semantically decoded [`ArgValue`](crate::schema::ArgValue)
//! instances into Rust expressions suitable for generated proof harnesses.
//! It recursively lowers YAML-sourced values into `vec![...]` macro calls for
//! lists and struct literals for maps, guided by expected Rust parameter types.
//!
//! The lowering layer operates outside the schema boundary (`ADR-3`): it
//! consumes decoded argument values and type information, but does not
//! participate in YAML deserialization or semantic validation.

use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::quote;

use crate::schema::TheoremValue;
use crate::schema::arg_value::{ArgValue, LiteralValue};

/// Errors produced during argument lowering.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum LoweringError {
    /// The expected type shape is not supported for lowering.
    #[error("unsupported type shape for parameter '{param}': {reason}")]
    UnsupportedType {
        /// Parameter name for context.
        param: String,
        /// Human-readable reason.
        reason: String,
    },

    /// Encountered a recursive decoding error for a nested value.
    #[error("failed to decode nested value in parameter '{param}': {detail}")]
    NestedDecodeError {
        /// Parameter name for context.
        param: String,
        /// Error detail from nested decoding.
        detail: String,
    },
}

/// Lowers a decoded [`ArgValue`] into a Rust expression token stream.
///
/// This is the primary entry point for argument lowering. It accepts a
/// semantically decoded argument value and the expected Rust type (as a
/// parsed [`syn::Type`]), then produces a token stream suitable for
/// inclusion in generated proof harness code.
///
/// # Lowering rules
///
/// - **Scalar literals** (`LiteralValue`) are emitted as Rust literal
///   tokens (`true`, `42`, `3.14`, `"hello"`).
/// - **References** (`ArgValue::Reference`) are emitted as identifier
///   path expressions.
/// - **Sequences** (`ArgValue::RawSequence`) are lowered recursively to
///   `vec![...]` macro calls.
/// - **Maps** (`ArgValue::RawMap`) are lowered to struct literals using
///   the type name from `expected_type`. Field values are lowered
///   recursively.
///
/// # Type-driven lowering
///
/// The `expected_type` parameter guides struct literal synthesis: the
/// outer type name is extracted from the parsed type and used to construct
/// the struct literal expression. Field type information is not yet used
/// for nested lowering (future work may inspect struct field types via
/// compile-time probes).
///
/// # Errors
///
/// Returns [`LoweringError::UnsupportedType`] if the expected type shape
/// cannot be handled by the current lowering logic.
///
/// Returns [`LoweringError::NestedDecodeError`] if a nested composite
/// value contains invalid data that cannot be recursively lowered.
///
/// # Examples
///
/// ```rust,ignore
/// use theoremc::arg_lowering::lower_arg_value;
/// use theoremc::schema::{ArgValue, LiteralValue};
///
/// let value = ArgValue::Literal(LiteralValue::Integer(42));
/// let ty = syn::parse_str("i32").unwrap();
/// let tokens = lower_arg_value("count", &value, &ty)?;
/// // tokens represents: 42
/// ```
pub fn lower_arg_value(
    param_name: &str,
    value: &ArgValue,
    expected_type: &syn::Type,
) -> Result<TokenStream, LoweringError> {
    match value {
        ArgValue::Literal(lit) => Ok(lower_literal(lit)),
        ArgValue::Reference(name) => Ok(lower_reference(name)),
        ArgValue::RawSequence(elements) => lower_sequence(param_name, elements),
        ArgValue::RawMap(fields) => lower_map(param_name, fields, expected_type),
    }
}

/// Lowers a scalar [`LiteralValue`] to a Rust literal token.
fn lower_literal(value: &LiteralValue) -> TokenStream {
    match value {
        LiteralValue::Bool(b) => quote! { #b },
        LiteralValue::Integer(n) => {
            // Use unsuffixed literal to avoid type suffix (42 not 42i64)
            let lit = proc_macro2::Literal::i64_unsuffixed(*n);
            quote! { #lit }
        }
        LiteralValue::Float(f) => {
            // Use a literal token for the float to preserve notation
            let lit = proc_macro2::Literal::f64_unsuffixed(*f);
            quote! { #lit }
        }
        LiteralValue::String(s) => quote! { #s },
    }
}

/// Lowers a reference identifier to a path expression.
fn lower_reference(name: &str) -> TokenStream {
    // Parse the identifier and emit it as a path expression.
    // The identifier was already validated by schema::arg_value decoding,
    // so we can safely parse it here.
    let ident = syn::parse_str::<syn::Ident>(name)
        .unwrap_or_else(|_| panic!("validated identifier failed to parse: {name}"));
    quote! { #ident }
}

/// Lowers a sequence of [`TheoremValue`] to a `vec![...]` expression.
///
/// Each element is recursively decoded and lowered. Nested sequences,
/// maps, scalars, and references are all handled.
fn lower_sequence(
    param_name: &str,
    elements: &[TheoremValue],
) -> Result<TokenStream, LoweringError> {
    let element_results: Result<Vec<TokenStream>, LoweringError> = elements
        .iter()
        .map(|elem| lower_theorem_value(param_name, elem))
        .collect();
    let lowered_elements = element_results?;
    Ok(quote! { vec![#(#lowered_elements),*] })
}

/// Lowers a raw [`TheoremValue`] (used for nested composite values).
///
/// This helper recursively decodes and lowers nested values that appear
/// inside sequences and maps. It mirrors the top-level `lower_arg_value`
/// logic but operates on raw `TheoremValue` instead of decoded `ArgValue`.
fn lower_theorem_value(
    param_name: &str,
    value: &TheoremValue,
) -> Result<TokenStream, LoweringError> {
    match value {
        TheoremValue::Bool(b) => Ok(quote! { #b }),
        TheoremValue::Integer(n) => {
            let lit = proc_macro2::Literal::i64_unsuffixed(*n);
            Ok(quote! { #lit })
        }
        TheoremValue::Float(f) => {
            let lit = proc_macro2::Literal::f64_unsuffixed(*f);
            Ok(quote! { #lit })
        }
        TheoremValue::String(s) => Ok(quote! { #s }),
        TheoremValue::Sequence(elements) => lower_sequence(param_name, elements),
        TheoremValue::Mapping(_fields) => {
            // Nested maps within composite values don't have explicit type
            // information in this implementation. Phase 3 compile-time type
            // probes will enable field-type introspection. For now, this
            // limitation means deeply nested struct literals must be
            // expressed differently (e.g., via references to let-bindings).
            Err(LoweringError::UnsupportedType {
                param: param_name.to_owned(),
                reason: concat!(
                    "nested maps require type information that is not available ",
                    "without compile-time field type probes (Phase 3); ",
                    "use explicit let-bindings for nested struct construction"
                )
                .to_owned(),
            })
        }
    }
}

/// Lowers a map of [`TheoremValue`] to a struct literal expression.
///
/// The struct type name is extracted from `expected_type`. Field values
/// are lowered recursively. No validation of field names or types is
/// performed here; mismatches will surface during Rust compilation.
fn lower_map(
    param_name: &str,
    fields: &IndexMap<String, TheoremValue>,
    expected_type: &syn::Type,
) -> Result<TokenStream, LoweringError> {
    // Extract the type name from the expected type.
    // For now, we support simple path types (e.g., `Node`, `MyStruct`).
    // Future work may support more complex type shapes.
    let type_path = extract_type_path(param_name, expected_type)?;

    // Lower each field value recursively.
    let field_assignment_results: Result<Vec<TokenStream>, LoweringError> = fields
        .iter()
        .map(|(key, value)| {
            let field_ident = syn::parse_str::<syn::Ident>(key).map_err(|_| {
                LoweringError::NestedDecodeError {
                    param: param_name.to_owned(),
                    detail: format!("field name '{key}' is not a valid Rust identifier"),
                }
            })?;
            let field_value = lower_theorem_value(param_name, value)?;
            Ok(quote! { #field_ident: #field_value })
        })
        .collect();
    let field_assignments = field_assignment_results?;

    Ok(quote! {
        #type_path {
            #(#field_assignments),*
        }
    })
}

/// Extracts a type path from a [`syn::Type`] for struct literal synthesis.
///
/// Currently supports simple path types like `MyStruct` or `module::Type`.
/// Returns an error for unsupported type shapes (generics, references, etc.).
fn extract_type_path(param_name: &str, ty: &syn::Type) -> Result<syn::Path, LoweringError> {
    match ty {
        syn::Type::Path(type_path) => Ok(type_path.path.clone()),
        _ => Err(LoweringError::UnsupportedType {
            param: param_name.to_owned(),
            reason: format!(
                "expected a simple type path (e.g., MyStruct), found: {}",
                quote! { #ty }
            ),
        }),
    }
}

#[cfg(test)]
#[path = "arg_lowering_tests.rs"]
mod tests;
