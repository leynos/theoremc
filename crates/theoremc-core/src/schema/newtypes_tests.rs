//! Tests for the shared identifier-deserialization helper.
//!
//! Both identifier newtypes delegate their `Deserialize` implementation to one
//! macro, so these cases pin the contract that macro must keep: accept Rust
//! identifiers, reject everything else, and report the rejection as a serde
//! error rather than panicking.

use rstest::rstest;

use super::{ForallVar, TheoremName};

#[rstest]
#[case("account", "account")]
#[case("_private", "_private")]
#[case("has1digit", "has1digit")]
fn theorem_names_accept_rust_identifiers(#[case] input: &str, #[case] expected: &str) {
    let name: TheoremName =
        serde_saphyr::from_str(input).expect("a Rust identifier should deserialize");

    assert_eq!(name.as_str(), expected);
}

#[rstest]
#[case("1starts_with_digit")]
#[case("has-dash")]
#[case("type")]
fn theorem_names_reject_non_identifiers(#[case] input: &str) {
    let result: Result<TheoremName, _> = serde_saphyr::from_str(input);

    assert!(
        result.is_err(),
        "'{input}' must not deserialize as a theorem name"
    );
}

#[rstest]
#[case("account", "account")]
#[case("_private", "_private")]
fn forall_vars_accept_rust_identifiers(#[case] input: &str, #[case] expected: &str) {
    let var: ForallVar =
        serde_saphyr::from_str(input).expect("a Rust identifier should deserialize");

    assert_eq!(var.as_str(), expected);
}

#[rstest]
#[case("1starts_with_digit")]
#[case("has-dash")]
#[case("type")]
fn forall_vars_reject_non_identifiers(#[case] input: &str) {
    let result: Result<ForallVar, _> = serde_saphyr::from_str(input);

    assert!(
        result.is_err(),
        "'{input}' must not deserialize as a quantified variable"
    );
}
