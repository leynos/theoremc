# Make the lint pipeline workspace-complete and identical in Make and CI

This ExecPlan is a living document. The sections `Progress`,
`Surprises & discoveries`, `Decision log`, and `Outcomes & retrospective` must
be kept up to date as work proceeds.

This document must be maintained in accordance with `AGENTS.md`.

## Purpose / big picture

Issue #99 recorded that TheoremC's lint pipeline inspected only the root
package. The root `Cargo.toml` is a hybrid manifest that declares both
`[workspace]` and `[package]`, so a bare `cargo` invocation selects the root
`theoremc` package alone. Every member crate had already opted into
`[lints] workspace = true`, but `CARGO_FLAGS` carried no `--workspace`, rustdoc
ran with neither `--workspace` nor `--all-features`, `RUSTDOC_FLAGS` had no
default, there was no Whitaker target, CI never installed Whitaker, and no test
held any of it in place.

After this work, `make lint` is the single entry point locally and in CI. It
runs rustdoc, Clippy, and Whitaker in order over all six workspace packages,
with denied warnings, and a finding in any member fails it. Contract tests fail
if the Makefile narrows a stage, if CI stops installing a pinned Whitaker or
stops invoking the aggregate, if a lint step gains a bypass key or an
environment override, if a recipe can mask a failure, if an exclusion is added
without a reason, if a member stops inheriting the workspace lint tables, or if
a crate directory is added without a `members` entry.

The observable acceptance is a clean, unmodified `make lint` run and a passing
`make test`, with each acceptance criterion mapped to a Makefile line, a CI
line, a contract test, and a recorded run result.

## Conformance basis

The governing artefacts are issue #99 and its recorded coding plan, ADR 0013
(`docs/adrs/adr-0013-workspace-complete-lint-gate.md`), issue #98 for the
`leynos/netsuke` lint baseline, and `AGENTS.md`. The Netsuke reference is
`https://github.com/leynos/netsuke/blob/main/Makefile` and
`https://github.com/leynos/netsuke/blob/main/dylint.toml`, both read on
2026-11-23.

The trace is
`issue #99 criterion -> Makefile/CI line -> contract test -> gate result`. The
issue #98 trace is
`issue #98 baseline -> [workspace.lints.*] ->
tests/lint_workspace_inheritance_contract.rs -> make lint`.

## Context and orientation

`Makefile` names the lint scope once: `LINT_PACKAGES ?= --workspace`,
`LINT_TARGET_FLAGS ?= --all-targets --all-features`, `CARGO_FLAGS`,
`CLIPPY_FLAGS`,
`RUSTDOC_TARGET_FLAGS ?= $(LINT_PACKAGES) --all-features --no-deps`,
`RUST_FLAGS ?= -D warnings`, `RUSTDOC_FLAGS ?= -D warnings`, and
`WHITAKER ?= whitaker`. The aggregate recurses through `$(MAKE)` into
`lint-rustdoc`, `lint-clippy`, and `lint-whitaker`, so the stages stay
sequential under `make -j` and a failing stage stops the aggregate.

`.github/workflows/ci.yml` installs `cargo-orthohelp`, `mdtablefix`, uv, and
Whitaker through `leynos/shared-actions/.github/actions/install-whitaker`
pinned to a full commit SHA, then runs `make lint`.

`tests/lint_makefile_contract.rs` reads the recipes as text.
`tests/lint_ci_contract.rs` reads the workflow and `dylint.toml`.
`tests/lint_gate_execution.rs` runs the recipes against stub `cargo` and
`whitaker` executables. `tests/lint_workspace_execution.rs` writes a
two-package workspace and runs the real `cargo doc` and `cargo clippy`
invocations.

The workflow text readers live in `tests/workflow_contract_support/mod.rs`,
shared through `#[path]` by `tests/spelling_gate_contract_support/mod.rs` and
`tests/lint_ci_contract.rs`. Only the primitives both contracts use live there,
because each integration test is its own crate and an unused `pub(crate)` item
would fail the `-D warnings` build.

## Constraints

- `make lint` remains the aggregate entry point used locally and in CI.
- Rustdoc keeps `--workspace --all-features --no-deps` with
  `RUSTDOCFLAGS="-D warnings"`.
- Clippy keeps `--workspace --all-targets --all-features -- -D warnings`.
- Whitaker keeps `--all --workspace -- --all-targets --all-features` with
  `DYLINT_RUSTFLAGS="-D warnings"` and an empty `RUSTFLAGS`.
- No contract test may add a TOML or YAML parser dependency.
- No workspace lint level changes, and no per-rule pinning, in this work.
- Do not edit `.github/workflows/ci.yml` or the installer SHA without a
  recorded owner and QG-002 decision on the suite pin.
- Every new abstraction records its scope and re-use policy in the relevant
  design document.

## Tolerances (exception triggers)

- Scope: stop if the change exceeds roughly 900 net lines of code across the
  contract tests. The ExecPlan itself is excluded from this count.
- Dependency: stop if a contract test cannot be written without a TOML or YAML
  parser dependency.
- Drift: stop if `make lint` reports a finding that cannot be fixed at source
  and would need a `dylint.toml` exclusion.
- Decision: stop and escalate if a change would need the CI workflow or the
  installer SHA to change.

## Risks

- The shared test-support split could create dead-code warnings in either
  integration test binary. Mitigation: keep only the intersection of used
  primitives in the shared module and verify with Clippy over all targets.
- The full lint gate competes with other agents for compiler admission, so a
  queued run can look like a hang. Mitigation: watch `build-limits-status` and
  the heartbeat lines; never start a second build to "help".
- A synthetic workflow case could accidentally satisfy the real contract.
  Mitigation: every new rule carries a failing case and at least one accepted
  case.
- Editing `clippy.toml` could surface a finding. Mitigation: add only a
  disallowed method the workspace does not call, and re-run Clippy.

## Verification plan

Each obligation below is checked by a contract test that fails when the
implementation is wrong, plus one recorded gate run.

1. Rustdoc covers every package and all features, with warnings denied and
   `--no-deps`. Verified by
   `tests/lint_makefile_contract.rs::rustdoc_stage_denies_warnings_over_the_workspace`,
   `tests/lint_gate_execution.rs::aggregate_runs_rustdoc_then_clippy_then_whitaker`,
   and `tests/lint_workspace_execution.rs`.
2. Clippy covers every package, target, and feature with `-D warnings`.
   Verified by the matching Clippy contract tests and the stub run.
3. Whitaker covers every package, target, and feature with
   `DYLINT_RUSTFLAGS="-D warnings"`. Verified by the Whitaker contract test and
   the stub run.
4. Whitaker configuration exists with narrowly scoped, justified exclusions
   only. Verified by `the_dylint_configuration_exempts_no_whole_crate` and
   `every_committed_exclusion_is_justified`.
5. CI installs a pinned Whitaker and invokes the aggregate. Verified by
   `ci_runs_the_aggregate_lint_with_a_pinned_installer`, which checks every job
   that runs `make lint`.
6. A contract test prevents silent Make/CI drift, including bypass keys, make
   arguments, and environment overrides. Verified by the synthetic cases in
   `tests/lint_ci_contract.rs`.
7. The #98 baseline applies to the root package and every member. Verified by
   `tests/lint_workspace_inheritance_contract.rs` and by a direct comparison of
   `[workspace.lints.*]` against Netsuke, which found no missing rule.

Non-vacuity: each rule has a synthetic case that fails for the intended reason.
The repository-level cases are also negative controls in practice: they fail if
the committed file changes in the way the rule forbids.

## Milestones

1. Verify the shared gate and record evidence. Run `make lint` unmodified and
   `make -n lint`, and record the per-stage arguments and result.
2. Harden the Make, CI, and workspace inheritance contracts. End state: every
   drift path named above fails `make test`.
3. Confirm the #98 baseline and record the Whitaker suite-pin decision status.
   End state: ADR 0013 and the developer's guide describe the actual coverage,
   the inheritance rule, and the open suite-pin decision.

## Progress

- [x] (2026-11-23) Confirmed the earlier #99 work (PR #232) already landed the
      Makefile variables, the three stage targets, `dylint.toml`, and the CI
      Whitaker install.
- [x] (2026-11-23) Split the workflow text readers into
      `tests/workflow_contract_support/mod.rs` and re-exported them from
      `tests/spelling_gate_contract_support`.
- [x] (2026-11-23) Hardened `tests/lint_ci_contract.rs`: every lint job,
      bypass keys, make arguments, environment overrides, installer pin, and
      exclusion justifications, each with synthetic failing cases.
- [x] (2026-11-23) Rejected `exit 0` in `tests/lint_makefile_contract.rs`.
- [x] (2026-11-23) Added `tests/lint_workspace_inheritance_contract.rs`.
- [x] (2026-11-23) Asserted the lint name and member source path in
      `tests/lint_workspace_execution.rs`.
- [x] (2026-11-23) Contract tests, Clippy, formatting, Markdown lint, Mermaid,
      ADR checks, and spelling pass.
- [x] (2026-11-23) Recorded the unmodified `make lint` result for the final
      tree: exit 0, zero errors, all three stages in order.
- [x] (2026-11-23) Pushed the branch and opened draft pull request 279.
- [x] (2026-11-24) Answered ten CodeRabbit review passes, fixing every finding
      at source with a case that fails when the fix is reverted. Hosted CI was
      green on each code commit.

## Surprises & discoveries

- The issue's "validated gaps" section was already stale: PR #232, merged on
  2026-11-08, had landed the Makefile variables, the three stage targets,
  `dylint.toml`, the CI install, and four contract tests. The remaining work
  was the hardening and decision record described in the coding plan.
- Issue #98 had also landed, in PRs #233, #234, and #235. Comparing
  `[workspace.lints.*]` against Netsuke's current `main` found no missing rule;
  TheoremC adds `cognitive_complexity` on top.
- `clippy.toml` was missing two Netsuke `disallowed-methods` entries.
  `std::io::IsTerminal::is_terminal` was added; `shell_quote::QuoteRefExt` is
  omitted because TheoremC has no `shell-quote` dependency, and the omission is
  recorded beside the list.
- A shared test-support module cannot simply be included by two integration
  tests: an item used by only one binary fails `dead_code` under `-D warnings`
  in the other. The module was reduced to the shared primitives, and the
  spelling-specific composition stayed with the spelling contract.

## Decision log

- Keep `make lint` as the shared entry point and harden it rather than adding a
  second CI-only target. This preserves the "identical locally and in CI"
  property the issue asks for.
- Reuse the existing workflow readers through `#[path]` rather than copying
  them, and split them into `tests/workflow_contract_support/mod.rs` so each
  binary uses every item. Recorded here because the split is a structural
  change the plan anticipated but did not name.
- Do not treat the inheritance contract as completion of #98. #98 landed
  separately; the contract is the delivery mechanism that keeps its baseline
  applied to every package.
- Leave the CI workflow and installer SHA untouched. The suite pin remains an
  open owner and QG-002 decision; ADR 0013 records that only the installer
  action is pinned and the suite is rolling.

## Surprises & discoveries (review loop)

CodeRabbit ran ten times against the branch. The first review found seven
findings, the second ten, and later passes found five to nine each. They were
not repetitions: each named a distinct YAML or TOML form the handwritten
readers treated as absent, and several were real bypasses rather than
robustness suggestions. A second job could run `make -s lint` or
`env make lint` behind a valid first job; a `}` inside a `#` comment closed a
flow mapping before the override beneath it; a step slice could absorb a
sibling key holding `run: make spelling`.

The decisive change was not any single fix but the stance. The readers now
distinguish "no such key" from "a form I cannot interpret", and report the
second as an error. Without that, every tolerance added widened the set of
inputs that passed by being skipped. With it, a syntax the checker does not
understand fails the contract, which is the safe direction for a gate.

## Outcomes & retrospective

Delivered: a workspace-complete lint pipeline behind one aggregate target, with
contract tests that fail on each known drift path, plus documentation that
states the real per-stage coverage and the inheritance rule. The most useful
lesson is that "the workspace" and "the policy" are separate contracts: a gate
can visit every package while a package silently replaces the policy, and only
the inheritance contract catches that.

## Revision note

2026-11-23: initial record, written after the implementation because the coding
plan was supplied with the issue. It captures the discovered state, the
decisions taken, and the evidence still outstanding at the time of writing.
