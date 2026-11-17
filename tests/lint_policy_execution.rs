//! Proves the workspace lint policy is live, not merely declared.
//!
//! The test copies the real `Cargo.toml` lint tables into a throwaway workspace
//! and materializes one `MemberFixture`. Removing or downgrading a deny,
//! including `missing_assert_message`, makes the matching case fail; the stage
//! then accepts the violation. The fixture has no dependencies, so every case
//! builds offline.

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir as Utf8Dir;
use rstest::rstest;
use std::process::{Command, Output};
use tempfile::TempDir;

/// The member manifest, which opts into the workspace lints.
const MEMBER_MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"lint-member\"\n",
    "version = \"0.1.0\"\n",
    "edition = \"2024\"\n",
    "\n",
    "[lints]\n",
    "workspace = true\n",
);

/// The repository manifest text, wrapped so helpers take a domain value.
#[derive(Clone, Copy)]
struct Manifest<'a>(
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
struct Line<'a>(
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
enum LintGroup {
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

/// A member crate that either passes every stage or violates one lint.
#[derive(Clone, Copy)]
enum MemberFixture {
    /// A member with nothing for any stage to report.
    Clean,
    /// A member that names a lint that does not exist.
    UnknownLint,
    /// A member with a public item that carries no documentation.
    Undocumented,
    /// A member whose documentation links to an item that does not exist.
    BrokenLink,
    /// A member whose assertion carries no explanatory message.
    AssertWithoutMessage,
}

impl MemberFixture {
    /// Returns the member's `src/lib.rs` contents.
    const fn source(self) -> &'static str {
        match self {
            Self::Clean => concat!(
                "//! A member with nothing to report.\n\n",
                "/// Reports a number.\n",
                "pub fn member() -> u8 {\n    2\n}\n",
            ),
            Self::UnknownLint => concat!(
                "//! A member with an unknown lint attribute.\n\n",
                "/// Reports a number.\n#[warn(no_such_lint)]\n",
                "pub fn member() -> u8 {\n    1\n}\n",
            ),
            Self::Undocumented => concat!(
                "//! A member with an undocumented public item.\n\n",
                "pub fn undocumented() -> u8 {\n    3\n}\n",
            ),
            Self::BrokenLink => concat!(
                "//! A member whose documentation links to an absent item.\n\n",
                "/// See [`absent_item`].\n",
                "pub fn member() -> u8 {\n    4\n}\n",
            ),
            Self::AssertWithoutMessage => concat!(
                "//! A member whose assertion has no message.\n\n",
                "/// Checks that `value` is positive.\n",
                "pub fn check(value: u8) {\n    assert!(value > 0);\n}\n",
            ),
        }
    }

    /// Returns text the failing stage's output must contain; empty for `Clean`.
    const fn evidence(self) -> &'static [&'static str] {
        match self {
            Self::Clean => &[],
            Self::UnknownLint => &["unknown_lints", "no_such_lint"],
            Self::Undocumented => &["missing_docs", "missing documentation"],
            Self::BrokenLink => &["broken_intra_doc_links", "unresolved link"],
            Self::AssertWithoutMessage => &["missing_assert_message"],
        }
    }
}

/// The lint policy materialized from the repository manifest.
struct LintPolicy {
    /// The `[workspace.lints.rust]` table, including its header line.
    rust: String,
    /// The `[workspace.lints.rustdoc]` table, including its header line.
    rustdoc: String,
    /// The `missing_assert_message` entry from `[workspace.lints.clippy]`.
    missing_assert_message: String,
}

/// One file to write into the throwaway workspace.
#[derive(Clone, Copy)]
struct FixtureFile<'a> {
    /// The workspace-relative path, which is always valid UTF-8.
    path: &'a Utf8Path,
    /// The file contents.
    contents: &'a str,
}

/// One Cargo invocation that exercises one lint group.
struct Stage {
    /// The invocation name used in failure messages.
    name: &'static str,
    /// The Cargo subcommand and its arguments.
    arguments: &'static [&'static str],
}

/// Checks rustc lints, including `unknown_lints` and `missing_docs`.
static CHECK_STAGE: Stage = Stage {
    name: "cargo check",
    arguments: &["check", "--workspace"],
};

/// Checks Clippy lints configured through the lints table.
static CLIPPY_STAGE: Stage = Stage {
    name: "cargo clippy",
    arguments: &["clippy", "--workspace"],
};

/// Checks rustdoc lints, including `broken_intra_doc_links`.
static DOC_STAGE: Stage = Stage {
    name: "cargo doc",
    arguments: &["doc", "--workspace", "--no-deps"],
};

/// Reads the repository manifest named by `CARGO_MANIFEST_DIR` and extracts the
/// lint tables this test materializes.
///
/// # Errors
///
/// Returns the reason when the manifest cannot be read or lacks an expected
/// table or entry.
fn load_lint_policy() -> Result<LintPolicy, String> {
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
fn extract_lint_table(manifest: Manifest<'_>, group: LintGroup) -> Result<String, String> {
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
fn extract_missing_assert_message(manifest: Manifest<'_>) -> Result<String, String> {
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
fn render_root_manifest(policy: &LintPolicy) -> String {
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

/// Writes a throwaway workspace whose member holds `fixture`'s source, and
/// returns the directory with its path as UTF-8.
///
/// # Errors
///
/// Returns the reason when the directory or a file cannot be created, or when
/// the directory path is not valid UTF-8.
fn write_workspace(
    policy: &LintPolicy,
    fixture: MemberFixture,
) -> Result<(TempDir, Utf8PathBuf), String> {
    let directory =
        TempDir::new().map_err(|error| format!("creating a temporary directory: {error}"))?;
    let temporary = directory.path();
    // `tempfile` yields a `std::path::Path`, so convert it at this boundary.
    let workspace = Utf8Path::from_path(temporary)
        .ok_or_else(|| format!("the temporary directory {temporary:?} is not valid UTF-8"))?
        .to_path_buf();
    let fs = Utf8Dir::open_ambient_dir(&workspace, ambient_authority())
        .map_err(|error| format!("opening the temporary directory: {error}"))?;
    let root_manifest = render_root_manifest(policy);
    let files = [
        FixtureFile {
            path: Utf8Path::new("Cargo.toml"),
            contents: root_manifest.as_str(),
        },
        FixtureFile {
            path: Utf8Path::new("member/Cargo.toml"),
            contents: MEMBER_MANIFEST,
        },
        FixtureFile {
            path: Utf8Path::new("member/src/lib.rs"),
            contents: fixture.source(),
        },
    ];
    for file in files {
        write_file(&fs, file)?;
    }
    Ok((directory, workspace))
}

/// Writes `file` into the workspace, creating parent directories first.
///
/// # Errors
///
/// Returns the reason when a directory or the file cannot be created.
fn write_file(fs: &Utf8Dir, file: FixtureFile<'_>) -> Result<(), String> {
    let FixtureFile { path, contents } = file;
    if let Some(parent) = path.parent() {
        fs.create_dir_all(parent)
            .map_err(|error| format!("creating {path}: {error}"))?;
    }
    fs.write(path, contents)
        .map_err(|error| format!("writing {path}: {error}"))
}

/// Runs `stage` in `workspace`, with its own target directory and the outer
/// suite's flag variables removed. Cargo reads the encoded variables first, so
/// clearing all four stops an outer `-D warnings` deciding the result.
///
/// # Errors
///
/// Returns the reason when Cargo cannot be started.
fn run_stage(workspace: &Utf8Path, stage: &Stage) -> Result<Output, String> {
    // `std::process::Command` takes `std::path::Path`, so bridge through
    // camino's `as_std_path`; every caller still works in UTF-8.
    let target_dir: Utf8PathBuf = workspace.join("target");
    Command::new("cargo")
        .args(stage.arguments)
        .current_dir(workspace.as_std_path())
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTDOCFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTDOCFLAGS")
        .env("CARGO_TARGET_DIR", target_dir.as_str())
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .map_err(|error| format!("running {}: {error}", stage.name))
}

/// Verifies that the materialized policy accepts a clean member in every stage.
#[rstest]
#[case::check(&CHECK_STAGE)]
#[case::clippy(&CLIPPY_STAGE)]
#[case::doc(&DOC_STAGE)]
fn a_clean_member_passes(#[case] stage: &Stage) -> Result<(), String> {
    let policy = load_lint_policy()?;
    let (_directory, workspace) = write_workspace(&policy, MemberFixture::Clean)?;
    let output = run_stage(&workspace, stage)?;
    if !output.status.success() {
        return Err(format!(
            "{} must pass a clean member; stderr was:\n{}",
            stage.name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Verifies that a member violating a materialized lint fails the stage and
/// that the output names the lint that decided it.
#[rstest]
#[case::unknown_lint(&CHECK_STAGE, MemberFixture::UnknownLint)]
#[case::missing_docs(&CHECK_STAGE, MemberFixture::Undocumented)]
#[case::broken_doc_link(&DOC_STAGE, MemberFixture::BrokenLink)]
#[case::missing_assert_message(&CLIPPY_STAGE, MemberFixture::AssertWithoutMessage)]
fn a_violation_fails_the_stage(
    #[case] stage: &Stage,
    #[case] fixture: MemberFixture,
) -> Result<(), String> {
    let policy = load_lint_policy()?;
    let (_directory, workspace) = write_workspace(&policy, fixture)?;
    let output = run_stage(&workspace, stage)?;
    if output.status.success() {
        return Err(format!(
            "{} must fail on a member with a lint violation, but it passed",
            stage.name
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let evidence = fixture.evidence();
    if !evidence.iter().any(|text| stderr.contains(*text)) {
        return Err(format!(
            "{} output must name one of {evidence:?}; stderr was:\n{stderr}",
            stage.name
        ));
    }
    Ok(())
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
