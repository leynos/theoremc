# Investigation: nested Cargo test delays

Generated: 2026-10-20. Related issue: #50.

Falsification sub-agent: `alchemist`. The planning agent prepares this
investigation; the sub-agent executes the falsification experiment.

## Problem and evidence

The first full test gate ended with one failing fixture and 19 cancelled tests
after 4,673 seconds. Nested fixture builds waited for Cargo's package-cache
lock and compiler admission. One build failed with
`build-limits: the supervisor refused: launch-failed`; the final trybuild test
was separately observed waiting in `cargo clean`. Neither observation
identified a Rust source diagnostic. At the start of the investigation, no
Nextest configuration serialized Cargo-spawning tests.

## H1: fixture isolation repeatedly discards compiled dependencies

Prediction: independent fixtures use different target directories, so their
identical dependencies cannot reuse Cargo artefacts. The in-process mutex
cannot serialize Nextest's separate test processes.

Falsification: inspect both fixture constructors and Cargo command builders,
then inspect the retained test log for repeated dependency compilations and
simultaneous fixture Cargo processes. Do not compile or run a full gate.
Disprove this hypothesis if fixtures already share a persistent target, or if
execution uses a cross-process serialization mechanism. Report separately
whether existing evidence establishes repetition or only its possibility.

Tools: source reads, retained `/tmp/test-*.out` logs and process snapshots. A
negative result decisively rejects the proposed fixture-cache fix.

## H2: an admission refusal caused the observed failed test

Prediction: the failing fixture's first fatal diagnostic comes from the
compiler wrapper, before any source-level diagnostic.

Falsification: inspect only the failed fixture's captured output. Disprove this
hypothesis if a Rust source diagnostic precedes the admission refusal. Do not
modify the supervisor, bypass wrappers, or retry the old process.

Tools: retained test log. A negative result redirects work to the source
failure. Missing logs make the result inconclusive.

## Execution and termination

Execute H1 first, then H2 as a separate bounded investigation. Compare the
findings with recent netsuke and skyjoust fixes before implementation. Stop
when each has an evidence-backed verdict; if both are falsified, revise the
hypotheses. No gate, timeout increase, or test skip is authorized by this plan.

## Findings

Both hypotheses survived their bounded falsification tests. The macro runner
explicitly selects a target under each temporary fixture. Retained process
snapshots show separate Nextest processes simultaneously running nested Cargo
commands despite the mutex. Successful fixture output was not retained, so
repeated compilation of identical dependency versions cannot be counted from
the historical log.

The failed multi-document fixture's first fatal diagnostic was the admission
refusal, followed by Cargo reporting failure compiling `syn`. That result was
an admission failure, not a Rust source error or a test timeout. The unfinished
trybuild test was a separate delay.

A later baseline run using the old private fixture target passed
`existing_theorem_files_are_discovered_recursively` after 5,102.619 seconds.
Its next discovery test remained active beyond 1,320 seconds. During that
build, nested Cargo held both the shared Cargo package-cache mutation lock and
`target/fixture-builds/debug/.cargo-lock`, while compiler requests queued for
admission. Because the fixture target was separate from the workspace target,
the build could not reuse workspace artefacts and amplified host-wide lock and
admission pressure.

Netsuke commit `051358ee360a97fcbebe2a836e25e106917ed845` reuses matching build
artefacts for package verification. Skyjoust commit
`6c7a6b837d0a5de55e10e87ccce3d9fe803f88be` serializes nested compilation tests.
These precedents support persistent fixture build artefacts and a Nextest
scheduling group. Raising timeouts alone does not address either avoidable
recompilation or admission failures.

The repair's conclusion stands: fixtures need persistent, reusable dependency
artefacts plus cross-process serialization of Cargo-spawning tests. The target
branch had independently landed that repair in #228 and #231 before this branch
was rebased, so the rebase adopted the target's implementation and dropped the
branch's parallel one.

What remains from this investigation:

- Fixtures build into a warm target shared by every fixture,
  `theoremc-fixture-builds` under `$CARGO_TARGET_DIR` or the repository
  `target`. Each fixture copies the workspace `Cargo.lock` and gets a unique
  package name, and `generated_suite_contents` reads only the directories named
  after its own package.
- `tests/fixture_cache_contract.rs` pins that contract behaviourally: two
  fixtures share one target, keep distinct source directories, and each
  generated suite lists only its own theorem.
- The `nested-cargo-builds` Nextest group serializes Cargo-spawning test
  binaries across separate test processes, and its filter includes the new
  contract test.
- Cargo-spawning tests keep the process-local `FIXTURE_CARGO_LOCK` mutex for
  the libtest threads inside one process.

The branch's own implementation — fixture sources under
`target/fixture-sources`, selection of the generated suite from Cargo's JSON
messages, and the separate `fixture_cache`, `fixture_output` and
`fixture_manifest` helper modules — duplicated the target's design. The rebase
resolved those conflicts in the target's favour, so none of it is part of the
result.

The branch was rebased onto `origin/main` at `61b738d`, using `2bb0251` as the
verified old-base boundary. Eleven of the twelve commits were replayed. The
remaining commit only relaxed a `Result`-returning test that the resolution
removed, so it replayed empty and was dropped. No test skip or timeout increase
is part of the repair.
