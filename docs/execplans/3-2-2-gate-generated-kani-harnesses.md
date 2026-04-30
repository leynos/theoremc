# Step 3.2.2: gate generated Kani harnesses

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

Complete the second Roadmap checkbox under Phase 3, Step 3.2 by making the
existing `theorem_file!` proc-macro expansion emit Kani proof harness metadata
without breaking ordinary Rust builds.

After this change, each generated harness stub remains in the deterministic
per-file module and backend submodule created by Step 3.2.1, but the Kani
backend module is compiled only when Rust is being checked with `cfg(kani)`.
When that configuration is active, each generated harness has the Kani
attributes required for discovery and bounded checking: `#[kani::proof]` and
`#[kani::unwind(n)]`, where `n` comes from the validated `Evidence.kani.unwind`
field in the theorem document. This satisfies the compile-time connectedness
requirement while deliberately stopping short of final theorem step semantics,
which remain Phase 4 work.

Observable success:

1. A normal non-Kani `cargo build` of a fixture crate with discovered theorem
   files succeeds without requiring the `kani` crate or any Kani-specific
   dependency.
2. The `theorem_file!` expansion contains a `#[cfg(kani)]` Kani backend module
   and emits one proof harness per theorem document with both `#[kani::proof]`
   and `#[kani::unwind(n)]`.
3. The value of `n` in each `#[kani::unwind(n)]` attribute is taken from that
   theorem document's validated `Evidence.kani.unwind` value, preserving
   document order for multi-document theorem files.
4. Kani-targeted fixture builds can discover the generated harness path under
   the existing deterministic layout:
   `__theoremc__file__...::kani::theorem__...__h...`.
5. Invalid evidence still fails before code generation through existing schema
   validation. Examples include `unwind: 0`, a missing witness when vacuity is
   not allowed, and `allow_vacuous: true` without `vacuity_because`.
6. Unit tests prove the exact generated token shape for single-document and
   multi-document theorem files, including different unwind values.
7. Behavioural tests using `rstest-bdd` v0.5.0 cover the theorem-author
   workflow for non-Kani compilation, Kani-configured discovery, and evidence
   rejection.
8. `docs/theoremc-design.md` records the implementation decision for Step
   3.2.2, `docs/users-guide.md` explains the behaviour visible to library
   consumers, and `docs/roadmap.md` marks the Step 3.2.2 checkbox done only
   after implementation and gates pass.
9. `make check-fmt`, `make lint`, and `make test` pass. Because documentation
   changes are in scope, `make fmt`, `make markdownlint`, and `make nixie` must
   also pass before the implementation is complete.

## Constraints

- This plan must not be implemented until the user explicitly approves it.
- Scope is limited to Roadmap Step `3.2.2`. Do not implement Phase 4 Kani
  backend semantics such as `kani::any`, `kani::assume`, `assert!`,
  `kani::cover!`, action lowering inside harness bodies, result policy, or
  concrete playback.
- Preserve the generated suite contract from Step 3.1.2:
  `build.rs` still emits one bare `theorem_file!("path/to/file.theorem");`
  invocation per discovered theorem file.
- Preserve the Step 3.2.1 per-file module contract:
  `theorem_file!("P")` still emits a private module named by
  `mangle_module_path(P)`, includes the source with
  `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", P))`, and uses
  `mangle_theorem_harness(P, T)` for each theorem document `T`.
- The Kani-specific attributes and any references to the `kani` crate must be
  under `#[cfg(kani)]` so non-Kani `cargo build` does not resolve the `kani`
  crate or Kani attribute macros.
- The Kani backend submodule name remains `kani`; do not rename it or change
  the full harness path shape documented by `TFS-6` and Step 3.2.1.
- The generated harness functions remain parameterless `fn()` stubs in this
  step. The body may stay empty unless a minimal marker is needed for discovery
  or lint neutrality.
- `Evidence.kani.unwind` is already required and validated as positive by
  `theoremc-core`; code generation should consume that typed value rather than
  reparsing YAML or duplicating validation.
- No new public API should be required. If a helper is extracted for testability
  inside `crates/theoremc-macros`, keep it crate-private unless a later
  approved step needs it publicly.
- Do not add a new external dependency unless the existing fixture and macro
  test infrastructure cannot prove Kani discovery in a maintainable way.
- Behavioural tests that model theorem-author workflows must use
  `rstest-bdd` v0.5.0, matching the repository's existing BDD suites.
- Run code quality gates sequentially and capture long output with `tee` into
  `/tmp` logs. Do not run format checks, lints, or tests in parallel.
- Documentation and comments must use en-GB-oxendict spelling and grammar.
- Keep Rust source files under 400 lines. If macro tests or fixture helpers
  grow too large, split them into focused sibling files.

## Tolerances (exception triggers)

- Approval: if the plan is not explicitly approved, do not make implementation
  changes.
- Scope: if implementation requires changing build discovery, generated suite
  rendering, schema validation semantics, or Phase 4 harness bodies, stop and
  ask for direction.
- Public API: if preserving existing `theoremc`, `theoremc-core`, and
  `theoremc-macros` public surfaces requires a breaking change, stop and
  present options.
- Dependencies: if proving Kani-targeted discovery appears to require adding a
  new crates.io dependency, stop after one prototype and document the trade-off.
- Kani tooling: if the local environment does not have `cargo kani` or cannot
  run a true Kani discovery command, keep deterministic token-shape tests plus
  a `cfg(kani)` fixture compile check if possible, record the limitation, and
  ask whether to require Kani in the gate.
- Fixture complexity: if the existing fixture crate harness cannot support a
  Kani-configured compile check after two focused attempts, stop and propose a
  smaller validation strategy instead of accumulating brittle scaffolding.
- Size: if the implementation grows beyond roughly 10 changed code files or
  600 net lines before documentation updates, stop and reassess whether this
  should be split.
- Validation: if any of `make check-fmt`, `make lint`, or `make test` still
  fails after five consecutive fix attempts, stop with the captured logs and
  summarize the remaining failure.

## Risks

- Risk: a non-Kani build may still try to resolve `kani` attributes if the
  generated `#[cfg(kani)]` is applied only to the function and not to the
  containing module or attribute tokens. Severity: high. Likelihood: medium.
  Mitigation: put `#[cfg(kani)]` on the generated backend `mod kani` and keep
  the non-Kani symbol anchor behind the same cfg or remove it when it would
  reference gated functions.

- Risk: existing behavioural tests refer to generated private symbols from
  ordinary Rust tests. If the whole `kani` module becomes `#[cfg(kani)]`, those
  tests will fail in normal builds. Severity: medium. Likelihood: high.
  Mitigation: update the fixture assertions so non-Kani tests prove successful
  compilation and macro expansion diagnostics, while Kani-specific symbol
  assertions run only under an explicit `cfg(kani)` compile path.

- Risk: a Kani-configured fixture compile may fail because `#[kani::proof]`
  requires the real Kani toolchain or an injected `kani` crate that is not
  present during ordinary CI. Severity: medium. Likelihood: medium. Mitigation:
  prefer a real `cargo kani list` or equivalent discovery command when
  available. If unavailable, prove emitted attributes with direct expansion
  tests and document the local tooling limitation.

- Risk: token snapshots can become noisy if they compare raw
  `TokenStream::to_string()` output after adding attributes. Severity: low.
  Likelihood: medium. Mitigation: continue the current `prettyplease` snapshot
  approach in `crates/theoremc-macros/src/tests.rs`, redacting only unstable
  hash suffixes.

- Risk: later Phase 4 semantic work may want additional marker attributes or
  metadata on generated harnesses. Severity: low. Likelihood: medium.
  Mitigation: keep this step narrowly aligned with `DES-7`, `DES-8`, and
  `TFS-6`; note future marker work in the design document instead of adding
  speculative API now.

- Risk: the prompt references `docs/rstest-bdd-users-guide.md`, but that file
  is absent in this checkout. Severity: low. Likelihood: high. Mitigation: use
  existing `tests/*_bdd.rs` suites and `Cargo.toml` as the local `rstest-bdd`
  v0.5.0 style reference, and record the absence in this plan.

## Progress

- [x] 2026-05-01: loaded the `execplans`, `leta`, `rust-router`,
  `arch-crate-design`, `rust-types-and-apis`, and `commit-message` skills
  relevant to planning this Rust proc-macro change and committing the plan.
- [x] 2026-05-01: confirmed the current branch is
  `feat/kani-harness-gate`, not `main`.
- [x] 2026-05-01: reviewed `AGENTS.md`, `docs/roadmap.md`,
  `docs/theoremc-design.md`, `docs/theorem-file-specification.md`,
  `docs/users-guide.md`, the Step 3.1.2 ExecPlan, and the Step 3.2.1 ExecPlan.
- [x] 2026-05-01: used a Wyvern agent team for read-only planning
  reconnaissance over design/spec signposts, testing documentation, and current
  proc-macro code shape.
- [x] 2026-05-01: confirmed the implementation centre is
  `crates/theoremc-macros/src/lib.rs`, with supporting tests in
  `crates/theoremc-macros/src/tests.rs` and behavioural coverage in
  `tests/theorem_file_macro_bdd.rs`.
- [x] 2026-05-01: confirmed Kani evidence is already parsed and validated by
  `crates/theoremc-core/src/schema/types.rs` and
  `crates/theoremc-core/src/schema/validate.rs`.
- [x] 2026-05-01: drafted this ExecPlan.
- [ ] Await explicit user approval before implementation.
- [ ] Milestone 0: add or adjust failing tests that describe the exact desired
  Kani-gated expansion shape.
- [ ] Milestone 1: update `theorem_file!` code generation so Kani harnesses are
  cfg-gated and carry `#[kani::proof]` plus evidence-derived
  `#[kani::unwind(n)]`.
- [ ] Milestone 2: update behavioural fixture coverage for non-Kani build
  success and Kani-configured harness discovery.
- [ ] Milestone 3: update `docs/theoremc-design.md`, `docs/users-guide.md`,
  and `docs/roadmap.md`.
- [ ] Milestone 4: run formatting, documentation, lint, and test gates
  sequentially with captured logs.
- [ ] Milestone 5: commit the completed implementation if all gates pass.

## Surprises & Discoveries

- 2026-05-01: `docs/rstest-bdd-users-guide.md` is referenced by the prompt, but
  it is not present in this checkout. Existing local BDD suites are the
  practical style reference for `rstest-bdd` v0.5.0.
- 2026-05-01: Step 3.2.1 has already created the proc-macro crate and the
  per-file module scaffold. Step 3.2.2 should not need a new architecture split.
- 2026-05-01: current normal-build behavioural tests refer directly to
  `super::<module>::kani::<harness>` from fixture unit tests. That assertion
  strategy must change when the `kani` module becomes unavailable outside
  `cfg(kani)`.
- 2026-05-01: validation for `unwind > 0`, witness requirements, and vacuity
  justification already exists in `theoremc-core`; this step should test that
  those failures still surface through the macro path rather than duplicating
  validation in the macro crate.

## Decision Log

- 2026-05-01: plan to gate the generated backend module with `#[cfg(kani)]`
  rather than only gating individual functions. Rationale: the module contains
  all Kani-specific attributes and keeps ordinary builds from resolving the
  `kani` attribute namespace.

- 2026-05-01: plan to keep the generated harness bodies as stubs. Rationale:
  Step 3.2.2 is about harness discovery metadata; final backend semantics are
  explicitly Phase 4 work.

- 2026-05-01: plan to source unwind values directly from validated
  `TheoremDoc.evidence.kani.unwind`. Rationale: `TFS-6` makes unwind required
  and positive, and `theoremc-core` is already the single source of schema
  validation truth.

- 2026-05-01: plan to preserve the build-script and generated-suite contract.
  Rationale: `DES-7` requires theorem files to stay connected to normal Rust
  compilation, and Step 3.1 already established the stable `theorem_file!(...)`
  handoff.

## Outcomes & Retrospective

No implementation has been performed yet. This ExecPlan is a draft awaiting
explicit approval.

## Implementation plan

### Milestone 0: establish failing tests

Start in `crates/theoremc-macros/src/tests.rs` and
`crates/theoremc-macros/src/tests_support.rs`. Extend the existing golden
expansion assertions so the expected generated code includes:

```rust
#[cfg(kani)]
pub(super) mod kani {
    #[kani::proof]
    #[kani::unwind(1)]
    pub(crate) fn theorem__smoke__hHASH() {}
}
```

For the multi-document test, use different unwind values such as `1` and `3` so
the test proves each attribute comes from the corresponding theorem document
rather than from a constant.

Add a focused negative macro-path test for an invalid evidence fixture with
`unwind: 0`. This should assert that the existing schema validation diagnostic
still reaches the macro error path before any tokens are emitted. If existing
schema tests already cover `unwind: 0`, keep the new test at the macro boundary
only.

Expected red result before implementation:

```plaintext
assertion failed: expansion is missing #[cfg(kani)] and Kani attributes
```

### Milestone 1: update macro generation

Modify `render_expansion` in `crates/theoremc-macros/src/lib.rs` so each
generated harness has an associated unwind literal. A straightforward approach
is to collect pairs of `(harness_ident, unwind_literal)` from
`theorem_docs.iter()`.

Use the existing validated schema shape. The code should not manually inspect
raw YAML. If the typed path to Kani evidence is optional in the Rust type, the
macro may treat `None` as unreachable after validation only if the code makes
that invariant explicit without panicking in production code. Prefer a small
helper returning `Result<Vec<GeneratedHarness>, MacroExpansionError>` so any
unexpected missing Kani evidence becomes a compile error with a clear message.

Render the generated module so Kani-only references are also cfg-gated. The
normal-build const anchor from Step 3.2.1 currently references
`kani::<harness>`. After gating the module, that anchor must either move inside
the `#[cfg(kani)] mod kani` body, become `#[cfg(kani)] const _: ...`, or be
removed if the generated items are otherwise lint-neutral. The selected shape
must keep `make lint` clean in both normal tests and any Kani-configured
fixture compile used by this step.

### Milestone 2: add behavioural coverage

Add `tests/features/kani_harness_gating.feature` and
`tests/kani_harness_gating_bdd.rs`, or extend the existing `theorem_file_macro`
feature if the scenarios remain cohesive. Prefer a new feature file if it keeps
Step 3.2.2 behaviour easy to find.

Cover these scenarios:

- A fixture crate with a valid theorem file builds with ordinary `cargo build`
  and no Kani dependency.
- A fixture crate built under a Kani configuration discovers the generated
  harness path or at least compiles code that references the cfg-gated harness
  path from within a `#[cfg(kani)]` test or item.
- A theorem file with invalid Kani evidence fails compilation with an
  actionable diagnostic at the macro callsite.

If the local machine has real Kani tooling available, use a targeted command
that lists or discovers harnesses rather than running full proofs. If that
tooling is unavailable, keep the BDD test to `cfg(kani)` compilation only when
that can be made deterministic, and document the limitation in
`Surprises & Discoveries` plus the final implementation note.

The fixture runner in `tests/theorem_file_macro_bdd/cargo_runner.rs` currently
supports `cargo build` and `cargo test`. If a `cfg(kani)` build is needed, add
a minimal, explicit runner variant that passes `RUSTFLAGS=--cfg kani` only for
that scenario. Do not mutate process-wide environment variables in tests.

### Milestone 3: update documentation

Update `docs/theoremc-design.md` near section 7.2 with the Step 3.2.2 decision:
the Kani module is now `#[cfg(kani)]`, generated harnesses carry
`#[kani::proof]` and `#[kani::unwind(n)]`, and harness bodies remain stubs
until Phase 4.

Update `docs/users-guide.md` in the build discovery and theorem harness naming
sections. Explain that normal builds still compile theorem files and validate
them through the proc macro, but the generated Kani proof harness module is
available only under `cfg(kani)`. Include the consumer-visible implication:
ordinary builds do not need Kani installed, while Kani runs can use the stable
full or short harness names.

Update `docs/roadmap.md` only after implementation and gates pass. Mark the
Step 3.2.2 checkbox done and leave later Phase 4 entries untouched.

### Milestone 4: validate sequentially

Use Makefile targets and capture output to `/tmp` logs. The exact log suffix
may use the current branch name:

```sh
make fmt 2>&1 | tee /tmp/fmt-theoremc-feat-kani-harness-gate.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-feat-kani-harness-gate.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-feat-kani-harness-gate.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-feat-kani-harness-gate.out
make lint 2>&1 | tee /tmp/lint-theoremc-feat-kani-harness-gate.out
make test 2>&1 | tee /tmp/test-theoremc-feat-kani-harness-gate.out
```

Expected successful output includes zero failures from each Make target. If a
command fails, read the corresponding `/tmp` log before changing code.

### Milestone 5: commit

After all gates pass, inspect the diff and commit the implementation as one
atomic change. Use the repository's file-based commit-message workflow:

```sh
git status --short
git diff --check
git diff
git add crates/theoremc-macros tests docs
COMMIT_MSG_DIR="$(mktemp -d)"
$EDITOR "$COMMIT_MSG_DIR/COMMIT_MSG.md"
git commit -F "$COMMIT_MSG_DIR/COMMIT_MSG.md"
rm -rf "$COMMIT_MSG_DIR"
```

The commit subject should be imperative, for example:

```plaintext
Gate generated Kani proof harnesses
```

Do not commit if any required quality gate fails.

## Documentation and skill signposts

Use these source documents while implementing:

- `docs/roadmap.md`: Phase 3, Step 3.2, second checkbox.
- `docs/theoremc-design.md`: `DES-7` build integration and `DES-8` Kani
  backend MVP.
- `docs/theorem-file-specification.md`: `TFS-6` evidence schema and section
  7.5 full harness path layout.
- `docs/name-mangling-rules.md`: stable module and harness naming constraints.
- `docs/users-guide.md`: consumer-facing build and harness behaviour.
- `docs/rust-testing-with-rstest-fixtures.md`: fixture style and parameterized
  test guidance.
- `docs/rust-doctest-dry-guide.md`: documentation example discipline.
- `docs/reliable-testing-in-rust-via-dependency-injection.md`: avoid global
  environment mutation in tests.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`: keep helpers
  small and split code when complexity rises.
- Existing local `tests/*_bdd.rs` suites: practical `rstest-bdd` v0.5.0 style
  reference, because `docs/rstest-bdd-users-guide.md` is absent.

Use these skills:

- `execplans`: keep this plan self-contained and update living sections during
  implementation.
- `leta`: navigate Rust symbols such as `render_expansion`,
  `mangle_theorem_harness`, and schema evidence types.
- `rust-router`: route Rust-specific design questions to the smallest useful
  specialist skill.
- `arch-crate-design`: preserve the existing `theoremc-core`,
  `theoremc-macros`, and root facade ownership split.
- `rust-types-and-apis`: keep evidence-to-generation data typed and avoid
  unnecessary public API expansion.
- `commit-message`: use a file-based commit message when committing the
  approved implementation.
