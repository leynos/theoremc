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
