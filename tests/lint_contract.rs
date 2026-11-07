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

#[test]
fn clippy_stage_denies_warnings_over_the_workspace() {
    let line = single_recipe_line(MAKEFILE, "lint-clippy");
    assert!(
        line.contains("$(CARGO) clippy $(CLIPPY_FLAGS)"),
        "lint-clippy must run Clippy with the workspace flags; found `{line}`"
    );
}

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

#[test]
fn ci_installs_a_pinned_whitaker_before_running_the_aggregate() {
    let install_line = CI
        .lines()
        .find(|line| line.contains(WHITAKER_INSTALLER))
        .unwrap_or_default();
    assert!(
        !install_line.is_empty(),
        "CI must install Whitaker through the shared action"
    );

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

    let install_step = CI
        .lines()
        .position(|line| line.contains(WHITAKER_INSTALLER))
        .unwrap_or(usize::MAX);
    let lint_step = CI
        .lines()
        .position(|line| line.trim() == "run: make lint")
        .unwrap_or(usize::MAX);
    assert!(
        lint_step != usize::MAX,
        "CI must run `make lint`, the same aggregate target used locally"
    );
    assert!(
        install_step < lint_step,
        "CI must install Whitaker before the lint step"
    );
}

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
