# Step 3.1.1: `build.rs` scanning of theorem files

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

## Purpose / big picture

Implement the first half of Roadmap Phase 3, Step 3.1 so theorem files become
real Cargo build inputs instead of an out-of-band convention.

After this change, a crate using theoremc will gain a `build.rs` script that
recursively scans `theorems/**/*.theorem` under the crate root and emits
`cargo::rerun-if-changed=` lines for the relevant directories and theorem
files. Editing an existing theorem file must therefore cause Cargo to rerun the
build script on the next build, without requiring `cargo clean` or manual
touching of Rust source files. This work is intentionally limited to discovery
and change tracking. Generating `OUT_DIR/theorem_suite.rs` and wiring
`include!()` remain Step 3.1.2.

Observable success:

1. A thin root-level `build.rs` exists and successfully scans
   `theorems/**/*.theorem` from `CARGO_MANIFEST_DIR`.
2. Direct unit tests cover missing directories, nested theorem trees, ignored
   non-`.theorem` files, deterministic ordering, and emitted rerun path sets.
3. Behavioural tests using `rstest-bdd` v0.5.0 prove build behaviour in an
   isolated temporary fixture crate:
   - a first build runs the build script,
   - a second build without theorem changes does not rerun it, and
   - editing a discovered `.theorem` file causes the next build to rerun the
     build script.
4. `docs/theoremc-design.md` records the implementation decisions,
   `docs/users-guide.md` explains the new build-time behaviour that consumers
   must know, and `docs/roadmap.md` marks only the Step 3.1.1 checkbox done
   once the implementation and gates pass.
5. `make check-fmt`, `make lint`, and `make test` pass. Because documentation
   changes are part of the scope, `make fmt`, `make markdownlint`, and
   `make nixie` must also pass before the change is considered complete.

This plan covers the roadmap checkbox for implementing `build.rs` scanning of
`theorems/**/*.theorem` and emitting `cargo::rerun-if-changed` lines for
directory and file paths, along with the design requirement `DES-7`. It does
not implement theorem execution, proc-macro expansion, or generated suite
inclusion.

## Constraints

- Scope is limited to Roadmap Step `3.1.1`. Do not generate
  `OUT_DIR/theorem_suite.rs`, call `theorem_file!(...)`, or add `include!()`
  wiring in this change.
- The implementation must keep theorem discovery reusable for Step `3.1.2`.
  Discovery logic should therefore return a deterministic data structure that a
  later suite-generation step can consume without rescanning the filesystem.
- The repository currently has no root-level `build.rs` and no committed
  `theorems/` directory. The new build integration must therefore not break
  builds for crates that have zero theorem files today.
- Scan only under the crate-root `theorems/` tree rooted at
  `CARGO_MANIFEST_DIR`. Do not search outside that subtree.
- Discovery must recurse into nested directories and include only files ending
  in `.theorem`.
- Emitted `cargo::rerun-if-changed=` lines must cover directory paths and file
  paths robustly enough that theorem edits trigger rebuilds reliably. At
  minimum, emit the root `theorems` directory path and every discovered theorem
  file path. If nested directory watch coverage is needed for robustness, emit
  those directory paths as well.
- Discovery output order must be deterministic. Sort crate-relative theorem
  paths lexicographically before returning or emitting them so future generated
  suite files are stable under filesystem traversal differences.
- Preserve crate-relative theorem path strings suitable for later mangling and
  suite generation. Normalise discovered path strings to forward-slash form
  (`theorems/nested/example.theorem`) rather than platform-specific separator
  forms.
- Keep `build.rs` thin. Put traversal, path normalisation, filtering, and
  rerun-path derivation in a small helper module so the logic is directly
  testable without spawning Cargo for every case.
- Follow repository filesystem guidance: prefer `cap_std` and `camino` over
  `std::fs` and `std::path` in new filesystem-heavy code.
- Keep files under 400 lines. If the helper and its tests grow too large, use a
  sibling `*_tests.rs` file or a focused test helper module.
- Behavioural tests must use `rstest-bdd` v0.5.0 where they exercise the
  theorem-author build workflow.
- Documentation and comments must use en-GB-oxendict spelling and grammar.

## Tolerances

- Missing-directory semantics: if Cargo does not reliably rerun the build
  script when `cargo::rerun-if-changed=theorems` points to an absent directory
  on the supported toolchain, stop after the prototype milestone and choose one
  of these explicit follow-ups before proceeding:
  1. commit an empty `theorems/.gitkeep`-style placeholder so the watched
     directory always exists, or
  2. document the directory-existence requirement and confirm that product
     behaviour is acceptable.
- Scope creep: if satisfying the acceptance criteria requires any proc-macro,
  harness-generation, or `include!()` work, stop and split that into the
  already-planned Step `3.1.2` change.
- API churn: if a clean testable design seems to require exposing new public
  library APIs rather than keeping discovery internal to build support, stop
  and document the options before continuing.
- Test harness complexity: if the build-behaviour tests cannot be made stable
  after two approaches, fall back to one isolated integration strategy and
  document the compromise rather than accumulating multiple partial harnesses.
- Validation instability: if `make check-fmt`, `make lint`, or `make test`
  fails more than five consecutive fix attempts, stop and escalate with the
  captured logs.
- File-count growth: if the implementation grows beyond roughly 10 changed
  files or 600 net lines, stop and reassess whether part of the work belongs in
  Step `3.1.2`.

## Risks

- Risk: Cargo directory-watch semantics for an absent `theorems/` directory may
  differ from the behaviour assumed by the design note. Mitigation: start with
  a prototype milestone that proves the behaviour on the current toolchain
  before finalising the missing-directory contract.

- Risk: raw filesystem traversal order is not stable across platforms or
  filesystems. Mitigation: collect crate-relative theorem paths into a vector,
  normalise them, and sort before deriving rerun paths or future suite inputs.

- Risk: testing `build.rs` behaviour inside the live repository can recurse
  into Cargo building the package under test. Mitigation: use temporary fixture
  crates created under `tempfile` directories and invoke `cargo build -vv` in
  those isolated copies.

- Risk: Windows path separators can drift from the forward-slash theorem path
  strings already used by the mangling rules. Mitigation: keep discovery and
  future suite inputs in normalised forward-slash crate-relative form even when
  scanning host-native paths.

- Risk: a monolithic build-discovery function can quickly become a "bumpy road"
  mixture of traversal, filtering, formatting, and emission. Mitigation:
  separate the work into small helper functions such as
  `discover_theorem_files`, `collect_watched_directories`,
  `normalise_relative_path`, and `emit_rerun_if_changed_lines`.

## Progress

- [x] 2026-03-26: reviewed `docs/roadmap.md`, `docs/theoremc-design.md`,
  `docs/theorem-file-specification.md`, the existing Step 2 ExecPlans, current
  `Cargo.toml`, current `src/lib.rs`, current test layout, and the Makefile
  gate targets.
- [x] 2026-03-26: confirmed the current repository state relevant to this step:
  there is no root-level `build.rs`, no `theorem_file!` macro yet, and no
  committed `theorems/` directory at the crate root.
- [x] 2026-03-26: confirmed `rstest-bdd = "0.5.0"` is already present in
  `Cargo.toml` and existing BDD files under `tests/` provide the local style
  reference.
- [x] 2026-03-26: drafted this ExecPlan.
- [ ] Milestone 0: prototype missing-directory and edit-trigger behaviour in an
  isolated temporary Cargo fixture so the build contract is proven before the
  final implementation is locked in.
- [ ] Milestone 1: add a shared discovery helper module and direct unit tests
  for recursion, filtering, sorting, and rerun-path derivation.
- [ ] Milestone 2: add root-level `build.rs` that delegates to the shared
  helper and emits `cargo::rerun-if-changed=` lines.
- [ ] Milestone 3: add behavioural `rstest-bdd` coverage proving first build,
  no-op rebuild, and edit-triggered rebuild behaviour.
- [ ] Milestone 4: update `docs/theoremc-design.md`, `docs/users-guide.md`,
  and `docs/roadmap.md`.
- [ ] Milestone 5: run `make fmt`, `make markdownlint`, `make nixie`,
  `make check-fmt`, `make lint`, and `make test`, each through `tee` with
  `set -o pipefail`.

## Surprises & Discoveries

- 2026-03-26: the current crate is still pre-Phase-3 in a literal sense: there
  is no `build.rs`, no proc-macro crate, and no theorem-suite include wiring
  yet. This makes it especially important to keep Step `3.1.1` bounded and not
  smuggle in Step `3.1.2`.
- 2026-03-26: the repository root does not currently contain a `theorems/`
  directory. Missing-directory behaviour is therefore not an edge case; it is
  the immediate default state that the first build after adding `build.rs` must
  handle.
- 2026-03-26: the prompt references `docs/rstest-bdd-users-guide.md`, but that
  file is not present in this checkout. Existing in-repo BDD suites and
  `Cargo.toml` are the available local style reference.
- 2026-03-26: existing naming helpers in `src/mangle_path.rs` and
  `src/mangle_harness.rs` already assume stable crate-relative theorem path
  strings, which makes forward-slash normalisation in discovery important even
  before Step `3.1.2`.

## Decision Log

- 2026-03-26: this plan intentionally covers only the first Step 3.1 checkbox
  and leaves suite generation for a separate change. Rationale: the user asked
  for `3-1-1-build-rs-scanning-of-theorems`, and splitting the work preserves
  atomic roadmap execution.

- 2026-03-26: plan the implementation around a shared internal helper module
  compiled by both `build.rs` and tests, rather than burying all logic directly
  in the build script. Rationale: the acceptance criteria require unit tests
  and reliable behaviour tests, which is impractical if the discovery logic
  only exists behind Cargo's build-script entrypoint.

- 2026-03-26: use isolated temporary Cargo fixture crates for behavioural
  coverage. Rationale: running `cargo build` against the live repository from
  inside its own integration tests is needlessly fragile and can couple test
  outcomes to workspace state.

- 2026-03-26: plan to sort and normalise theorem paths now, even though Step
  `3.1.1` only emits rerun lines. Rationale: Step `3.1.2` will need stable
  theorem path ordering to generate deterministic suite files, and doing the
  normalisation in discovery avoids reinterpreting path identity later.

- 2026-03-26: keep the missing-directory contract as an explicit prototype
  decision rather than assuming Cargo behaviour. Rationale: the design note
  cites directory watching, but the current repository state makes this detail
  user-visible on day one.

## Outcomes & Retrospective

This section is intentionally incomplete because the plan has not yet been
executed. Update it during implementation with:

- the final build-discovery contract that shipped,
- any deviations from the proposed milestones,
- the exact test suites added,
- the documentation sections updated, and
- the final gate results.

## Context and orientation

The current theoremc crate has completed Phase 2 naming and argument-lowering
work, but nothing in the package yet connects `.theorem` files to Cargo build
inputs. The implementation for this step should orient around these locations:

- `Cargo.toml`
  Defines current dependencies, dev-dependencies, test entries, and lint
  policy. This is where any new `[build-dependencies]` and any new BDD test
  target entry will be added.
- `src/lib.rs`
  Current crate root. If the shared build-discovery helper lives under `src/`,
  this is where a hidden internal module may be declared for unit testing.
- `build.rs`
  Does not yet exist. This step introduces it as a thin wrapper around the
  shared discovery helper.
- `tests/module_naming_bdd.rs` and `tests/harness_naming_bdd.rs`
  Current local examples of `rstest-bdd` v0.5.0 structure and feature-file
  wiring.
- `tests/common/mod.rs`
  Existing place for shared integration-test helpers if the new behavioural
  tests need cargo-fixture utilities.
- `docs/theoremc-design.md`
  Normative design text for build integration lives in §7.1.
- `docs/users-guide.md`
  User-facing build-time expectations belong here once this feature ships.
- `docs/roadmap.md`
  Currently has both Step 3.1 checkboxes unchecked. Only the first checkbox
  should be marked done by this work.

Suggested implementation file layout:

1. `build.rs`
   Thin build-script entrypoint that loads `CARGO_MANIFEST_DIR`, delegates to
   the helper, and prints `cargo::rerun-if-changed=` lines.
2. `src/build_discovery.rs` or `build_support.rs`
   Shared discovery logic. Pick one location and keep it free of application
   runtime concerns so both `build.rs` and tests can compile it.
3. `src/build_discovery_tests.rs` or a focused sibling test file
   Direct unit coverage for discovery semantics if the helper lives under
   `src/`.
4. `tests/build_discovery_bdd.rs`
   Behavioural build-integration scenarios using temporary fixture crates.
5. `tests/features/build_discovery.feature`
   Feature text for the BDD scenarios.

The strongest candidate design is:

```text
build.rs
  -> discover theorem tree under CARGO_MANIFEST_DIR/theorems
  -> return BuildDiscovery { theorem_files, watched_directories }
  -> print root directory + watched directories + theorem files via
     cargo::rerun-if-changed=
```

with the helper producing crate-relative forward-slash path strings in sorted
order.

## Plan of work

### Stage A: preflight and red tests first

Start by proving the real Cargo behaviour in a temporary fixture crate rather
than guessing. Build a small test helper that can:

1. create a temporary cargo package with:
   - a minimal `Cargo.toml`,
   - a stub `src/lib.rs`,
   - copies of the repository's current `build.rs` and shared build-discovery
     helper under test, and
   - an optional `theorems/` tree populated per scenario;
2. run `cargo build -vv --color never` inside that fixture; and
3. capture stdout and stderr for assertions.

Before implementing the production code, add failing tests that express the
required behaviour:

1. a direct unit test for recursive discovery and sorted relative paths;
2. a behavioural test that runs `cargo build` twice with no theorem changes and
   expects the second build not to rerun the build script; and
3. a behavioural test that edits a discovered theorem file and expects the next
   `cargo build` to rerun the build script.

If the missing-directory case is intended to be supported, include a failing
prototype test for:

1. building successfully when `theorems/` is absent; then
2. creating `theorems/first.theorem`; then
3. observing whether the next `cargo build` reruns the build script.

Use that prototype to settle the contract recorded in `Decision Log` and
`docs/theoremc-design.md`.

### Stage B: implement the shared discovery helper

Create a small helper module that owns filesystem traversal and path shaping.
Its responsibilities should be:

1. resolve the crate-root theorem directory from `CARGO_MANIFEST_DIR`;
2. recurse into nested directories when the root exists;
3. include only `.theorem` files;
4. compute the set of watched directories needed for reliable change
   detection;
5. normalise discovered theorem file paths into crate-relative forward-slash
   strings; and
6. sort the final vectors deterministically.

Keep the helper API additive and future-proof. A shape like the following is
appropriate:

```rust
struct BuildDiscovery {
    theorem_files: Vec<Utf8PathBuf>,
    watched_directories: Vec<Utf8PathBuf>,
}

fn discover_theorem_inputs(manifest_dir: &Utf8Path)
    -> Result<BuildDiscovery, BuildDiscoveryError>;
```

The error type does not need to be public API, but it must produce actionable
messages when traversal fails.

Write direct tests against this helper covering:

1. no `theorems/` directory;
2. an empty `theorems/` directory;
3. nested directories containing `.theorem` files;
4. ignored sibling files such as `.txt`, `.yaml`, or editor temp files;
5. deterministic ordering regardless of creation order; and
6. forward-slash normalisation of returned theorem paths.

### Stage C: add the thin `build.rs` entrypoint

Once the helper is green, add the actual root-level `build.rs`. Keep it small:

1. read `CARGO_MANIFEST_DIR`;
2. call the helper;
3. print `cargo::rerun-if-changed=theorems` unconditionally;
4. print additional `cargo::rerun-if-changed=` lines for discovered watched
   directories and theorem files; and
5. fail the build with a clear panic message only for genuine traversal errors,
   not for the ordinary "directory absent" case.

Do not generate `OUT_DIR/theorem_suite.rs` in this step. If it helps future
work, allow the helper to expose sorted theorem files now, but leave the
generation callsite unused until Step `3.1.2`.

### Stage D: add behavioural `rstest-bdd` coverage

Add a new feature file and BDD test module that exercises build behaviour from
the user perspective. Prefer three scenarios:

1. `Existing theorem files are discovered recursively`
   - Given a crate with nested theorem files,
   - when I build twice and then edit one theorem file,
   - then the second unchanged build stays fresh and the edited build reruns
     the build script.
2. `Non-theorem files do not participate in discovery`
   - Given sibling files under `theorems/` that do not end in `.theorem`,
   - when I build and then edit only the ignored file,
   - then Cargo does not rerun the build script because the watched theorem
     inputs are unchanged.
3. `Missing theorem directory is handled explicitly`
   - If the prototype proves absent-directory watching works, the scenario
     should assert that the first later theorem addition reruns the build.
   - If the prototype disproves it, replace this with a scenario asserting the
     documented directory-seeding requirement.

Use log assertions that are robust but not overfitted. The goal is to prove
that Cargo reran the build script, not to snapshot every line of verbose cargo
output. Match on a stable substring such as `build-script-build` together with
the fixture package name.

An expected successful behavioural transcript should look like:

```plaintext
first build:   contains "Running" and "build-script-build"
second build:  does not contain a new build-script invocation
edited build:  contains "Running" and "build-script-build" again
```

### Stage E: document the shipped contract

Update the design and user-facing documentation in the same change:

1. `docs/theoremc-design.md`
   Add a short implementation-decision subsection under §7.1 describing:
   - how missing `theorems/` is handled,
   - why discovery normalises and sorts paths, and
   - why directory rerun lines include nested directories if that is the chosen
     robustness strategy.
2. `docs/users-guide.md`
   Document the new build-time behaviour that library consumers should know:
   - theorem files are auto-discovered from `theorems/**/*.theorem`,
   - edits trigger Cargo rebuilds through the build script, and
   - any requirement about creating or seeding the `theorems/` directory.
3. `docs/roadmap.md`
   Mark only the first checkbox under Step 3.1 as done after all gates pass.

### Stage F: validate end to end

Run the focused tests first, then the full repository gates. Because command
output is truncated in this environment, capture every run through `tee` with
`set -o pipefail`.

Targeted test commands:

```sh
set -o pipefail; cargo test --all-features build_discovery \
  | tee /tmp/build-discovery-targeted.log
```

Full validation commands:

```sh
set -o pipefail; make fmt | tee /tmp/make-fmt.log
set -o pipefail; make markdownlint | tee /tmp/make-markdownlint.log
set -o pipefail; make nixie | tee /tmp/make-nixie.log
set -o pipefail; make check-fmt | tee /tmp/make-check-fmt.log
set -o pipefail; make lint | tee /tmp/make-lint.log
set -o pipefail; make test | tee /tmp/make-test.log
```

Success criteria:

1. the new unit and behavioural tests pass;
2. `make check-fmt`, `make lint`, and `make test` pass as required by the
   roadmap task;
3. `make fmt`, `make markdownlint`, and `make nixie` pass because documentation
   changed; and
4. `docs/roadmap.md` shows the Step `3.1.1` checkbox marked done while the
   Step `3.1.2` checkbox remains open.
