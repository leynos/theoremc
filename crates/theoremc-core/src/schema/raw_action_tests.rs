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

/// The variant a single-key binding or step decoded to.
///
/// Keeping the variant explicit lets the accept tests reject an absent entry
/// and, for steps, `RawStep::Maybe`, rather than accepting anything that is
/// merely not the other supported key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SingleKey {
    Call,
    Must,
}

impl From<&RawLetBinding> for SingleKey {
    fn from(binding: &RawLetBinding) -> Self {
        match binding {
            RawLetBinding::Call(_) => Self::Call,
            RawLetBinding::Must(_) => Self::Must,
        }
    }
}

/// The supported variant a raw step decoded to, or `None` for `maybe`.
fn step_key(step: &RawStep) -> Option<SingleKey> {
    match step {
        RawStep::Call(_) => Some(SingleKey::Call),
        RawStep::Must(_) => Some(SingleKey::Must),
        RawStep::Maybe(_) => None,
    }
}

/// The action name carried by a `call` or `must` step.
fn step_action(step: &RawStep) -> Option<&Spanned<String>> {
    match step {
        RawStep::Call(call) => Some(&call.call.action),
        RawStep::Must(must) => Some(&must.must.action),
        RawStep::Maybe(_) => None,
    }
}

#[rstest]
#[case::let_call("call:\n  action: account.deposit\n  args: {}\n", SingleKey::Call)]
#[case::let_must("must:\n  action: account.deposit\n  args: {}\n", SingleKey::Must)]
fn raw_let_binding_accepts_a_single_supported_key(#[case] yaml: &str, #[case] expected: SingleKey) {
    let binding = serde_saphyr::from_str::<RawLetBinding>(yaml)
        .expect("a single supported key should decode");

    assert_eq!(
        SingleKey::from(&binding),
        expected,
        "the decoded variant should follow the mapping key"
    );

    let action = match &binding {
        RawLetBinding::Call(call) => &call.call.action,
        RawLetBinding::Must(must) => &must.must.action,
    };
    assert_that!(action.value.as_str(), eq("account.deposit"));
}

#[rstest]
#[case::step_call("call:\n  action: account.deposit\n  args: {}\n", SingleKey::Call)]
#[case::step_must("must:\n  action: account.deposit\n  args: {}\n", SingleKey::Must)]
fn raw_step_accepts_a_single_supported_key(#[case] yaml: &str, #[case] expected: SingleKey) {
    let step =
        serde_saphyr::from_str::<RawStep>(yaml).expect("a single supported key should decode");

    assert_eq!(
        step_key(&step),
        Some(expected),
        "the decoded variant should follow the mapping key"
    );

    let action = step_action(&step).expect("call and must steps carry an action");
    assert_that!(action.value.as_str(), eq("account.deposit"));
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
