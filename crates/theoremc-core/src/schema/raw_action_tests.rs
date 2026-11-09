//! Unit tests for raw action conversion.

use googletest::prelude::*;
use indexmap::IndexMap;
use rstest::rstest;

use super::*;

fn ref_arg(value: TheoremValue) -> TheoremValue {
    TheoremValue::Mapping(IndexMap::from([("ref".to_owned(), value)]))
}

fn action_with_arg(arg_name: &str, value: TheoremValue) -> RawActionCall {
    RawActionCall {
        // These conversion tests do not depend on source locations, so the
        // unknown sentinel keeps the fixture independent of the YAML parser.
        action: Spanned::new(
            "account.deposit".to_owned(),
            Location::UNKNOWN,
            Location::UNKNOWN,
        ),
        args: IndexMap::from([(arg_name.to_owned(), value)]),
        as_binding: None,
    }
}

// ── Single-key dispatch ─────────────────────────────────────────────

#[rstest]
#[case::two_supported_keys(
    concat!(
        "call:\n  action: account.deposit\n  args: {}\n",
        "must:\n  action: account.deposit\n  args: {}\n",
    ),
    "mapping must contain exactly one"
)]
#[case::extra_key_after_call(
    concat!(
        "call:\n  action: account.deposit\n  args: {}\n",
        "spurious: true\n",
    ),
    "mapping must contain exactly one"
)]
#[case::unknown_key(
    "invoke:\n  action: account.deposit\n  args: {}\n",
    "unknown variant `invoke`"
)]
#[case::empty_mapping("{}", "expected a mapping with exactly one key")]
fn raw_let_binding_rejects_malformed_mappings(#[case] yaml: &str, #[case] expected: &str) {
    let error = serde_saphyr::from_str::<RawLetBinding>(yaml)
        .expect_err("malformed binding mapping should be rejected");

    assert_that!(error.to_string(), contains_substring(expected));
}

#[rstest]
#[case::two_supported_keys(
    concat!(
        "call:\n  action: account.deposit\n  args: {}\n",
        "maybe:\n  because: nested\n  do:\n",
        "    - call:\n        action: account.deposit\n        args: {}\n",
    ),
    "mapping must contain exactly one"
)]
#[case::extra_key_after_must(
    concat!(
        "must:\n  action: account.deposit\n  args: {}\n",
        "spurious: true\n",
    ),
    "mapping must contain exactly one"
)]
#[case::unknown_key(
    "perform:\n  action: account.deposit\n  args: {}\n",
    "unknown variant `perform`"
)]
#[case::empty_mapping("{}", "expected a mapping with exactly one key")]
fn raw_step_rejects_malformed_mappings(#[case] yaml: &str, #[case] expected: &str) {
    let error = serde_saphyr::from_str::<RawStep>(yaml)
        .expect_err("malformed step mapping should be rejected");

    assert_that!(error.to_string(), contains_substring(expected));
}

#[rstest]
#[case::let_call("call:\n  action: account.deposit\n  args: {}\n")]
#[case::let_must("must:\n  action: account.deposit\n  args: {}\n")]
fn raw_let_binding_accepts_a_single_supported_key(#[case] yaml: &str) {
    let binding = serde_saphyr::from_str::<RawLetBinding>(yaml)
        .expect("a single supported key should decode");

    let action = match binding {
        RawLetBinding::Call(call) => call.call.action.value,
        RawLetBinding::Must(must) => must.must.action.value,
    };
    assert_that!(action.as_str(), eq("account.deposit"));
}

#[rstest]
#[case::step_call("call:\n  action: account.deposit\n  args: {}\n")]
#[case::step_must("must:\n  action: account.deposit\n  args: {}\n")]
fn raw_step_accepts_a_single_supported_key(#[case] yaml: &str) {
    let step =
        serde_saphyr::from_str::<RawStep>(yaml).expect("a single supported key should decode");

    assert!(
        matches!(step, RawStep::Call(_) | RawStep::Must(_)),
        "expected a single call or must step",
    );
}

#[test]
fn nested_maybe_do_decode_error_includes_step_prefix() {
    let step = RawStep::Maybe(RawStepMaybe {
        maybe: RawMaybeBlock {
            because: "branch reason".to_owned(),
            do_steps: vec![RawStep::Call(RawStepCall {
                call: action_with_arg("account", ref_arg(TheoremValue::String(String::new()))),
            })],
        },
    });

    let error = convert_step(&step).expect_err("empty reference should fail");

    assert_that!(
        error,
        eq(&RawActionCallDecodeError::Arg {
            source: ArgDecodeError::EmptyRefTarget {
                param: "maybe.do step 1: account".to_owned(),
            },
        }),
    );
    pretty_assertions::assert_eq!(
        error.to_string(),
        "argument 'maybe.do step 1: account': ref value must not be empty",
    );
}
