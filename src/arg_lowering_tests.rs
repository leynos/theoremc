//! Unit tests for argument-expression lowering.

use indexmap::IndexMap;
use quote::quote;
use rstest::rstest;

use super::{LoweringError, extract_type_path, lower_arg_value, lower_literal, lower_reference};
use crate::schema::TheoremValue;
use crate::schema::arg_value::{ArgValue, LiteralValue};

/// Helper: compare token streams by their string representation.
fn tokens_eq(left: &proc_macro2::TokenStream, right: &proc_macro2::TokenStream) -> bool {
    left.to_string() == right.to_string()
}

/// Helper: lower an [`ArgValue`] against `ty_str`, returning a `Result`.
fn lower_ok(
    param: &str,
    arg: &ArgValue,
    ty_str: &str,
) -> Result<proc_macro2::TokenStream, Box<dyn std::error::Error>> {
    let ty: syn::Type = syn::parse_str(ty_str)?;
    Ok(lower_arg_value(param, arg, &ty)?)
}

#[rstest]
#[case::bool_true(LiteralValue::Bool(true), quote! { true })]
#[case::bool_false(LiteralValue::Bool(false), quote! { false })]
fn test_lower_literal_bool(
    #[case] value: LiteralValue,
    #[case] expected: proc_macro2::TokenStream,
) {
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::positive(42, "42")]
#[case::negative(-99, "- 99")]
#[case::zero(0, "0")]
fn test_lower_literal_integer(#[case] n: i64, #[case] expected: &str) {
    let result = lower_literal(&LiteralValue::Integer(n));
    assert_eq!(result.to_string(), expected);
}

#[test]
fn test_lower_literal_float() {
    let value = LiteralValue::Float(99.5);
    let result = lower_literal(&value);
    // Compare string representation for floats
    assert_eq!(result.to_string(), "99.5");
}

#[rstest]
#[case::simple(LiteralValue::String("hello".to_owned()), quote! { "hello" })]
#[case::empty(LiteralValue::String(String::new()), quote! { "" })]
#[case::with_escapes(
    LiteralValue::String("hello\nworld".to_owned()),
    quote! { "hello\nworld" }
)]
fn test_lower_literal_string_cases(
    #[case] input: LiteralValue,
    #[case] expected: proc_macro2::TokenStream,
) {
    let value = input;
    let result = lower_literal(&value);
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::simple("graph")]
#[case::with_underscore("my_var")]
#[case::with_digits("var123")]
fn test_lower_reference_valid(#[case] input: &str) {
    let result = lower_reference("param", input).expect("valid identifier should lower");
    let expected: proc_macro2::TokenStream =
        syn::parse_str(input).expect("test input must parse as TokenStream");
    assert!(tokens_eq(&result, &expected));
}

#[test]
fn test_lower_reference_non_identifier_returns_error() {
    let result = lower_reference("param", "foo::bar");
    assert!(result.is_err());
    if let Err(LoweringError::NestedDecodeError { param, detail }) = result {
        assert_eq!(param, "param");
        assert!(detail.contains("foo::bar"));
    } else {
        panic!("expected NestedDecodeError for non-identifier reference");
    }
}

#[test]
fn test_lower_arg_value_literal_integer() {
    let arg = ArgValue::Literal(LiteralValue::Integer(42));
    let result = lower_ok("count", &arg, "i32").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { 42 }));
}

#[test]
fn test_lower_arg_value_literal_string() {
    let arg = ArgValue::Literal(LiteralValue::String("test".to_owned()));
    let result = lower_ok("name", &arg, "String").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { "test" }));
}

#[test]
fn test_lower_arg_value_reference() {
    let arg = ArgValue::Reference("binding".to_owned());
    let result = lower_ok("graph", &arg, "Graph").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { binding }));
}

#[test]
fn test_lower_arg_value_empty_sequence() {
    let arg = ArgValue::RawSequence(vec![]);
    let result = lower_ok("items", &arg, "Vec<i32>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec![] }));
}

#[test]
fn test_lower_arg_value_sequence_integers() {
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
        TheoremValue::Integer(3),
    ]);
    let result = lower_ok("nums", &arg, "Vec<i32>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec![1, 2, 3] }));
}

#[test]
fn test_lower_arg_value_sequence_strings() {
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::String("a".to_owned()),
        TheoremValue::String("b".to_owned()),
    ]);
    let result = lower_ok("strs", &arg, "Vec<String>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec!["a", "b"] }));
}

#[test]
fn test_lower_arg_value_sequence_mixed_scalars() {
    let arg = ArgValue::RawSequence(vec![TheoremValue::Integer(1), TheoremValue::Bool(true)]);
    let result = lower_ok("mixed", &arg, "Vec<Value>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec![1, true] }));
}

#[test]
fn test_lower_arg_value_nested_sequence() {
    let arg = ArgValue::RawSequence(vec![TheoremValue::Sequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
    ])]);
    let result = lower_ok("nested", &arg, "Vec<Vec<i32>>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec![vec![1, 2]] }));
}

#[test]
fn test_lower_arg_value_sequence_with_nested_ref() {
    // A sequence containing { ref: graph } should decode the sentinel
    // and lower to a reference identifier.
    let mut sentinel = IndexMap::new();
    sentinel.insert("ref".to_owned(), TheoremValue::String("graph".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(sentinel)]);
    let result = lower_ok("items", &arg, "Vec<Graph>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec![graph] }));
}

#[test]
fn test_lower_arg_value_sequence_with_nested_literal() {
    // A sequence containing { literal: "ref" } should decode the sentinel
    // and lower to a string literal (not a reference).
    let mut sentinel = IndexMap::new();
    sentinel.insert("literal".to_owned(), TheoremValue::String("ref".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(sentinel)]);
    let result = lower_ok("items", &arg, "Vec<String>").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { vec!["ref"] }));
}

#[test]
fn test_lower_arg_value_map_field_with_nested_ref() {
    // A struct field containing { ref: binding } should decode the sentinel
    // and lower to a reference identifier in the field value position.
    let mut sentinel = IndexMap::new();
    sentinel.insert("ref".to_owned(), TheoremValue::String("binding".to_owned()));
    let mut outer = IndexMap::new();
    outer.insert("graph".to_owned(), TheoremValue::Mapping(sentinel));
    let arg = ArgValue::RawMap(outer);
    let result = lower_ok("cfg", &arg, "Config").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { Config { graph: binding } }));
}

#[test]
fn test_lower_arg_value_map_field_with_nested_literal() {
    // A struct field containing { literal: "ref" } should decode the sentinel
    // and lower to a string literal (preserving the string "ref").
    let mut sentinel = IndexMap::new();
    sentinel.insert("literal".to_owned(), TheoremValue::String("ref".to_owned()));
    let mut outer = IndexMap::new();
    outer.insert("name".to_owned(), TheoremValue::Mapping(sentinel));
    let arg = ArgValue::RawMap(outer);
    let result = lower_ok("cfg", &arg, "Config").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { Config { name: "ref" } }));
}

#[test]
fn test_lower_arg_value_empty_map() {
    let arg = ArgValue::RawMap(IndexMap::new());
    let result = lower_ok("node", &arg, "Node").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { Node {} }));
}

#[test]
fn test_lower_arg_value_map_single_field() {
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(42));
    let arg = ArgValue::RawMap(map);
    let result = lower_ok("node", &arg, "Node").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { Node { id: 42 } }));
}

#[test]
fn test_lower_arg_value_map_multiple_fields() {
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(1));
    map.insert("name".to_owned(), TheoremValue::String("test".to_owned()));
    map.insert("active".to_owned(), TheoremValue::Bool(true));
    let arg = ArgValue::RawMap(map);
    let result = lower_ok("node", &arg, "Node").expect("lower_ok failed");
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
    let result = lower_ok("node", &arg, "Node").expect("lower_ok failed");
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
    let result = lower_ok("point", &arg, "module::Point").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { module::Point { x: 10 } }));
}

#[test]
fn test_extract_type_path_simple() {
    let ty: syn::Type = syn::parse_str("MyStruct").expect("failed to parse type");
    let path = extract_type_path("param", &ty).expect("extraction failed");
    assert_eq!(quote! { #path }.to_string(), "MyStruct");
}

#[test]
fn test_extract_type_path_qualified() {
    let ty: syn::Type = syn::parse_str("crate::module::Type").expect("failed to parse type");
    let path = extract_type_path("param", &ty).expect("extraction failed");
    assert_eq!(quote! { #path }.to_string(), "crate :: module :: Type");
}

#[test]
fn test_extract_type_path_rejects_reference() {
    let ty: syn::Type = syn::parse_str("&MyStruct").expect("failed to parse type");
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
    let ty: syn::Type = syn::parse_str("(i32, i32)").expect("failed to parse type");
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
}

#[test]
fn test_extract_type_path_rejects_generic() {
    let ty: syn::Type = syn::parse_str("Vec<i32>").expect("failed to parse type");
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "param");
        assert!(reason.contains("generic type paths"));
    } else {
        panic!("expected UnsupportedType error for generic path");
    }
}

#[test]
fn test_extract_type_path_rejects_qself() {
    let ty: syn::Type = syn::parse_str("<T as Trait>::Assoc").expect("failed to parse type");
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "param");
        assert!(reason.contains("qualified-self paths"));
    } else {
        panic!("expected UnsupportedType error for qself path");
    }
}

#[test]
fn test_lower_arg_value_map_nested_map_fails() {
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::Integer(5));
    let mut outer = IndexMap::new();
    outer.insert("inner".to_owned(), TheoremValue::Mapping(inner));
    let arg = ArgValue::RawMap(outer);
    let ty: syn::Type = syn::parse_str("Outer").expect("failed to parse type");
    let result = lower_arg_value("nested", &arg, &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "nested");
        assert!(reason.contains("nested map with keys"));
    } else {
        panic!("expected UnsupportedType error for nested maps");
    }
}
