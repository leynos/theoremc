//! Unit tests for argument-expression lowering.

use indexmap::IndexMap;
use quote::quote;

use super::{LoweringError, extract_type_path, lower_arg_value, lower_literal, lower_reference};
use crate::schema::TheoremValue;
use crate::schema::arg_value::{ArgValue, LiteralValue};

// Helper to compare token streams by their string representation
fn tokens_eq(left: &proc_macro2::TokenStream, right: &proc_macro2::TokenStream) -> bool {
    left.to_string() == right.to_string()
}

#[test]
fn test_lower_literal_bool_true() {
    let value = LiteralValue::Bool(true);
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { true }));
}

#[test]
fn test_lower_literal_bool_false() {
    let value = LiteralValue::Bool(false);
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { false }));
}

#[test]
fn test_lower_literal_integer_positive() {
    let value = LiteralValue::Integer(42);
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { 42 }));
}

#[test]
fn test_lower_literal_integer_negative() {
    let value = LiteralValue::Integer(-99);
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { -99 }));
}

#[test]
fn test_lower_literal_integer_zero() {
    let value = LiteralValue::Integer(0);
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { 0 }));
}

#[test]
fn test_lower_literal_float() {
    let value = LiteralValue::Float(99.5);
    let result = lower_literal(&value);
    // Compare string representation for floats
    assert_eq!(result.to_string(), "99.5");
}

#[test]
fn test_lower_literal_string_simple() {
    let value = LiteralValue::String("hello".to_owned());
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { "hello" }));
}

#[test]
fn test_lower_literal_string_empty() {
    let value = LiteralValue::String(String::new());
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { "" }));
}

#[test]
fn test_lower_literal_string_with_escapes() {
    let value = LiteralValue::String("hello\nworld".to_owned());
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &quote! { "hello\nworld" }));
}

#[test]
fn test_lower_reference_simple() {
    let result = lower_reference("graph");
    assert!(tokens_eq(&result, &quote! { graph }));
}

#[test]
fn test_lower_reference_with_underscore() {
    let result = lower_reference("my_var");
    assert!(tokens_eq(&result, &quote! { my_var }));
}

#[test]
fn test_lower_reference_with_digits() {
    let result = lower_reference("var123");
    assert!(tokens_eq(&result, &quote! { var123 }));
}

#[test]
fn test_lower_arg_value_literal_integer() {
    let arg = ArgValue::Literal(LiteralValue::Integer(42));
    let ty = syn::parse_str("i32").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("count", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { 42 }));
}

#[test]
fn test_lower_arg_value_literal_string() {
    let arg = ArgValue::Literal(LiteralValue::String("test".to_owned()));
    let ty = syn::parse_str("String").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("name", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { "test" }));
}

#[test]
fn test_lower_arg_value_reference() {
    let arg = ArgValue::Reference("binding".to_owned());
    let ty = syn::parse_str("Graph").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("graph", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { binding }));
}

#[test]
fn test_lower_arg_value_empty_sequence() {
    let arg = ArgValue::RawSequence(vec![]);
    let ty = syn::parse_str("Vec<i32>").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("items", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { vec![] }));
}

#[test]
fn test_lower_arg_value_sequence_integers() {
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
        TheoremValue::Integer(3),
    ]);
    let ty = syn::parse_str("Vec<i32>").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("nums", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { vec![1, 2, 3] }));
}

#[test]
fn test_lower_arg_value_sequence_strings() {
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::String("a".to_owned()),
        TheoremValue::String("b".to_owned()),
    ]);
    let ty = syn::parse_str("Vec<String>").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("strs", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { vec!["a", "b"] }));
}

#[test]
fn test_lower_arg_value_sequence_mixed_scalars() {
    let arg = ArgValue::RawSequence(vec![TheoremValue::Integer(1), TheoremValue::Bool(true)]);
    let ty = syn::parse_str("Vec<Value>").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("mixed", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { vec![1, true] }));
}

#[test]
fn test_lower_arg_value_nested_sequence() {
    let arg = ArgValue::RawSequence(vec![TheoremValue::Sequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
    ])]);
    let ty =
        syn::parse_str("Vec<Vec<i32>>").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("nested", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { vec![vec![1, 2]] }));
}

#[test]
fn test_lower_arg_value_empty_map() {
    let arg = ArgValue::RawMap(IndexMap::new());
    let ty = syn::parse_str("Node").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("node", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { Node {} }));
}

#[test]
fn test_lower_arg_value_map_single_field() {
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(42));
    let arg = ArgValue::RawMap(map);
    let ty = syn::parse_str("Node").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("node", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { Node { id: 42 } }));
}

#[test]
fn test_lower_arg_value_map_multiple_fields() {
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(1));
    map.insert("name".to_owned(), TheoremValue::String("test".to_owned()));
    map.insert("active".to_owned(), TheoremValue::Bool(true));
    let arg = ArgValue::RawMap(map);
    let ty = syn::parse_str("Node").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("node", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(
        &result,
        &quote! { Node { id: 1, name: "test", active: true } }
    ));
}

#[test]
fn test_lower_arg_value_map_with_list_field() {
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(1));
    map.insert(
        "tags".to_owned(),
        TheoremValue::Sequence(vec![
            TheoremValue::String("a".to_owned()),
            TheoremValue::String("b".to_owned()),
        ]),
    );
    let arg = ArgValue::RawMap(map);
    let ty = syn::parse_str("Node").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("node", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(
        &result,
        &quote! { Node { id: 1, tags: vec!["a", "b"] } }
    ));
}

#[test]
fn test_lower_arg_value_map_with_qualified_type() {
    let mut map = IndexMap::new();
    map.insert("x".to_owned(), TheoremValue::Integer(10));
    let arg = ArgValue::RawMap(map);
    let ty =
        syn::parse_str("module::Point").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result =
        lower_arg_value("point", &arg, &ty).unwrap_or_else(|e| panic!("lowering failed: {e}"));
    assert!(tokens_eq(&result, &quote! { module::Point { x: 10 } }));
}

#[test]
fn test_extract_type_path_simple() {
    let ty = syn::parse_str("MyStruct").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let path = extract_type_path("param", &ty).unwrap_or_else(|e| panic!("extraction failed: {e}"));
    assert_eq!(quote! { #path }.to_string(), "MyStruct");
}

#[test]
fn test_extract_type_path_qualified() {
    let ty = syn::parse_str("crate::module::Type")
        .unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let path = extract_type_path("param", &ty).unwrap_or_else(|e| panic!("extraction failed: {e}"));
    assert_eq!(quote! { #path }.to_string(), "crate :: module :: Type");
}

#[test]
fn test_extract_type_path_rejects_reference() {
    let ty = syn::parse_str("&MyStruct").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "param");
        assert!(reason.contains("simple type path"));
    } else {
        panic!("expected UnsupportedType error");
    }
}

#[test]
fn test_extract_type_path_rejects_tuple() {
    let ty = syn::parse_str("(i32, i32)").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
}

#[test]
fn test_lower_arg_value_map_nested_map_fails() {
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::Integer(5));
    let mut outer = IndexMap::new();
    outer.insert("inner".to_owned(), TheoremValue::Mapping(inner));
    let arg = ArgValue::RawMap(outer);
    let ty = syn::parse_str("Outer").unwrap_or_else(|e| panic!("failed to parse type: {e}"));
    let result = lower_arg_value("nested", &arg, &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "nested");
        assert!(reason.contains("nested maps require type information"));
    } else {
        panic!("expected UnsupportedType error for nested maps");
    }
}
