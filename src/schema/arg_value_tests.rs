//! Unit tests for argument value decoding.

use indexmap::IndexMap;
use rstest::rstest;

use super::*;
use crate::schema::value::TheoremValue;

// ── Scalar literal decoding ─────────────────────────────────────────

#[rstest]
#[case::plain_string(
    TheoremValue::String("hello".into()),
    ArgValue::Literal(LiteralValue::String("hello".into()))
)]
#[case::boolean_true(TheoremValue::Bool(true), ArgValue::Literal(LiteralValue::Bool(true)))]
#[case::boolean_false(
    TheoremValue::Bool(false),
    ArgValue::Literal(LiteralValue::Bool(false))
)]
#[case::integer(
    TheoremValue::Integer(42),
    ArgValue::Literal(LiteralValue::Integer(42))
)]
#[case::float(
    TheoremValue::Float(99.5),
    ArgValue::Literal(LiteralValue::Float(99.5))
)]
fn scalar_values_decode_as_literals(#[case] input: TheoremValue, #[case] expected: ArgValue) {
    let result = decode_arg_value("param", input);
    assert_eq!(result.expect("should decode"), expected);
}

// ── Valid reference decoding ────────────────────────────────────────

#[rstest]
#[case::simple_name("graph", "graph")]
#[case::underscore_prefix("_private", "_private")]
#[case::with_digits("x42", "x42")]
#[case::single_letter("a", "a")]
fn valid_ref_decodes_as_reference(#[case] name: &str, #[case] expected: &str) {
    let map = IndexMap::from([("ref".to_owned(), TheoremValue::String(name.to_owned()))]);
    let result = decode_arg_value("param", TheoremValue::Mapping(map));
    assert_eq!(
        result.expect("should decode"),
        ArgValue::Reference(expected.to_owned())
    );
}

// ── Invalid reference decoding ──────────────────────────────────────

#[test]
fn empty_ref_name_is_rejected() {
    let map = IndexMap::from([("ref".to_owned(), TheoremValue::String(String::new()))]);
    let err = decode_arg_value("param", TheoremValue::Mapping(map)).expect_err("should fail");
    assert_eq!(
        err,
        ArgDecodeError::EmptyRefTarget {
            param: "param".into()
        }
    );
}

#[rstest]
#[case::keyword_fn("fn")]
#[case::keyword_let("let")]
fn keyword_ref_name_is_rejected(#[case] name: &str) {
    let map = IndexMap::from([("ref".to_owned(), TheoremValue::String(name.to_owned()))]);
    let err = decode_arg_value("param", TheoremValue::Mapping(map)).expect_err("should fail");
    assert_eq!(
        err,
        ArgDecodeError::ReservedKeyword {
            param: "param".into(),
            name: name.into(),
        }
    );
}

#[rstest]
#[case::starts_with_digit("123bad")]
#[case::contains_hyphen("foo-bar")]
fn invalid_identifier_ref_name_is_rejected(#[case] name: &str) {
    let map = IndexMap::from([("ref".to_owned(), TheoremValue::String(name.to_owned()))]);
    let err = decode_arg_value("param", TheoremValue::Mapping(map)).expect_err("should fail");
    assert_eq!(
        err,
        ArgDecodeError::InvalidIdentifier {
            param: "param".into(),
            name: name.into(),
        }
    );
}

#[rstest]
#[case::integer_value(TheoremValue::Integer(42), "an integer")]
#[case::boolean_value(TheoremValue::Bool(true), "a boolean")]
#[case::float_value(TheoremValue::Float(1.0), "a float")]
fn ref_with_non_string_value_is_rejected(
    #[case] value: TheoremValue,
    #[case] expected_kind: &'static str,
) {
    let map = IndexMap::from([("ref".to_owned(), value)]);
    let err = decode_arg_value("param", TheoremValue::Mapping(map)).expect_err("should fail");
    assert_eq!(
        err,
        ArgDecodeError::NonStringRefTarget {
            param: "param".into(),
            kind: expected_kind,
        }
    );
}

// ── Pass-through forms ──────────────────────────────────────────────

#[test]
fn other_single_key_map_is_raw_map() {
    let map = IndexMap::from([("other_key".to_owned(), TheoremValue::String("value".into()))]);
    let result = decode_arg_value("param", TheoremValue::Mapping(map.clone()));
    assert_eq!(result.expect("should decode"), ArgValue::RawMap(map));
}

#[test]
fn multi_key_map_with_ref_is_raw_map() {
    let map = IndexMap::from([
        ("ref".to_owned(), TheoremValue::String("name".into())),
        ("extra".to_owned(), TheoremValue::Integer(1)),
    ]);
    let result = decode_arg_value("param", TheoremValue::Mapping(map.clone()));
    assert_eq!(result.expect("should decode"), ArgValue::RawMap(map));
}

#[test]
fn empty_map_is_raw_map() {
    let map = IndexMap::new();
    let result = decode_arg_value("param", TheoremValue::Mapping(map.clone()));
    assert_eq!(result.expect("should decode"), ArgValue::RawMap(map));
}

#[test]
fn sequence_is_raw_sequence() {
    let seq = vec![TheoremValue::Integer(1), TheoremValue::Integer(2)];
    let result = decode_arg_value("param", TheoremValue::Sequence(seq.clone()));
    assert_eq!(result.expect("should decode"), ArgValue::RawSequence(seq));
}

// ── Error message includes parameter name ───────────────────────────

#[test]
fn error_message_includes_param_name() {
    let map = IndexMap::from([("ref".to_owned(), TheoremValue::String("fn".into()))]);
    let err = decode_arg_value("graph_ref", TheoremValue::Mapping(map)).expect_err("should fail");
    assert_eq!(
        err,
        ArgDecodeError::ReservedKeyword {
            param: "graph_ref".into(),
            name: "fn".into(),
        }
    );
    // Display format also includes the parameter name.
    let msg = err.to_string();
    assert!(
        msg.contains("graph_ref"),
        "expected display to mention 'graph_ref', got: {msg}"
    );
}
