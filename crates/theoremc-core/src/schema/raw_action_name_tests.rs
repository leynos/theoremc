//! Unit tests for raw-to-domain action-name conversion.

use googletest::prelude::*;
use proptest::prelude::{ProptestConfig, prop_assert, proptest};
use rstest::rstest;

use super::{RawDocDecodeError, RawTheoremDoc};

const VALID_DOCUMENT: &str = r"
Theorem: RawBoundary
About: Exercises raw conversion
Actions:
  account.deposit:
    params: {}
Do:
  - call:
      action: account.deposit
      args: {}
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
";

fn parse_one_raw_doc(yaml: &str) -> RawTheoremDoc {
    serde_saphyr::from_multiple::<RawTheoremDoc>(yaml)
        .expect("raw document should deserialize")
        .into_iter()
        .next()
        .expect("test input should contain one document")
}

#[rstest]
fn raw_conversion_carries_canonical_action_names_into_domain() {
    let raw_doc = parse_one_raw_doc(VALID_DOCUMENT);
    let doc = raw_doc
        .to_theorem_doc()
        .expect("valid canonical action names should convert");

    let action = doc
        .do_steps
        .first()
        .and_then(|step| match step {
            super::super::types::Step::Call(call) => Some(call.call.action.as_str()),
            _ => None,
        })
        .expect("first step should be a call action");

    assert_that!(doc.actions.get("account.deposit"), some(anything()));
    assert_that!(action, eq("account.deposit"));
}

#[rstest]
fn raw_conversion_rejects_invalid_action_signature_key() {
    let raw_doc = parse_one_raw_doc(&VALID_DOCUMENT.replace("account.deposit:", "deposit:"));
    let error = raw_doc
        .to_theorem_doc()
        .expect_err("invalid Actions key should be rejected");

    assert!(
        matches!(
            &error,
            RawDocDecodeError::ActionSignature { action, .. } if action == "deposit"
        ),
        "expected invalid Actions key error, got: {error}",
    );
    assert_that!(
        error.to_string(),
        contains_substring("must contain at least two dot-separated segments")
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn raw_conversion_rejects_one_invalid_action_name_segment(
        (name, _reason) in crate::canonical_action_name::test_support::one_invalid_segment_strategy(),
    ) {
        let yaml = format!(
            concat!(
                "Theorem: InvalidAction\n",
                "About: raw conversion rejection\n",
                "Actions:\n  {name}:\n    params: {{}}\n",
                "Do:\n  - call:\n      action: {name}\n      args: {{}}\n",
                "Prove:\n  - assert: 'true'\n    because: trivially true\n",
                "Evidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n",
                "Witness:\n  - cover: 'true'\n    because: always reachable\n",
            ),
            name = name,
        );
        let raw_doc = parse_one_raw_doc(&yaml);
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("a malformed action name must be rejected");

        prop_assert!(
            matches!(&error, RawDocDecodeError::ActionSignature { .. }),
            "expected an Actions key rejection, got: {error}",
        );
    }
}

#[rstest]
fn raw_conversion_rejects_invalid_action_call_name() {
    let raw_doc =
        parse_one_raw_doc(&VALID_DOCUMENT.replace("action: account.deposit", "action: deposit"));
    let error = raw_doc
        .to_theorem_doc()
        .expect_err("invalid action call name should be rejected");

    assert!(
        matches!(&error, RawDocDecodeError::DoStep { index: 1, .. }),
        "expected Do step action-name error, got: {error}",
    );
    assert_that!(
        error.to_string(),
        contains_substring("action 'deposit': invalid canonical action name")
    );
}
