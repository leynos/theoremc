//! Loader tests for source-aware parsing and validation diagnostics.

use std::error::Error;

use rstest::rstest;

use super::{SourceId, load_theorem_docs_with_source};

#[rstest]
fn parse_diagnostics_include_explicit_source() {
    let yaml = "Theorem: T\nAbout: bad\nUnknown: key\n";
    let result = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_unknown_key.theorem"),
        yaml,
    );
    assert!(result.is_err(), "fixture should fail parsing");

    let error = result.expect_err("error expected");
    let diagnostic = error.diagnostic().expect("diagnostic expected");
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_unknown_key.theorem"
    );
    assert_eq!(diagnostic.location.line, 3);
    assert_eq!(diagnostic.location.column, 1);
}

#[rstest]
fn validation_diagnostics_include_source_and_location() {
    let yaml = r"
Theorem: InvalidAbout
About: ''
Prove:
  - assert: 'true'
    because: trivial
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: reachable
";
    let result = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_empty_about.theorem"),
        yaml,
    );
    assert!(result.is_err(), "fixture should fail validation");

    let error = result.expect_err("error expected");
    let diagnostic = error.diagnostic().expect("diagnostic expected");
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_empty_about.theorem"
    );
    assert!(diagnostic.location.line > 0);
    assert!(diagnostic.location.column > 0);
}

#[rstest]
fn decode_failures_preserve_source_error() {
    let yaml = r"
Theorem: InvalidRef
About: Invalid ref target
Let:
  y:
    call:
      action: account.deposit
      args:
        target: 1

  x:
    call:
      action: account.deposit
      args:
        target:
          ref: 'not valid'
Prove:
  - assert: 'true'
    because: trivial
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: reachable
";
    let error = load_theorem_docs_with_source(
        &SourceId::new("tests/fixtures/invalid_ref_target.theorem"),
        yaml,
    )
    .expect_err("invalid ref target should fail decoding");

    let source = error.source().expect("decode failure should be preserved");
    let diagnostic = error.diagnostic().expect("diagnostic expected");

    assert!(
        source.to_string().contains("Let binding 'x'"),
        "unexpected source error: {source}"
    );
    assert_eq!(
        diagnostic.location.source,
        "tests/fixtures/invalid_ref_target.theorem"
    );
    assert_eq!(diagnostic.location.line, 15);
}

/// An invalid action name must point at the rejected value itself.
///
/// The typed `Spanned` location decides this, so the case set covers the
/// shapes a line-oriented rescan cannot follow: lowercase section aliases,
/// steps nested inside `maybe.do`, and a flow mapping.
#[rstest]
#[case::titlecase_let(
    concat!(
        "Theorem: BadActionName\n",
        "About: Invalid action name\n",
        "Let:\n",
        "  x:\n",
        "    call:\n",
        "      action: deposit\n",
        "      args: {}\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivial\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    ),
    6,
    15
)]
#[case::lowercase_aliases(
    concat!(
        "theorem: BadActionName\n",
        "about: Invalid action name\n",
        "do:\n",
        "  - call:\n",
        "      action: deposit\n",
        "      args: {}\n",
        "prove:\n",
        "  - assert: 'true'\n",
        "    because: trivial\n",
        "evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    ),
    5,
    15
)]
#[case::nested_maybe_do(
    concat!(
        "Theorem: BadActionName\n",
        "About: Invalid action name\n",
        "Do:\n",
        "  - maybe:\n",
        "      because: branch\n",
        "      do:\n",
        "        - call:\n",
        "            action: deposit\n",
        "            args: {}\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivial\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    ),
    8,
    21
)]
#[case::flow_mapping(
    concat!(
        "Theorem: BadActionName\n",
        "About: Invalid action name\n",
        "Do:\n",
        "  - call: {action: deposit, args: {}}\n",
        "Prove:\n",
        "  - assert: 'true'\n",
        "    because: trivial\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
        "Witness:\n",
        "  - cover: 'true'\n",
        "    because: reachable\n",
    ),
    4,
    20
)]
fn invalid_action_names_locate_the_action_value(
    #[case] yaml: &str,
    #[case] expected_line: usize,
    #[case] expected_column: usize,
) {
    let error = load_theorem_docs_with_source(&SourceId::new("bad_action.theorem"), yaml)
        .expect_err("malformed action name should fail decoding");

    let diagnostic = error.diagnostic().expect("diagnostic expected");

    assert_eq!(
        diagnostic.code,
        super::SchemaDiagnosticCode::ValidationFailure
    );
    assert_eq!(diagnostic.location.source, "bad_action.theorem");
    assert_eq!(
        diagnostic.location.line, expected_line,
        "diagnostic should point at the action value: {diagnostic:?}"
    );
    assert_eq!(
        diagnostic.location.column, expected_column,
        "diagnostic should point at the action value: {diagnostic:?}"
    );
}
