//! Unit tests for raw action conversion.

use googletest::prelude::*;
use indexmap::IndexMap;

use super::*;

fn ref_arg(value: TheoremValue) -> TheoremValue {
    TheoremValue::Mapping(IndexMap::from([("ref".to_owned(), value)]))
}

fn action_with_arg(arg_name: &str, value: TheoremValue) -> RawActionCall {
    RawActionCall {
        action: "account.deposit".to_owned(),
        args: IndexMap::from([(arg_name.to_owned(), value)]),
        as_binding: None,
    }
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
        eq(&ArgDecodeError::EmptyRefTarget {
            param: "maybe.do step 1: account".to_owned(),
        }),
    );
    pretty_assertions::assert_eq!(
        error.to_string(),
        "argument 'maybe.do step 1: account': ref value must not be empty",
    );
}
