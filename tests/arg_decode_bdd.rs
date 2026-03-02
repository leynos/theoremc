//! Behavioural tests for argument value decoding.

mod common;

use common::load_fixture;
use rstest_bdd_macros::{given, scenario, then};
use theoremc::schema::{ArgValue, LiteralValue, load_theorem_docs};

// ── Helpers ─────────────────────────────────────────────────────────

/// Loads a fixture file and returns the decoded documents.
fn load_ok(fixture: &str) -> Result<Vec<theoremc::schema::TheoremDoc>, String> {
    let yaml = load_fixture(fixture).map_err(|e| format!("failed to load fixture: {e}"))?;
    load_theorem_docs(&yaml).map_err(|e| format!("fixture should load: {e}"))
}

/// Loads a fixture file and returns the error string.
fn load_err(fixture: &str) -> Result<String, String> {
    let yaml = load_fixture(fixture).map_err(|e| format!("failed to load fixture: {e}"))?;
    match load_theorem_docs(&yaml) {
        Err(e) => Ok(e.to_string()),
        Ok(_) => Err(format!("fixture {fixture} should fail")),
    }
}

/// Extracts the first action call's args from the first Let binding.
fn first_let_args(
    docs: &[theoremc::schema::TheoremDoc],
) -> Result<&indexmap::IndexMap<String, ArgValue>, String> {
    let doc = docs.first().ok_or("no documents")?;
    let (_, binding) = doc.let_bindings.first().ok_or("no let bindings")?;
    let ac = match binding {
        theoremc::schema::LetBinding::Call(c) => &c.call,
        theoremc::schema::LetBinding::Must(m) => &m.must,
    };
    Ok(&ac.args)
}

// ── Scenario: Plain string arguments are decoded as literals ────────

#[given("a theorem file with plain string arguments")]
fn given_theorem_with_plain_string_args() {}

#[then("loading succeeds and arguments are string literals")]
fn then_args_are_string_literals() -> Result<(), String> {
    let docs = load_ok("valid_arg_string_literal.theorem")?;
    let args = first_let_args(&docs)?;
    let name_arg = args.get("name").ok_or("missing 'name' arg")?;
    let tag_arg = args.get("tag").ok_or("missing 'tag' arg")?;
    if *name_arg != ArgValue::Literal(LiteralValue::String("hello".into())) {
        return Err(format!("expected string literal 'hello', got {name_arg:?}"));
    }
    if *tag_arg != ArgValue::Literal(LiteralValue::String("world".into())) {
        return Err(format!("expected string literal 'world', got {tag_arg:?}"));
    }
    Ok(())
}

// ── Scenario: Explicit ref arguments are decoded as references ──────

#[given("a theorem file with explicit ref arguments")]
fn given_theorem_with_ref_args() {}

#[then("loading succeeds and arguments are variable references")]
fn then_args_are_variable_references() -> Result<(), String> {
    let docs = load_ok("valid_arg_ref.theorem")?;
    let doc = docs.first().ok_or("no documents")?;
    // The second Let binding ("result") uses { ref: graph }.
    let (_, binding) = doc
        .let_bindings
        .get_index(1)
        .ok_or("missing second let binding")?;
    let ac = match binding {
        theoremc::schema::LetBinding::Call(c) => &c.call,
        theoremc::schema::LetBinding::Must(m) => &m.must,
    };
    let target_arg = ac.args.get("target").ok_or("missing 'target' arg")?;
    if *target_arg != ArgValue::Reference("graph".into()) {
        return Err(format!("expected Reference(\"graph\"), got {target_arg:?}"));
    }
    Ok(())
}

// ── Scenario: Integer and boolean arguments are decoded as literals ──

#[given("a theorem file with integer and boolean arguments")]
fn given_theorem_with_mixed_scalar_args() {}

#[then("loading succeeds and arguments are scalar literals")]
fn then_args_are_scalar_literals() -> Result<(), String> {
    let docs = load_ok("valid_arg_mixed_scalars.theorem")?;
    let args = first_let_args(&docs)?;

    let count = args.get("count").ok_or("missing 'count' arg")?;
    if *count != ArgValue::Literal(LiteralValue::Integer(42)) {
        return Err(format!("expected Integer(42), got {count:?}"));
    }

    let enabled = args.get("enabled").ok_or("missing 'enabled' arg")?;
    if *enabled != ArgValue::Literal(LiteralValue::Bool(true)) {
        return Err(format!("expected Bool(true), got {enabled:?}"));
    }

    let label = args.get("label").ok_or("missing 'label' arg")?;
    if *label != ArgValue::Literal(LiteralValue::String("test".into())) {
        return Err(format!("expected String(\"test\"), got {label:?}"));
    }
    Ok(())
}

// ── Scenario: Invalid ref target is rejected ────────────────────────

#[given("a theorem file with an invalid ref target")]
fn given_theorem_with_invalid_ref_target() {}

#[then("loading fails with an actionable error message")]
fn then_loading_fails_with_error() -> Result<(), String> {
    let err_keyword = load_err("invalid_arg_ref_keyword.theorem")?;
    if !err_keyword.contains("Rust reserved keyword") {
        return Err(format!("expected keyword error, got: {err_keyword}"));
    }

    let err_empty = load_err("invalid_arg_ref_empty.theorem")?;
    if !err_empty.contains("must not be empty") {
        return Err(format!("expected empty-ref error, got: {err_empty}"));
    }
    Ok(())
}

// ── Scenario: Adding a binding cannot alter literal semantics ────────

/// Template for a theorem with a Do step that uses a plain string
/// argument. The `Let` section is configurable.
const STABILITY_BASE: &str = r#"
Theorem: Stability
About: Semantic stability of plain string arguments
{LET_SECTION}
Do:
  - call:
      action: label.set
      args:
        param: "x"
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

#[given("a theorem with a plain string argument matching a binding name")]
fn given_theorem_with_matching_binding_name() {}

#[then("the argument remains a string literal regardless of bindings")]
fn then_arg_remains_literal_regardless_of_bindings() -> Result<(), String> {
    // Case 1: no Let bindings — "x" is a string literal.
    let yaml_no_let = STABILITY_BASE.replace("{LET_SECTION}", "");
    let docs_no_let =
        load_theorem_docs(&yaml_no_let).map_err(|e| format!("no-let case failed: {e}"))?;
    let param_no_let = first_do_arg(&docs_no_let, "param")?;
    assert_is_string_literal(param_no_let, "x", "no-let case")?;

    // Case 2: Let binding named "x" exists — "x" is STILL a string
    // literal, NOT a reference.
    let let_section = concat!(
        "Let:\n",
        "  x:\n",
        "    call:\n",
        "      action: other.action\n",
        "      args: {}\n",
    );
    let yaml_with_binding = STABILITY_BASE.replace("{LET_SECTION}", let_section);
    let docs_with_binding = load_theorem_docs(&yaml_with_binding)
        .map_err(|e| format!("with-binding case failed: {e}"))?;
    let param_with_binding = first_do_arg(&docs_with_binding, "param")?;
    assert_is_string_literal(param_with_binding, "x", "with-binding case")?;

    // Case 3: explicit { ref: x } — this IS a reference.
    let yaml_ref = r"
Theorem: StabilityRef
About: Explicit ref produces a reference
Let:
  x:
    call:
      action: other.action
      args: {}
Do:
  - call:
      action: label.set
      args:
        param: { ref: x }
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
    let docs_ref = load_theorem_docs(yaml_ref).map_err(|e| format!("ref case failed: {e}"))?;
    let param_ref = first_do_arg(&docs_ref, "param")?;
    if *param_ref != ArgValue::Reference("x".into()) {
        return Err(format!(
            "ref case: expected Reference(\"x\"), got {param_ref:?}"
        ));
    }

    Ok(())
}

/// Extracts a named arg from the first Do step's action call.
fn first_do_arg<'a>(
    docs: &'a [theoremc::schema::TheoremDoc],
    arg_name: &str,
) -> Result<&'a ArgValue, String> {
    let doc = docs.first().ok_or("no documents")?;
    let step = doc.do_steps.first().ok_or("no do steps")?;
    let ac = match step {
        theoremc::schema::Step::Call(c) => &c.call,
        theoremc::schema::Step::Must(m) => &m.must,
        theoremc::schema::Step::Maybe(_) => return Err("unexpected maybe step".into()),
    };
    ac.args
        .get(arg_name)
        .ok_or_else(|| format!("missing '{arg_name}' arg"))
}

/// Asserts that an `ArgValue` is a string literal with the expected value.
fn assert_is_string_literal(value: &ArgValue, expected: &str, context: &str) -> Result<(), String> {
    if *value != ArgValue::Literal(LiteralValue::String(expected.into())) {
        return Err(format!(
            "{context}: expected Literal(String(\"{expected}\")), got {value:?}"
        ));
    }
    Ok(())
}

// ── Scenario wiring ────────────────────────────────────────────────

#[scenario(
    path = "tests/features/arg_decode.feature",
    name = "Plain string arguments are decoded as literals"
)]
fn plain_string_arguments_are_decoded_as_literals() {}

#[scenario(
    path = "tests/features/arg_decode.feature",
    name = "Explicit ref arguments are decoded as references"
)]
fn explicit_ref_arguments_are_decoded_as_references() {}

#[scenario(
    path = "tests/features/arg_decode.feature",
    name = "Integer and boolean arguments are decoded as literals"
)]
fn integer_and_boolean_arguments_are_decoded_as_literals() {}

#[scenario(
    path = "tests/features/arg_decode.feature",
    name = "Invalid ref target is rejected"
)]
fn invalid_ref_target_is_rejected() {}

#[scenario(
    path = "tests/features/arg_decode.feature",
    name = "Adding a binding cannot alter literal argument semantics"
)]
fn adding_a_binding_cannot_alter_literal_argument_semantics() {}
