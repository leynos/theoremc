//! Reads the repository manifest's lint tables for `lint_policy_execution`.
//!
//! The behavioural tests materialize the real workspace policy into a
//! throwaway workspace, so this module reads and scans that policy. It lives
//! apart from the behavioural tests to keep each file inside the 400-line
//! limit.

use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir as Utf8Dir;
use rstest::rstest;

/// The repository manifest text, wrapped so helpers take a domain value.
#[derive(Clone, Copy)]
pub struct Manifest<'a>(
    /// The full text of the repository's `Cargo.toml`.
    &'a str,
);

impl<'a> Manifest<'a> {
    /// Iterates over the manifest's lines.
    fn lines(self) -> std::str::Lines<'a> {
        self.0.lines()
    }
}

/// One manifest line, scanned as TOML rather than as raw text.
#[derive(Clone, Copy)]
pub struct Line<'a>(
    /// The line, without its terminator.
    &'a str,
);

impl<'a> Line<'a> {
    /// Returns the line before any trailing comment and its net brace count.
    ///
    /// A `#` or brace inside a quoted value is data, not syntax, so only
    /// double-quoted escapes and single-quoted literals hide it. That is the
    /// full extent of the value syntax these lint tables use.
    fn scan(self) -> (&'a str, i32) {
        let (mut depth, mut quote, mut escaped) = (0, None, false);
        for (index, character) in self.0.char_indices() {
            match (quote, character) {
                _ if escaped => escaped = false,
                (Some('"'), '\\') => escaped = true,
                (Some(open), close) if open == close => quote = None,
                (None, '"' | '\'') => quote = Some(character),
                (None, '#') => return (self.0.split_at(index).0, depth),
                (None, '{') => depth += 1,
                (None, '}') => depth -= 1,
                _ => {}
            }
        }
        (self.0, depth)
    }
}

/// One of the workspace lint tables this test materializes.
pub enum LintGroup {
    /// Compiler lints from `[workspace.lints.rust]`.
    Rust,
    /// Rustdoc lints from `[workspace.lints.rustdoc]`.
    Rustdoc,
    /// Clippy lints from `[workspace.lints.clippy]`.
    Clippy,
}

impl LintGroup {
    /// Returns the TOML table header for this group.
    const fn header(self) -> &'static str {
        match self {
            Self::Rust => "[workspace.lints.rust]",
            Self::Rustdoc => "[workspace.lints.rustdoc]",
            Self::Clippy => "[workspace.lints.clippy]",
        }
    }
}

/// The lint policy materialized from the repository manifest.
pub struct LintPolicy {
    /// The `[workspace.lints.rust]` table, including its header line.
    rust: String,
    /// The `[workspace.lints.rustdoc]` table, including its header line.
    rustdoc: String,
    /// The `missing_assert_message` entry from `[workspace.lints.clippy]`.
    missing_assert_message: String,
}

/// Reads the repository manifest named by `CARGO_MANIFEST_DIR` and extracts the
/// lint tables this test materializes.
///
/// # Errors
///
/// Returns the reason when the manifest cannot be read or lacks an expected
/// table or entry.
pub fn load_lint_policy() -> Result<LintPolicy, String> {
    let manifest_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    let fs = Utf8Dir::open_ambient_dir(manifest_dir, ambient_authority())
        .map_err(|error| format!("opening {manifest_dir}: {error}"))?;
    let manifest_text = fs
        .read_to_string("Cargo.toml")
        .map_err(|error| format!("reading the repository manifest: {error}"))?;
    let manifest = Manifest(manifest_text.as_str());
    Ok(LintPolicy {
        rust: extract_lint_table(manifest, LintGroup::Rust)?,
        rustdoc: extract_lint_table(manifest, LintGroup::Rustdoc)?,
        missing_assert_message: extract_missing_assert_message(manifest)?,
    })
}

/// Extracts the table selected by `group`, from its header to the next table
/// header. A trailing comment on either header does not defeat matching.
///
/// # Errors
///
/// Returns the reason when the manifest has no such table.
pub fn extract_lint_table(manifest: Manifest<'_>, group: LintGroup) -> Result<String, String> {
    let header = group.header();
    let mut lines = manifest.lines();
    let Some(first_line) = lines.find(|line| Line(line).scan().0.trim() == header) else {
        return Err(format!("the repository manifest has no {header} table"));
    };
    let mut block = String::from(first_line);
    block.push('\n');
    for line in lines {
        if Line(line).scan().0.trim().starts_with('[') {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    Ok(block)
}

/// Extracts the single `missing_assert_message` entry from the Clippy table,
/// keeping every line of a multi-line entry and no sibling entries. Only this
/// entry is materialized, so an unrelated denied Clippy lint cannot decide the
/// assertion case.
///
/// # Errors
///
/// Returns the reason when the Clippy table is absent or has no such entry.
pub fn extract_missing_assert_message(manifest: Manifest<'_>) -> Result<String, String> {
    let clippy = extract_lint_table(manifest, LintGroup::Clippy)?;
    let mut lines = clippy.lines();
    let Some(first_line) = lines.find(|line| {
        Line(line)
            .scan()
            .0
            .trim_start()
            .strip_prefix("missing_assert_message")
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    }) else {
        return Err(String::from(
            "the [workspace.lints.clippy] table has no missing_assert_message entry",
        ));
    };
    let mut entry = String::new();
    entry.push_str(first_line.trim());
    entry.push('\n');
    // The value is a string or an inline table; only the table spans lines.
    let mut depth = Line(first_line).scan().1;
    while depth > 0 {
        let Some(line) = lines.next() else {
            break;
        };
        entry.push_str(line.trim());
        entry.push('\n');
        depth += Line(line).scan().1;
    }
    Ok(entry)
}

/// Renders the fixture root manifest from `policy`.
pub fn render_root_manifest(policy: &LintPolicy) -> String {
    format!(
        concat!(
            "[workspace]\n",
            "members = [\"member\"]\n",
            "resolver = \"3\"\n",
            "\n",
            "{}",
            "{}",
            "[workspace.lints.clippy]\n",
            "{}",
        ),
        policy.rust, policy.rustdoc, policy.missing_assert_message,
    )
}

/// Verifies that comments and quoted values never change the span of the
/// `missing_assert_message` entry, and so never pull in a sibling entry.
#[rstest]
#[case::trailing_comment_with_opening_brace(
    concat!(
        "missing_assert_message = \"deny\" # {\n",
        "missing_const_for_fn = \"deny\"\n",
    ),
    "missing_assert_message = \"deny\" # {\n",
)]
#[case::multi_line_inline_table(
    concat!(
        "missing_assert_message = { level = \"deny\",\n",
        "                          priority = 3 }\n",
        "missing_const_for_fn = \"deny\"\n",
    ),
    concat!(
        "missing_assert_message = { level = \"deny\",\n",
        "priority = 3 }\n",
    ),
)]
#[case::quoted_hash_and_brace(
    concat!(
        "missing_assert_message = \"deny # {\"\n",
        "missing_const_for_fn = \"deny\"\n",
    ),
    "missing_assert_message = \"deny # {\"\n",
)]
#[case::trailing_comment_with_closing_brace(
    concat!(
        "missing_assert_message = { level = \"deny\", # }\n",
        "                          priority = 3 }\n",
        "missing_const_for_fn = \"deny\"\n",
    ),
    concat!(
        "missing_assert_message = { level = \"deny\", # }\n",
        "priority = 3 }\n",
    ),
)]
fn missing_assert_message_ignores_comments(
    #[case] clippy: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    let manifest = format!("[workspace.lints.clippy]\n{clippy}");
    let entry = extract_missing_assert_message(Manifest(manifest.as_str()))?;
    if entry != expected {
        return Err(format!("expected {expected:?}, extracted {entry:?}"));
    }
    Ok(())
}
