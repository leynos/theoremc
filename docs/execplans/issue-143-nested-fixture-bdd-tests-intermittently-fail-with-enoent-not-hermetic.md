# Issue 143: make nested-fixture BDD failures self-identifying and hermetic

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: IN PROGRESS

Branch:
`issue-143-nested-fixture-bdd-tests-intermittently-fail-with-enoent-not-hermetic`

Roadmap item: none. This closes GitHub issue
[#143](https://github.com/leynos/theoremc/issues/143).

## Purpose / big picture

Three integration suites (`build_discovery_bdd`, `build_suite_bdd` and
`theorem_file_macro_bdd`) build temporary fixture crates with a nested `cargo`
invocation. They occasionally fail with a bare
`No such file or directory (os error 2)`, and which tests fail changes from run
to run. The error names neither the operation nor the path, so the failure
cannot be diagnosed from the captured output.

After this change, every fixture, spawn and generated-suite lookup failure
names the operation, the affected path, the fixture's package name, its
temporary root and the shared target. Two confirmed isolation gaps are closed:
suffix allocation becomes an atomic cross-process reservation, and the child
`cargo` has its build layout pinned against inherited configuration.
Deterministic regressions cover fixture lifetime, reservation concurrency and a
real concurrent build with sibling clean-up. The warm shared target from ADR
0012 is preserved; no private per-fixture target is introduced.

Observable success:

1. A fixture filesystem failure reports
   `<operation> <path>: <io error>` and the fixture identity, instead of a bare
   `io::Error` display.
2. A fixture `cargo build` that cannot start reports the command, the working
   directory and the underlying error; a completed failure reports the exit
   status plus separately labelled stdout and stderr.
3. A generated-suite lookup that cannot open `debug/build` names the full path;
   a lookup that finds no suite names the package, the target and every
   attempted candidate path.
4. Two processes racing to allocate a fixture suffix cannot receive the same
   suffix; the reservation is a `create_dir` marker under
   `<shared-target>/fixture-reservations`.
5. The child `cargo` receives `CARGO_BUILD_BUILD_DIR` equal to its
   `CARGO_TARGET_DIR`, and the `CARGO_BUILD_TARGET` environment variable is
   removed, so an exported `build.target` cannot retarget the build. A
   `build.target` set in a Cargo configuration file is not cleared by this and
   remains a documented limitation: Cargo offers no supported way to unset a
   configured target, and output then moves to `<target>/<triple>/debug/build`.
6. The existing four named tests remain selected in CI; no name-based local
   exemption is mirrored into repository configuration.

## Constraints

- Keep the fixture error contract `Result<_, String>`; do not introduce a typed
  fixture error enum.
- Keep `BuildLog` semantics and its `ran`/`contains`/`omits` contract unchanged.
- Keep the ADR 0012 shared warm target
  (`<CARGO_TARGET_DIR>/theoremc-fixture-builds`); do not introduce private
  per-fixture targets.
- Do not change `.config/nextest.toml`: the new real-Cargo test belongs to the
  existing `build_suite_bdd` binary, which already runs in the serialized
  `nested-cargo-builds` group.
- Do not modify the CI workflow selection, the lint gates, or the Makefile.
- Leave `BuildSuiteError` in production code, JSON `OUT_DIR` observation and
  environment clearing out of scope.
- Every module begins with a `//!` comment; every public and private item has
  Rustdoc; assertions carry custom diagnostics.
- Files stay at or below 400 lines. `fixture_crate.rs` is currently 353 lines,
  so new helpers must live in new `tests/common/` modules rather than growing
  it past the limit.
- Documentation uses en-GB-oxendict spelling and is wrapped at 80 columns.
- Historical ADR text is preserved. The current ADR serial is 0012; the issue
  and the CodeRabbit plan call it ADR 011, which the record's renumbering
  addendum explains.

## Tolerances (exception triggers)

- Files: if implementation changes more than 18 non-ExecPlan files, stop and
  escalate. Exclude this ExecPlan file from the count.
- Code: if changes exceed 900 net lines of code across tests and helpers, stop
  and escalate.
- Documentation: if changes exceed 250 net lines excluding ExecPlan files, stop
  and escalate.
- Dependencies: if a new dependency is required, stop and escalate. This plan
  uses only `std`, `tempfile`, `camino`, `cap-std` and `rstest`, all already
  declared.
- Interface: if a public test-helper API change beyond `hermetic_cargo`
  returning `Result<Command, String>` is needed, stop and escalate.
- Iterations: if a gate still fails after three focused fix attempts, stop and
  escalate with the failing log path.
- Ambiguity: if the ENOENT cause becomes determinable and contradicts the
  chosen hardening, record it in `Decision log` and continue only with changes
  that are strictly compatible with both accounts.

## Risks

- Risk: adding context to `FixtureCrate` errors changes substrings that existing
  tests assert. Severity: medium. Likelihood: medium. Mitigation: keep the
  underlying `io::Error` text intact inside each message and append identity;
  run the fixture unit tests and BDD suites after each milestone.
- Risk: the reservation marker directory accumulates entries across runs.
  Severity: low. Likelihood: high. Mitigation: markers are inert and removed by
  `cargo clean`, exactly like stale build directories; document this in the ADR.
- Risk: setting `CARGO_BUILD_BUILD_DIR` equal to `CARGO_TARGET_DIR` could move
  build-script output on a Cargo version whose `build.build-dir` semantics
  differ. Severity: medium. Likelihood: low. Mitigation: the value equals the
  default layout (`<target>/debug/build`), the pinned toolchain exercises it in
  the real-Cargo test, and the change is reverted if the suite fails.
- Risk: the concurrent real-Cargo regression is slow on a cold target.
  Severity: low. Likelihood: medium. Mitigation: reuse `fixture_with_theorem`
  and the warm shared target; the binary already has a 600 s allowance.
- Risk: `cap_std::fs_utf8::Dir` may not be `Send`, blocking the threaded test.
  Severity: low. Likelihood: low. Mitigation: if it is not `Send`, construct
  the fixture inside the worker thread and pass only its suite contents back.

## Progress

- [x] (2026-11-23 17:30Z) Reconnaissance: read issue #143, the CodeRabbit plan,
      ADR 0012, `tests/common/*`, the three BDD suites and the gate recipes.
- [x] (2026-11-23 17:35Z) Branch renamed and Lody session titled.
- [x] (2026-11-23 19:05Z) `EP-M1` Context-preserving fixture diagnostics and
      their deterministic tests. `tests/common/io_context.rs`,
      `command_run.rs` and `fixture_build.rs` added; `fixture_isolation.rs`,
      `fixture_crate.rs`, `suite_layout.rs` and the macro runner now name the
      operation and path. The 41 `test-helpers` unit tests pass.
- [x] (2026-11-23 19:05Z) `EP-M2` Atomic suffix reservations
      (`Reservations::claim`), vanishing-entry tolerance and the pinned child
      build layout (`CARGO_BUILD_BUILD_DIR`, `CARGO_BUILD_TARGET` removal,
      relative-target rejection). Unit tests pass.
- [x] (2026-11-24 03:10Z) `EP-M3` Lifetime, reservation-concurrency and
      real-Cargo regressions. `build_suite_bdd` passed 5/5 in 4480 s under host
      contention, including the new concurrent sibling test.
- [x] (2026-11-24 06:20Z) `EP-M4` ADR 0012 addendum and developers' guide
      written. Gate evidence, all under shared-admission contention:
      `make fmt`, `make check-fmt`, `make markdownlint`, `make nixie`,
      `cargo clippy --workspace`, `cargo doc --workspace`,
      `make typecheck` and `make lint-whitaker` all pass; doctests pass.
      The combined `make test` run reported one TIMEOUT at nextest's fixed
      600 s allowance in the unchanged
      `build_discovery_bdd::existing_theorem_files_are_discovered_recursively`;
      running each fixture suite directly with `cargo test` passed it:
      `build_suite_bdd` 5/5, `build_discovery_bdd` 3/3,
      `theorem_file_macro_bdd` 13/13 (after the two Kani tests passed
      individually), `lint_workspace_execution` 4/4, macros trybuild 1/1 and
      `arg_lowering` compile-fail 6/6.
- [x] (2026-11-25) `EP-M5` CodeRabbit review cleared over six rounds, each
      finding fixed and re-verified (barrier synchronization, `Drop` reaping,
      pre-compiled barrier, file-redirected logs with a regression test, named
      unreadable logs, platform-correct path assertions). Final gate evidence:
      `make check-fmt`, `make markdownlint`, `make nixie`, `cargo clippy
      --workspace`, `cargo doc --workspace`, `make lint-whitaker` and
      `make typecheck` all pass; `test-helpers` unit tests pass 43/43;
      `build_suite_bdd` passes 5/5 and `build_discovery_bdd` 3/3 with real
      Cargo.
- [ ] Push the branch and open the draft pull request.

## Surprises & discoveries

- Observation: the failure cited in issue #143 points at
  `tests/build_suite_bdd.rs:126` on commit `8cda1fe`, which is the
  `#[scenario(...)]` attribute for
  `multiple_theorem_files_compile_in_deterministic_suite_order`. Evidence:
  `git show 8cda1fe:tests/build_suite_bdd.rs`. Impact: the ENOENT is raised
  inside the generated scenario body, i.e. by a `then` step, and the macro
  reports it at the attribute's line. It does not identify the failing helper;
  hence diagnostics come first.
- Observation: `/data/tmp` does not exist on this host and `TMPDIR` is unset,
  so the reported path came from the environment that produced the evidence (a
  custom `TMPDIR`, as the issue states). Evidence: `echo ${TMPDIR:-<unset>}` and
  `ls -ld /data/tmp`. Impact: the reproduction guidance must set `TMPDIR`
  explicitly.
- Observation: `hermetic_cargo` does not currently touch `RUSTFLAGS`; the plan
  phrase "inherited `RUSTFLAGS` unchanged" means "leave that behaviour alone".
  Evidence: `tests/common/hermetic_cargo.rs`. Impact: no `RUSTFLAGS` change is
  made.
- Observation: Cargo documents `build.build-dir` as defaulting to
  `build.target-dir` and as overridable through `CARGO_BUILD_BUILD_DIR`; build
  script output lives at `<build-dir>/debug/build/`. Pinning the variable to
  the shared target therefore reproduces the default layout exactly and defeats
  a configured `build.build-dir`. Evidence:
  <https://doc.rust-lang.org/cargo/reference/config.html> and
  <https://doc.rust-lang.org/cargo/reference/build-cache.html>. Impact: the
  second isolation gap is closed without inventing a new layout.
- Observation: the host's shared compile admission pool was continuously full
  during implementation, with hundreds of queued compiler groups from other
  agents. A full `build_suite_bdd` run exceeded nextest's fixed 600 s per-test
  allowance for the pre-existing scenarios as well as the new one, so the
  timeout measured host contention rather than this change. Evidence:
  `build-limits-status` showed capacity 4, held 4, healthy, with waits over 150
  s per crate; the unchanged `a_single_theorem_file_is_included_automatically`
  also reported `SLOW` past 240 s. Impact: the new real-Cargo test pre-warms
  the shared target once before the measured concurrent pair, keeping its
  intrinsic cost close to one cold build plus two warm builds; final gate
  evidence must be collected when the queue allows.

## Decision log

- D1. Decision: keep `Result<_, String>` and route fallible steps through one
  context helper. Rationale: it removes the diagnostic gap without changing the
  public contract or the substring assertions that fixture tests rely on. A
  typed error enum was rejected as disproportionate. Date/Author: 2026-11-23,
  implementing agent.
- D2. Decision: qualify allocation by an atomic `create_dir` reservation rather
  than a cross-process file lock. Rationale: one process can hold two live
  fixtures, so a lifetime lock would deadlock. A reservation is lock-free,
  portable and keeps the warm target. Date/Author: 2026-11-23, implementing
  agent.
- D3. Decision: pin `CARGO_BUILD_BUILD_DIR` to the shared target and remove
  `CARGO_BUILD_TARGET`, rather than clearing the child environment. Rationale:
  environment values override configuration files, the value equals Cargo's
  default layout, and clearing the environment would lose toolchain and
  `RUSTFLAGS` parity. Date/Author: 2026-11-23, implementing agent.
- D4. Decision: `hermetic_cargo` returns `Result<Command, String>` and rejects a
  relative target instead of silently resolving it. Rationale: the child runs
  in the fixture directory, so a relative target would mean something different
  in the child; rejecting it keeps every caller's intent explicit. Callers are
  test-only and change together. Date/Author: 2026-11-23, implementing agent.
- D5. Decision: put the generated-suite lookup and the three-way failure split
  in a new `tests/common/fixture_build.rs`, and leave `suite_layout.rs` owning
  only the two Cargo layouts. Rationale: `fixture_crate.rs` was 353 lines and
  must stay under 400; the lookup needs the fixture identity, the shared target
  and the command, which `fixture_crate` would otherwise duplicate. The
  absent-`debug/build` and no-suite tests live in `fixture_build_tests.rs`
  because they exercise that boundary, while `suite_layout` keeps its layout
  tests. Date/Author: 2026-11-23, implementing agent.
- D6. Decision: reserve suffixes with a directory marker under an opened
  `cap_std` handle rather than a file `create_new`. Rationale: `create_dir`
  gives the same atomic `AlreadyExists` signal, needs no extra dependency, and
  matches the build-directory shape the marker stands for. Date/Author:
  2026-11-23, implementing agent.
- D7. Decision (review-driven): document that a configured `build.target` is a
  limitation rather than claiming that removing `CARGO_BUILD_TARGET` prevents
  it. Rationale: removing the environment variable cannot override a
  `build.target` in an ancestor or `CARGO_HOME` `config.toml`; Cargo rejects
  `--config 'build.target=[]'` with a type error and
  `--config 'build.target=""'` with "target was empty", so no supported
  command-line override exists. The ADR, developers' guide, exec plan and
  `hermetic_cargo` docs now say so. Date/Author: 2026-11-24, implementing
  agent, from CodeRabbit review.
- D8. Decision (review-driven): prove the sibling-drop overlap by spawning the
  builder's child and requiring `is_running()` before the drop, instead of
  relying on two threads intersecting by chance. Rationale: a child that has
  already exited cannot overlap the clean-up, and a signal sent before
  `cargo_build` proves nothing. `spawn_cargo_build` plus a live-child check
  ties the synchronization point to the active build; `wait` drains both pipes
  so a verbose build cannot block. Date/Author: 2026-11-24, implementing agent,
  from CodeRabbit review.
- D9. Decision (review-driven): make the fixture-path tests Unix-only and derive
  the `hermetic_cargo` doctest paths from a real temporary directory. Rationale:
  `/fixture/target` is not absolute on Windows, so the guard rejected it and
  the fixture panicked; the repository's other path-shape suites are already
  Unix-only, and a temporary directory is platform-correct in the doctest. The
  `race_allocators` barrier moved before any fallible setup so one worker's
  failure cannot strand the others. Date/Author: 2026-11-24, implementing
  agent, from CodeRabbit review.

- D10. Decision (review-driven): replace `try_wait()` overlap detection with a
  child-controlled release barrier. Rationale: `try_wait()` reports an
  observation, not a guarantee that the child is still alive, so a correct
  build could exit between the check and the drop.
  `FixtureCrate::with_release_barrier` installs a build script that writes
  `barrier-ready` and then blocks until `barrier-release` exists, with an
  internal 300 s deadline. The test waits for readiness, drops the sibling,
  then releases the build, so the overlap is guaranteed rather than hoped for.
  The test passed end to end with real Cargo in 625 s. Date/Author: 2026-11-24,
  implementing agent, from CodeRabbit review.
- D11. Decision (review-driven): replace the Unix-only error-string assertion
  with one derived from the operating system's own text. Rationale: "No such
  file or directory" is Unix wording; Windows reports "The system cannot find
  the file specified". The test now formats `io::Error::from_raw_os_error(2)`
  and asserts the message contains its `Display`, which holds on both
  platforms. Only the `sh`-based test stays Unix-gated, and that case genuinely
  needs a POSIX shell. Date/Author: 2026-11-24, implementing agent, from
  CodeRabbit review.
- D12. Decision (review-driven): give `RunningBuild` a `Drop` that kills and
  reaps an unwaited child, and pre-compile the barrier script before measuring
  readiness. Rationale: an early return left the Cargo child holding the shared
  build-directory lock, blocking later fixtures; the child now lives in an
  `Option` that `wait` takes, so a completed build leaves nothing to clean up.
  Compilation of the barrier script otherwise consumed the readiness allowance,
  so `precompile_barrier_script` runs the barrier once with its release already
  present, and the measured build is forced to rerun by touching a
  `rerun-if-changed` input. The test passed end to end in 823 s. Date/Author:
  2026-11-24, implementing agent, from CodeRabbit review.
- D13. Decision (review-driven): redirect a fixture build's streams to files
  in the shared target and report them through `RunningBuild::abort`.
  Rationale: piped streams that nobody drains can fill and block Cargo, and a
  build that blocks never reaches a readiness barrier; files also survive a
  failed readiness check, so the exit status and both streams are reported
  instead of a bare timeout. The logs live in the shared target because that
  directory always exists, so creating them cannot mask the spawn failure the
  missing-working-directory test exists to observe. The barrier test passed in
  947 s with this shape. Date/Author: 2026-11-24, implementing agent, from
  CodeRabbit review.
- D14. Decision (review-driven): read the redirected logs on the normal
  completion path and fail if they cannot be read. Rationale:
  `Child::wait_with_output` captures only piped streams, so file redirection
  made every successful `cargo_build_log` return an empty log, which silently
  broke `BuildLog::ran` and the discovery assertions. `wait` now waits for the
  status and reads both files, propagating a read failure rather than reporting
  silence. `spawn_redirected` is the seam a unit test uses to prove a stub
  child's both streams reach the returned log; `build_discovery_bdd` then
  passed 3/3 with real Cargo in 1363 s. Date/Author: 2026-11-24, implementing
  agent, from CodeRabbit review.
- D15. Decision: run `make spelling` as part of the gate set, and keep the
  en-GB-oxendict `-ize`/`-ization` forms throughout. Rationale: the first CI
  run failed the Spelling stage on `synchronisation`, `synchronises` and
  `recognise`, which the project's Oxford spelling rejects in favour of
  `synchronization`, `synchronizes` and `recognize`. The gate had not been part
  of the local run, so this plan now records it explicitly. Date/Author:
  2026-11-25, implementing agent.

## Outcomes & retrospective

Implementation is complete and every gate passes when its suites are run
without nextest's fixed 600 s per-test allowance, which the saturated host
cannot meet for nested Cargo builds. The two Kani scenarios were proved not to
be caused by this change: `kani_lists_the_generated_proof_harness` passed both
with and without the `CARGO_BUILD_BUILD_DIR` pin when run alone, and its
earlier failure captured `Blocking waiting for file lock on package cache`
alongside a concurrent build of another worktree's `<shared-target>/kani`, so
the failure tracked cross-process contention on a shared directory rather than
the pin.

Lessons: the host's compile queue is the dominant cost for nested-build suites
and must be allowed for in every timeout; and a shared target that includes
Kani's own `kani` build directory inherits a cross-worktree lock, which is
worth recording even though it is out of scope here.

## Context and orientation

`tests/common/mod.rs` is compiled twice: as the private `integration` module of
`crates/test-helpers/src/lib.rs`, and (through that crate) as the
`test_helpers` dependency of the integration binaries under `tests/`. Its
`#[cfg(test)]` submodule files therefore run as unit tests of the
`test-helpers` library, not inside the integration binaries.

`tests/common/fixture_isolation.rs` owns suffix allocation (`unique_suffix`,
`first_unused_suffix`, `suffix_is_taken`) and the shared target
(`shared_target_dir`). `tests/common/fixture_crate.rs` owns `FixtureCrate`: a
`tempfile::TempDir` source root, scaffold files, mtime helpers,
`cargo_build_log` and `generated_suite_contents`.
`tests/common/suite_layout.rs` maps Cargo's two build-directory layouts.
`tests/common/hermetic_cargo.rs` builds the child command.
`tests/theorem_file_macro_bdd/cargo_runner.rs` is the separate spawn boundary
for the macro suite.

The current failure shape is a bare `io::Error` display produced by
`.map_err(|error| error.to_string())` at many sites. The cited failing tests
are in `tests/build_suite_bdd.rs`, `tests/build_discovery_bdd.rs` and
`tests/theorem_file_macro_bdd.rs`; all are selected by the
`nested-cargo-builds` override in `.config/nextest.toml`.

Budget survey: `fixture_crate.rs` is 353 lines and must stay under 400, so new
context and command-running helpers go in new `tests/common/` modules.
`suite_layout.rs` is 147 lines and can absorb the generated-suite lookup.
`docs/developers-guide.md` is 1088 lines; a bounded new subsection is cheap. No
comparable historical ExecPlan increased its budget repeatedly.

## Conformance basis

- `docs/adrs/adr-0012-shared-warm-target-for-fixture-crates.md` (accepted
  2026-11-06, addenda 2026-11-06 and 2026-11-12): shared warm target, unique
  package names, workspace lock, offline builds, serialized nested builds.
- `docs/developers-guide.md` sections on fixture helpers and gate policy.
- `.config/nextest.toml`: `nested-cargo-builds` group and its override.
- GitHub issue #143 and its CodeRabbit coding plan.

Traced requirements:

```plaintext
REQ-DIAGNOSTICS -> EP-M1 -> tests/common suite and command tests
REQ-RESERVATION -> EP-M2 -> Barrier allocator test
REQ-BUILD-LAYOUT -> EP-M2 -> hermetic_cargo tests and real-Cargo test
REQ-LIFETIME -> EP-M3 -> two-fixture drop test
REQ-CI-SELECTION -> EP-M4 -> unchanged nextest filter, documented rule
```

## Verification plan

Invariants:

1. Every fixture-scoped error contains the operation and the path.
   Verified by named unit tests for each failure class: absent `debug/build`,
   build root without a suite, spawn with a missing working directory,
   non-existent program, and a nonzero-exit stub. Non-vacuity: each test builds
   the failing precondition and asserts on the operation and path text, and the
   tests fail against the pre-change code because it emits a bare error.
2. A suffix is unique across concurrent allocators.
   Verified by a `std::sync::Barrier` test with a collision-forcing generator
   and real reservation directories; the test fails if allocation reverts to a
   non-atomic check because two threads would receive the same suffix.
3. A pre-existing reservation marker is skipped.
   Verified by pre-creating `fixture-reservations/fx_<suffix>` and asserting
   the allocator moves on.
4. Dropping one fixture removes only its own root.
   Verified by creating two fixtures, dropping one, and probing both roots, the
   sibling file and the shared target.
5. A child Cargo command always sees `CARGO_TARGET_DIR == CARGO_BUILD_BUILD_DIR`
   and no `CARGO_BUILD_TARGET`, `CARGO_BUILD_TARGET_DIR`. Verified by
   inspecting the prepared `Command` in `hermetic_cargo_tests.rs`, and by a
   real concurrent build that still finds its own suite. The guarantee covers
   environment variables only; a configured `build.target` is a stated
   limitation, not a verified prevention.
6. The four named tests remain in the shipped selection.
   Verified by reading `.config/nextest.toml` in the final diff; it is
   unchanged.

External axioms: `tempfile` creates a unique directory atomically; Cargo writes
build-script output under `<target>/debug/build` when `build-dir` equals the
target and no build target is configured; `cap-std` `create_dir` is atomic with
respect to a concurrent creator. These are documented third-party interfaces
and are exercised at the boundary by the real-Cargo test.

The ENOENT root cause remains unverified by design; the diagnostics are the
instrument that will identify it on the next occurrence. No formal proof
obligation arises: the change adds no algorithmic invariant beyond the
reservation uniqueness exercised above.

## Milestones

`EP-M1` adds `tests/common/io_context.rs` and `tests/common/command_run.rs`,
threads context through `fixture_isolation.rs`, `fixture_crate.rs`,
`suite_layout.rs` and the macro runner, and adds deterministic tests.
Acceptance: `cargo nextest run -p test-helpers --lib` and
`cargo nextest run -p theoremc --test build_suite_bdd` pass; the new tests fail
on the pre-change error text.

`EP-M2` adds reservation allocation, vanishing-entry tolerance and the
`hermetic_cargo` contract. Acceptance: the `test-helpers` unit tests and
`hermetic_cargo_tests` pass; `generated_suite_contents` still finds suites
after a real build.

`EP-M3` adds the two-fixture lifetime test, the Barrier allocator test and the
real concurrent-build test. Acceptance: `build_suite_bdd` passes serially and
the new tests are in the existing group.

`EP-M4` updates ADR 0012 and the developers' guide and runs the full gates.

`EP-M5` runs `coderabbit review --agent`, clears findings, pushes and opens the
draft PR.

## Revision note

- 2026-11-23: initial draft derived from issue #143, the CodeRabbit coding plan
  and repository reconnaissance.
