//! Unit tests for theorem suite generation.
//!
//! These tests prove exact generated suite contents for empty, single-file,
//! and multi-file inputs, including deterministic ordering and newline policy.

use camino::Utf8PathBuf;
use rstest::rstest;

use super::render_theorem_suite;

/// Test case for theorem suite rendering.
struct RenderCase {
    paths: Vec<&'static str>,
    expected: &'static str,
    description: &'static str,
}

#[rstest]
#[case::empty_suite(RenderCase {
    paths: vec![],
    expected: "\n",
    description: "empty suite should contain only a trailing newline",
})]
#[case::single_theorem(RenderCase {
    paths: vec!["theorems/example.theorem"],
    expected: "theorem_file!(\"theorems/example.theorem\");\n",
    description: "single theorem should render one invocation with trailing newline",
})]
#[case::multiple_theorems(RenderCase {
    paths: vec![
        "theorems/z.theorem",
        "theorems/a.theorem",
        "theorems/nested/b.theorem",
    ],
    expected: concat!(
        "theorem_file!(\"theorems/z.theorem\");\n",
        "theorem_file!(\"theorems/a.theorem\");\n",
        "theorem_file!(\"theorems/nested/b.theorem\");\n",
    ),
    description: "multiple theorems should render in supplied order",
})]
#[case::nested_paths(RenderCase {
    paths: vec!["theorems/nested/deep/file.theorem", "theorems/root.theorem"],
    expected: concat!(
        "theorem_file!(\"theorems/nested/deep/file.theorem\");\n",
        "theorem_file!(\"theorems/root.theorem\");\n",
    ),
    description: "nested paths should render correctly",
})]
#[case::paths_with_quotes(RenderCase {
    paths: vec!["theorems/file_with_\"quotes\".theorem"],
    expected: "theorem_file!(\"theorems/file_with_\\\"quotes\\\".theorem\");\n",
    description: "quotes in path should be properly escaped",
})]
#[case::paths_with_backslashes(RenderCase {
    paths: vec!["theorems/dir\\file.theorem"],
    expected: "theorem_file!(\"theorems/dir\\\\file.theorem\");\n",
    description: "backslashes in path should be properly escaped",
})]
fn render_theorem_suite_produces_expected_output(#[case] case: RenderCase) {
    let paths: Vec<Utf8PathBuf> = case.paths.iter().map(|p| Utf8PathBuf::from(*p)).collect();
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    assert_eq!(rendered, case.expected, "{}", case.description);
}
