# Architectural decision record (ADR) 0012: Shared warm target for fixture crates

## Status

Accepted. Fixture crates that integration tests build with `cargo` share one
warm target directory, start from the workspace lock, and build offline.

## Date

2026-11-06

## Context and problem statement

Three integration suites (`build_discovery_bdd`, `build_suite_bdd` and
`theorem_file_macro_bdd`) build temporary fixture crates that depend on a copy
of the build-support crate. Each fixture built into a private temporary target,
so every test compiled the build-support dependencies from nothing: 120 to 290
seconds a test on a contended host, 818 tests in 420 seconds with the compiler
suites running together. The estate's nested-cargo rules (NC-004, NC-005) need
those tests to run one at a time with explicit timeouts, and with private cold
builds that serial group could not finish inside the job.

## Decision drivers

- Cold nested builds are what the nested-cargo rules exist to remove, so the
  answer is to make them warm, not to exempt the tests.
- The tests read the generated `theorem_suite.rs` back from the build
  directory, so a fixture must still be able to find its own output.
- A fixture must not wait on the network, and two fixtures must not resolve a
  crate to different versions, because a different version is a different unit
  and the shared target would rebuild it.

## Decision

- All fixture crates build into `theoremc-fixture-builds`, a directory inside
  the caller's `CARGO_TARGET_DIR` or the repository's `target`. Third-party
  dependencies compile once per run, and once per `cargo clean`.
- Each fixture's package is renamed to a short unique name,
  `fx_<pid>_<sequence>_<clock>`: the process id separates concurrent processes,
  the sequence number separates fixtures within one, and the clock separates
  runs that reuse a process id. The name is short because Kani derives a file
  name from the crate name. `generated_suite_contents` reads only directories
  named after that package.
- The workspace `Cargo.lock` is copied into each fixture, and the fixture's
  `cargo` runs with `CARGO_NET_OFFLINE=true`. `CARGO_BUILD_TARGET_DIR` and
  `CARGO_BUILD_BUILD_DIR` are removed from its environment so a caller cannot
  move its artefacts or share a lock.
- The tests that start a child compiler run in the `nested-cargo-builds`
  nextest group, one at a time, with explicit timeouts. The compile-fail unit
  test that starts `rustc` itself also carries its own deadline.

## Addendum (2026-11-06): how a name is kept unique

The decision above says the clock separates runs that reuse a process id. It
does not, on its own: sixteen clock bits wrap, so a reused process id could
meet its own earlier name. A fixture therefore also takes the first suffix for
which the shared target holds no `fx_<suffix>-*` build directory, and a scan
that cannot read the build directory fails the fixture instead of calling the
name free. The clock and sequence number make a clash unlikely; the directory
check makes it impossible.

## Addendum (2026-11-12): renumbering

This record was formerly filed as ADR 011 in
`docs/adr-011-shared-warm-target-for-fixture-crates.md`. The documentation
style guide now requires a four-digit, zero-padded serial that is unique among
the ADRs, in a canonical `adrs/adr-nnnn-title-slug.md` filename. The record now
carries serial 0012, and its serial is stable from this point.

References to "ADR 011" or "ADR-011" in earlier discussions describe this
decision. They are historical context only and do not name a current record.

## Addendum (2026-11-23): fetch once, then build frozen

The decision above made fixtures warm and offline, but left their `cargo` calls
taking the caller's `$CARGO_HOME/.package-cache` lock. Offline mode does not
avoid that lock: `--offline`, `--frozen` and `CARGO_NET_OFFLINE=true` all
acquire it exclusively. Three BDD suites (`build_discovery_bdd`,
`build_suite_bdd` and `theorem_file_macro_bdd`) therefore stalled behind any
other repository's `cargo` on a shared host, and in CI each run resolved the
crates.io index again. Issue #199 records the stall.

- **A dedicated Cargo home.** Fixture builds resolve from
  `THEOREMC_FIXTURE_CARGO_HOME` when it is set, and otherwise from
  `<repository>/target/theoremc-fixture-cargo-home`, canonicalized so a
  relative configuration cannot be resolved against a fixture's own working
  directory. `make fetch` populates that home and `make test` exports its path,
  so the fetch and the build agree on one directory. The home has its own
  `.package-cache`, so a fixture build never contends for the caller's lock,
  and the caller's target stays warm. This is the only way to satisfy the
  contention criterion: no Cargo flag avoids the lock, so the lock has to be a
  different file.
- **`--frozen` on every fixture build.** A build or a `cargo build -vv` in the
  fixture suites passes `--frozen`, which implies `--locked` and `--offline`.
  `cargo kani` is deliberately left without it, because the Kani driver manages
  its own Cargo invocation; those calls keep `CARGO_NET_OFFLINE=true` and the
  seeded lock, and they run in the dedicated home too.
- **Committed template locks.** Each fixture dependency graph has a template
  project under `tests/fixtures/`: a manifest, an empty `src/lib.rs` and a
  `Cargo.lock` pruned from the workspace lock. A fixture seeds its lock from
  the template with the template's root entry renamed to the generated `fx_`
  name, so `--frozen` accepts it unchanged. `make fixture-locks` regenerates
  both locks offline, and `tests/fixture_lock_contract.rs` holds every
  template's registry packages to the root lock by name, version and source
  (concordat NC-001).
- **`make fetch` before anything that builds a frozen fixture.** Both
  `make test` and `make kani` depend on it. The proofs workflow runs
  `make kani` and never ran a fetch of its own, so without the dependency its
  fixture build failed with a missing-package error naming `cap-std`. The
  target runs `cargo fetch --locked` for the workspace and, with
  `CARGO_HOME=$(FIXTURE_CARGO_HOME)`, once per fixture manifest. `ci.yml` and
  `coverage-main.yml` run it where they previously ran a bare `cargo fetch`. CI
  gains a step: the fixture home is not under `~/.cargo`, so the runner's cargo
  cache does not cover it and each CI run fetches it. That cost buys the
  removal of the index update and the lock wait on every run.

Trade-offs and limits:

- A package the fetch misses now fails fast with Cargo's offline diagnostic
  instead of resolving from the network. That is the intended behaviour: the
  failure names the crate, and the fix is the missing manifest in the fetch
  list.
- The dedicated home duplicates the registry cache: the fixture graphs need
  about 300 MB on a cold host. It lives under `target/`, so `cargo clean`
  removes it and the next run fetches it again.
- The outer `cargo nextest run` and `cargo test --doc` still use the caller's
  `$CARGO_HOME`, so `make test` itself can still wait on the caller's package
  cache at start-up. Only the nested fixture builds are isolated. Coordinating
  the outer commands is not this change's scope.

### Verification (2026-11-24)

Measured on the shared development host, which was heavily contended throughout:
`build-limits-status` recorded hundreds of crates queued for a compile slot,
and unrelated repositories were building at the same time. Wall times below are
therefore queue-inclusive and are evidence of behaviour, not a performance
comparison.

- **Lock isolation.** With `flock -x ~/.cargo/.package-cache` held for 75 s,
  `cargo update --workspace --offline` in the caller's home printed
  `Blocking waiting for file lock on package cache` and was still waiting when
  a 60 s `timeout` killed it (exit 124). The same command in the dedicated
  fixture home exited 0 immediately with no blocking message and left the
  fixture lock byte-identical.
- **Empty ambient home.** The fixture suites were run with `CARGO_HOME` set to
  a fresh empty temporary directory and `THEOREMC_FIXTURE_CARGO_HOME` unset.
  `build_suite_bdd`'s shared-target scenario and the offline-failure scenario
  both passed, and the ambient directory was still empty afterwards: the
  fixture builds never read or wrote it.
- **Fail fast without the fetch.** The new scenario
  `an_unfetched_fixture_fails_fast_with_the_offline_diagnostic` runs the frozen
  build against an empty temporary Cargo home. Cargo fails in well under a
  second with `no matching package named \`camino\` found` and the `offline
  mode (via \`--frozen\`)` note, rather than waiting.
- **Suites.** All three pass with the dedicated home and committed templates:
  `build_discovery_bdd` 3/3, `build_suite_bdd` 5/5 (including the offline
  failure) and `theorem_file_macro_bdd` 13/13, the two Kani scenarios included.
- **`make fetch`.** A cold fetch into a new home took 9 m 51 s; a warm repeat
  took 7 m 39 s of wall time but 1.1 s of user time, the difference being the
  compile-slot queue. The dedicated home holds about 295 MB.

A whole-gate `make test` run on this host did not finish green, for a reason
outside the change: 1017 of 1043 nextest tests passed and
`build_discovery_bdd::existing_theorem_files_are_discovered_recursively` was
killed at its 600 s allowance while its child `cargo build --frozen` sat in the
compile-admission queue (558 to 659 crates from other repositories were
waiting, and the fixture target took no file writes while the queue entry
aged). The test is first in the serialized `nested-cargo-builds` group, so the
group's other 25 tests never ran in that invocation. Run directly against the
same commit, outside nextest's per-test allowance, all three suites pass — 3/3,
5/5 and 13/13 — and a repeat fixture build is a true no-op (0 crates compiled,
0.22 s), so the queue is the only cost. `make lint` (rustdoc, Clippy and
Whitaker), `make typecheck` and the doctests all pass on the same commit. A
cold-host measurement of `make test` and the group allowance on a quiet host
remain open, tracked with DF12-109.

## Consequences

- A run on a warm target takes seconds; the first run pays for the dependencies
  once. CI measured the test step at about a quarter to a third of its old
  duration.
- The shared directory persists between runs, so stale package directories
  accumulate until `cargo clean`. They are inert: no fixture reads a name it
  did not generate.
- A fixture that needs a private target must say so in its helper and here.
