//! Unit tests for schema domain types.

use indexmap::IndexMap;

use super::ActionSignature;

fn signature(params: &[(&str, &str)], returns: &str) -> ActionSignature {
    let mut map = IndexMap::new();
    for (name, ty) in params {
        map.insert((*name).to_owned(), (*ty).to_owned());
    }
    ActionSignature {
        params: map,
        returns: returns.to_owned(),
    }
}

#[test]
fn whitespace_only_difference_is_equivalent() {
    let a = signature(&[("v", "Vec<u8>")], "u64");
    let b = signature(&[("v", "Vec <u8>")], "u64 ");
    assert!(a.is_semantically_equivalent(&b));
    assert!(b.is_semantically_equivalent(&a));
}

#[test]
fn parameter_name_drift_is_not_equivalent() {
    let a = signature(&[("v", "u64")], "()");
    let b = signature(&[("w", "u64")], "()");
    assert!(!a.is_semantically_equivalent(&b));
}

#[test]
fn parameter_order_difference_is_not_equivalent() {
    let a = signature(&[("a", "u64"), ("b", "u32")], "()");
    let b = signature(&[("b", "u32"), ("a", "u64")], "()");
    assert!(!a.is_semantically_equivalent(&b));
}

#[test]
fn return_type_difference_is_not_equivalent() {
    let a = signature(&[("v", "u64")], "u64");
    let b = signature(&[("v", "u64")], "u32");
    assert!(!a.is_semantically_equivalent(&b));
}

#[test]
fn malformed_types_fall_back_to_trimmed_equality() {
    let a = signature(&[("v", "::not a type::")], "()");
    let b = signature(&[("v", "  ::not a type::  ")], "()");
    assert!(a.is_semantically_equivalent(&b));
}
