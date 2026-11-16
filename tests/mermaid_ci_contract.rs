//! Contract test for the CI half of the Mermaid gate.
//!
//! `make nixie` validates every Mermaid block in the documentation, so it is a
//! repository gate rather than a convenience. CI must therefore run it in the
//! job the ruleset requires, and it must install Nixie there first, pinned to a
//! commit, or the renderer behind the gate could change without a review. The
//! workflow is read as text, the same way `tests/lint_ci_contract.rs` reads its
//! gate.

const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));

/// The job the ruleset requires, so the only job whose steps gate a merge.
const REQUIRED_JOB: &str = "build-test";

/// The provisioning action CI must use for the Mermaid renderer.
const NIXIE_INSTALLER: &str = "leynos/shared-actions/.github/actions/install-nixie@";

/// The workflow step that runs the Mermaid gate.
const MERMAID_COMMAND: &str = "run: make nixie";

/// A job key under `jobs:`.
#[derive(Clone, Copy)]
struct JobName<'a>(&'a str);

impl JobName<'_> {
    /// Returns the header line this job would have.
    fn header(self) -> String {
        format!("{}:", self.0)
    }
}

/// A step command as a workflow spells it.
#[derive(Clone, Copy)]
struct Command<'a>(&'a str);

/// An action named by a `uses:` step.
#[derive(Clone, Copy)]
struct Action<'a>(&'a str);

/// One line of a workflow.
#[derive(Clone, Copy)]
struct Line<'a>(&'a str);

impl<'a> Line<'a> {
    /// Reports whether the line opens a job.
    ///
    /// A job key is indented by exactly two spaces under `jobs:`; step keys are
    /// indented further and comments are skipped, so neither can match.
    fn is_job_header(self) -> bool {
        let trimmed = self.0.trim_start();
        self.0.starts_with("  ")
            && !self.0.starts_with("   ")
            && !trimmed.starts_with('#')
            && trimmed.trim_end().ends_with(':')
    }

    /// Reports whether the line opens `name`.
    fn opens(self, name: JobName<'_>) -> bool {
        self.is_job_header() && self.0.trim() == name.header()
    }

    /// Returns the line without its YAML list marker, so a step written as
    /// `- run: ...` and one written as `- name: ...` followed by `run: ...`
    /// are read the same way.
    fn without_marker(self) -> &'a str {
        let trimmed = self.0.trim();
        trimmed.strip_prefix("- ").unwrap_or(trimmed).trim_start()
    }

    /// Reports whether the line is a `uses:` step naming `action`.
    fn uses(self, action: Action<'_>) -> bool {
        self.without_marker().starts_with("uses:") && self.0.contains(action.0)
    }

    /// Reports whether the line runs `command`.
    fn runs(self, command: Command<'_>) -> bool {
        self.without_marker() == command.0
    }

    /// Returns what follows the last `@` of a `uses:` line, for a pin check.
    fn reference(self) -> &'a str {
        self.0
            .rsplit('@')
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .next()
            .unwrap_or_default()
    }
}

/// A workflow file, viewed as the lines it holds.
#[derive(Clone, Copy)]
struct Workflow<'a>(&'a str);

impl<'a> Workflow<'a> {
    /// Returns the lines of the job the ruleset requires, or nothing when the
    /// workflow has no such job.
    ///
    /// Searching the whole file instead would keep passing after the installer
    /// moved to another job, which is the arrangement this contract rejects.
    fn required_job(self) -> Vec<Line<'a>> {
        let lines: Vec<Line<'a>> = self.0.lines().map(Line).collect();
        let name = JobName(REQUIRED_JOB);
        let Some(start) = lines.iter().position(|line| line.opens(name)) else {
            return Vec::new();
        };
        let end = lines
            .iter()
            .skip(start + 1)
            .position(|line| line.is_job_header())
            .map_or(lines.len(), |offset| start + 1 + offset);
        lines.into_iter().skip(start).take(end - start).collect()
    }

    /// Returns the complaints about how the required job provisions the
    /// Mermaid gate, in the order a reader would fix them.
    fn problems(self) -> Vec<String> {
        let job = self.required_job();
        if job.is_empty() {
            return vec![format!("CI must define the `{REQUIRED_JOB}` job")];
        }
        let command = Command(MERMAID_COMMAND);
        let Some(gate) = job.iter().position(|line| line.runs(command)) else {
            return vec![format!(
                "the `{REQUIRED_JOB}` job must run `make nixie`, the Mermaid target contributors run locally"
            )];
        };
        let action = Action(NIXIE_INSTALLER);
        let Some(install) = job.iter().position(|line| line.uses(action)) else {
            return vec![String::from(
                "the job that runs `make nixie` must install Nixie through the shared action, in the same job",
            )];
        };
        let mut complaints = Vec::new();
        let reference = job.get(install).map_or("", |line| line.reference());
        if !is_commit(reference) {
            complaints.push(format!(
                "the Nixie action must be pinned to a full commit SHA; found `{reference}`"
            ));
        }
        if install > gate {
            complaints.push(String::from(
                "CI must install Nixie before the Mermaid step, in the same job",
            ));
        }
        complaints
    }
}

/// Reports whether `reference` is a full commit SHA.
fn is_commit(reference: &str) -> bool {
    reference.len() == 40 && reference.chars().all(|digit| digit.is_ascii_hexdigit())
}

/// The text a complaint must contain, so a case names its own evidence.
#[derive(Clone, Copy)]
struct Expected<'a>(&'a str);

/// Verifies that the required job installs a pinned Nixie and then runs the
/// Mermaid gate.
#[test]
fn the_required_job_provisions_the_mermaid_gate() {
    let problems = Workflow(CI).problems();
    assert!(
        problems.is_empty(),
        "CI must provision the Mermaid gate; found:\n{}",
        problems.join("\n")
    );
}

/// Verifies that each way of breaking the arrangement is rejected, so the
/// contract above cannot pass vacuously.
#[rstest::rstest]
#[case::installer_in_another_job(
    Workflow(concat!(
        "jobs:\n",
        "  lint:\n",
        "    steps:\n",
        "      - uses: leynos/shared-actions/.github/actions/install-nixie@",
        "0123456789abcdef0123456789abcdef01234567\n",
        "  build-test:\n",
        "    steps:\n",
        "      - run: make nixie\n",
    )),
    Expected("in the same job")
)]
#[case::installer_pinned_to_a_tag(
    Workflow(concat!(
        "jobs:\n",
        "  build-test:\n",
        "    steps:\n",
        "      - uses: leynos/shared-actions/.github/actions/install-nixie@v1\n",
        "      - run: make nixie\n",
    )),
    Expected("full commit SHA")
)]
#[case::gate_missing(
    Workflow(concat!(
        "jobs:\n",
        "  build-test:\n",
        "    steps:\n",
        "      - uses: leynos/shared-actions/.github/actions/install-nixie@",
        "0123456789abcdef0123456789abcdef01234567\n",
    )),
    Expected("must run `make nixie`")
)]
#[case::gate_before_installer(
    Workflow(concat!(
        "jobs:\n",
        "  build-test:\n",
        "    steps:\n",
        "      - run: make nixie\n",
        "      - uses: leynos/shared-actions/.github/actions/install-nixie@",
        "0123456789abcdef0123456789abcdef01234567\n",
    )),
    Expected("before the Mermaid step")
)]
#[case::required_job_missing(
    Workflow(concat!(
        "jobs:\n",
        "  other:\n",
        "    steps:\n",
        "      - run: make nixie\n",
    )),
    Expected("must define the `build-test` job")
)]
fn a_broken_arrangement_is_rejected(
    #[case] workflow: Workflow<'_>,
    #[case] expected: Expected<'_>,
) {
    let problems = workflow.problems();
    assert!(
        problems.iter().any(|problem| problem.contains(expected.0)),
        "the contract must report `{}`; found {problems:?}",
        expected.0
    );
}
