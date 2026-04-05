//! Unit tests for theorem suite generation.
//!
//! These tests prove exact generated suite contents for empty, single-file,
//! and multi-file inputs, including deterministic ordering and newline policy.

use camino::Utf8PathBuf;

use super::render_theorem_suite;

/// Helper to create a collection of theorem file paths for testing.
fn theorem_paths(paths: &[&str]) -> Vec<Utf8PathBuf> {
    paths.iter().map(|p| Utf8PathBuf::from(*p)).collect()
}

#[test]
fn empty_theorem_list_renders_empty_suite_with_trailing_newline() {
    let paths: Vec<Utf8PathBuf> = vec![];
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    assert_eq!(
        rendered, "\n",
        "empty suite should contain only a trailing newline"
    );
}

#[test]
fn single_theorem_renders_one_theorem_file_invocation() {
    let paths = theorem_paths(&["theorems/example.theorem"]);
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    assert_eq!(
        rendered, "theorem_file!(\"theorems/example.theorem\");\n",
        "single theorem should render one invocation with trailing newline"
    );
}

#[test]
fn multiple_theorems_render_in_supplied_order() {
    // Note: These are supplied out of lexical order to prove the renderer
    // respects the order given (which should already be sorted by BuildDiscovery)
    let paths = theorem_paths(&[
        "theorems/z.theorem",
        "theorems/a.theorem",
        "theorems/nested/b.theorem",
    ]);
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    let expected = concat!(
        "theorem_file!(\"theorems/z.theorem\");\n",
        "theorem_file!(\"theorems/a.theorem\");\n",
        "theorem_file!(\"theorems/nested/b.theorem\");\n",
    );

    assert_eq!(
        rendered, expected,
        "multiple theorems should render in supplied order"
    );
}

#[test]
fn nested_paths_render_correctly() {
    let paths = theorem_paths(&["theorems/nested/deep/file.theorem", "theorems/root.theorem"]);
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    let expected = concat!(
        "theorem_file!(\"theorems/nested/deep/file.theorem\");\n",
        "theorem_file!(\"theorems/root.theorem\");\n",
    );

    assert_eq!(rendered, expected);
}

#[test]
fn paths_with_special_characters_render_escaped() {
    // Paths with quotes or backslashes should be properly escaped
    let paths = theorem_paths(&["theorems/file_with_\"quotes\".theorem"]);
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    // Assert the full rendered line, not just that it contains escaped quotes
    let expected = "theorem_file!(\"theorems/file_with_\\\"quotes\\\".theorem\");\n";
    assert_eq!(
        rendered, expected,
        "quotes in path should be properly escaped"
    );
}

#[test]
fn backslashes_in_paths_are_escaped() {
    // Windows-style paths with backslashes should be escaped
    let paths = theorem_paths(&["theorems/dir\\file.theorem"]);
    let rendered = render_theorem_suite(paths.iter().map(Utf8PathBuf::as_path));

    // Assert the full rendered line, not just that it contains escaped backslashes
    let expected = "theorem_file!(\"theorems/dir\\\\file.theorem\");\n";
    assert_eq!(
        rendered, expected,
        "backslashes in path should be properly escaped"
    );
}
