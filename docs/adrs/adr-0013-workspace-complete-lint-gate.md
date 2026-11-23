# Architectural decision record (ADR) 0013: Workspace-complete lint gate

## Status

Accepted. Rustdoc, Clippy, and the Whitaker Dylint suite all run over every
workspace package, and Clippy and Whitaker also cover every target, behind one
sequential `make lint` aggregate that CI invokes unchanged.

The three stages do not carry identical scope. `cargo doc` rejects
`--all-targets`, so rustdoc covers the packages and the documentation targets
only. `--all-features` enables every feature together; it does not build each
feature combination in turn.

## Date

2026-11-07

## Context and problem statement

The root `Cargo.toml` is a hybrid manifest: it declares both `[workspace]` and
`[package]`. Cargo therefore selects only the root `theoremc` package when a
command carries no package flag. Every member crate already opted into
`[lints] workspace = true`, but the lint recipes did not visit the members.

Issue #99 recorded the resulting gaps. `CARGO_FLAGS` held
`--all-targets --all-features` but not `--workspace`, so the Whitaker stage
checked the root package alone. The rustdoc stage ran `cargo doc --no-deps`
with neither `--workspace` nor `--all-features`. There was no per-stage target,
no root `dylint.toml`, and no contract test, so any of these could drift back
without notice.

Two further facts shaped the decision. Whitaker runs under the toolchain its
installer manages, not the repository toolchain, so its stage must clear
`RUSTFLAGS`. And the three stages write to shared build state, so running them
concurrently wastes more than it saves.

## Decision Drivers

- Make under-checking impossible rather than merely unlikely.
- Keep one entry point, `make lint`, for local use and CI.
- Keep the stages sequential without relying on how a particular `make`
  invocation happens to schedule prerequisites.
- Fail loudly: no stage may swallow an error, and no gate may soft-skip when a
  tool is missing.
- Keep the lint policy visible in one place, with exemptions justified where
  they are declared.

## Options considered

### Option A: keep the stages as prerequisites of `lint`

Rejected. `lint: lint-rustdoc lint-clippy lint-whitaker` is ordered only when
`make` runs serially. Under `make -j` the three stages start together, contend
for the shared Cargo target directory and the compiler-slot pool, and produce
interleaved output that hides which stage failed.

### Option B: declare `.NOTPARALLEL`

Rejected. `.NOTPARALLEL` changes scheduling for the whole `make` invocation, or
for whatever set of targets the installed `make` version applies it to. A local
ordering requirement should not reach beyond the target that needs it.

### Option C: recurse through `$(MAKE)` from the `lint` recipe

Accepted. Each stage occupies its own recipe line, so `make` runs them in the
listed order even under `-j`. A failing line aborts the recipe, and `make`
reports that stage's failure as its own. Command-line overrides such as
`CARGO=` and `WHITAKER=` propagate to the recursive invocations, which keeps
the stub-executable contract test possible.

## Decision

1. `LINT_PACKAGES ?= --workspace` and
   `LINT_TARGET_FLAGS ?= --all-targets --all-features` name the lint scope once.
   `CARGO_FLAGS ?= $(LINT_PACKAGES) $(LINT_TARGET_FLAGS)` and
   `CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)` derive from them, so a
   gate cannot fall back to the root package without changing a variable every
   gate shares.
2. `RUSTDOC_TARGET_FLAGS ?= $(LINT_PACKAGES) --all-features --no-deps` gives
   documentation its own flags, because `cargo doc` rejects `--all-targets`.
   `RUSTDOC_FLAGS ?= -D warnings` denies documentation warnings.
3. `make lint` recurses into `lint-rustdoc`, `lint-clippy`, then
   `lint-whitaker`. The Whitaker stage runs
   `whitaker --all --workspace -- --all-targets --all-features` with
   `DYLINT_RUSTFLAGS="-D warnings"` and an empty `RUSTFLAGS`. `--workspace` is
   Whitaker's own flag and Whitaker forwards it to the `cargo check` it runs,
   so repeating it after `--` makes Cargo reject the run as a duplicate; the
   forwarded arguments carry only `LINT_TARGET_FLAGS`.
4. A root `dylint.toml` carries the exclusion policy. It declares no
   exclusions, because the suite is clean over every package, target, and
   feature set. A future exemption must use the narrowest key that works,
   `excluded_paths` before `excluded_crates`, with the reason recorded beside
   it. `tests/lint_dylint_contract.rs` rejects an active `excluded_paths` entry
   with no comment immediately above it, so an exemption cannot arrive
   unexplained.
5. Six tests hold the wiring, at four levels.
   `tests/lint_makefile_contract.rs`, `tests/lint_ci_contract.rs`, and
   `tests/lint_dylint_contract.rs` read the Makefile, the workflow, and
   `dylint.toml` as text and fail if a gate loses workspace coverage, a stage
   disappears, a failure can be masked, CI stops installing a pinned Whitaker,
   a lint step gains a bypass key or an environment override, or the
   configuration gains an exclusion with no recorded reason.
   `tests/lint_gate_execution.rs` runs the recipes against stub executables to
   prove the stage order, the arguments each stage receives, and that a failing
   stage stops the aggregate. `tests/lint_workspace_execution.rs` goes one
   level further: it writes a two-package workspace that mirrors the hybrid
   manifest, runs the real `cargo doc` and `cargo clippy` invocations the
   recipes use, and shows that a clean workspace passes while a finding in the
   non-root member fails the stage, naming the lint and the member source path.
   The stubs prove the wiring; this proves Cargo acts on it.
   `tests/lint_workspace_inheritance_contract.rs` closes the last gap between
   "the gate visits the workspace" and "the policy binds every package": it
   requires `[lints] workspace = true` in the root manifest and in each member,
   rejects a member that defines its own `[lints.clippy]`, `[lints.rust]`, or
   `[lints.rustdoc]` table, and fails for a `crates/*` directory that no
   `members` entry names.
6. CI installs Whitaker through the shared `install-whitaker` action pinned to a
   full commit SHA and then runs `make lint`, the same target contributors run
   locally. The installer-action pin is the revision of the provisioning
   action; it does not pin the Whitaker suite the action installs, which
   remains rolling until a suite-pin decision is recorded.

## Consequences

- `make lint` is slower than before, because it now documents and lints five
  member crates and the root package rather than the root package alone. The
  work is bounded and cached, and the gates run sequentially by design.
- A finding in any member crate fails the same target that CI runs, so the
  under-checked window that issue #99 described is closed.
- Two source findings surfaced when the suite first ran workspace-wide: a test
  module without an inner doc comment, and a test module three lines over the
  400-line budget. Both were fixed at source rather than exempted.
- The end-to-end test adds compiler work to `make test`. It joins the
  `nested-cargo-builds` group in `.config/nextest.toml`, which runs one such
  test at a time so the shared package-cache lock and compiler admission do not
  turn a cold build into an apparent hang.
- `dylint.toml` exists, so the exclusion policy has a home. It declares no
  exclusions today, which the contract test holds.
- The Netsuke lint-table alignment tracked by issue #98 landed separately, in
  pull requests 233, 234, and 235. The `[workspace.lints.*]` tables now carry
  that baseline, and every one of the six packages inherits it. Inheritance is
  enforced rather than assumed: `tests/lint_workspace_inheritance_contract.rs`
  fails if a package stops opting in, if a package defines a local lint table,
  or if a crate directory is added without a `members` entry.
- The Whitaker suite pin remains an open decision. CI pins the
  `install-whitaker` action to a full commit SHA, so the provisioning action is
  reproducible, but the suite revision that action installs is rolling. Until
  the owner and QG-002 record a suite-pin decision, a suite update can change
  which lints `make lint` runs without any change in this repository. Local
  contributors install through `whitaker-installer`, which carries the same
  property.

## Addendum (2026-11-12): renumbering

This record was formerly filed as ADR 012 in
`docs/adr-012-workspace-complete-lint-gate.md`. The documentation style guide
now requires a four-digit, zero-padded serial that is unique among the ADRs, in
a canonical `adrs/adr-nnnn-title-slug.md` filename. The record now carries
serial 0013, and its serial is stable from this point.

References to "ADR 012" or "ADR-012" in earlier discussions describe this
decision. They are historical context only and do not name a current record.

## Related documents

- [ADR-0005: Rust build standard](adr-0005-adopt-the-rust-build-standard-for-development-builds.md)
- [ADR-0010: reusable-library MSRV for serde-saphyr
  2.0](adr-0010-reusable-library-msrv-for-serde-saphyr-2-0.md)
- [Developer's guide](../developers-guide.md)
- [Theoremc design](../theoremc-design.md)
