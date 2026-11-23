# Serialise and warm the fixture `cargo` builds behind the BDD suites

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
`Outcomes & Retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: IN PROGRESS

## Purpose / big picture

Three BDD suites (`theorem_file_macro_bdd`, `build_suite_bdd` and
`build_discovery_bdd`) build temporary fixture crates with a real `cargo build`.
Under `cargo nextest run` each test runs in its own process, so the in-process
`static Mutex<()>` in `tests/theorem_file_macro_bdd/cargo_runner.rs` serialised
nothing: all nine `theorem_file_macro_bdd` scenarios built at once, each into
its own cold target directory, and the host's Cargo package-cache lock then
queued 62 other processes behind them. The suite had to be killed by hand after
78 minutes (issue #197).

After this change:

1. At most one fixture `cargo` process runs at a time under
   `cargo nextest run`, enforced by a cross-process advisory file lock that each
   fixture command holds for its whole lifetime.
2. A second run on a warm workspace does not recompile fixture dependencies:
   fixture sources live under the shared fixture target, so Cargo's own
   freshness check is what skips them.
3. An interrupted run leaves no fixture source directories in `$TMPDIR`.
4. Every fixture `cargo` and `cargo kani` command is bounded by a deadline that
   fails the test, so a stalled fixture names itself instead of holding the host.
5. The intermittent ENOENT failures of issue #143 are re-evaluated with recorded
   evidence once serialisation is real.

Observable success, as commands a reviewer can run:

```sh
cargo nextest run -E 'binary(theorem_file_macro_bdd)'
```

completes, and while it runs `pgrep -fc 'cargo build'` never exceeds 1.

## Constraints

- No production code changes. Every edit is in `tests/`, `crates/test-helpers/`,
  `.config/nextest.toml`, `Cargo.toml` (dev-dependencies only) or `docs/`.
- `crates/test-helpers` is a workspace-internal path dependency, not a published
  crate, so its API may change freely; no compatibility shim is introduced.
- Lint policy is absolute: `clippy.toml` caps function arguments at four,
  cognitive complexity at nine, function length at 70 lines; the workspace
  denies `unwrap_used`, `expect_used`, `indexing_slicing`, `missing_docs` and
  `missing_docs_in_private_items`. `no_std_fs_operations` (Whitaker) forbids
  `std::fs`; filesystem access goes through `cap_std::fs_utf8::Dir`.
- `clippy.toml` forbids `std::env::var`/`var_os` outside a declared boundary.
  The existing helpers read `CARGO_TARGET_DIR` under
  `#[expect(clippy::disallowed_methods, reason = ...)]`; the new deadline
  override does the same at exactly one site.
- No new third-party dependency except `rustix` (Unix only, `process` and `fs`
  features) in `crates/test-helpers`, and `toml` as a root dev-dependency. Both
  versions are taken from `Cargo.lock` (rustix 1.1.5, toml 1.1.8), and every
  requirement uses a caret range.
- Files stay under 400 lines; tests move to sibling `*_tests.rs` modules via
  `#[path = ...]` when a module grows past the limit.
- Comments and documentation use en-GB-oxendict spelling. Markdown prose wraps
  at 80 columns; code blocks at 120.
- `tests/common/mod.rs` is compiled into `test-helpers` through
  `#[path = "../../../tests/common/mod.rs"]`, so anything added there must build
  for the library as well as the integration binaries.
- Non-Unix hosts must still compile and pass. Process-group kill, `rustix`, and
  the `sh`-based stub tests are `#[cfg(unix)]`; the non-Unix path kills and reaps
  the direct child only, and the limitation is documented.

## Tolerances (exception triggers)

- Scope: if the change exceeds 20 non-ExecPlan files or 1,800 net lines of code
  plus 400 lines of documentation, stop and escalate. (ExecPlan files are
  excluded from every size budget, as the planning guidance requires.)
- Dependencies: if a crate other than `rustix` or `toml` is needed, stop and
  escalate.
- API: if a change would require modifying `theoremc-core`, `theoremc-macros` or
  `theoremc-build-support` source, stop and escalate.
- Measurements: if the after-change fixture suite is not materially faster on a
  warm workspace, or the shared target does not eliminate the second compile,
  stop and record the finding rather than papering over it.
- Issue #143: if repeated runs still fail intermittently, do not close #143;
  record the evidence and leave it open.
- Iterations: if a gate fails three times for the same reason after fixes, stop
  and escalate with the log.

## Risks

- Risk: `File::try_lock` needs Rust 1.89; the workspace MSRV is 1.89, so it is
  available, but the nightly in `rust-toolchain.toml` is what actually runs the
  tests. Severity: low. Likelihood: low. Mitigation: `make msrv` runs
  `cargo +1.89 check`, and the lock helper is only compiled into test code, so it
  must also pass that check.
- Risk: a file lock held across `fork`/`exec` inheritance could deadlock when a
  child `cargo` inherits the descriptor. Severity: medium. Likelihood: medium.
  Mitigation: open the lock file in the parent, hold it there, and do not pass it
  to the child; `Command` only inherits stdin/stdout/stderr, and the lock file is
  not one of them. A test asserts the lock is free after a fixture command
  returns.
- Risk: sharing one target across fixture crates with unique package names can
  still collide if two fixtures pick the same suffix. Severity: low. Likelihood:
  low. Mitigation: existing `first_unused_suffix` already prevents it, and the
  existing `fixtures_sharing_a_target_read_only_their_own_generated_suite`
  regression stays.
- Risk: moving fixture sources under the target directory puts them in a path
  `cargo clean` removes while a run is in flight. Severity: low. Likelihood: low.
  Mitigation: that is the intent — a `cargo clean` during a test run already
  breaks the build. Tests do not run `cargo clean` concurrently.
- Risk: the deadline is too short on a contended host, turning a slow build into
  a false failure. Severity: medium. Likelihood: medium. Mitigation: the default
  is 600 s, matching the nextest group's own allowance, with a documented
  override through `THEOREMC_FIXTURE_COMMAND_TIMEOUT_SECS`; the contract test
  asserts the group allowance exceeds the deadline.
- Risk: adding `rustix` with the `process` feature pulls `linux-raw-sys/prctl`
  into test-only builds. Severity: low. Likelihood: low. Mitigation: it is
  Unix-gated and dev-only in effect; the dependency policy gate
  (`cargo deny`) is unaffected because the crate is already in the graph.
- Risk: a build queued behind the shared compile-slot pool makes gates look
  slow. Severity: low. Likelihood: high. Mitigation: expect
  `[build-limits] CRATE: waiting for a compile slot` and allow for it; never
  start parallel builds.

## Progress

- [x] (2026-11-23T17:05Z) Reconnaissance: read `cargo_runner.rs`,
      `fixture_crate.rs` (both copies), `fixture_isolation.rs`,
      `hermetic_cargo.rs`, `build_log.rs`, `suite_layout.rs`,
      `.config/nextest.toml`, ADR-0012, ADR-0013, the developers' guide fixture
      section, the Makefile gates, and issue #143. Confirmed the in-process
      mutex is the defect: nextest gives each test its own process.
- [x] (2026-11-23T17:05Z) Branch renamed to
      `issue-197-theorem-file-macro-bdd-runs-nine-cold-fixture-builds-in-parallel-under-nextest-and-blocks-the-host`;
      Lody session title set.
- [ ] Milestone 1: cross-process lock plus bounded process runner.
- [ ] Milestone 2: fixture sources under the shared target, checkout-relative
      build-support path, reuse assertion.
- [ ] Milestone 3: nextest configuration contract, ADR, documentation,
      measurements, #143 re-evaluation, draft PR.

## Surprises & discoveries

- The codegraph MCP server failed to connect (`CONNECT_TIMEOUT`) at session
  start, so navigation used the repository's own tools. Re-attempting is not
  necessary for this work; it is recorded because the skill was requested.
- `cargo nextest show-config test-groups` recursively rebuilds the workspace
  metadata and took longer than two minutes on this contended host before any
  configuration existed to read. It is therefore documented for manual use and
  deliberately kept out of the test suite.
- The shared fixture target (`target/theoremc-fixture-builds`) did not exist in
  this worktree, because no fixture test had run here yet. The first fixture run
  is genuinely cold, which is the honest before-measurement.
- `Cargo.lock` holds three `toml` versions (0.5.11, 0.8.23, 1.1.8) and one
  `rustix` (1.1.5). The plan pins the newest `toml` and the only `rustix`.

## Decision log

- Decision: serialise with a cross-process advisory `flock` on
  `<shared-target>/fixture-cargo.lock`, polled with a bounded wait, rather than
  relying on the nextest test group alone. Rationale: the test group is correct
  but is configuration that a human can silently break, and it does not protect
  a direct `cargo test --test theorem_file_macro_bdd` run, which `make kani`
  uses. The lock is the mechanism; the group is defence in depth, and a contract
  test holds the group.
- Decision: keep `cargo build` rather than downgrade to `cargo check`. Rationale:
  `generated_suite_contents` reads `theorem_suite.rs` from `debug/build`, and the
  build script only runs on a build. `cargo check` runs build scripts too, so the
  suite file would appear, but the scenarios also assert that the *compile*
  succeeds or fails with a specific diagnostic, which `check` does provide — yet
  the Kani scenarios need real artefacts. Keeping one command keeps the suite
  honest, and Step 2's warm shared target removes the cost that motivated the
  question.
- Decision: copy the workspace `Cargo.lock` and build offline (already true) but
  do not add `--locked`. Rationale: the fixture's own `Cargo.toml` differs from
  the workspace's, so a copied lock is a seed, not a contract; `--locked` would
  fail on the first fixture that adds a package. `--offline` already prevents
  index access, which is what the package-cache lock contention came from.
- Decision: put fixture sources under `<shared-target>/sources` instead of
  `$TMPDIR`. Rationale: `cargo clean` already removes the target, so the
  lifetime matches, and an interrupted run cannot leave anything in `$TMPDIR`.
  Each fixture still gets a `TempDir` (now inside that directory) so the `Drop`
  path is unchanged.
- Decision: give each generated fixture manifest an empty `[workspace]` table.
  Rationale: without it, a fixture created under a path that is inside this
  repository's tree is treated as a workspace member candidate, and Cargo walks
  up to the repository manifest. An empty `[workspace]` makes the fixture its own
  workspace root regardless of where it lives.
- Decision: reference `crates/theoremc-build-support` by path from the checkout
  rather than copying its sources into each fixture. Rationale: the copy made
  every fixture's build-support package a different unit for the shared target,
  so nothing was reused; a stable checkout path makes it one unit. This changes
  `theorem_file_macro_bdd` only; `FixtureCrate` (the shared helper) keeps its
  copy so `build_suite_bdd` and `build_discovery_bdd` stay unchanged in fidelity.

## Outcomes & retrospective

Pending. To be completed after Milestone 3.

## Conformance basis

- Upstream requirement: issue #197 (acceptance criteria quoted below).
- Governing standards: `AGENTS.md` (code style, testing, dependency and error
  policy), `docs/developers-guide.md` §3.3 (fixture crates),
  `docs/documentation-style-guide.md` (ADR format, Markdown wrapping).
- Amended record: ADR-0012 "Shared warm target for fixture crates".
- Trace chain:

```plaintext
issue-197/ac-1 one fixture cargo at a time  -> EP-M1 -> fixture_lock tests
issue-197/ac-2 warm second run              -> EP-M2 -> build_suite_bdd reuse test
issue-197/ac-3 no $TMPDIR leaks             -> EP-M2 -> interruption test
issue-197/ac-4 #143 re-evaluated            -> EP-M3 -> recorded evidence
```

## Verification plan

Invariants introduced, and how each is discharged:

1. Mutual exclusion. At most one fixture `cargo` child holds the lock.
   Discharged by `fixture_lock` unit tests (a second acquire while held times out
   and reports the path and elapsed time; a dropped guard frees it) and by the
   `pgrep` observation recorded in `Outcomes & retrospective`. Non-vacuity: the
   timeout test fails if the lock is not actually exclusive, because the second
   `try_lock` would succeed and the test would observe success where it expects
   a timeout.
2. Bounded lifetime. Every fixture command either completes inside its deadline
   or is killed with its process group and reported as a failure.
   Discharged by `bounded_command` tests that run `sh` stubs: both streams
   captured, output larger than a pipe buffer, nonzero status, timeout, direct
   child reaped, and a descendant that keeps stdout open after its parent exits.
   Non-vacuity: the timeout case uses a child that would otherwise run 30 s, so a
   missing deadline shows up as a 30 s test rather than a pass.
3. Warm reuse. A second fixture build compiles only its own `fx_*` package.
   Discharged by extending `fixtures_sharing_a_target_read_only_their_own_generated_suite`
   to parse `-vv` `Compiling` lines and assert no registry or checkout path
   appears in the second build's package set. Non-vacuity: the assertion reads a
   non-empty package set (the `fx_*` crate) and would fail if Cargo recompiled a
   dependency, which the cold first build demonstrates it can.
4. Placement. Fixture sources live under the shared target, and an interrupted
   run leaves `$TMPDIR` empty. Discharged by a Unix-gated interruption test that
   re-runs this test binary with an empty dedicated `TMPDIR`, kills the owner
   after a handshake, and asserts both placement and an empty `TMPDIR` before and
   after the kill. Non-vacuity: the test asserts the owner actually created a
   fixture (the handshake path exists) before asserting absence in `$TMPDIR`.
5. Configuration. The nextest group exists, has `max-threads = 1`, assigns the
   three fixture binaries, and allows more than the exported deadline. Discharged
   by a contract test reading `.config/nextest.toml` through `include_str!` with
   a `toml` dev-dependency, plus a non-vacuity check that each filter matches the
   corresponding binary name.

No new lemma or proof obligation arises: the change is test infrastructure, and
`kani`/`verus` obligations are unchanged. The one external axiom is that
`std::fs::File::try_lock` is an advisory lock honoured across processes on the
host filesystem, documented by the standard library for 1.89 and evidenced by
the timeout test running in two separate processes.

## Milestones

### Milestone 1 EP-M1: cross-process lock and bounded runner

End state: a fixture `cargo` or `cargo kani` command cannot start until it holds
the shared lock, and cannot outlive its deadline.

Work:

1. Add `tests/common/fixture_lock.rs`, re-exported through
   `crates/test-helpers/src/lib.rs`. Resolve the shared target with the existing
   `shared_target_dir`, use `<shared-target>/fixture-cargo.lock`, poll
   `File::try_lock` with a bounded wait, and return an RAII guard. Timeout errors
   name the lock path and the elapsed wait. Release on drop is the `File` close.
2. Add `tests/common/bounded_command.rs`, likewise re-exported. Drain piped
   stdout and stderr on separate threads, poll `try_wait`, bound the join on the
   readers, and on Unix put the child in its own process group
   (`CommandExt::process_group(0)`) so a timeout kills the group with
   `rustix::process::kill_process_group` before reaping.
3. Provide one fixture-command entry point that acquires the lock for the whole
   command and runs it under the deadline. Export
   `DEFAULT_FIXTURE_COMMAND_TIMEOUT` (600 s) and honour
   `THEOREMC_FIXTURE_COMMAND_TIMEOUT_SECS`.
4. Add `rustix` to `crates/test-helpers/Cargo.toml`, Unix-gated, with the `fs`
   and `process` features.
5. Route `cargo_build_log`, the macro suite's `cargo build --color never`, and
   `cargo kani list` through the entry point. Keep combined `BuildLog` output and
   the labelled stdout/stderr failure format; add the exit status to non-zero
   build errors and the command, deadline and partial log to build timeouts.
6. Change `kani_is_installed` to return available, unavailable with a reason, or
   timed out; run the probe through the locked entry point with a short deadline;
   skip with the reason when unavailable, fail on timeout.
7. Delete `FIXTURE_CARGO_LOCK`, `CargoGuard`, the mutex-poison test and the
   guard plumbing, and correct the parallelism and CI-timeout doc comments.

Acceptance: `cargo nextest run -E 'binary(theorem_file_macro_bdd)'` passes,
`make check-fmt`, `make lint` and `make test` pass, and the new unit tests fail
if the lock or the deadline is removed.

### Milestone 2 EP-M2: sources under the shared target

End state: a second run recompiles nothing but the `fx_*` crates.

Work:

1. `FixtureCrate::new` creates `<shared-target>/sources` and uses
   `tempfile::Builder::tempdir_in` with a recognisable prefix and the default
   `0700` permissions. `target_dir()` is unchanged.
2. Add an empty `[workspace]` table to manifests generated by
   `tests/common/build_support_fixture.rs` and
   `tests/theorem_file_macro_bdd/fixture_crate.rs`.
3. Move the macro fixture's path-normalisation and escaping helper into
   `tests/common/`, use it for both fixture families, and reference the
   checkout's `crates/theoremc-build-support` by path from the macro fixture.
   Remove the per-fixture copy and its copy-only tests, keeping the discovery and
   rerun scenarios unchanged.
4. Add a `BuildLog` query returning the packages reported as `Compiling` in a
   `-vv` log, and extend the two-fixture regression to assert the second build
   compiles only its own package.
5. Add a Unix-gated interruption test binary using an ignored owner test
   selected by an environment variable, a printed ready handshake, an empty
   dedicated `TMPDIR`, and an external `CARGO_TARGET_DIR` set with
   `Command::env`.

Acceptance: as above, plus the reuse and interruption tests.

### Milestone 3 EP-M3: contract, ADR, documentation, measurement

End state: the guarantees are documented and mechanically held, and the effect
is recorded.

Work:

1. Add a fast contract test using `include_str!` for `.config/nextest.toml` and a
   root `toml` dev-dependency, asserting the group name, `max-threads = 1`, the
   override's three exact binary filters, and that the group allowance exceeds
   the exported deadline; include a non-vacuity check.
2. Add ADR-0014 amending ADR-0012, and add the `Amended by` banner and
   `## Amendments` section to ADR-0012. Cover the lock and its path; the
   deadline, process-group cleanup and the non-Unix limitation; source placement
   and the `[workspace]` table; the checkout build-support dependency; the `cargo
   build` decision and reasons against `cargo check` and `--locked`; and
   demonstrated versus configuration-dependent reuse.
3. Correct the "compile once per run" Rustdoc, update
   `docs/developers-guide.md` with the timeout override, source location, lock
   behaviour and a one-off command to remove legacy `/tmp` fixture leaks, and add
   the ExecPlan and ADR to `docs/contents.md`.
4. Measure before and after: warm the workspace, then time
   `cargo nextest run -E 'binary(theorem_file_macro_bdd)'` with
   `/usr/bin/time -v`, and count `rustc` and `kani-compiler` executions with
   `strace -f -e trace=execve`.
5. Re-run the #143 reproduction repeatedly with `CARGO_TARGET_DIR="$PWD/target"`
   and `CARGO_BUILD_BUILD_DIR` unset; record timings, invocation counts and
   results here; post a summary on #143 and close it only if repeated runs are
   clean.

Acceptance: every gate passes, CodeRabbit reports no outstanding concern, and a
draft PR is open.

## Idempotence and recovery

Every step is re-runnable: created directories use `create_dir_all`, the lock
file is created if absent and truncated never, and the contract tests only read.
If a fixture run is interrupted, the next run creates fresh source directories
under the target and leaves the stale ones inert; `cargo clean` removes them. If
a gate fails, read the cited log under `/tmp` rather than re-running the gate
before the fix.

## Interfaces and dependencies

- `test_helpers::FixtureLock::acquire() -> Result<FixtureLock, String>` — RAII
  guard, held for one fixture command.
- `test_helpers::run_fixture_command(Command, &Utf8Path, CargoSubcommand) ->
  Result<CommandOutput, FixtureCommandError>` — the single entry point that takes
  the lock and applies the deadline. Defined in Milestone 1.
- `test_helpers::DEFAULT_FIXTURE_COMMAND_TIMEOUT: Duration` — 600 s.
- `theoremc_core::path_format::{normalize_path_separators,
  toml_basic_string_value}` — existing, reused; the macro fixture stops carrying
  its own copy.

## Revision note

Initial draft, 2026-11-23. No revisions yet.
