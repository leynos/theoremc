//! Unit tests for raw-to-domain action-name conversion.

use googletest::prelude::*;
use proptest::prelude::{ProptestConfig, prop_assert, prop_assert_eq, proptest};
use rstest::rstest;

use crate::canonical_action_name::InvalidCanonicalActionName;

use super::super::raw_action::RawActionCallDecodeError;
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

/// A complete document whose `Actions` key is `name`.
fn document_with_actions_key(name: &str) -> String {
    format!(
        concat!(
            "Theorem: InvalidAction\n",
            "About: raw conversion rejection\n",
            "Actions:\n  {name}:\n    params: {{}}\n",
            "Prove:\n  - assert: 'true'\n    because: trivially true\n",
            "Evidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n",
            "Witness:\n  - cover: 'true'\n    because: always reachable\n",
        ),
        name = name,
    )
}

/// A complete document whose only `Do` call uses `name`.
///
/// The document deliberately declares no `Actions` mapping, so the `Do` call
/// is the only place the generated name is converted and the failure cannot be
/// attributed to an earlier `Actions` key check.
fn document_with_do_call(name: &str) -> String {
    format!(
        concat!(
            "Theorem: InvalidAction\n",
            "About: raw conversion rejection\n",
            "Do:\n  - call:\n      action: {name}\n      args: {{}}\n",
            "Prove:\n  - assert: 'true'\n    because: trivially true\n",
            "Evidence:\n  kani:\n    unwind: 1\n    expect: SUCCESS\n",
            "Witness:\n  - cover: 'true'\n    because: always reachable\n",
        ),
        name = name,
    )
}

/// The typed validation failure an `Actions` key rejection carries.
fn actions_key_error(error: &RawDocDecodeError) -> Option<&InvalidCanonicalActionName> {
    match error {
        RawDocDecodeError::ActionSignature { source, .. } => Some(source),
        RawDocDecodeError::LetBinding { .. } | RawDocDecodeError::DoStep { .. } => None,
    }
}

/// The typed validation failure a first-`Do`-step action-name rejection
/// carries.
fn do_step_action_name_error(error: &RawDocDecodeError) -> Option<&InvalidCanonicalActionName> {
    match error {
        RawDocDecodeError::DoStep {
            index: 1,
            source: step_error,
        } => match step_error.as_ref() {
            RawActionCallDecodeError::InvalidActionName { source, .. } => Some(source),
            RawActionCallDecodeError::Arg { .. } => None,
        },
        RawDocDecodeError::ActionSignature { .. }
        | RawDocDecodeError::LetBinding { .. }
        | RawDocDecodeError::DoStep { .. } => None,
    }
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
    fn raw_conversion_rejects_one_invalid_actions_key_segment(
        (name, expected) in crate::canonical_action_name::test_support::one_invalid_segment_strategy(),
    ) {
        let raw_doc = parse_one_raw_doc(&document_with_actions_key(&name));
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("a malformed Actions key must be rejected");

        prop_assert!(
            matches!(&error, RawDocDecodeError::ActionSignature { .. }),
            "expected an Actions key rejection, got: {error}",
        );
        prop_assert_eq!(
            actions_key_error(&error).map(InvalidCanonicalActionName::reason),
            Some(&expected),
        );
    }

    #[test]
    fn raw_conversion_rejects_one_invalid_do_call_segment(
        (name, expected) in crate::canonical_action_name::test_support::one_invalid_segment_strategy(),
    ) {
        let raw_doc = parse_one_raw_doc(&document_with_do_call(&name));
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("a malformed Do call name must be rejected");

        prop_assert!(
            matches!(&error, RawDocDecodeError::DoStep { .. }),
            "expected a Do step rejection, got: {error}",
        );
        prop_assert_eq!(
            do_step_action_name_error(&error).map(InvalidCanonicalActionName::reason),
            Some(&expected),
        );
    }

    #[test]
    fn raw_conversion_reports_the_first_invalid_actions_key_segment(
        (name, expected) in
            crate::canonical_action_name::test_support::multiple_invalid_segments_strategy(),
    ) {
        let raw_doc = parse_one_raw_doc(&document_with_actions_key(&name));
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("a malformed Actions key must be rejected");

        prop_assert_eq!(
            actions_key_error(&error).map(InvalidCanonicalActionName::reason),
            Some(&expected),
            "the earliest malformed Actions key segment must supply the reason",
        );
    }

    #[test]
    fn raw_conversion_reports_the_first_invalid_do_call_segment(
        (name, expected) in
            crate::canonical_action_name::test_support::multiple_invalid_segments_strategy(),
    ) {
        let raw_doc = parse_one_raw_doc(&document_with_do_call(&name));
        let error = raw_doc
            .to_theorem_doc()
            .expect_err("a malformed Do call name must be rejected");

        prop_assert_eq!(
            do_step_action_name_error(&error).map(InvalidCanonicalActionName::reason),
            Some(&expected),
            "the earliest malformed Do call segment must supply the reason",
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
