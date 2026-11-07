//! Contract tests for the workspace-wide lint gate.
//!
//! Rustdoc, Clippy, and Whitaker must each cover every workspace package,
//! target, and feature set, with warnings denied, and `make lint` must be the
//! single entry point that CI invokes. These tests read the Makefile, the
//! workflow, and the Dylint configuration as text, so a change that narrows a
//! gate, drops a stage, or lets a failure escape fails here rather than after a
//! merge. `tests/lint_gate_execution.rs` proves the same wiring by running the
//! recipes against stub executables.

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const DYLINT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dylint.toml"));

/// The provisioning action CI must use for the Whitaker suite.
const WHITAKER_INSTALLER: &str = "leynos/shared-actions/.github/actions/install-whitaker@";

/// Returns the default value of a `?=` variable, without its trailing newline.
///
/// The comparison is anchored on the whole name, so `RUSTDOC_FLAGS` never
/// matches `RUSTDOC_TARGET_FLAGS`. An absent variable yields `None`, which the
/// caller reports as a missing default rather than as an empty one.
fn variable_default<'a>(makefile: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name} ?=");
    makefile
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
        .map(str::trim)
}

/// Returns the recipe lines of a target, without the target line or its tab.
///
/// An absent target yields an empty list, so the shape assertions fail on the
/// missing recipe instead of panicking.
fn recipe_lines<'a>(makefile: &'a str, target: &str) -> Vec<&'a str> {
    let header = format!("{target}:");
    let mut lines = makefile.lines();
    if !lines.any(|line| line.starts_with(&header)) {
        return Vec::new();
    }
    lines
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t').trim())
        .collect()
}

/// Returns a target's only recipe line, failing when it has more than one.
///
/// Each lint stage is one command, so a second recipe line could run after a
/// failure and hide it. Asserting the shape here keeps the three stage tests
/// from repeating the same two lines.
fn single_recipe_line<'a>(makefile: &'a str, target: &str) -> &'a str {
    let recipe = recipe_lines(makefile, target);
    assert_eq!(
        recipe.len(),
        1,
        "{target} must be exactly one recipe line, so a failure cannot be hidden behind a later line; found {recipe:?}"
    );
    recipe.first().copied().unwrap_or_default()
}

/// Returns the configuration lines that are not comments or blank lines.
///
/// `dylint.toml` explains the exclusion policy in prose, and that prose names
/// the keys it warns against. Filtering comments keeps the description of the
/// policy from reading as the policy itself.
fn active_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// Reports whether every needle appears in the haystack.
fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

/// Verifies that every lint variable composes the workspace-wide scope.
#[test]
fn every_lint_variable_scopes_the_whole_workspace() {
    assert_eq!(
        variable_default(MAKEFILE, "LINT_PACKAGES"),
        Some("--workspace"),
        "LINT_PACKAGES must name the workspace, because a bare `cargo` invocation selects only the root package"
    );
    assert_eq!(
        variable_default(MAKEFILE, "RUSTDOC_FLAGS"),
        Some("-D warnings"),
        "RUSTDOC_FLAGS must deny warnings by default"
    );
    assert_eq!(
        variable_default(MAKEFILE, "RUST_FLAGS"),
        Some("-D warnings"),
        "RUST_FLAGS must deny warnings by default, because Clippy and Whitaker inherit it"
    );

    let target_flags = variable_default(MAKEFILE, "LINT_TARGET_FLAGS").unwrap_or_default();
    assert_eq!(
        target_flags, "--all-targets --all-features",
        "LINT_TARGET_FLAGS must cover every target and feature"
    );

    let cargo_flags = variable_default(MAKEFILE, "CARGO_FLAGS").unwrap_or_default();
    assert!(
        contains_all(cargo_flags, &["$(LINT_PACKAGES)", "$(LINT_TARGET_FLAGS)"]),
        "CARGO_FLAGS must compose the package and target scopes; found `{cargo_flags}`"
    );

    let rustdoc_flags = variable_default(MAKEFILE, "RUSTDOC_TARGET_FLAGS").unwrap_or_default();
    assert!(
        contains_all(
            rustdoc_flags,
            &["$(LINT_PACKAGES)", "--all-features", "--no-deps"]
        ),
        "RUSTDOC_TARGET_FLAGS must cover every package and feature without documenting dependencies; found `{rustdoc_flags}`"
    );

    let clippy_flags = variable_default(MAKEFILE, "CLIPPY_FLAGS").unwrap_or_default();
    assert!(
        contains_all(clippy_flags, &["$(CARGO_FLAGS)", "-- $(RUST_FLAGS)"]),
        "CLIPPY_FLAGS must inherit the workspace flags and deny warnings; found `{clippy_flags}`"
    );
}

/// Verifies that `make lint` recurses into each stage exactly once, in order.
#[test]
fn the_aggregate_runs_each_stage_once_in_order() {
    assert_eq!(
        recipe_lines(MAKEFILE, "lint"),
        vec![
            "$(MAKE) lint-rustdoc",
            "$(MAKE) lint-clippy",
            "$(MAKE) lint-whitaker",
        ],
        "`make lint` must recurse into each stage in order, so the stages stay sequential under `make -j` and a failing stage stops the aggregate"
    );
}

/// Verifies that the rustdoc stage covers the workspace with warnings denied.
#[test]
fn rustdoc_stage_denies_warnings_over_the_workspace() {
    let line = single_recipe_line(MAKEFILE, "lint-rustdoc");
    assert!(
        contains_all(
            line,
            &[
                "RUSTDOCFLAGS=\"$(RUSTDOC_FLAGS)\"",
                "$(CARGO) doc $(RUSTDOC_TARGET_FLAGS)",
            ]
        ),
        "lint-rustdoc must build workspace documentation with warnings denied; found `{line}`"
    );
}

/// Verifies that the Clippy stage covers the workspace with warnings denied.
#[test]
fn clippy_stage_denies_warnings_over_the_workspace() {
    let line = single_recipe_line(MAKEFILE, "lint-clippy");
    assert!(
        line.contains("$(CARGO) clippy $(CLIPPY_FLAGS)"),
        "lint-clippy must run Clippy with the workspace flags; found `{line}`"
    );
}

/// Verifies that the Whitaker stage covers the workspace and clears `RUSTFLAGS`.
#[test]
fn whitaker_stage_denies_warnings_over_the_workspace() {
    let line = single_recipe_line(MAKEFILE, "lint-whitaker");
    assert!(
        contains_all(
            line,
            &[
                "DYLINT_RUSTFLAGS=\"$(RUST_FLAGS)\"",
                "$(WHITAKER) --all $(LINT_PACKAGES) -- $(LINT_TARGET_FLAGS)",
            ]
        ),
        "lint-whitaker must load every lint library against every workspace package with warnings denied; found `{line}`"
    );
    assert!(
        line.starts_with("RUSTFLAGS= "),
        "lint-whitaker must clear RUSTFLAGS, because the suite brings its own toolchain; found `{line}`"
    );
}

/// Verifies that no lint recipe can swallow a failing command.
#[test]
fn no_lint_stage_can_mask_a_failure() {
    for target in ["lint", "lint-rustdoc", "lint-clippy", "lint-whitaker"] {
        for line in recipe_lines(MAKEFILE, target) {
            assert!(
                !line.starts_with('-'),
                "{target} must not ignore a recipe failure: `{line}`"
            );
            assert!(
                !line.contains("||"),
                "{target} must not swallow a failure behind a shell operator: `{line}`"
            );
            assert!(
                !line.contains("; true"),
                "{target} must not exit successfully after a failure: `{line}`"
            );
        }
    }
}

/// Reports whether a line opens a CI job.
///
/// A job is a key indented by exactly two spaces under `jobs:`. Steps are
/// indented further, so they never match.
fn is_job_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    line.starts_with("  ")
        && !line.starts_with("   ")
        && !trimmed.starts_with('#')
        && trimmed.ends_with(':')
}

/// Reports whether a line is a `uses:` step, as opposed to a comment that
/// merely names an action.
fn is_uses_step(line: &str) -> bool {
    line.trim_start().starts_with("uses:")
}

/// Returns the lines of the CI job that runs `make lint`.
///
/// Comparing the first occurrence of each string across the whole file would
/// keep passing if a second job ran `make lint` without installing Whitaker.
/// Scoping both searches to the one job keeps the ordering assertion honest.
fn lint_job_lines(ci: &str) -> Vec<&str> {
    let lines: Vec<&str> = ci.lines().collect();
    let Some(lint_index) = lint_step_index(&lines) else {
        return Vec::new();
    };
    let start = lines
        .iter()
        .take(lint_index)
        .rposition(|line| is_job_header(line))
        .unwrap_or(0);
    let end = lines
        .iter()
        .skip(lint_index)
        .position(|line| is_job_header(line))
        .map_or(lines.len(), |offset| lint_index + offset);
    lines
        .iter()
        .skip(start)
        .take(end - start)
        .copied()
        .collect()
}

/// Returns the index of the `run: make lint` line in `ci`.
fn lint_step_index(ci: &[&str]) -> Option<usize> {
    ci.iter().position(|line| line.trim() == "run: make lint")
}

/// Verifies that the job which runs `make lint` installs a pinned Whitaker.
#[test]
fn ci_installs_a_pinned_whitaker_before_running_the_aggregate() {
    let job = lint_job_lines(CI);
    assert!(
        !job.is_empty(),
        "CI must run `make lint`, the same aggregate target used locally"
    );

    let install_index = job
        .iter()
        .position(|line| is_uses_step(line) && line.contains(WHITAKER_INSTALLER))
        .unwrap_or(usize::MAX);
    assert!(
        install_index != usize::MAX,
        "the job that runs `make lint` must install Whitaker through the shared action"
    );

    let install_line = job.get(install_index).copied().unwrap_or_default();
    let reference = install_line
        .split(WHITAKER_INSTALLER)
        .nth(1)
        .unwrap_or_default();
    let sha = reference.split_whitespace().next().unwrap_or_default();
    assert_eq!(
        sha.len(),
        40,
        "the Whitaker action must be pinned to a full commit SHA; found `{sha}`"
    );
    assert!(
        sha.chars().all(|character| character.is_ascii_hexdigit()),
        "the Whitaker action pin must be hexadecimal; found `{sha}`"
    );

    let lint_index = lint_step_index(&job).unwrap_or(usize::MAX);
    assert!(
        install_index < lint_index,
        "CI must install Whitaker before the lint step in the same job"
    );
}

/// Verifies that `dylint.toml` declares no whole-crate exclusion.
#[test]
fn the_dylint_configuration_exempts_no_whole_crate() {
    assert!(
        !DYLINT.trim().is_empty(),
        "dylint.toml must exist at the workspace root so the exclusion policy has a home"
    );
    assert!(
        active_lines(DYLINT)
            .iter()
            .all(|line| !line.starts_with("excluded_crates")),
        "dylint.toml must not exempt a whole crate; prefer `excluded_paths` with a recorded reason"
    );
}
