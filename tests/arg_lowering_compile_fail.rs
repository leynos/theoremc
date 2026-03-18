//! Compile-fail tests for argument lowering.
//!
//! These tests verify that type mismatches in lowered expressions surface
//! as Rust compilation errors, not theoremc validation errors.

use std::fs;
use std::process::Command;

use indexmap::IndexMap;

use theoremc::schema::TheoremValue;
use theoremc::schema::arg_value::ArgValue;

/// Bundles the three inputs that every lowering call requires, reducing
/// string-heavy argument lists.
#[derive(Clone, Copy)]
struct LoweringInput<'a> {
    arg: &'a ArgValue,
    param: &'a str,
    ty_str: &'a str,
}

/// Compiles a Rust snippet and returns `(success, stderr)`.
fn compile_snippet(code: &str) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let source_path = temp_dir.path().join("test.rs");
    fs::write(&source_path, code)?;

    let output = Command::new("rustc")
        .arg(&source_path)
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        // Emit output inside the temp dir so artefacts don't pollute the
        // project root.
        .arg("--out-dir")
        .arg(temp_dir.path())
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((output.status.success(), stderr))
}

/// Generates a simple test harness for a lowered expression.
fn wrap_in_harness(expr: &str, expected_type: &str) -> String {
    format!(
        r"
#![allow(unused)]
pub fn test_harness() {{
    let _value: {expected_type} = {expr};
}}
"
    )
}

/// Lowers an [`ArgValue`] via [`LoweringInput`], assembles Rust code via
/// `make_code(tokens_str, ty_str)`, compiles it, and returns `(success, stderr)`.
fn lower_and_compile(
    input: LoweringInput<'_>,
    make_code: impl FnOnce(&str, &str) -> String,
) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let ty: syn::Type = syn::parse_str(input.ty_str)?;
    let tokens = theoremc::arg_lowering::lower_arg_value(input.param, input.arg, &ty)?;
    compile_snippet(&make_code(&tokens.to_string(), input.ty_str))
}

/// Unwraps a `lower_and_compile` result, panicking with a descriptive message
/// on infrastructure errors (not compile failures — those are expected).
fn unwrap_lower_and_compile(
    result: Result<(bool, String), Box<dyn std::error::Error>>,
) -> (bool, String) {
    match result {
        Ok(pair) => pair,
        Err(e) => panic!("lower_and_compile infrastructure error: {e}"),
    }
}

/// Helper: lowers an [`ArgValue`] and asserts it compiles successfully.
fn assert_lowers_and_compiles(input: LoweringInput<'_>) {
    let (success, stderr) = unwrap_lower_and_compile(lower_and_compile(input, wrap_in_harness));
    assert!(
        success,
        "expected valid code to compile, but got errors:\n{stderr}"
    );
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts it compiles.
fn assert_lowers_and_compiles_with_struct(input: LoweringInput<'_>, struct_def: &str) {
    let (success, stderr) = unwrap_lower_and_compile(lower_and_compile(input, |expr, ty| {
        format!(
            "#![allow(unused)]\n{struct_def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n"
        )
    }));
    assert!(
        success,
        "expected valid struct literal to compile, but got errors:\n{stderr}"
    );
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts compilation fails,
/// with at least one of the expected fragments present in stderr.
fn assert_lowers_and_compile_fails_with_struct(
    input: LoweringInput<'_>,
    struct_def: &str,
    expected_fragments: &[&str],
) {
    let (success, stderr) = unwrap_lower_and_compile(lower_and_compile(input, |expr, ty| {
        format!(
            "#![allow(unused)]\n{struct_def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n"
        )
    }));
    assert!(!success, "expected compilation to fail");
    assert!(
        expected_fragments.iter().any(|f| stderr.contains(f)),
        "expected one of {expected_fragments:?} in stderr, got:\n{stderr}"
    );
}

/// Helper: lowers an [`ArgValue`] and asserts compilation fails, with at least
/// one of `expected_fragments` present in stderr.
fn assert_lowers_and_compile_fails(input: LoweringInput<'_>, expected_fragments: &[&str]) {
    let (success, stderr) = unwrap_lower_and_compile(lower_and_compile(input, wrap_in_harness));
    assert!(!success, "expected compilation to fail");
    assert!(
        expected_fragments.iter().any(|f| stderr.contains(f)),
        "expected one of {expected_fragments:?} in stderr, got:\n{stderr}"
    );
}

#[test]
fn positive_control_scalar_compiles() {
    // This test verifies our compile harness works by checking a valid case compiles.
    let arg = ArgValue::Literal(theoremc::schema::arg_value::LiteralValue::Integer(42));
    assert_lowers_and_compiles(LoweringInput {
        arg: &arg,
        param: "x",
        ty_str: "i32",
    });
}

#[test]
fn compile_fail_wrong_scalar_type_in_struct_field() {
    // YAML provides an integer for a field that expects a string.
    // The generated code should fail Rust compilation.
    let mut map = IndexMap::new();
    map.insert(
        "id".to_owned(),
        TheoremValue::String("not_an_int".to_owned()),
    );
    assert_lowers_and_compile_fails_with_struct(
        LoweringInput {
            arg: &ArgValue::RawMap(map),
            param: "node",
            ty_str: "Node",
        },
        "struct Node { id: i32 }",
        &["mismatched types", "expected `i32`, found `&str`"],
    );
}

#[test]
fn compile_fail_wrong_list_element_type() {
    // YAML provides a list of strings where integers are expected.
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::String("a".to_owned()),
        TheoremValue::String("b".to_owned()),
    ]);
    assert_lowers_and_compile_fails(
        LoweringInput {
            arg: &arg,
            param: "nums",
            ty_str: "Vec<i32>",
        },
        &["mismatched types", "expected integer"],
    );
}

#[test]
fn compile_fail_unknown_struct_field() {
    // YAML provides a field that doesn't exist in the struct.
    let mut map = IndexMap::new();
    map.insert("unknown_field".to_owned(), TheoremValue::Integer(42));
    assert_lowers_and_compile_fails_with_struct(
        LoweringInput {
            arg: &ArgValue::RawMap(map),
            param: "node",
            ty_str: "Node",
        },
        "struct Node { id: i32 }",
        &["has no field named `unknown_field`", "E0560"],
    );
}

#[test]
fn compile_fail_nested_mismatch_in_list_of_structs() {
    // List of structs where one field has wrong type.
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::String("wrong".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(inner)]);
    let ty: syn::Type = match syn::parse_str("Vec<Point>") {
        Ok(ty) => ty,
        Err(e) => panic!("failed to parse type: {e}"),
    };

    // This should fail during lowering because nested maps aren't supported yet
    let result = theoremc::arg_lowering::lower_arg_value("points", &arg, &ty);
    assert!(
        result.is_err(),
        "expected lowering to fail for nested map (not yet supported)"
    );
}

#[test]
fn positive_control_struct_compiles() {
    // Valid struct literal should compile.
    let mut map = IndexMap::new();
    map.insert("id".to_owned(), TheoremValue::Integer(1));
    map.insert("name".to_owned(), TheoremValue::String("test".to_owned()));
    let arg = ArgValue::RawMap(map);
    assert_lowers_and_compiles_with_struct(
        LoweringInput {
            arg: &arg,
            param: "node",
            ty_str: "Node",
        },
        "struct Node { id: i32, name: &'static str }",
    );
}

#[test]
fn positive_control_list_compiles() {
    // Valid list should compile.
    let arg = ArgValue::RawSequence(vec![
        TheoremValue::Integer(1),
        TheoremValue::Integer(2),
        TheoremValue::Integer(3),
    ]);
    assert_lowers_and_compiles(LoweringInput {
        arg: &arg,
        param: "nums",
        ty_str: "Vec<i32>",
    });
}
