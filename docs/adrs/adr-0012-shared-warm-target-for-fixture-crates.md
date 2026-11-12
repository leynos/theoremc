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

## Consequences

- A run on a warm target takes seconds; the first run pays for the dependencies
  once. CI measured the test step at about a quarter to a third of its old
  duration.
- The shared directory persists between runs, so stale package directories
  accumulate until `cargo clean`. They are inert: no fixture reads a name it
  did not generate.
- A fixture that needs a private target must say so in its helper and here.
