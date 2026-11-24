# Architectural decision record (ADR) 0014: Serialize and bound fixture cargo commands

## Status

Accepted. Every fixture `cargo` and `cargo kani` command holds a cross-process
advisory lock for its whole lifetime and runs under a deadline that fails the
test when it overruns. Amends ADR-0012.

## Date

2026-11-23

## Context and problem statement

Three integration suites build temporary fixture crates with a real
`cargo build`: `theorem_file_macro_bdd`, `build_suite_bdd` and
`build_discovery_bdd`. ADR-0012 gave them one warm target and put them in the
`nested-cargo-builds` nextest group, which runs one such test at a time.

That was not enough. Issue #197 recorded a `make test` run on a shared host in
which all nine `theorem_file_macro_bdd` scenarios were still running after 78
minutes and had to be killed by hand:

- nextest ran the nine scenarios as nine processes at once, each in its own
  process group, so each had its own copy of the suite's `static Mutex<()>`.
  The mutex serialized nothing, and the doc comment that claimed it protected
  "in-process test parallelism" described a guarantee the runner never provided.
- The nine builds between them had 145 compiler invocations in flight or queued
  at once. One fixture held Cargo's host-wide `~/.cargo/.package-cache` lock
  while it waited for compiler admission, and 62 other Cargo processes on the
  host queued behind it.
- A killed run skipped the fixtures' `TempDir` destructors. One host had
  accumulated 49 fixture directories totalling about 2 GB under `/tmp`.
- The suites had no timeout of their own. Their doc comment deferred to CI's job
  timeout, which a local gate does not have, so a stalled scenario held the
  host's resources indefinitely.

The nextest group is the right default and stays, but it is configuration: a
renamed binary or a lost override silently removes it, and it does not cover a
direct `cargo test --test theorem_file_macro_bdd` run such as `make kani` uses.

## Decision drivers

- The serialization must hold across processes, because nextest's unit of
  isolation is the process.
- A stalled fixture must fail the test that started it, with the command and the
  output, rather than wait for an outer timeout that names nothing.
- A warm run must not rebuild what the previous run built.
- A killed run must leave nothing outside the build directory.
- The guarantees must be cheap to check and hard to dismantle by accident.

## Decision

### Serialization

Every fixture command takes an advisory whole-file lock on
`<shared-target>/fixture-cargo.lock` before it starts and holds it until it
returns. The lock is polled non-blockingly with a bounded wait, and a timeout
names the lock path and the elapsed wait. It is `flock` on Unix, reached through
`rustix::fs::flock` on a capability-held file, because the repository's
`no_std_fs_operations` policy forbids `std::fs`. The standard library's
advisory lock is used on non-Unix hosts.

Two handles opened separately in one process conflict, because the lock belongs
to the open file description rather than to the process. That is what makes the
guarantee hold when nextest turns each test into a process. The operating
system releases the lock when the descriptor closes and when the process exits
for any reason, including a signal that skips destructors, so a killed run
cannot wedge later runs.

### Deadline and cleanup

Every fixture command runs under a deadline exported as
`DEFAULT_FIXTURE_COMMAND_TIMEOUT` (540 seconds) and overridable through
`THEOREMC_FIXTURE_COMMAND_TIMEOUT_SECS` for a slower host. The deadline sits
below the nextest group's own allowance so this code reports an overrun with
the command and its output before nextest terminates the test with nothing but
a timeout notice. The group's allowance was raised from 600 to 1200 seconds on
measurement: a cold fixture build was still running at 540 seconds while every
compiler slot on the host was held by other work, and the default deadline
correctly failed it. The default stays below 600 seconds so an ordinary host
reports the overrun itself; the larger allowance exists so a saturated host can
raise the deadline without nextest killing the test first.

On Unix the child runs in its own process group, and a timeout kills the group
before reaping the direct child. Killing only `cargo` would leave the `rustc`
and `kani-compiler` work it started running, still holding the resources the
deadline exists to release. On non-Unix hosts only the direct child is killed;
the compiler work a `cargo` started can then run to completion on its own. That
is a documented limitation of the fallback, and this repository's gates run on
Unix.

The `cargo kani` availability probe is part of the same policy. It runs under
the lock with its own short deadline and reports one of three outcomes:
available; unavailable with the reason; or timed out. Unavailability skips the
scenario unless `THEOREMC_REQUIRE_KANI=1`, and a timeout is always a failure. A
hung probe is a broken environment, not evidence that Kani is absent.

### Placement

Fixture sources are created in `<shared-target>/sources`, not in `$TMPDIR`, so
`cargo clean` removes them with the artefacts they produced and an interrupted
run cannot leave them behind. Each fixture is still a `TempDir`, so the
ordinary exit path is unchanged. Each generated manifest carries an empty
`[workspace]` table: a fixture created inside the repository tree would
otherwise be treated as a candidate member of the repository workspace.

### Reuse

Every generated manifest names the checkout's `crates/theoremc-build-support`
by path instead of copying that crate into the fixture. A copy gave each
fixture a different package identity, so the shared target recompiled it for
every fixture; a stable checkout path gives it one identity, so it compiles
once.

The suites keep a real `cargo build`. `cargo check` runs build scripts and
would produce the generated suite, and it would report the compile diagnostics
the suite asserts, but the Kani scenarios need the artefacts a build produces,
and one command keeps the suite honest. The warm shared target, not a weaker
command, is what removes the cost that motivated the question.

Fixtures do not pass `--locked`. The fixture's manifest differs from the
workspace's — it has a different package and a different build-dependency path
— so the copied `Cargo.lock` is a seed, not a contract; `--locked` would fail
on the first fixture that resolves anything differently.
`CARGO_NET_OFFLINE=true` already prevents index access, which is what the
package-cache contention came from.

### What reuse is demonstrated, and what is configuration

The in-run claim — that a second fixture build compiles nothing but its own
package — is demonstrated by `tests/build_suite_bdd.rs`, which parses the
`Compiling` lines of a `-vv` log and fails if a registry or checkout package
appears. The cross-run claim depends on the fixture target surviving between
runs: `cargo clean` removes it, and a different `CARGO_TARGET_DIR` gives a
different target. Both are deliberate — a cleaned target is a cold target — but
they mean the cross-run saving is a property of the target directory's
persistence, not a guarantee this record makes.

### The nextest group stays

The `nested-cargo-builds` group keeps `max-threads = 1` and covers the three
fixture binaries. `tests/nextest_config_contract.rs` reads
`.config/nextest.toml` and fails if the group loses its limit, loses one of the
three exact binary filters, allows less than the exported deadline, or if the
run-level `global-timeout` does not exceed the group allowance.

Serializing the suites has a consequence for the run budget: a suite whose
tests used to overlap now takes the sum of its tests' allowances. A contended
run of `theorem_file_macro_bdd` reached 2700 s with sixteen tests passed and
none failed, and nextest's 45-minute `global-timeout` cancelled it. The run
allowance is therefore 2 h, which this record treats as the same kind of bound
as the per-test one: it terminates a hang, it does not decide whether slow work
is allowed to finish.

## Consequences

- A fixture build starts only when every other fixture command has finished, so
  the suite's wall time is the sum of its builds rather than their maximum. On
  a warm target each build is a few seconds; the serial order is what makes the
  shared target safe to use at all.
- A second run on a warm workspace reuses the shared target and recompiles only
  the `fx_*` packages.
- A killed run leaves fixture sources under the target, where `cargo clean`
  removes them, and never in `$TMPDIR`.
- A stalled command fails its own test within 540 seconds, naming the command
  and showing its output.
- Fixture packages are still renamed uniquely (`fx_<pid>_<sequence>_<clock>`),
  because the shared target would otherwise let two fixtures read each other's
  generated suite.
- A developer can raise the deadline for a slow host without editing code.

## Alternatives considered

### Keep the in-process mutex and rely on the nextest group

Rejected. The mutex never serialized anything under nextest; keeping it would
preserve a comment that states a false guarantee. The group alone does not cover
`make kani`'s direct `cargo test` run.

### Per-fixture private target directories

Rejected. That is the ADR-0012 baseline: it rebuilds the whole dependency graph
for every fixture, which is what made the suite slow enough that the fan-out
mattered.

### `cargo check` instead of `cargo build`

Rejected for the reasons above: the Kani scenarios need built artefacts, and
one command keeps the suite's meaning uniform.

### `--locked` fixtures

Rejected: the fixture manifest is not the workspace manifest, so the copied
lock cannot describe it.

## Amendments

None. This record amends ADR-0012 and is not itself amended.
