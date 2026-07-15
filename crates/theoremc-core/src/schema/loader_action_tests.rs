//! Action signature loading tests.

use rstest::*;

use super::load_theorem_docs;

fn assert_parse_error_contains(yaml: &str, expected_substring: &str) {
    let error = load_theorem_docs(yaml).expect_err("expected parser to reject fixture");
    let message = error.to_string();
    assert!(
        message.contains(expected_substring),
        "expected parse error to contain '{expected_substring}', got: {message}"
    );
}

#[rstest]
fn action_signatures_parse_with_ordered_params_and_default_return() {
    let yaml = r"
Theorem: HasActions
About: Declares action signatures
Actions:
  account.deposit:
    params:
      account: '&mut crate::account::Account'
      amount: u64
Let:
  updated:
    call:
      action: account.deposit
      args:
        account: { ref: account }
        amount: { ref: amount }
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
    let docs = load_theorem_docs(yaml).expect("should parse action signature");
    let signature = docs[0]
        .actions
        .get("account.deposit")
        .expect("signature should be present");

    assert_eq!(signature.returns, "()");
    assert_eq!(
        signature
            .params
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["account", "amount"]
    );
}

#[rstest]
fn missing_action_signature_for_referenced_action_is_rejected() {
    let yaml = r"
Theorem: MissingSignature
About: References an undeclared action
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

    assert_parse_error_contains(
        yaml,
        "referenced action 'account.deposit' is missing an Actions signature entry",
    );
}

#[rstest]
fn invalid_action_signature_type_is_rejected() {
    let yaml = r"
Theorem: InvalidSignature
About: Declares an invalid action signature
Actions:
  account.deposit:
    params:
      amount: 'not a type %'
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

    assert_parse_error_contains(
        yaml,
        "Actions entry 'account.deposit': amount type is not a valid Rust type",
    );
}
