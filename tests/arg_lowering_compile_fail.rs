//! Compile-fail tests for argument lowering.
//!
//! These tests verify that type mismatches in lowered expressions surface
//! as Rust compilation errors, not theoremc validation errors.

use std::fs;
use std::process::Command;

use indexmap::IndexMap;

use theoremc::schema::TheoremValue;
use theoremc::schema::arg_value::ArgValue;

/// Compiles a Rust snippet and returns (success, stderr).
fn compile_snippet(code: &str) -> (bool, String) {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("failed to create temp dir: {e}"));
    let source_path = temp_dir.path().join("test.rs");
    fs::write(&source_path, code).unwrap_or_else(|e| panic!("failed to write test file: {e}"));

    let output = Command::new("rustc")
        .arg(&source_path)
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        // Emit output inside the temp dir so artefacts don't pollute the
        // project root.
        .arg("--out-dir")
        .arg(temp_dir.path())
        .output()
        .unwrap_or_else(|e| panic!("failed to run rustc: {e}"));

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
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

/// Lowers `arg` for `param` against `ty_str`, assembles Rust code via
/// `make_code(tokens_str, ty_str)`, compiles it, and returns `(success, stderr)`.
fn lower_and_compile(
    arg: &ArgValue,
    param: &str,
    ty_str: &str,
    make_code: impl FnOnce(&str, &str) -> String,
) -> (bool, String) {
    let ty = syn::parse_str(ty_str).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let tokens = theoremc::arg_lowering::lower_arg_value(param, arg, &ty)
        .unwrap_or_else(|e| panic!("lowering failed: {e}"));
    compile_snippet(&make_code(&tokens.to_string(), ty_str))
}

/// Helper: lowers an [`ArgValue`] and asserts it compiles successfully.
fn assert_lowers_and_compiles(arg: &ArgValue, param: &str, ty_str: &str) {
    let (success, stderr) = lower_and_compile(arg, param, ty_str, wrap_in_harness);
    assert!(
        success,
        "expected valid code to compile, but got errors:\n{stderr}"
    );
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts it compiles.
fn assert_lowers_and_compiles_with_struct(
    arg: &ArgValue,
    param: &str,
    ty_str: &str,
    struct_def: &str,
) {
    let (success, stderr) = lower_and_compile(arg, param, ty_str, |expr, ty| {
        format!(
            "#![allow(unused)]\n{struct_def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n"
        )
    });
    assert!(
        success,
        "expected valid struct literal to compile, but got errors:\n{stderr}"
    );
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts compilation fails,
/// with at least one of the expected fragments present in stderr.
fn assert_lowers_and_compile_fails_with_struct(
    arg: &ArgValue,
    param: &str,
    ty_and_struct: (&str, &str),
    expected_fragments: &[&str],
) {
    let (ty_str, struct_def) = ty_and_struct;
    let (success, stderr) = lower_and_compile(arg, param, ty_str, |expr, ty| {
        format!(
            "#![allow(unused)]\n{struct_def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n"
        )
    });
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
    assert_lowers_and_compiles(&arg, "x", "i32");
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
        &ArgValue::RawMap(map),
        "node",
        ("Node", "struct Node { id: i32 }"),
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
    let ty = syn::parse_str("Vec<i32>").unwrap_or_else(|e| panic!("parse failed: {e}"));
    let tokens = theoremc::arg_lowering::lower_arg_value("nums", &arg, &ty)
        .unwrap_or_else(|e| panic!("lowering failed: {e}"));
    let code = wrap_in_harness(&tokens.to_string(), "Vec<i32>");

    let (success, stderr) = compile_snippet(&code);
    assert!(
        !success,
        "expected compilation to fail for element type mismatch"
    );
    assert!(
        stderr.contains("mismatched types") || stderr.contains("expected integer"),
        "expected Rust type error, got:\n{stderr}"
    );
}

#[test]
fn compile_fail_unknown_struct_field() {
    // YAML provides a field that doesn't exist in the struct.
    let mut map = IndexMap::new();
    map.insert("unknown_field".to_owned(), TheoremValue::Integer(42));
    assert_lowers_and_compile_fails_with_struct(
        &ArgValue::RawMap(map),
        "node",
        ("Node", "struct Node { id: i32 }"),
        &["has no field named `unknown_field`", "E0560"],
    );
}

#[test]
fn compile_fail_nested_mismatch_in_list_of_structs() {
    // List of structs where one field has wrong type.
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::String("wrong".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(inner)]);
    let ty = syn::parse_str("Vec<Point>").unwrap_or_else(|e| panic!("parse failed: {e}"));

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
        &arg,
        "node",
        "Node",
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
    assert_lowers_and_compiles(&arg, "nums", "Vec<i32>");
}
