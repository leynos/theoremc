//! Contract tests for the Concordat spelling gate.
//!
//! The gate is one pinned command in the Makefile, run by CI after Markdown
//! lint. These tests read the wiring as text, so a change to the pin, the
//! scope, or the CI ordering fails here rather than in a later review. The
//! gate itself runs in CI; these tests prove the repository still invokes it
//! as the adopted baseline requires.

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const TYPOS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/typos.toml"));
const LOCAL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/typos.local.toml"));

/// The recipe the `spelling` target must run, exactly as the baseline pins it.
const PINNED_RECIPE: &str = "$(SPELLING_COMMAND)";

/// Returns the recipe lines of a Makefile target, without the target line.
///
/// The leading tab is stripped so a caller compares the command text alone.
/// An absent target yields an empty list, which fails the equality assertion
/// rather than panicking.
fn recipe_lines<'a>(makefile: &'a str, target: &str) -> Vec<&'a str> {
    let header = format!("{target}:");
    let mut lines = makefile.lines();
    if !lines.any(|line| line.starts_with(&header)) {
        return Vec::new();
    }
    lines
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// Returns the default value of a `NAME ?= value` assignment, trimmed.
///
/// Returns `None` when the assignment is absent, so a missing override fails
/// a comparison instead of silently matching.
fn default_assignment<'a>(makefile: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name} ?= ");
    makefile
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(str::trim)
}

/// Returns a Makefile variable definition with its continued lines joined.
///
/// A backslash at the end of a line continues the definition on the next
/// line; the fragments are joined with single spaces so a caller can search
/// the whole command without depending on the physical line breaks.
fn variable_definition(makefile: &str, name: &str) -> Option<String> {
    let plain = format!("{name} =");
    let default = format!("{name} ?=");
    let specifies = |line: &str| {
        line.strip_prefix(plain.as_str())
            .is_some_and(|rest| rest.starts_with(' '))
            || line
                .strip_prefix(default.as_str())
                .is_some_and(|rest| rest.starts_with(' '))
    };
    let lines: Vec<&str> = makefile.lines().collect();
    let start = lines.iter().position(|line| specifies(line))?;
    let mut definition = String::new();
    for line in lines.get(start..)? {
        let is_continued = line.ends_with('\\');
        if !definition.is_empty() {
            definition.push(' ');
        }
        definition.push_str(line.trim_end_matches('\\').trim());
        if !is_continued {
            break;
        }
    }
    Some(definition)
}

#[test]
fn spelling_target_runs_the_pinned_gate() {
    let command = variable_definition(MAKEFILE, "SPELLING_COMMAND")
        .expect("SPELLING_COMMAND must be defined");
    for fragment in [
        "$(SPELLING_BUILDER)",
        "@$(SPELLING_BUILDER_REF)",
        "--python 3.14",
        "typos-config-builder gate",
        "--repository $(SPELLING_REPOSITORY)",
        "--scope $(SPELLING_SCOPE)",
    ] {
        assert!(
            command.contains(fragment),
            "SPELLING_COMMAND must contain {fragment:?}, got {command:?}"
        );
    }
    assert_eq!(
        default_assignment(MAKEFILE, "SPELLING_BUILDER"),
        Some("git+https://github.com/leynos/typos-config-builder.git"),
        "the builder must default to the pinned HTTPS URL"
    );
    assert_eq!(
        default_assignment(MAKEFILE, "SPELLING_BUILDER_REF"),
        Some("v0.1.3"),
        "the builder revision must default to the adopted pin"
    );
    assert_eq!(
        default_assignment(MAKEFILE, "SPELLING_REPOSITORY"),
        Some("."),
        "the spelling gate must default to this repository"
    );
    assert_eq!(
        default_assignment(MAKEFILE, "SPELLING_SCOPE"),
        Some("all"),
        "the spelling gate must default to the full scope"
    );
    assert_eq!(
        recipe_lines(MAKEFILE, "spelling"),
        vec![PINNED_RECIPE],
        "the spelling target must run SPELLING_COMMAND"
    );
}

#[test]
fn ci_runs_the_spelling_gate_after_markdown_lint() {
    let markdown_lint = CI
        .find("markdownlint-cli2-action@")
        .expect("CI must run Markdown lint");
    let spelling = CI
        .find("run: make spelling")
        .expect("CI must run the spelling gate");
    assert!(
        spelling > markdown_lint,
        "the spelling gate must follow Markdown lint in the CI job"
    );
}

#[test]
fn generated_configuration_is_committed() {
    assert!(
        TYPOS.starts_with("# Generated from the shared en-GB-oxendict dictionary."),
        "typos.toml must be the generated file, not a handwritten one"
    );
    assert!(
        LOCAL.contains("schema = 1"),
        "typos.local.toml must declare the overlay schema"
    );
}
