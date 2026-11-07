//! Error-reporting tests for theorem-file macro expansion.
//!
//! Each case stages a malformed or absent theorem file and asserts that the
//! private expansion pipeline renders the diagnostic fragment callers rely on.
//! The shape, ordering, and snapshot assertions live in `tests.rs`; this module
//! keeps the failure paths separate so neither concern crowds the other.

use super::tests_support::{
    TheoremFixture, expansion_error_message, temp_fixture_dir, write_fixture,
};
use camino::Utf8Path;
use rstest::rstest;

#[rstest]
#[case::invalid_schema(
    "theorems/invalid.theorem",
    Some(concat!(
        "Theorem: BrokenMacro\n",
        "About: \"\"\n",
        "Witness:\n",
        "  - cover: \"true\"\n",
        "    because: \"reachable\"\n",
        "Prove:\n",
        "  - assert: \"true\"\n",
        "    because: \"trivial\"\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 1\n",
        "    expect: SUCCESS\n",
    )),
    "schema.validation_failure|theorems/invalid.theorem:",
)]
#[case::missing_file(
    "theorems/missing.theorem",
    None,
    "failedtoreadtheoremfile'theorems/missing.theorem'"
)]
#[case::empty_file(
    "theorems/empty.theorem",
    Some(""),
    "doesnotcontainanytheoremdocuments"
)]
#[case::zero_unwind(
    "theorems/zero-unwind.theorem",
    Some(concat!(
        "Theorem: ZeroUnwindMacro\n",
        "About: Invalid Kani evidence coverage\n",
        "Witness:\n",
        "  - cover: \"true\"\n",
        "    because: \"reachable\"\n",
        "Prove:\n",
        "  - assert: \"true\"\n",
        "    because: \"trivial\"\n",
        "Evidence:\n",
        "  kani:\n",
        "    unwind: 0\n",
        "    expect: SUCCESS\n",
    )),
    "Evidence.kani.unwindmustbeapositiveinteger"
)]
fn theorem_file_errors_report_expected_compile_error(
    #[case] path: &str,
    #[case] fixture_content: Option<&str>,
    #[case] expected_fragment: &str,
) {
    let (_temp_dir, fixture_dir) =
        temp_fixture_dir().expect("should create temp fixture dir for error fixture");
    let relative_path = Utf8Path::new(path);

    if let Some(content) = fixture_content {
        write_fixture(
            &fixture_dir,
            relative_path,
            &TheoremFixture(content.to_owned()),
        )
        .expect("should write theorem fixture");
    }

    let error_string = expansion_error_message(&fixture_dir, relative_path)
        .expect("should render expansion error");
    assert!(
        error_string.contains(expected_fragment),
        "expected '{expected_fragment}' in compile error, got: {error_string}"
    );
}
