//! Contract tests for the workspace-wide lint gate.
//!
//! Rustdoc, Clippy, and Whitaker must each cover every workspace package,
//! target, and feature set, with warnings denied, and `make lint` must be the
//! single entry point that CI invokes. These tests read the Makefile, the
//! workflow, and the Dylint configuration as text, so a change that narrows a
//! gate, drops a stage, or lets a failure escape fails here rather than after a
//! merge. `tests/lint_gate_execution.rs` proves the same wiring by running the
//! recipes against stub executables, and `tests/lint_workspace_execution.rs`
//! proves that Cargo acts on the workspace scope those recipes pass.

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));
const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const DYLINT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dylint.toml"));

/// The provisioning action CI must use for the Whitaker suite.
const WHITAKER_INSTALLER: &str = "leynos/shared-actions/.github/actions/install-whitaker@";

/// The workflow step that runs the aggregate lint target.
const LINT_COMMAND: &str = "run: make lint";

/// The Makefile under test, viewed as text.
///
/// Wrapping the text keeps the readers below from taking a bare `&str` that
/// could be any file, and lets each one name only the variable or target it
/// looks up.
struct Makefile<'a>(&'a str);

impl<'a> Makefile<'a> {
    /// Returns the default value of a `?=` variable, without its trailing
    /// newline.
    ///
    /// The comparison is anchored on the whole name, so `RUSTDOC_FLAGS` never
    /// matches `RUSTDOC_TARGET_FLAGS`. An absent variable yields `None`, which
    /// the caller reports as a missing default rather than as an empty one.
    fn variable_default(&self, name: &str) -> Option<&'a str> {
        let prefix = format!("{name} ?=");
        self.0
            .lines()
            .find_map(|line| line.trim().strip_prefix(prefix.as_str()))
            .map(str::trim)
    }

    /// Returns the recipe lines of a target, without the target line or its
    /// tab.
    ///
    /// An absent target yields an empty list, so the shape assertions fail on
    /// the missing recipe instead of panicking.
    fn recipe_lines(&self, target: &str) -> Vec<&'a str> {
        let header = format!("{target}:");
        let mut lines = self.0.lines();
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
    /// Each lint stage is one command, so a second recipe line could run after
    /// a failure and hide it. Asserting the shape here keeps the three stage
    /// tests from repeating the same two lines.
    fn single_recipe_line(&self, target: &str) -> &'a str {
        let recipe = self.recipe_lines(target);
        assert_eq!(
            recipe.len(),
            1,
            "{target} must be exactly one recipe line, so a failure cannot be hidden behind a later line; found {recipe:?}"
        );
        recipe.first().copied().unwrap_or_default()
    }
}

/// A CI workflow, viewed as text.
struct Workflow<'a>(&'a str);

impl<'a> Workflow<'a> {
    /// Returns the lines of the job whose steps include `command`.
    ///
    /// Comparing the first occurrence of each string across the whole file
    /// would keep passing if a second job ran the same command without
    /// installing Whitaker. Scoping both searches to that one job keeps the
    /// ordering assertion honest.
    fn job_running(&self, command: &str) -> Vec<&'a str> {
        let lines: Vec<&'a str> = self.0.lines().collect();
        let Some(step) = lines.iter().position(|line| line.trim() == command) else {
            return Vec::new();
        };
        let start = lines
            .iter()
            .take(step)
            .rposition(|line| is_job_header(line))
            .unwrap_or(0);
        let end = lines
            .iter()
            .skip(step)
            .position(|line| is_job_header(line))
            .map_or(lines.len(), |offset| step + offset);
        lines.into_iter().skip(start).take(end - start).collect()
    }
}

/// Returns the `dylint.toml` lines that are not comments or blank lines.
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

/// A text value and the fragments it must contain.
struct Contains<'a> {
    /// The text under test.
    value: &'a str,
    /// The fragments that must appear in it.
    fragments: &'a [&'a str],
}

impl Contains<'_> {
    /// Asserts that every fragment appears.
    ///
    /// Each fragment is reported on its own, so a failure names the part that
    /// went missing rather than only the whole value.
    fn assert_all(&self) {
        for fragment in self.fragments {
            assert!(
                self.value.contains(fragment),
                "expected `{}` to contain `{fragment}`",
                self.value
            );
        }
    }
}

/// Verifies that every lint variable composes the workspace-wide scope.
#[test]
fn every_lint_variable_scopes_the_whole_workspace() {
    let makefile = Makefile(MAKEFILE);
    assert_eq!(
        makefile.variable_default("LINT_PACKAGES"),
        Some("--workspace"),
        "LINT_PACKAGES must name the workspace, because a bare `cargo` invocation selects only the root package"
    );
    assert_eq!(
        makefile.variable_default("RUSTDOC_FLAGS"),
        Some("-D warnings"),
        "RUSTDOC_FLAGS must deny warnings by default"
    );
    assert_eq!(
        makefile.variable_default("RUST_FLAGS"),
        Some("-D warnings"),
        "RUST_FLAGS must deny warnings by default, because Clippy and Whitaker inherit it"
    );
    assert_eq!(
        makefile.variable_default("LINT_TARGET_FLAGS"),
        Some("--all-targets --all-features"),
        "LINT_TARGET_FLAGS must cover every target and feature"
    );

    let cargo_flags = makefile.variable_default("CARGO_FLAGS").unwrap_or_default();
    Contains {
        value: cargo_flags,
        fragments: &["$(LINT_PACKAGES)", "$(LINT_TARGET_FLAGS)"],
    }
    .assert_all();

    let rustdoc_flags = makefile
        .variable_default("RUSTDOC_TARGET_FLAGS")
        .unwrap_or_default();
    Contains {
        value: rustdoc_flags,
        fragments: &["$(LINT_PACKAGES)", "--all-features", "--no-deps"],
    }
    .assert_all();

    let clippy_flags = makefile
        .variable_default("CLIPPY_FLAGS")
        .unwrap_or_default();
    Contains {
        value: clippy_flags,
        fragments: &["$(CARGO_FLAGS)", "-- $(RUST_FLAGS)"],
    }
    .assert_all();
}

/// Verifies that `make lint` recurses into each stage exactly once, in order.
#[test]
fn the_aggregate_runs_each_stage_once_in_order() {
    assert_eq!(
        Makefile(MAKEFILE).recipe_lines("lint"),
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
    let line = Makefile(MAKEFILE).single_recipe_line("lint-rustdoc");
    Contains {
        value: line,
        fragments: &[
            "RUSTDOCFLAGS=\"$(RUSTDOC_FLAGS)\"",
            "$(CARGO) doc $(RUSTDOC_TARGET_FLAGS)",
        ],
    }
    .assert_all();
}

/// Verifies that the Clippy stage covers the workspace with warnings denied.
#[test]
fn clippy_stage_denies_warnings_over_the_workspace() {
    let line = Makefile(MAKEFILE).single_recipe_line("lint-clippy");
    Contains {
        value: line,
        fragments: &["$(CARGO) clippy $(CLIPPY_FLAGS)"],
    }
    .assert_all();
}

/// Verifies that the Whitaker stage covers the workspace and clears `RUSTFLAGS`.
#[test]
fn whitaker_stage_denies_warnings_over_the_workspace() {
    let line = Makefile(MAKEFILE).single_recipe_line("lint-whitaker");
    Contains {
        value: line,
        fragments: &[
            "DYLINT_RUSTFLAGS=\"$(RUST_FLAGS)\"",
            "$(WHITAKER) --all $(LINT_PACKAGES) -- $(LINT_TARGET_FLAGS)",
        ],
    }
    .assert_all();
    assert!(
        line.starts_with("RUSTFLAGS= "),
        "lint-whitaker must clear RUSTFLAGS, because the suite brings its own toolchain; found `{line}`"
    );
}

/// Verifies that no lint recipe can swallow a failing command.
#[test]
fn no_lint_stage_can_mask_a_failure() {
    let makefile = Makefile(MAKEFILE);
    for target in ["lint", "lint-rustdoc", "lint-clippy", "lint-whitaker"] {
        for line in makefile.recipe_lines(target) {
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

/// Verifies that the job which runs `make lint` installs a pinned Whitaker.
#[test]
fn ci_installs_a_pinned_whitaker_before_running_the_aggregate() {
    let job = Workflow(CI).job_running(LINT_COMMAND);
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
    let sha = install_line
        .split(WHITAKER_INSTALLER)
        .nth(1)
        .and_then(|reference| reference.split_whitespace().next())
        .unwrap_or_default();
    assert_eq!(
        sha.len(),
        40,
        "the Whitaker action must be pinned to a full commit SHA; found `{sha}`"
    );
    assert!(
        sha.chars().all(|character| character.is_ascii_hexdigit()),
        "the Whitaker action pin must be hexadecimal; found `{sha}`"
    );

    let lint_index = job
        .iter()
        .position(|line| line.trim() == LINT_COMMAND)
        .unwrap_or(usize::MAX);
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
