//! Unit tests for Rust type lifetime detection.

use super::free_named_lifetime;
use rstest::rstest;

// `dyn for<'a> Fn(&'a crate::Account)` covers the parenthesized-input
// branch of the function-trait scan with an input binder, the mirror of
// the function-pointer binder above.
/// Confirms that higher-ranked lifetimes bound by `for<'a>` are not free.
#[rstest]
#[case("for<'a> fn(&'a crate::Account)")]
#[case("dyn for<'a> Trait<&'a crate::Account>")]
#[case("dyn for<'a> Fn(&'a crate::Account)")]
fn bound_hrtb_lifetimes_are_not_free(#[case] ty: &str) {
    assert_eq!(free_named_lifetime(ty), None);
}

/// Checks that a plain named reference lifetime is reported as free.
#[test]
fn plain_named_reference_lifetime_is_free() {
    assert_eq!(
        free_named_lifetime("&'a crate::Account"),
        Some("a".to_owned())
    );
}

/// Confirms that the reserved lifetimes `'static` and `'_` are not reported.
#[rstest]
#[case("&'static crate::Account")]
#[case("&'_ crate::Account")]
fn reserved_lifetime_names_are_not_free(#[case] ty: &str) {
    assert_eq!(free_named_lifetime(ty), None);
}

/// Finds a free named lifetime when nested in compound Rust type syntax.
#[rstest]
#[case("[&'a crate::Account; 1]")]
#[case("fn() -> &'a crate::Account")]
#[case("impl Trait<&'a crate::Account>")]
#[case("(&'a crate::Account)")]
#[case("*const &'a crate::Account")]
#[case("[&'a crate::Account]")]
#[case("dyn Trait<'a>")]
#[case("dyn Fn(crate::Account) -> &'a crate::Account")]
#[case("dyn Trait<Item = &'a crate::Account>")]
#[case("dyn Trait<Assoc: Other<'a>>")]
fn nested_named_lifetimes_are_free(#[case] ty: &str) {
    assert_eq!(free_named_lifetime(ty), Some("a".to_owned()));
}

// Regression guard for the `syn 3` input-scanning paths: both the function
// pointer and the `Fn`-family parenthesized arguments now store their
// inputs as `NamedArg` rather than a bare `Type`, so a free lifetime in an
// input must still be reported. An implementation that dropped either input
// walk would pass every return-type case above and fail here.
/// Guards lifetime scanning in function-pointer and `Fn`-family inputs.
#[rstest]
#[case("fn(&'a crate::Account)")]
#[case("dyn Fn(&'a crate::Account)")]
fn free_lifetime_in_callable_inputs_is_reported(#[case] ty: &str) {
    assert_eq!(free_named_lifetime(ty), Some("a".to_owned()));
}

/// Confirms that macro type tokens do not contribute scanned lifetimes.
#[test]
fn macro_types_have_no_scanned_lifetimes() {
    assert_eq!(free_named_lifetime("type_macro!()"), None);
}
