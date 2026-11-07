//! Contract tests for the CI and Dylint sides of the workspace-wide lint gate.
//!
//! The workflow must install a pinned Whitaker and then run `make lint`, the
//! same aggregate target contributors run locally, and the root `dylint.toml`
//! must not exempt a whole crate. These tests read both files as text, so a
//! gate cannot be provisioned differently from the way it is documented or
//! quietly widened.
//!
//! `tests/lint_makefile_contract.rs` holds the same contract for the recipes,
//! and `tests/lint_gate_execution.rs` proves the recipes really run.

const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));
const DYLINT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/dylint.toml"));

/// The provisioning action CI must use for the Whitaker suite.
const WHITAKER_INSTALLER: &str = "leynos/shared-actions/.github/actions/install-whitaker@";

/// The workflow step that runs the aggregate lint target.
const LINT_COMMAND: &str = "run: make lint";

/// One line of a CI workflow, with the shape predicates its readers need.
struct Line<'a>(&'a str);

impl Line<'_> {
    /// Reports whether the line opens a CI job.
    ///
    /// A job is a key indented by exactly two spaces under `jobs:`. Steps are
    /// indented further, so they never match.
    fn is_job_header(&self) -> bool {
        let trimmed = self.0.trim_start();
        self.0.starts_with("  ")
            && !self.0.starts_with("   ")
            && !trimmed.starts_with('#')
            && trimmed.ends_with(':')
    }

    /// Reports whether the line is a `uses:` step, as opposed to a comment that
    /// merely names an action.
    fn is_uses_step(&self) -> bool {
        self.0.trim_start().starts_with("uses:")
    }
}

/// A CI workflow, viewed as text.
struct Workflow<'a>(&'a str);

impl<'a> Workflow<'a> {
    /// Returns the lines of the job that runs `make lint`.
    ///
    /// Comparing the first occurrence of each string across the whole file
    /// would keep passing if a second job ran the same command without
    /// installing Whitaker. Scoping both searches to that one job keeps the
    /// ordering assertion honest.
    fn lint_job(&self) -> Vec<&'a str> {
        let lines: Vec<&'a str> = self.0.lines().collect();
        let Some(step) = lines.iter().position(|line| line.trim() == LINT_COMMAND) else {
            return Vec::new();
        };
        let start = lines
            .iter()
            .take(step)
            .rposition(|line| Line(line).is_job_header())
            .unwrap_or(0);
        let end = lines
            .iter()
            .skip(step)
            .position(|line| Line(line).is_job_header())
            .map_or(lines.len(), |offset| step + offset);
        lines.into_iter().skip(start).take(end - start).collect()
    }
}

/// The `dylint.toml` configuration, viewed as text.
struct DylintConfig<'a>(&'a str);

impl<'a> DylintConfig<'a> {
    /// Reports whether the configuration exists and carries content.
    fn is_present(&self) -> bool {
        !self.0.trim().is_empty()
    }

    /// Returns the lines that are not comments or blank lines.
    ///
    /// `dylint.toml` explains the exclusion policy in prose, and that prose
    /// names the keys it warns against. Filtering comments keeps the
    /// description of the policy from reading as the policy itself.
    fn active_lines(&self) -> Vec<&'a str> {
        self.0
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect()
    }
}

/// Verifies that the job which runs `make lint` installs a pinned Whitaker.
#[test]
fn ci_installs_a_pinned_whitaker_before_running_the_aggregate() {
    let job = Workflow(CI).lint_job();
    assert!(
        !job.is_empty(),
        "CI must run `make lint`, the same aggregate target used locally"
    );

    let install_index = job
        .iter()
        .position(|line| Line(line).is_uses_step() && line.contains(WHITAKER_INSTALLER))
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
    let configuration = DylintConfig(DYLINT);
    assert!(
        configuration.is_present(),
        "dylint.toml must exist at the workspace root so the exclusion policy has a home"
    );
    assert!(
        configuration
            .active_lines()
            .iter()
            .all(|line| !line.starts_with("excluded_crates")),
        "dylint.toml must not exempt a whole crate; prefer `excluded_paths` with a recorded reason"
    );
}
