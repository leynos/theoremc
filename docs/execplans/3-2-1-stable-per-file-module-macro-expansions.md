# Step 3.2.1: stable per-file `theorem_file!` macro expansions

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Complete the first Roadmap checkbox under Phase 3, Step 3.2 by replacing the
temporary hidden `macro_rules! theorem_file` bridge with a real proc-macro
expansion that owns per-file theorem compilation structure.

After this change, the generated `OUT_DIR/theorem_suite.rs` file will stay
exactly as it is today, with one `theorem_file!("path/to/file.theorem");`
invocation per discovered theorem file. What changes is the meaning of that
callsite. Instead of expanding only to `include_str!`, it will expand into a
stable private per-file Rust module named from the literal theorem path, a
compile-time theorem source include, a backend submodule scaffold, and one
generated harness stub per theorem document in the file. This satisfies the
Phase 3 requirement that theorem files are always connected to normal Rust
compilation, while intentionally stopping short of final Kani semantics and
attributes, which remain Step 3.2.2.

Observable success:

1. The current generated suite contract remains stable: `build.rs` still emits
   bare `theorem_file!(...)` lines in deterministic order, and the crate still
   includes `OUT_DIR/theorem_suite.rs` automatically.
2. Each `theorem_file!("P")` invocation expands to a private module named with
   `mangle_module_path(P)`, and that module contains a compile-time
   `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", P))`.
3. For every theorem document `T` loaded from file `P`, the expansion emits
   one stable harness stub whose identifier matches
   `mangle_theorem_harness(P, T)`.
4. Macro-expansion snapshot tests prove that repeated expansion of the same
   theorem input produces byte-for-byte stable token output.
5. Behavioural tests using `rstest-bdd` v0.5.0 cover happy and unhappy paths:
   a valid single-document theorem file compiles, a valid multi-document file
   compiles with all expected symbols, and an invalid theorem file fails
   compilation during macro expansion with actionable diagnostics.
6. `docs/theoremc-design.md` records the implementation decisions taken for
   this step, `docs/users-guide.md` explains the new compile-time behaviour
   visible to library consumers, and `docs/roadmap.md` marks only the first
   Step 3.2 checkbox done after the implementation and all gates pass.
7. `make check-fmt`, `make lint`, and `make test` pass. Because
   documentation changes are in scope, `make fmt`, `make markdownlint`, and
   `make nixie` must also pass before the implementation is complete.

## Constraints

- Scope is limited to Roadmap Step `3.2.1`. Do not implement the second Step
  `3.2` checkbox in this change. In particular, do not add final
  `#[cfg(kani)]`, `#[kani::proof]`, or `#[kani::unwind(...)]` semantics here.
  This step owns stable structure and stub generation only.
- The generated callsite shape from Step `3.1.2` must remain unchanged:
  `theorem_file!("crate/relative/path.theorem");`. Step `3.2.1` changes only
  the imported macro implementation behind that callsite.
- The emitted per-file module name must follow
  `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}` exactly, per
  `NMR-1` and `ADR-2`.
- The emitted harness stub identifier for theorem `T` in file `P` must follow
  `theorem__{theorem_slug(T)}__h{hash12(P#T)}` exactly, per `NMR-1` and `ADR-2`.
- Multi-document `.theorem` files must preserve theorem document order when
  generating harness stubs. If a file contains documents `A`, then `B`, the
  generated stub order must remain `A`, then `B`.
- The expansion must include the theorem source via
  `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", P))`, not a plain
  relative `include_str!(P)`. The macro callsites live in a file generated
  under `OUT_DIR`, so plain relative includes are not acceptable.
- Invalid theorem files must fail at compile time through the proc-macro
  expansion path, using the same schema-loading contract already established by
  the current library code. Do not introduce a second, weaker theorem parser in
  the macro path.
- The current public library surface must remain source-compatible for existing
  consumers of `theoremc::schema`, `theoremc::mangle`, and
  `theoremc::collision`.
- The repository currently remains a single root package with build-script
  wiring and a temporary hidden suite bridge. Because a real proc macro cannot
  live in the same library target that consumes it, this step must introduce a
  bounded crate split without changing the user-facing crate name `theoremc`.
- Keep Rust source files under 400 lines. Existing files already close to the
  limit include `src/mangle.rs` and `src/mangle_harness.rs`, so new macro logic
  and tests must be split into focused sibling files rather than appended into
  existing modules.
- Behavioural tests must use `rstest-bdd` v0.5.0 where they model theorem
  author workflow.
- Documentation and comments must use en-GB-oxendict spelling and grammar.

## Tolerances (exception triggers)

- Architecture: if breaking the proc-macro dependency cycle requires moving
  substantially more than the schema, mangle, collision, and hidden lowering
  support into a shared crate, stop and escalate before proceeding.
- Workspace churn: if the minimal viable proc-macro split grows beyond roughly
  18 changed files or 1,400 net lines before documentation updates, stop and
  reassess whether part of the work should be split into a dedicated
  architectural groundwork change.
- Public API drift: if preserving `theoremc::schema`, `theoremc::mangle`, or
  `theoremc::collision` paths requires a breaking API change, stop and document
  the options before proceeding.
- Macro path resolution: if the proc macro cannot reliably locate the
  consuming crate's manifest directory using a stable mechanism after one
  focused prototype and one fallback attempt, stop and escalate.
- Test harness complexity: if behavioural fixture workspaces cannot be kept
  stable after two approaches, keep one focused BDD suite plus direct snapshot
  tests and document the compromise instead of accumulating brittle harnesses.
- Validation instability: if any of `make check-fmt`, `make lint`, or
  `make test` fails more than five consecutive fix attempts, stop and escalate
  with the captured logs.

## Risks

- Risk: the real proc macro introduces a dependency cycle because it needs the
  current parser and mangling logic, but the root crate also needs to import
  the macro. Mitigation: begin with an explicit architecture milestone that
  introduces a minimal shared crate boundary and keeps the root `theoremc`
  package as the public facade.

- Risk: compile-time file resolution inside the proc macro may accidentally use
  the macro crate's working directory or the generated `OUT_DIR` location
  rather than the consuming crate root. Mitigation: prototype path resolution
  before finalizing the crate split, and keep the emitted `include_str!`
  anchored to `env!("CARGO_MANIFEST_DIR")`.

- Risk: generating stub functions without the final Kani gating can trigger
  dead-code or unused-item lints in normal builds. Mitigation: make the stub
  shape intentionally lint-neutral, for example by emitting an internal const
  reference set that marks the generated function items as used.

- Risk: snapshot tests may become unreadable if they compare raw
  `TokenStream::to_string()` output from large expansions. Mitigation: keep the
  expansion helper small, snapshot only representative cases, and prefer
  normalized generated strings over shelling out to unstable `cargo expand`
  tooling.

- Risk: the existing behavioural fixture tests for Step `3.1.2` only validate
  the temporary bridge. They will not prove the real symbol layout. Mitigation:
  extend the fixture crates so their own unit tests refer to the expected
  generated private symbol paths from inside the same crate.

- Risk: the older Step `3.1.2` ExecPlan and parts of the developer guide
  describe the temporary bridge as the live state. Mitigation: update the
  design and user guide during this step, and update developer-facing
  documentation as needed so the architectural story remains current.

## Progress

- [x] 2026-04-10: reviewed `docs/roadmap.md`, `docs/theoremc-design.md`,
  `docs/theorem-file-specification.md`, `docs/name-mangling-rules.md`,
  `docs/users-guide.md`, `docs/developers-guide.md`, and the shipped Step
  `3.1.1` and Step `3.1.2` ExecPlans.
- [x] 2026-04-10: confirmed the current repository state relevant to this
  change: `build.rs` already writes `OUT_DIR/theorem_suite.rs`, `src/lib.rs`
  still contains a temporary hidden `macro_rules! theorem_file` bridge, and
  there is still no proc-macro crate in the workspace.
- [x] 2026-04-10: confirmed the repository already exposes stable path and
  harness naming helpers through
  `theoremc::mangle::{mangle_module_path, mangle_theorem_harness}` and already
  uses checked-in string snapshots for diagnostic stability tests.
- [x] 2026-04-10: confirmed the prompt references
  `docs/rstest-bdd-users-guide.md`, but that file is not present in this
  checkout. Existing local `rstest-bdd` suites are the style reference.
- [x] 2026-04-10: drafted this ExecPlan.
- [x] Milestone 0: prove the minimal architecture seam that breaks the
  proc-macro dependency cycle without changing the public crate name.
- [x] Milestone 1: add failing unit and snapshot tests for stable per-file
  expansion and failing behavioural tests for macro-driven compile success and
  compile failure.
- [x] Milestone 2: introduce the proc-macro crate and the shared support crate
  boundary needed by the macro.
- [x] Milestone 3: replace the temporary hidden bridge with the imported proc
  macro while preserving the generated suite callsite.
- [x] Milestone 4: update `docs/theoremc-design.md`,
  `docs/users-guide.md`, `docs/developers-guide.md`, and `docs/roadmap.md`.
- [x] 2026-04-21: Milestone 5 complete. Ran `make fmt`,
  `make markdownlint`, `make nixie`, `make check-fmt`, `make lint`, and
  `make test`, with logs captured during validation.

## Surprises & Discoveries

- 2026-04-10: the repository is already beyond the assumptions in the older
  Step `3.1.2` planning text. The generated suite and include wiring already
  exist; the remaining work is specifically to replace the hidden bridge with a
  real proc macro.
- 2026-04-10: the design document has always described a workspace containing a
  separate `theoremc-macros` proc-macro crate, but the live repository still
  has only a single root `theoremc` package. Step `3.2.1` is therefore both a
  feature step and the first concrete workspace-boundary step toward the
  documented architecture.
- 2026-04-10: the build-suite fixture tests currently compile because they do
  not need a real proc macro at all. Once Step `3.2.1` lands, those fixtures
  will need to validate actual generated symbols, not just successful
  `include_str!` expansion.
- 2026-04-10: `docs/rstest-bdd-users-guide.md` is still absent. Existing local
  `tests/*_bdd.rs` files remain the practical reference for BDD style and
  `rstest-bdd` usage in this repository.

## Decision Log

- 2026-04-10: this plan treats the first Step `3.2` checkbox as a structural
  macro-expansion change only, not as the place to add final Kani proof
  attributes. Rationale: the roadmap splits stable per-file module generation
  from Kani-specific harness gating, and keeping those steps separate preserves
  atomic delivery.

- 2026-04-10: the plan assumes a minimal shared support crate is required to
  break the proc-macro cycle cleanly. Rationale: a real proc macro cannot live
  in the same library target that invokes it, and duplicating theorem parsing
  and naming logic inside the macro crate would create a second source of truth.

- 2026-04-10: behavioural fixture tests should refer to generated private
  symbols from inside the fixture crate's own unit tests. Rationale: this
  proves the actual private module and harness paths exist without making the
  generated items public just for testing.

- 2026-04-10: snapshot testing should target a deterministic internal expansion
  helper rather than unstable shell tooling. Rationale: the acceptance
  criterion is stable macro expansion, not a specific external expansion
  command, and existing repository practice already favors checked-in string
  snapshots.

## Outcomes & Retrospective

- Implemented the minimal workspace split described in the plan:
  `crates/theoremc-core` now owns shared theorem semantics,
  `crates/theoremc-macros` owns the real `theorem_file!` proc macro, and the
  root `theoremc` crate remains the public facade and build-integration owner.
- Replaced the temporary local bridge macro with the real proc macro while
  preserving the generated `theorem_file!(...)` callsite contract from Step
  `3.1.2`.
- Added direct proc-macro unit coverage in `crates/theoremc-macros` plus
  fixture-crate behavioural coverage in `tests/theorem_file_macro_bdd.rs`.
- One risk materialized during implementation: moving shared schema code into
  `theoremc-core` broke a test-fixture `include_str!` path that assumed the old
  root layout. The fix was to update the relative fixture path inside the moved
  module.
- Follow-up work remains for Step `3.2.2`: add final `#[cfg(kani)]`,
  `#[kani::proof]`, and `#[kani::unwind(...)]` semantics to the generated
  harness stubs.

## Context and orientation

The current implementation seam is split across four places:

1. `build.rs`

   This already discovers theorem files and writes `OUT_DIR/theorem_suite.rs`
   with one `theorem_file!(...)` line per discovered theorem path.

2. `src/build_suite.rs`

   This renders deterministic generated suite contents. The Step `3.2.1`
   implementation must preserve this callsite contract so `build.rs` stays
   unchanged apart from any imports needed by the new workspace layout.

3. `src/lib.rs`

   This currently defines the hidden `__theoremc_generated_suite` module and a
   temporary local `macro_rules! theorem_file` bridge that expands only to a
   compile-time `include_str!`.

4. `src/mangle*.rs` and `src/schema/`

   These modules already contain the stable naming and theorem loading rules
   the proc macro must reuse rather than reimplement independently.

The design documents already define the target generated shape:

- `docs/name-mangling-rules.md` defines the exact per-file module and harness
  naming algorithms.
- `docs/theoremc-design.md` §7.2 defines the proc-macro responsibilities:
  stable per-file module, compile-time `include_str!`, backend submodule, and
  one generated harness per theorem document.
- `docs/roadmap.md` splits this work into two checkboxes:
  Step `3.2.1` for stable structure and Step `3.2.2` for Kani-specific
  attributes and unwind metadata.

The major architectural constraint is the proc-macro cycle. The live root crate
is both the library that owns theorem parsing and the crate that includes
`OUT_DIR/theorem_suite.rs`. A real proc macro cannot be implemented inside that
same target. The minimal viable architecture for this step is therefore:

1. keep the public package name `theoremc`,
2. introduce a normal shared crate (preferred name: `crates/theoremc-core`)
   holding the parser and naming logic the proc macro must call,
3. introduce `crates/theoremc-macros` as a proc-macro crate depending on that
   shared crate, and
4. make the root `theoremc` crate a facade that re-exports the existing public
   modules and imports the proc macro for its hidden generated-suite module.

If implementation proves that a smaller split works without duplicating logic,
document that in the `Decision Log` and keep the public facade contract
unchanged.

## Plan of work

### Stage A: prove the architecture seam first

Start by proving the two hardest unknowns before changing user-facing build
behaviour.

First, add a tiny failing prototype or test-only spike that answers the path
resolution question: can the proc-macro implementation reliably read
`theorems/foo.theorem` from the consuming crate root using a stable input path
and a stable manifest-directory lookup? Do not guess. Record the mechanism that
works.

Second, decide the minimal shared crate split needed to break the proc-macro
cycle. The preferred outcome is:

- `crates/theoremc-core` for shared schema, mangling, collision, and hidden
  lowering support,
- `crates/theoremc-macros` for the proc macro itself, and
- the existing root `theoremc` package preserved as the public facade and build
  integration owner.

Before moving code, list exactly which modules need to move and which can stay
in the facade. The implementation should preserve public paths through
re-exports, not by asking users to adopt new crate names.

### Stage B: write the failing tests first

Add the tests that describe the target contract before implementing the macro.

For direct unit coverage, create an internal pure expansion helper in the new
macro crate that takes an explicit theorem path and manifest directory, parses
the file, and returns the generated token stream as a value suitable for
testing. Keep the actual `#[proc_macro]` entrypoint thin by delegating to that
helper.

Use that helper for snapshot-style unit tests covering at least:

1. a single-document theorem file with a simple theorem identifier;
2. a multi-document theorem file proving document order is preserved;
3. a nested or punctuation-heavy path proving the per-file module name follows
   the existing `mangle_module_path` rules;
4. a CamelCase theorem identifier proving harness names follow
   `mangle_theorem_harness`; and
5. the same input expanded twice, producing identical normalized output.

Follow existing repository snapshot practice by checking in expected strings
and comparing against trimmed actual output. Prefer a normalized generated
string over unstable external tooling.

For behavioural coverage, add a new `rstest-bdd` suite, likely
`tests/theorem_file_macro_bdd.rs`, plus a matching `.feature` file. Reuse the
temporary fixture-crate pattern from `tests/build_discovery_bdd.rs` and
`tests/build_suite_bdd.rs`, but update the fixture workspace so it includes the
real proc-macro architecture. The required scenarios are:

1. `A valid theorem file produces the expected generated symbol paths`
2. `A multi-document theorem file generates one harness stub per document`
3. `An invalid theorem file fails compilation during macro expansion`

The happy-path fixture crates should contain crate-local unit tests that refer
to the expected private symbol paths, for example by assigning the expected
generated harness items to `let _: fn() = ...;`. This proves the generated
module and harness names actually exist without exposing them publicly.

The unhappy-path fixture should compile-fail on an invalid theorem file and the
BDD assertion should check for the theorem path plus a stable diagnostic
fragment rather than the entire compiler output.

### Stage C: introduce the shared crate and proc-macro crate

Once the tests are red, introduce the bounded workspace architecture needed by
the macro.

Update the root `Cargo.toml` so the repository becomes a workspace with the
root package still named `theoremc` and new members under `crates/`. Move the
shared theorem logic needed by both the facade and the proc macro into the new
normal library crate. Preserve the current public library surface from the root
crate using `pub use` re-exports for the existing top-level modules.

The shared crate should own the real implementations for:

- `schema`,
- `mangle`,
- `collision`, and
- hidden argument lowering support if the macro-generated stubs need it now or
  will need it in Step `3.2.2`.

Keep build-discovery and build-suite generation in the root package, because
they are part of the root package's build integration contract rather than
generic theorem processing logic.

### Stage D: implement the proc-macro expansion

Create the real `#[proc_macro] theorem_file` entrypoint in
`crates/theoremc-macros`.

The entrypoint should:

1. parse one string literal path argument,
2. resolve the theorem file under the consuming crate root,
3. load theorem documents using the shared schema loader with a source ID based
   on the literal theorem path,
4. compute the stable per-file module name from that path,
5. compute one stable harness stub name per theorem document, and
6. emit the deterministic expansion.

For this step, the emitted expansion should be structurally complete but
backend-semantic-light. A suitable target shape is:

```rust
mod __theoremc__file__... {
    const _: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "theorems/example.theorem"));

    mod kani {
        pub(super) fn theorem__first__h...() {}
        pub(super) fn theorem__second__h...() {}
    }

    const _: [fn(); 2] = [kani::theorem__first__h..., kani::theorem__second__h...];
}
```

This shape gives Step `3.2.1` the required stable module layout and one entry
per theorem document, while avoiding premature Kani attributes. If a different
lint-neutral stub shape proves better, document the choice explicitly in the
`Decision Log` and in `docs/theoremc-design.md`.

The proc macro should report invalid theorem content as a compile error tied to
the macro call. Keep the error text focused and deterministic.

### Stage E: replace the temporary bridge

Update the root crate's hidden generated-suite module so it no longer defines a
local `macro_rules! theorem_file`.

Instead, import the real proc macro into that hidden module and keep the
existing:

```plaintext
include!(concat!(env!("OUT_DIR"), "/theorem_suite.rs"));
```

callsite intact.

This is the seam that keeps Step `3.1.2` stable while swapping in real per-file
expansion behind the same generated lines. Remove stale comments that describe
the local bridge as active behaviour.

### Stage F: document the shipped contract

Update the requested docs in the same change.

In `docs/theoremc-design.md`, add an implementation-decision subsection under
the Step `3.2` / proc-macro material covering:

- the bounded crate split used to support the proc macro,
- the exact per-file expansion shape shipped in Step `3.2.1`,
- why `include_str!` remains anchored to `CARGO_MANIFEST_DIR`, and
- why final Kani attributes remain deferred to Step `3.2.2`.

In `docs/users-guide.md`, describe the consumer-visible behaviour:

- theorem files now expand into hidden per-file modules at compile time,
- theorem syntax and validation failures now surface during normal Rust
  compilation through the generated suite path, and
- harness stubs exist structurally now, but Kani proof attributes and unwind
  semantics are still pending the next roadmap step.

Also update `docs/developers-guide.md` so it no longer describes the hidden
`macro_rules! theorem_file` bridge as the live architecture.

Finally, update `docs/roadmap.md` only after all tests and gates pass, marking
the first Step `3.2` checkbox done while leaving the second checkbox open.

### Stage G: validate end to end

Run focused tests first, then the full repository gates. Capture every long
command through `tee` with `set -o pipefail`.

The expected validation sequence is:

```plaintext
set -o pipefail; cargo test -p theoremc-macros theorem_file | tee /tmp/theorem-file-macro-unit.log
set -o pipefail; cargo test --test theorem_file_macro_bdd | tee /tmp/theorem-file-macro-bdd.log
set -o pipefail; make fmt | tee /tmp/make-fmt.log
set -o pipefail; make markdownlint | tee /tmp/make-markdownlint.log
set -o pipefail; make nixie | tee /tmp/make-nixie.log
set -o pipefail; make check-fmt | tee /tmp/make-check-fmt.log
set -o pipefail; make lint | tee /tmp/make-lint.log
set -o pipefail; make test | tee /tmp/make-test.log
```

Success means:

1. the direct expansion snapshot tests pass,
2. the behavioural macro BDD suite passes for happy and unhappy paths,
3. the temporary bridge is gone and the real proc macro is imported instead,
4. `make check-fmt`, `make lint`, and `make test` pass,
5. documentation validation passes, and
6. the roadmap shows only the first Step `3.2` checkbox as done.

## Concrete steps

Run these from the repository root when implementation begins:

```plaintext
git branch --show
git status --short
```

Confirm the branch is correct and the tree is clean before starting the crate
split.

```plaintext
cargo test --test build_suite_bdd
```

Use this as the behavioural baseline for the existing temporary bridge before
replacing it.

```plaintext
rg -n "macro_rules! theorem_file|__theoremc_generated_suite" src docs
```

Use this to locate every place that still describes the temporary bridge as the
active architecture.

During implementation, the intended order is:

```plaintext
1. prove manifest-dir path resolution and crate-split seam
2. add failing snapshot tests and failing macro BDD scenarios
3. introduce shared crate and proc-macro crate
4. implement stable per-file expansion and stub generation
5. remove the temporary bridge and import the proc macro
6. update design, developer, and user documentation
7. run the full validation sequence
8. mark the first Step 3.2 roadmap checkbox done
```

## Validation and acceptance

This plan satisfies the roadmap item only when all of the following are true:

- the generated suite still contains `theorem_file!(...)` callsites only,
- each callsite expands to the stable private per-file module name dictated by
  `NMR-1`,
- each theorem document in a file produces one stable harness stub identifier
  dictated by `ADR-2`,
- the expansion includes theorem source content with `include_str!`,
- expansion snapshots remain stable across repeated builds,
- happy and unhappy macro workflows are covered by unit and BDD tests,
- the temporary bridge is removed,
- the requested documentation is updated, and
- all required Makefile gates pass.

This historical ExecPlan records completed implementation work and no longer
requires approval before execution.
