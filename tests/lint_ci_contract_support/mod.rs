//! Readers and verifier for the CI side of the workspace-wide lint gate.
//!
//! `tests/lint_ci_contract.rs` states the contract and the synthetic cases;
//! this module holds the YAML readers and the verifier they exercise, so each
//! file stays inside the repository's size limit. The workflow primitives come
//! from `tests/workflow_contract_support/mod.rs`, which the spelling contract
//! shares.

#[path = "../workflow_contract_support/mod.rs"]
mod workflow_contract_support;

#[path = "environment.rs"]
mod environment;

pub(crate) use workflow_contract_support::{
    Line, Workflow, has_bypass_key, jobs, key_indent, step_run_line, steps,
};

#[path = "verifier.rs"]
mod verifier;

pub(crate) use verifier::verify_lint_gate;

/// A pinned Whitaker install step, as the synthetic jobs spell it.
pub(crate) const INSTALL_STEP: &str = concat!(
    "      - name: Install Whitaker\n",
    "        uses: leynos/shared-actions/.github/actions/install-whitaker@",
    "0eabb6bc07148981de5da395e30cd78d83f5c7c2\n",
);

/// The aggregate lint step, as the synthetic jobs spell it.
pub(crate) const LINT_STEP: &str = "      - name: Lint\n        run: make lint\n";

/// Wraps job bodies in the workflow skeleton the synthetic cases share.
pub(crate) fn workflow_with(job_bodies: &[(&str, &str)]) -> String {
    job_bodies
        .iter()
        .fold(String::from("jobs:\n"), |mut text, (name, body)| {
            text.push_str("  ");
            text.push_str(name);
            text.push_str(":\n    runs-on: ubuntu-latest\n    steps:\n");
            text.push_str(body);
            text
        })
}

/// A single-job workflow whose body is `body`.
pub(crate) fn single_job(body: &str) -> String {
    workflow_with(&[("build-test", body)])
}

/// The CI workflow under contract.
pub(crate) const CI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/.github/workflows/ci.yml"
));

/// The provisioning action CI must use for the Whitaker suite.
pub(crate) const WHITAKER_INSTALLER: &str =
    "leynos/shared-actions/.github/actions/install-whitaker@";

/// The aggregate target CI must invoke, exactly.
pub(crate) const LINT_COMMAND: &str = "make lint";

/// Variables that redirect a lint stage away from its workspace default.
///
/// A workflow-level, job-level, or step-level assignment of any of these would
/// let CI run a narrower gate than the one `make lint` defines, so the contract
/// rejects them outright.
pub(super) const MAKEFLAGS_VARIABLES: [&str; 2] = ["MAKEFLAGS", "GNUMAKEFLAGS"];

/// Variables that redirect a lint stage away from its workspace default.
pub(super) const COVERAGE_VARIABLES: [&str; 8] = [
    "LINT_PACKAGES",
    "LINT_TARGET_FLAGS",
    "CARGO_FLAGS",
    "CLIPPY_FLAGS",
    "RUSTDOC_TARGET_FLAGS",
    "RUST_FLAGS",
    "RUSTDOC_FLAGS",
    "WHITAKER",
];

/// Returns a step's own `run:` command, without the key.
///
/// Only the step's own key is read, so a nested `env: { run: … }` is data. A
/// block scalar's `|` or `>` marker is replaced by the scalar's body, because
/// the command a step actually runs is the body, not the marker.
/// # Errors
///
/// Returns the reason when the command uses a form the reader cannot
/// interpret, because an unreadable command must not be treated as one that
/// does not invoke the aggregate.
fn run_command(step: &[Line<'_>], indent: usize) -> Result<Option<String>, String> {
    let Some(run_line) = step_run_line(step, indent) else {
        return Ok(None);
    };
    let trimmed = run_line.trimmed();
    let Some(raw) = trimmed
        .strip_prefix("- run:")
        .or_else(|| trimmed.strip_prefix("run:"))
        .map(str::trim)
    else {
        return Ok(None);
    };
    if raw.starts_with(['{', '[']) || raw.starts_with(['&', '*']) {
        return Err(format!(
            "a `run:` value here is `{raw}`, which this contract cannot read"
        ));
    }
    let value = unquote(raw);
    if !value.starts_with(['|', '>']) {
        return Ok(Some(value.to_owned()));
    }
    let body = step
        .iter()
        .skip_while(|line| !std::ptr::eq(*line, run_line))
        .skip(1)
        .take_while(|line| line.trimmed().is_empty() || line.indent() > indent)
        .filter(|line| !line.trimmed().is_empty())
        .map(|line| line.trimmed())
        .collect::<Vec<_>>()
        .join("\n");
    if body.is_empty() {
        return Err("a `run:` block scalar has no body".to_owned());
    }
    Ok(Some(body))
}

/// Removes one layer of YAML quoting from a scalar value.
///
/// YAML permits a plain, single-quoted, or double-quoted scalar, and the
/// quotes are syntax rather than part of the command, so `'make lint'` runs the
/// same command as `make lint`.
fn unquote(value: &str) -> &str {
    let trimmed = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = trimmed
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    trimmed
}

/// Reports whether a command invokes the aggregate lint target.
///
/// Each line of a block-scalar body is checked, so a `run: |` step that calls
/// `make lint` is recognized and then rejected for not being the exact
/// command. The target name is matched as a whole token, so `make
/// lint-whitaker` is a different command.
fn invokes_aggregate(command: &str) -> bool {
    command
        .lines()
        .any(|line| make_goals(line).contains(&"lint"))
}

/// Returns every goal the command asks Make to build.
///
/// Make accepts options before the goals, and several of them take the next
/// token as their argument. Skipping both keeps `make -s lint` and
/// `make -f Makefile lint` classified by the word `lint`, and every goal is
/// returned because `make clean lint` still invokes the aggregate. A shell
/// separator starts a new command, so `true && make lint` is read too.
fn make_goals(command: &str) -> Vec<&str> {
    let mut goals = Vec::new();
    // A command is a small shell program: a separator starts a new command,
    // so each segment is scanned and a later `make lint` is found.
    for line in command.split([';', '|', '&', '\n']) {
        goals.extend(segment_goals(line));
    }
    goals
}

/// Returns the goals of one shell segment when it is a Make invocation.
///
/// A pre-command variable assignment (`FOO=1 make lint`) is an
/// execution-control input this contract cannot reason about, so that segment
/// contributes no goal and the lint step is rejected as inexact.
fn segment_goals(segment: &str) -> Vec<&str> {
    let mut tokens = segment.split_whitespace();
    // `FOO=1 make lint` still invokes the aggregate; the assignment is an
    // execution-control input, so the command is read and then rejected as
    // inexact rather than skipped.
    let Some(command) = tokens.find(|word| !word.contains('=')) else {
        return Vec::new();
    };
    if command != "make" {
        return Vec::new();
    }
    let mut goals = Vec::new();
    let mut options_done = false;
    while let Some(token) = tokens.next() {
        if !options_done && token == "--" {
            options_done = true;
        } else if !options_done && token.starts_with('-') {
            if takes_argument(token) {
                tokens.next();
            }
        } else {
            options_done = true;
            goals.push(token);
        }
    }
    goals
}

/// Reports whether a Make option consumes the following token.
fn takes_argument(option: &str) -> bool {
    matches!(option, "-f" | "-C" | "-I" | "-o" | "-W" | "-j" | "-l")
}

/// Returns the reference a step's own `uses:` key names.
///
/// Both the block form (`uses: owner/repo@ref`) and the sequence-item form
/// (`- uses: owner/repo@ref`) are recognized, because either can install the
/// suite.
fn uses_reference<'a>(step: &'a [Line<'_>], indent: usize) -> Option<&'a str> {
    step.iter().find_map(|line| {
        let trimmed = line.trimmed();
        let candidate = if line.indent() == indent {
            trimmed.strip_prefix("uses:")
        } else if line.indent() + 2 == indent {
            trimmed.strip_prefix("- uses:")
        } else {
            None
        };
        candidate.map(str::trim)
    })
}
