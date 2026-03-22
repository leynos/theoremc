//! Compile-fail tests for argument lowering.
//!
//! These tests verify that type mismatches in lowered expressions surface
//! as Rust compilation errors, not theoremc validation errors.

use std::error::Error;
use std::fs;
use std::io;
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

/// Bundles a Rust struct definition with the compiler diagnostic fragments
/// expected when code generation produces an ill-typed struct literal.
#[derive(Clone, Copy)]
struct StructHarness<'a> {
    def: &'a str,
    expected: DiagnosticMatch<'a>,
}

/// A set of compiler diagnostic substrings, at least one of which must
/// appear in `rustc` stderr for a compile-fail assertion to pass.
#[derive(Clone, Copy)]
struct DiagnosticMatch<'a>(&'a [&'a str]);

type TestResult = Result<(), Box<dyn Error>>;

/// Compiles a Rust snippet and returns `(success, stderr)`.
fn compile_snippet(code: &str) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let source_path = temp_dir.path().join("test.rs");
    fs::write(&source_path, code)?;

    let output = Command::new("rustc")
        .arg(&source_path)
        .arg("--crate-type=lib")
        .arg("--edition=2024")
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
) -> Result<(bool, String), Box<dyn Error>> {
    let ty: syn::Type = syn::parse_str(input.ty_str)?;
    let tokens = theoremc::arg_lowering::lower_arg_value(input.param, input.arg, &ty)?;
    compile_snippet(&make_code(&tokens.to_string(), input.ty_str))
}

fn test_failure(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

/// Helper: lowers an [`ArgValue`] and asserts it compiles successfully.
fn assert_lowers_and_compiles(input: LoweringInput<'_>) -> TestResult {
    let (success, stderr) = lower_and_compile(input, wrap_in_harness)?;
    if success {
        Ok(())
    } else {
        Err(test_failure(format!(
            "expected valid code to compile, but got errors:\n{stderr}"
        )))
    }
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts it compiles.
fn make_struct_harness_body(struct_def: &str, expr: &str, ty: &str) -> String {
    format!("{struct_def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n")
}

fn assert_lowers_and_compiles_with_struct(
    input: LoweringInput<'_>,
    harness: StructHarness<'_>,
) -> TestResult {
    let (success, stderr) = lower_and_compile(input, |expr, ty| {
        make_struct_harness_body(harness.def, expr, ty)
    })?;
    if success {
        Ok(())
    } else {
        Err(test_failure(format!(
            "expected valid struct literal to compile, but got errors:\n{stderr}"
        )))
    }
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts compilation fails,
/// with at least one of the expected fragments present in stderr.
fn assert_compile_failed(success: bool, stderr: &str, expected: DiagnosticMatch<'_>) -> TestResult {
    if success {
        return Err(test_failure("expected compilation to fail"));
    }
    if expected.0.iter().any(|f| stderr.contains(f)) {
        Ok(())
    } else {
        Err(test_failure(format!(
            "expected one of {:?} in stderr, got:\n{stderr}",
            expected.0
        )))
    }
}

fn assert_lowers_and_compile_fails_with_struct(
    input: LoweringInput<'_>,
    harness: StructHarness<'_>,
) -> TestResult {
    let (success, stderr) = lower_and_compile(input, |expr, ty| {
        make_struct_harness_body(harness.def, expr, ty)
    })?;
    assert_compile_failed(success, &stderr, harness.expected)
}

/// Helper: lowers an [`ArgValue`] and asserts compilation fails, with at least
/// one of `expected_fragments` present in stderr.
fn assert_lowers_and_compile_fails(
    input: LoweringInput<'_>,
    expected: DiagnosticMatch<'_>,
) -> TestResult {
    let (success, stderr) = lower_and_compile(input, wrap_in_harness)?;
    assert_compile_failed(success, &stderr, expected)
}

#[test]
fn positive_control_scalar_compiles() -> TestResult {
    // This test verifies our compile harness works by checking a valid case compiles.
    let arg = ArgValue::Literal(theoremc::schema::arg_value::LiteralValue::Integer(42));
    assert_lowers_and_compiles(LoweringInput {
        arg: &arg,
        param: "x",
        ty_str: "i32",
    })
}

#[test]
fn compile_fail_wrong_scalar_type_in_struct_field() -> TestResult {
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
        StructHarness {
            def: "struct Node { id: i32 }",
            expected: DiagnosticMatch(&["mismatched types", "expected `i32`, found `&str`"]),
        },
    )
}

#[test]
fn compile_fail_wrong_list_element_type() -> TestResult {
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
        DiagnosticMatch(&["mismatched types", "expected integer"]),
    )
}

#[test]
fn compile_fail_unknown_struct_field() -> TestResult {
    // YAML provides a field that doesn't exist in the struct.
    let mut map = IndexMap::new();
    map.insert("unknown_field".to_owned(), TheoremValue::Integer(42));
    assert_lowers_and_compile_fails_with_struct(
        LoweringInput {
            arg: &ArgValue::RawMap(map),
            param: "node",
            ty_str: "Node",
        },
        StructHarness {
            def: "struct Node { id: i32 }",
            expected: DiagnosticMatch(&["has no field named `unknown_field`", "E0560"]),
        },
    )
}

#[test]
fn compile_fail_nested_mismatch_in_list_of_structs() -> TestResult {
    // List of structs where one field has wrong type.
    let mut inner = IndexMap::new();
    inner.insert("x".to_owned(), TheoremValue::String("wrong".to_owned()));
    let arg = ArgValue::RawSequence(vec![TheoremValue::Mapping(inner)]);
    let ty: syn::Type = syn::parse_str("Vec<Point>")?;

    // This should fail during lowering because nested maps aren't supported yet
    let result = theoremc::arg_lowering::lower_arg_value("points", &arg, &ty);
    if result.is_err() {
        Ok(())
    } else {
        Err(test_failure(
            "expected lowering to fail for nested map (not yet supported)",
        ))
    }
}

#[test]
fn positive_control_struct_compiles() -> TestResult {
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
        StructHarness {
            def: "struct Node { id: i32, name: &'static str }",
            expected: DiagnosticMatch(&[]),
        },
    )
}

#[test]
fn positive_control_list_compiles() -> TestResult {
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
    })
}
