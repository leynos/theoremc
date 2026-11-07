//! Contract tests for the Makefile side of the workspace-wide lint gate.
//!
//! Rustdoc, Clippy, and Whitaker must each cover every workspace package,
//! target, and feature set, with warnings denied, and `make lint` must be the
//! single entry point that runs them in order. These tests read the Makefile as
//! text, so a change that narrows a gate, drops a stage, or lets a failure
//! escape fails here rather than after a merge.
//!
//! `tests/lint_ci_contract.rs` holds the same contract for the workflow and the
//! Dylint configuration. `tests/lint_gate_execution.rs` proves the wiring by
//! running the recipes against stub executables, and
//! `tests/lint_workspace_execution.rs` proves that Cargo acts on the workspace
//! scope those recipes pass.

const MAKEFILE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Makefile"));

/// A `?=` variable the lint gate composes its commands from.
///
/// Naming the variables as variants keeps a rename in the Makefile from
/// silently turning an assertion into a check of `None`.
#[derive(Clone, Copy, Debug)]
enum Variable {
    /// The package scope every lint command shares.
    LintPackages,
    /// The warning denial Clippy and Whitaker inherit.
    RustFlags,
    /// The warning denial the rustdoc stage passes.
    RustdocFlags,
    /// The target and feature scope every lint command shares.
    LintTargetFlags,
    /// The Cargo flags the Clippy stage composes.
    CargoFlags,
    /// The Cargo flags the rustdoc stage composes.
    RustdocTargetFlags,
    /// The Clippy flags, which append the warning denial.
    ClippyFlags,
}

impl Variable {
    /// The variable's name in the Makefile.
    fn name(self) -> &'static str {
        match self {
            Self::LintPackages => "LINT_PACKAGES",
            Self::RustFlags => "RUST_FLAGS",
            Self::RustdocFlags => "RUSTDOC_FLAGS",
            Self::LintTargetFlags => "LINT_TARGET_FLAGS",
            Self::CargoFlags => "CARGO_FLAGS",
            Self::RustdocTargetFlags => "RUSTDOC_TARGET_FLAGS",
            Self::ClippyFlags => "CLIPPY_FLAGS",
        }
    }
}

/// A Makefile target the lint gate is built from.
#[derive(Clone, Copy, Debug)]
enum Target {
    /// The aggregate CI invokes.
    Lint,
    /// The rustdoc stage.
    LintRustdoc,
    /// The Clippy stage.
    LintClippy,
    /// The Whitaker stage.
    LintWhitaker,
}

impl Target {
    /// Every target the contract covers.
    const ALL: [Self; 4] = [
        Self::Lint,
        Self::LintRustdoc,
        Self::LintClippy,
        Self::LintWhitaker,
    ];

    /// The target's name in the Makefile.
    fn name(self) -> &'static str {
        match self {
            Self::Lint => "lint",
            Self::LintRustdoc => "lint-rustdoc",
            Self::LintClippy => "lint-clippy",
            Self::LintWhitaker => "lint-whitaker",
        }
    }
}

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
    fn variable_default(&self, variable: Variable) -> Option<&'a str> {
        let prefix = format!("{} ?=", variable.name());
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
    fn recipe_lines(&self, target: Target) -> Vec<&'a str> {
        let header = format!("{}:", target.name());
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
    fn single_recipe_line(&self, target: Target) -> &'a str {
        let recipe = self.recipe_lines(target);
        assert_eq!(
            recipe.len(),
            1,
            "{} must be exactly one recipe line, so a failure cannot be hidden behind a later line; found {recipe:?}",
            target.name()
        );
        recipe.first().copied().unwrap_or_default()
    }
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
        makefile.variable_default(Variable::LintPackages),
        Some("--workspace"),
        "LINT_PACKAGES must name the workspace, because a bare `cargo` invocation selects only the root package"
    );
    assert_eq!(
        makefile.variable_default(Variable::RustdocFlags),
        Some("-D warnings"),
        "RUSTDOC_FLAGS must deny warnings by default"
    );
    assert_eq!(
        makefile.variable_default(Variable::RustFlags),
        Some("-D warnings"),
        "RUST_FLAGS must deny warnings by default, because Clippy and Whitaker inherit it"
    );
    assert_eq!(
        makefile.variable_default(Variable::LintTargetFlags),
        Some("--all-targets --all-features"),
        "LINT_TARGET_FLAGS must cover every target and feature"
    );

    let cargo_flags = makefile
        .variable_default(Variable::CargoFlags)
        .unwrap_or_default();
    Contains {
        value: cargo_flags,
        fragments: &["$(LINT_PACKAGES)", "$(LINT_TARGET_FLAGS)"],
    }
    .assert_all();

    let rustdoc_flags = makefile
        .variable_default(Variable::RustdocTargetFlags)
        .unwrap_or_default();
    Contains {
        value: rustdoc_flags,
        fragments: &["$(LINT_PACKAGES)", "--all-features", "--no-deps"],
    }
    .assert_all();

    let clippy_flags = makefile
        .variable_default(Variable::ClippyFlags)
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
        Makefile(MAKEFILE).recipe_lines(Target::Lint),
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
    let line = Makefile(MAKEFILE).single_recipe_line(Target::LintRustdoc);
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
    let line = Makefile(MAKEFILE).single_recipe_line(Target::LintClippy);
    Contains {
        value: line,
        fragments: &["$(CARGO) clippy $(CLIPPY_FLAGS)"],
    }
    .assert_all();
}

/// Verifies that the Whitaker stage covers the workspace and clears `RUSTFLAGS`.
#[test]
fn whitaker_stage_denies_warnings_over_the_workspace() {
    let line = Makefile(MAKEFILE).single_recipe_line(Target::LintWhitaker);
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
    for target in Target::ALL {
        for line in makefile.recipe_lines(target) {
            assert!(
                !line.starts_with('-'),
                "{} must not ignore a recipe failure: `{line}`",
                target.name()
            );
            assert!(
                !line.contains("||"),
                "{} must not swallow a failure behind a shell operator: `{line}`",
                target.name()
            );
            assert!(
                !line.contains("; true"),
                "{} must not exit successfully after a failure: `{line}`",
                target.name()
            );
        }
    }
}
