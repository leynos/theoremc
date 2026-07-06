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

const INVALID_ACTION_TYPE_YAML: &str = r"
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

const FREE_LIFETIME_ACTION_YAML: &str = r#"
Theorem: InvalidActionLifetime
About: Declares an unbound action lifetime
Actions:
  account.deposit:
    params:
      account: "&'a crate::Account"
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
"#;

const FREE_LIFETIME_ACTION_RETURN_YAML: &str = r#"
Theorem: InvalidActionReturnLifetime
About: Declares an unbound action return lifetime
Actions:
  account.deposit:
    returns: "&'a crate::DepositOutcome"
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
"#;

#[rstest]
#[case(
    INVALID_ACTION_TYPE_YAML,
    "Actions entry 'account.deposit': amount type is not a valid Rust type"
)]
#[case(
    FREE_LIFETIME_ACTION_YAML,
    "Actions entry 'account.deposit': account type contains a free named lifetime parameter 'a'"
)]
#[case(
    FREE_LIFETIME_ACTION_RETURN_YAML,
    "Actions entry 'account.deposit': returns type contains a free named lifetime parameter 'a'"
)]
fn invalid_action_type_or_free_lifetime_is_rejected(
    #[case] yaml: &str,
    #[case] expected_fragment: &str,
) {
    assert_parse_error_contains(yaml, expected_fragment);
}
