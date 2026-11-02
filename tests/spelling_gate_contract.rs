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
const PINNED_RECIPE: &str = concat!(
    "$(UV) tool run --python 3.14 ",
    "--from \"git+https://github.com/leynos/typos-config-builder.git@v0.1.3\" ",
    "typos-config-builder gate --repository . --scope all",
);

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

#[test]
fn spelling_target_runs_the_pinned_gate() {
    assert_eq!(
        recipe_lines(MAKEFILE, "spelling"),
        vec![PINNED_RECIPE],
        "the spelling target must run the pinned typos-config-builder gate"
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
