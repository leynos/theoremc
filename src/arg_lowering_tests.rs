//! Unit tests for argument-expression lowering.

use indexmap::IndexMap;
use quote::quote;
use rstest::{fixture, rstest};

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

#[fixture]
fn sentinel_map() -> impl Fn(&str, &str) -> IndexMap<String, TheoremValue> {
    |sentinel_key, payload| {
        let mut sentinel = IndexMap::new();
        sentinel.insert(
            sentinel_key.to_owned(),
            TheoremValue::String(payload.to_owned()),
        );
        sentinel
    }
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
    let result = lower_literal(&input);
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

#[rstest]
#[case::literal_integer(
    ArgValue::Literal(LiteralValue::Integer(42)),
    "count",
    "i32",
    quote! { 42 }
)]
#[case::literal_string(
    ArgValue::Literal(LiteralValue::String("test".to_owned())),
    "name",
    "String",
    quote! { "test" }
)]
#[case::reference(
    ArgValue::Reference("binding".to_owned()),
    "graph",
    "Graph",
    quote! { binding }
)]
fn test_lower_arg_value_scalar_cases(
    #[case] arg: ArgValue,
    #[case] param: &str,
    #[case] ty_str: &str,
    #[case] expected: proc_macro2::TokenStream,
) {
    let result = lower_ok(param, &arg, ty_str).expect("lower_ok failed");
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::empty(ArgValue::RawSequence(vec![]), "Vec<i32>", quote! { vec![] })]
#[case::integers(
    ArgValue::RawSequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
        TheoremValue::Integer(3),
    ]),
    "Vec<i32>",
    quote! { vec![1, 2, 3] }
)]
#[case::strings(
    ArgValue::RawSequence(vec![
        TheoremValue::String("a".to_owned()),
        TheoremValue::String("b".to_owned()),
    ]),
    "Vec<String>",
    quote! { vec!["a", "b"] }
)]
#[case::mixed_scalars(
    ArgValue::RawSequence(vec![TheoremValue::Integer(1), TheoremValue::Bool(true)]),
    "Vec<Value>",
    quote! { vec![1, true] }
)]
#[case::nested_sequence(
    ArgValue::RawSequence(vec![TheoremValue::Sequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
    ])]),
    "Vec<Vec<i32>>",
    quote! { vec![vec![1, 2]] }
)]
fn test_lower_arg_value_sequence_cases(
    #[case] arg: ArgValue,
    #[case] ty_str: &str,
    #[case] expected: proc_macro2::TokenStream,
) {
    let result = lower_ok("name", &arg, ty_str).expect("lower_ok failed");
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::nested_ref(("ref", "graph"), "Vec<Graph>", quote! { vec![graph] })]
#[case::nested_literal(
    ("literal", "ref"),
    "Vec<String>",
    quote! { vec!["ref"] }
)]
fn test_lower_arg_value_sequence_with_nested_sentinel(
    #[case] sentinel: (&str, &str),
    sentinel_map: impl Fn(&str, &str) -> IndexMap<String, TheoremValue>,
    #[case] target_type: &str,
    #[case] expected: proc_macro2::TokenStream,
) {
    let nested_sentinel = sentinel_map(sentinel.0, sentinel.1);
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(nested_sentinel)]);
    let result = lower_ok("items", &arg, target_type).expect("lower_ok failed");
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::nested_ref(
    "graph",
    ("ref", "binding"),
    quote! { Config { graph: binding } }
)]
#[case::nested_literal(
    "name",
    ("literal", "ref"),
    quote! { Config { name: "ref" } }
)]
fn test_lower_arg_value_map_field_with_nested_sentinel(
    #[case] field_name: &str,
    #[case] sentinel: (&str, &str),
    sentinel_map: impl Fn(&str, &str) -> IndexMap<String, TheoremValue>,
    #[case] expected: proc_macro2::TokenStream,
) {
    let nested_sentinel = sentinel_map(sentinel.0, sentinel.1);
    let mut outer = IndexMap::new();
    outer.insert(
        field_name.to_owned(),
        TheoremValue::Mapping(nested_sentinel),
    );
    let arg = ArgValue::RawMap(outer);
    let result = lower_ok("cfg", &arg, "Config").expect("lower_ok failed");
    assert!(tokens_eq(&result, &expected));
}

#[rstest]
#[case::empty(vec![], quote! { Node {} })]
#[case::single_field(
    vec![("id", TheoremValue::Integer(42))],
    quote! { Node { id: 42 } }
)]
#[case::multiple_fields(
    vec![
        ("id", TheoremValue::Integer(1)),
        ("name", TheoremValue::String("test".to_owned())),
        ("active", TheoremValue::Bool(true)),
    ],
    quote! { Node { id: 1, name: "test", active: true } }
)]
#[case::list_field(
    vec![
        ("id", TheoremValue::Integer(1)),
        (
            "tags",
            TheoremValue::Sequence(vec![
                TheoremValue::String("a".to_owned()),
                TheoremValue::String("b".to_owned()),
            ]),
        ),
    ],
    quote! { Node { id: 1, tags: vec!["a", "b"] } }
)]
fn test_lower_arg_value_map_cases(
    #[case] entries: Vec<(&str, TheoremValue)>,
    #[case] expected_tokens: proc_macro2::TokenStream,
) {
    let mut map = IndexMap::new();
    for (name, value) in entries {
        map.insert(name.to_owned(), value);
    }
    let result = lower_ok("node", &ArgValue::RawMap(map), "Node").expect("lower_ok failed");
    assert!(tokens_eq(&result, &expected_tokens));
}

#[test]
fn test_lower_arg_value_map_with_qualified_type() {
    let mut map = IndexMap::new();
    map.insert("x".to_owned(), TheoremValue::Integer(10));
    let arg = ArgValue::RawMap(map);
    let result = lower_ok("point", &arg, "module::Point").expect("lower_ok failed");
    assert!(tokens_eq(&result, &quote! { module::Point { x: 10 } }));
}

#[rstest]
#[case::simple("MyStruct", "MyStruct")]
#[case::qualified("crate::module::Type", "crate :: module :: Type")]
fn test_extract_type_path_cases(#[case] input: &str, #[case] expected: &str) {
    let ty: syn::Type = syn::parse_str(input).expect("failed to parse type");
    let path = extract_type_path("param", &ty).expect("extraction failed");
    assert_eq!(quote! { #path }.to_string(), expected);
}

#[rstest]
#[case::reference("&MyStruct", "simple type path")]
#[case::generic("Vec<i32>", "generic type paths")]
#[case::qself("<T as Trait>::Assoc", "qualified-self paths")]
#[case::tuple("(i32, i32)", "simple type path")]
fn test_extract_type_path_rejects(#[case] ty_str: &str, #[case] expected_reason: &str) {
    let ty: syn::Type = syn::parse_str(ty_str).expect("failed to parse type");
    let result = extract_type_path("param", &ty);
    assert!(result.is_err());
    if let Err(LoweringError::UnsupportedType { param, reason }) = result {
        assert_eq!(param, "param");
        assert!(
            reason.contains(expected_reason),
            "expected reason to contain {expected_reason:?}, got: {reason}"
        );
    } else {
        panic!("expected UnsupportedType error for type `{ty_str}`");
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

#[test]
fn test_lower_arg_value_sequence_nested_map_fails() {
    // List of structs where each element is a map - nested maps aren't supported yet
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::String("wrong".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(inner)]);
    let ty: syn::Type = syn::parse_str("Vec<Point>").expect("failed to parse type");
    let result = lower_arg_value("points", &arg, &ty);
    match result {
        Err(LoweringError::UnsupportedType { reason, .. }) if reason.contains("nested map") => {
            // Expected error
        }
        Err(err) => panic!("expected UnsupportedType for nested map, got: {err}"),
        Ok(_) => panic!("expected lowering to fail for nested map (not yet supported)"),
    }
}
