# Step 3.1.2: generate `OUT_DIR/theorem_suite.rs` and wire the crate include

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

Complete the second half of Roadmap Phase 3, Step 3.1 so theorem discovery no
longer stops at Cargo invalidation. After this change, every build will also
materialize a deterministic `OUT_DIR/theorem_suite.rs` file and the crate will
always include that generated suite during compilation.

The observable outcome is simple: a crate with no theorem files still compiles,
a crate with one theorem file compiles without manual wiring, and a crate with
many theorem files compiles in a stable order regardless of filesystem
traversal order. This step deliberately stops short of theorem execution and
real per-file harness generation. It establishes the compile-time suite seam
that Step 3.2 will later replace with the real `theorem_file!` proc-macro
expansion.

Observable success:

1. `build.rs` writes `OUT_DIR/theorem_suite.rs` from the already-sorted theorem
   file list returned by `src/build_discovery.rs`.
2. `src/lib.rs` includes that generated suite on every build through a hidden
   internal integration point, so empty, single-file, and multi-file theorem
   trees all compile.
3. Unit tests prove exact generated suite contents for empty, single-file, and
   multi-file inputs, including deterministic ordering and newline policy.
4. Behavioural tests using `rstest-bdd` v0.5.0 prove the real Cargo workflow
   for:
   - no theorem files,
   - one discovered theorem file, and
   - multiple discovered theorem files created in non-sorted order.
5. `docs/theoremc-design.md` records the design decisions for the generated
   suite seam, `docs/users-guide.md` explains the new always-included build
   behaviour, and `docs/roadmap.md` marks the Step 3.1 suite-generation entry
   done after all gates pass.
6. `make check-fmt`, `make lint`, and `make test` pass. Because documentation
   changes are also in scope, `make fmt`, `make markdownlint`, and
   `make nixie` must pass before implementation is considered complete.

## Constraints

- Scope is limited to Roadmap Step `3.1.2`. Do not implement theorem
  execution, Kani harness semantics, typed probes, or the real proc-macro
  expansion planned for Step `3.2`.
- The generated suite file must contain one `theorem_file!(...)` invocation per
  discovered theorem path, using the crate-relative forward-slash paths already
  produced by `src/build_discovery.rs`.
- The seam between Step `3.1.2` and Step `3.2` must stay narrow. Step `3.1.2`
  should establish the generated file format and include site, while Step
  `3.2` owns the real per-file expansion behind the same `theorem_file!(...)`
  callsite.
- The current repository has no proc-macro crate and no existing
  `theorem_file!` macro. This step must therefore compile without introducing
  the full proc-macro architecture prematurely.
- Empty theorem trees remain a first-class supported case. The generated suite
  and crate include wiring must not require a pre-seeded `theorems/`
  directory.
- Keep `build.rs` thin. Rendering and writing the suite file should live in a
  shared helper module under `src/` so direct unit tests can exercise it
  without shelling out to Cargo.
- Follow existing filesystem conventions: prefer `cap_std` and `camino` over
  `std::fs` and `std::path` in new filesystem-heavy code.
- Preserve the repository's file-size rule: no code file may exceed 400 lines.
  If the helper grows, split tests into sibling `*_tests.rs` files.
- Behavioural tests must use `rstest-bdd` v0.5.0 where they model the
  theorem-author Cargo workflow.
- Documentation and comments must use en-GB-oxendict spelling and grammar.

## Tolerances (exception triggers)

- If satisfying the acceptance criteria requires implementing substantial parts
  of Step `3.2` proc-macro expansion rather than a narrow bridge, stop and
  escalate before proceeding.
- If a clean solution appears to require a new external dependency, stop and
  document the trade-off first. The preferred outcome is to reuse existing
  dependencies only.
- If the generated-suite design requires a new public API or exported macro
  surface for external consumers, stop and revisit the approach. This step
  should stay internal to the crate.
- If Cargo behavioural tests for empty, single-file, and multi-file suites
  cannot be made stable after two fixture-harness approaches, keep one harness,
  document the compromise, and escalate rather than accumulating brittle
  coverage.
- If `make check-fmt`, `make lint`, or `make test` still fails after five
  consecutive fix attempts, stop and escalate with the captured logs.
- If the implementation grows beyond roughly 12 changed files or 700 net lines,
  stop and reassess whether part of the work belongs in Step `3.2`.

## Risks

- Risk: `include_str!` path resolution from a generated file in `OUT_DIR` will
  be wrong if it uses a plain relative path such as `"theorems/foo.theorem"`.
  Mitigation: route the bridge macro through
  `concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)` so the included theorem
  file stays crate-root relative.

- Risk: a temporary `macro_rules! theorem_file` bridge could accidentally
  become part of the public API surface or conflict with the future proc-macro
  name. Mitigation: keep it in a hidden internal module and make the generated
  suite the only caller.

- Risk: rewriting `OUT_DIR/theorem_suite.rs` on every build-script run can add
  needless churn and obscure whether the suite actually changed. Mitigation:
  render to memory and only rewrite the file when the bytes differ.

- Risk: behavioural tests that read generated files from Cargo target
  directories may be brittle. Mitigation: prove exact rendered contents with
  direct unit tests and reserve behavioural tests for compile-success workflow
  checks.

- Risk: the crate currently has no proc-macro architecture at all, so it is
  easy for this step to become a hidden partial implementation of Step `3.2`.
  Mitigation: make the bridge expansion intentionally minimal and document that
  it exists only to keep the generated callsite stable.

## Progress

- [x] 2026-04-05: reviewed `docs/roadmap.md`,
  `docs/theoremc-design.md`, `docs/theorem-file-specification.md`,
  `docs/users-guide.md`, `docs/developers-guide.md`, `build.rs`,
  `src/build_discovery.rs`, `src/lib.rs`, and the existing Step `3.1.1`
  ExecPlan.
- [x] 2026-04-05: confirmed the current repository state relevant to this
  change: `build.rs` already discovers and sorts theorem paths, but it does not
  yet write `OUT_DIR/theorem_suite.rs`, and `src/lib.rs` does not yet include
  any generated theorem suite.
- [x] 2026-04-05: confirmed there is still no proc-macro crate or existing
  `theorem_file!` implementation in this checkout, so this step needs a narrow
  compile-time bridge.
- [x] 2026-04-05: drafted this ExecPlan.
- [ ] Milestone 0: add failing tests that describe exact suite rendering and
  empty/single/multi-file compile behaviour.
- [ ] Milestone 1: add a shared generated-suite helper that renders deterministic
  `theorem_file!(...)` lines and writes `OUT_DIR/theorem_suite.rs`.
- [ ] Milestone 2: extend `build.rs` to call the new helper after theorem
  discovery.
- [ ] Milestone 3: add hidden crate-side include wiring with a temporary
  internal `theorem_file!` bridge macro.
- [ ] Milestone 4: add `rstest-bdd` behavioural coverage for empty, single, and
  multi-file suites.
- [ ] Milestone 5: update `docs/theoremc-design.md`, `docs/users-guide.md`,
  and `docs/roadmap.md`.
- [ ] Milestone 6: run `make fmt`, `make markdownlint`, `make nixie`,
  `make check-fmt`, `make lint`, and `make test`, sequentially, with captured
  logs.

## Surprises & Discoveries

- 2026-04-05: Step `3.1.1` already created a clean seam for this work. The
  existing `BuildDiscovery` API returns exactly the sorted theorem path list
  that Step `3.1.2` needs, so no discovery redesign should be necessary.
- 2026-04-05: the prompt references `docs/rstest-bdd-users-guide.md`, but that
  file is not present in this checkout. Existing local BDD
  (behaviour-driven development) suites and `Cargo.toml` are the available
  style references.
- 2026-04-05: there is no current `theorem_file!` implementation anywhere in
  the repository, which makes the callsite-versus-expansion boundary the key
  design decision for this step.

## Decision log

- 2026-04-05: plan to keep the generated suite format stable now and defer only
  the expansion body to Step `3.2`. Rationale: the roadmap explicitly requires
  `theorem_file!(...)` lines in Step `3.1.2`, so the cleanest seam is to make
  the generated file final in shape even if the macro body is only a bridge for
  now.

- 2026-04-05: plan to use a hidden internal `macro_rules! theorem_file` bridge
  that expands to a compile-time `include_str!` check anchored at
  `CARGO_MANIFEST_DIR`. Rationale: this proves that generated paths are valid
  compile inputs without prematurely building the full proc-macro crate.

- 2026-04-05: plan to place suite rendering and write-if-changed logic in a
  dedicated shared helper under `src/` rather than burying it in `build.rs`.
  Rationale: the acceptance criteria require direct unit tests for deterministic
  suite contents, which is awkward if the logic exists only inside the build
  script entrypoint.

- 2026-04-05: plan to keep behavioural tests focused on Cargo workflow success,
  while exact suite text is asserted in unit tests. Rationale: reading Cargo's
  generated `OUT_DIR` paths in fixture builds is brittle, and the exact text
  contract is easier to prove directly.

## Outcomes & Retrospective

This section will be completed during implementation. At minimum it must record
the final generated-suite contract, the exact tests added, the exact
documentation files updated, and the final gate results.

## Context and orientation

The current crate has already finished Phase 3 Step `3.1.1`. The important
current files are:

- `build.rs`
  Already loads `CARGO_MANIFEST_DIR`, delegates to `src/build_discovery.rs`,
  and emits `cargo::rerun-if-changed=` lines. It does not yet read `OUT_DIR`
  or write any generated Rust source.
- `src/build_discovery.rs`
  Already exposes `discover_theorem_inputs(manifest_dir)` and returns sorted,
  forward-slash theorem paths via `BuildDiscovery::theorem_files()`.
- `src/lib.rs`
  Currently exposes library modules only. There is no generated-suite include
  site and no `theorem_file!` macro.
- `tests/build_discovery_bdd.rs`
  Provides a working fixture-crate pattern for Cargo behavioural testing. This
  is the strongest local reference for Step `3.1.2` behavioural tests.
- `docs/theoremc-design.md`
  Design text for build integration lives in §7.1 and §7.2. This file already
  states that Step `3.1.2` should generate `OUT_DIR/theorem_suite.rs` and that
  `theorem_file!` later owns per-file expansion.
- `docs/users-guide.md`
  Already documents discovery and Cargo invalidation from Step `3.1.1`, but it
  does not yet say that the crate always includes a generated theorem suite.
- `docs/roadmap.md`
  The first Step `3.1` checkbox is done. The second checkbox for generated
  suite inclusion remains open and is the target of this plan.

Suggested implementation file layout:

1. `src/build_suite.rs`
   Shared helper for rendering and writing `OUT_DIR/theorem_suite.rs`.
2. `src/build_suite_tests.rs`
   Direct unit tests for exact suite contents and write-if-changed behaviour.
3. `build.rs`
   Extended to call the shared suite helper after discovery.
4. `src/lib.rs`
   Hidden generated-suite include site plus the temporary internal
   `theorem_file!` bridge macro.
5. `tests/build_suite_bdd.rs`
   Behavioural Cargo tests for empty, single, and multi-file suites.
6. `tests/features/build_suite.feature`
   `rstest-bdd` feature file for the behavioural scenarios.

The intended seam is:

```rust
mod __theoremc_generated_suite {
    macro_rules! theorem_file {
        ($path:literal) => {
            const _: &str =
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path));
        };
    }

    include!(concat!(env!("OUT_DIR"), "/theorem_suite.rs"));
}
```

This is intentionally minimal. Step `3.2` should be able to keep the same
generated callsite while replacing the macro body with real proc-macro-based
per-file expansion.

## Plan of work

### Stage A: establish the red tests first

Begin by describing the desired suite contract in tests before touching the
build script.

Add direct unit tests for a new shared suite helper. These tests should assert
the exact generated file contents for:

1. an empty theorem list;
2. a single theorem path; and
3. multiple theorem paths supplied out of lexical order.

The multi-file test should prove that the rendered output follows the sorted
crate-relative path order already established by `BuildDiscovery`.

Also add one direct test for write-if-changed semantics. The helper should
return whether it rewrote the file, or otherwise expose enough signal that the
test can prove identical contents do not trigger a second rewrite. This keeps
the build output stable and prevents unnecessary churn in `OUT_DIR`.

Add failing behavioural tests using the existing fixture-crate pattern. The
three required scenarios are:

1. an empty fixture crate with no `theorems/` directory compiles;
2. a fixture crate with one theorem file compiles; and
3. a fixture crate with several theorem files created in non-sorted order also
   compiles.

These fixture crates should include a copy of the Step `3.1.2` bridge wiring in
their `src/lib.rs`, not just the build script. The behavioural goal is to prove
that the real Cargo build path succeeds once the generated suite is always
included.

### Stage B: add the shared generated-suite helper

Create a small helper module, likely `src/build_suite.rs`, with two focused
responsibilities:

1. render deterministic suite contents from a theorem path iterator; and
2. write `OUT_DIR/theorem_suite.rs` only when the contents have changed.

The rendering contract should be intentionally small and stable. A suitable
shape is:

```rust
pub(crate) fn render_theorem_suite<'a>(
    theorem_files: impl IntoIterator<Item = &'a Utf8Path>,
) -> String
```

with one generated line per theorem file:

```rust
theorem_file!("theorems/example.theorem");
```

The rendered file should end with a trailing newline even for the empty case.
That makes snapshots and diffs stable.

The write helper should accept an `OUT_DIR` path, render the suite in memory,
compare against any existing file, and rewrite only on content changes. Keep
the error surface internal and actionable. Do not mix theorem parsing,
token-generation, or backend semantics into this module.

### Stage C: extend `build.rs`

Once the helper is green, extend `build.rs` so it does three things in order:

1. discover theorem inputs with the existing `discover_theorem_inputs()`
   helper;
2. write `OUT_DIR/theorem_suite.rs` using the new suite helper and the ordered
   theorem list; and
3. emit the existing `cargo::rerun-if-changed=` lines.

Keep `build.rs` thin. It should remain a coordination entrypoint, not the home
for rendering logic.

The generated suite path must come from `OUT_DIR`. If `OUT_DIR` is missing, the
build should fail with a clear panic message, because that is a genuine build
environment error.

### Stage D: wire the generated suite into the crate

Update `src/lib.rs` to include the generated suite on every build through a
hidden internal module. The include site should define the temporary
`macro_rules! theorem_file` bridge before the `include!()` line so the
generated file can invoke the macro directly.

The bridge expansion should do exactly one thing in Step `3.1.2`: force the
theorem path to participate in Rust compilation by expanding to a compile-time
`include_str!` anchored at `CARGO_MANIFEST_DIR`. This proves that:

1. the generated theorem paths are valid crate-relative inputs; and
2. empty, single-file, and multi-file suites compile without any additional
   manual wiring.

Do not generate per-file modules, theorem parsing, or harness code in this
bridge. Those belong to Step `3.2`.

### Stage E: add behavioural `rstest-bdd` coverage

Create `tests/build_suite_bdd.rs` plus a matching
`tests/features/build_suite.feature`. Reuse the fixture-crate approach from
`tests/build_discovery_bdd.rs`, but extend the fixture crate to include:

1. the root `build.rs`;
2. `src/build_discovery.rs`;
3. the new `src/build_suite.rs`; and
4. a fixture `src/lib.rs` that mirrors the hidden bridge wiring.

The scenarios should be phrased in theorem-author workflow terms:

1. `An empty crate still compiles with generated suite wiring`
2. `A single theorem file is included automatically`
3. `Multiple theorem files compile in deterministic suite order`

The behavioural assertions should focus on successful Cargo builds and on the
presence or absence of stable build-log markers. Exact suite text belongs in
unit tests, not in the Cargo fixture logs.

### Stage F: document the shipped contract

Update the requested documentation in the same implementation change:

1. `docs/theoremc-design.md`
   Add a short implementation-decision subsection under the build-integration
   area describing:
   - the generated `OUT_DIR/theorem_suite.rs` format,
   - the temporary internal `theorem_file!` bridge and why it exists,
   - why the bridge uses `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), ...))`,
     and
   - how this preserves a clean handoff to Step `3.2`.
2. `docs/users-guide.md`
   Document the user-visible behaviour:
   - theorem discovery now results in an always-included generated suite,
   - empty theorem trees still compile,
   - discovered theorem files become compile inputs automatically, and
   - no extra crate-side include wiring is required from the user.
3. `docs/roadmap.md`
   Mark the Step `3.1` generated-suite checkbox done only after all tests and
   gates pass.

### Stage G: validate end to end

Run focused tests first, then the full repository gates, sequentially. Because
this change touches code and documentation, the required final validation set
is:

```plaintext
set -o pipefail; cargo test build_suite | tee /tmp/build-suite-targeted.log
set -o pipefail; cargo test --test build_suite_bdd | tee /tmp/build-suite-bdd.log
set -o pipefail; make fmt | tee /tmp/make-fmt.log
set -o pipefail; make markdownlint | tee /tmp/make-markdownlint.log
set -o pipefail; make nixie | tee /tmp/make-nixie.log
set -o pipefail; make check-fmt | tee /tmp/make-check-fmt.log
set -o pipefail; make lint | tee /tmp/make-lint.log
set -o pipefail; make test | tee /tmp/make-test.log
```

Success means:

1. direct suite-rendering tests pass;
2. behavioural Cargo workflow tests pass for empty, single, and multi-file
   suites;
3. `make check-fmt`, `make lint`, and `make test` pass;
4. documentation validation passes; and
5. `docs/roadmap.md` shows the generated-suite checkbox done.

## Concrete steps

Run these from the repository root:

```plaintext
git branch --show
```

Confirm the working branch matches the planned ExecPlan path.

```plaintext
cargo test build_discovery
```

Use this as the current baseline for discovery-only behaviour before changing
suite generation.

```plaintext
cargo test --test build_discovery_bdd
```

Use the existing Cargo fixture harness as the behavioural reference before
adding the new Step `3.1.2` suite scenarios.

During implementation, the expected workflow is:

```plaintext
1. add failing unit tests for suite rendering and writing
2. add failing BDD scenarios for empty/single/multi-file suite builds
3. implement shared suite helper
4. extend build.rs
5. wire src/lib.rs include bridge
6. update design doc, user guide, and roadmap
7. run the full validation sequence
```

## Validation and acceptance

Acceptance is behavioural, not merely structural.

Before the change:

- there is no `OUT_DIR/theorem_suite.rs` generation,
- there is no generated-suite include site in `src/lib.rs`, and
- a fixture crate that always includes `OUT_DIR/theorem_suite.rs` would fail.

After the change:

- an empty theorem tree builds successfully because the generated suite exists
  even when discovery returns no theorem files;
- one theorem file produces one `theorem_file!(...)` line and the crate builds
  without manual include edits;
- several theorem files compile even when created in non-sorted order, because
  the suite rendering follows deterministic lexical path order; and
- the change passes `make check-fmt`, `make lint`, and `make test`, with the
  documentation gates passing as well.

## Idempotence and recovery

The generated suite helper should be idempotent. Re-running the build without
changing theorem inputs should yield identical suite contents and should not
rewrite `OUT_DIR/theorem_suite.rs` unnecessarily.

If an implementation attempt becomes tangled with real proc-macro work, stop,
record the reason in `Decision Log`, and split that work into Step `3.2`
instead of forcing both steps into one change.

If a partial implementation breaks the crate, the recovery path is to revert
the include wiring first, then the build-script generation call, returning the
repository to the already-complete Step `3.1.1` state.

## Artifacts and notes

The most important implementation artefacts should be:

- the exact rendered contents for empty, single-file, and multi-file suites;
- the hidden include bridge in `src/lib.rs`;
- the fixture-crate BDD scenarios proving real Cargo builds; and
- the validator transcripts recorded through `tee`.

An expected multi-file generated suite should look like:

```rust
theorem_file!("theorems/a.theorem");
theorem_file!("theorems/nested/b.theorem");
theorem_file!("theorems/z.theorem");
```

## Interfaces and dependencies

No new external dependency is expected for this step.

The internal interfaces that should exist by the end of implementation are:

```rust
pub(crate) fn render_theorem_suite<'a>(
    theorem_files: impl IntoIterator<Item = &'a camino::Utf8Path>,
) -> String;

pub(crate) fn write_theorem_suite(
    out_dir: &camino::Utf8Path,
    discovery: &crate::build_discovery::BuildDiscovery,
) -> Result<(), BuildSuiteError>;
```

or an equivalent internal shape that keeps suite rendering testable and keeps
`build.rs` thin.

The crate-side bridge should remain internal and hidden. It should not become a
documented public macro or API contract for consumers.

Revision note: initial draft created on 2026-04-05 to cover Roadmap Step
`3.1.2`, building directly on the completed Step `3.1.1` discovery seam.
