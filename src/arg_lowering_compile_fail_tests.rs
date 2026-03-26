//! Compile-fail tests for argument lowering.
//!
//! These tests verify that type mismatches in lowered expressions surface
//! as Rust compilation errors, not theoremc validation errors.

use std::error::Error;
use std::io;
use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use indexmap::IndexMap;

use crate::schema::TheoremValue;
use crate::schema::arg_value::ArgValue;

/// Bundles the three inputs that every lowering call requires, reducing
/// string-heavy argument lists.
#[derive(Clone, Copy)]
struct LoweringInput<'a> {
    arg: &'a ArgValue,
    param: &'a str,
    ty_str: &'a str,
}

/// Bundles a Rust struct definition for compile-success tests.
#[derive(Clone, Copy)]
struct StructDef<'a> {
    def: &'a str,
}

impl StructDef<'_> {
    /// Wraps a lowered expression in a struct-definition harness ready for
    /// `rustc`.
    fn make_code(self, expr: &str, ty: &str) -> String {
        format!(
            "{def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n",
            def = self.def
        )
    }
}

/// Bundles a Rust struct definition with the compiler diagnostic fragments
/// expected when code generation produces an ill-typed struct literal.
#[derive(Clone, Copy)]
struct StructHarness<'a> {
    def: &'a str,
    expected: DiagnosticMatch<'a>,
}

impl StructHarness<'_> {
    /// Wraps a lowered expression in a struct-definition harness ready for
    /// `rustc`.
    fn make_code(self, expr: &str, ty: &str) -> String {
        format!(
            "{def}\npub fn test_harness() {{\n    let _value: {ty} = {expr};\n}}\n",
            def = self.def
        )
    }
}

/// A set of compiler diagnostic substrings, at least one of which must
/// appear in `rustc` stderr for a compile-fail assertion to pass.
#[derive(Clone, Copy)]
struct DiagnosticMatch<'a>(&'a [&'a str]);

/// The outcome of a `rustc` compilation: success flag and captured stderr.
struct CompileOutcome {
    success: bool,
    stderr: String,
}

type TestResult = Result<(), Box<dyn Error>>;

/// Compiles a Rust snippet and returns `(success, stderr)`.
fn compile_snippet(code: &str) -> Result<(bool, String), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let temp_dir_utf8 = Utf8Path::from_path(temp_dir.path())
        .ok_or_else(|| io::Error::other("temp dir path is not valid UTF-8"))?;
    let source_path = temp_dir_utf8.join("test.rs");

    let dir = Dir::open_ambient_dir(temp_dir_utf8, ambient_authority())?;
    dir.write("test.rs", code)?;

    let output = Command::new("rustc")
        .arg(source_path.as_str())
        .arg("--crate-type=lib")
        .arg("--edition=2024")
        // Emit output inside the temp dir so artefacts don't pollute the
        // project root.
        .arg("--out-dir")
        .arg(temp_dir_utf8.as_str())
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
) -> Result<CompileOutcome, Box<dyn Error>> {
    let ty: syn::Type = syn::parse_str(input.ty_str)?;
    let tokens = super::lower_arg_value(input.param, input.arg, &ty)?;
    let (success, stderr) = compile_snippet(&make_code(&tokens.to_string(), input.ty_str))?;
    Ok(CompileOutcome { success, stderr })
}

fn test_failure(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

/// Asserts that a compile run succeeded, returning a descriptive error if it
/// did not.
fn assert_compile_succeeded(outcome: &CompileOutcome) -> TestResult {
    if outcome.success {
        Ok(())
    } else {
        Err(test_failure(format!(
            "expected compilation to succeed, but got errors:\n{}",
            outcome.stderr
        )))
    }
}

/// Helper: lowers an [`ArgValue`] and asserts it compiles successfully.
fn assert_lowers_and_compiles(input: LoweringInput<'_>) -> TestResult {
    let outcome = lower_and_compile(input, wrap_in_harness)?;
    assert_compile_succeeded(&outcome)
}

fn assert_lowers_and_compiles_with_struct(
    input: LoweringInput<'_>,
    def: StructDef<'_>,
) -> TestResult {
    let outcome = lower_and_compile(input, |expr, ty| def.make_code(expr, ty))?;
    assert_compile_succeeded(&outcome)
}

/// Helper: lowers an [`ArgValue`] with a struct definition and asserts compilation fails,
/// with at least one of the expected fragments present in stderr.
fn assert_compile_failed(outcome: &CompileOutcome, expected: DiagnosticMatch<'_>) -> TestResult {
    if outcome.success {
        return Err(test_failure("expected compilation to fail"));
    }
    if expected.0.iter().any(|f| outcome.stderr.contains(f)) {
        Ok(())
    } else {
        Err(test_failure(format!(
            "expected one of {:?} in stderr, got:\n{}",
            expected.0, outcome.stderr
        )))
    }
}

fn assert_lowers_and_compile_fails_with_struct(
    input: LoweringInput<'_>,
    harness: StructHarness<'_>,
) -> TestResult {
    let outcome = lower_and_compile(input, |expr, ty| harness.make_code(expr, ty))?;
    assert_compile_failed(&outcome, harness.expected)
}

/// Helper: lowers an [`ArgValue`] and asserts compilation fails, with at least
/// one of `expected_fragments` present in stderr.
fn assert_lowers_and_compile_fails(
    input: LoweringInput<'_>,
    expected: DiagnosticMatch<'_>,
) -> TestResult {
    let outcome = lower_and_compile(input, wrap_in_harness)?;
    assert_compile_failed(&outcome, expected)
}

#[test]
fn positive_control_scalar_compiles() -> TestResult {
    // This test verifies our compile harness works by checking a valid case compiles.
    let arg = ArgValue::Literal(crate::schema::arg_value::LiteralValue::Integer(42));
    assert_lowers_and_compiles(LoweringInput {
        arg: &arg,
        param: "x",
        ty_str: "i32",
    })
}

#[test]
fn compile_fail_wrong_scalar_type_in_struct_field() -> TestResult {
    // YAML provides a string for a field that expects an integer.
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
            expected: DiagnosticMatch(&[
                "the trait bound `i32: From<&str>` is not satisfied",
                "E0277",
            ]),
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
        DiagnosticMatch(&[
            "the trait bound `i32: From<&str>` is not satisfied",
            "E0277",
        ]),
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
fn positive_control_struct_compiles() -> TestResult {
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
        StructDef {
            def: "struct Node { id: i32, name: String }",
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
