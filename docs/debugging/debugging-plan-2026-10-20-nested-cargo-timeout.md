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

The repair keeps temporary source isolation, selects generated output for the
exact fixture, and leaves compiler admission and the shared Cargo cache intact.
Fixture sources now live under `target/fixture-sources`; fixture dependencies
reuse the workspace Cargo target, normally `target`. The `nested-cargo` Nextest
group serializes Cargo-spawning test binaries for build discovery, suite
generation, common support and theorem-file macros, plus the compile-error
suite, across separate test processes. Cargo JSON output is filtered to JSON
object lines before parsing because verbose build-script output can be
interleaved on stdout. Tests cover source isolation, exact manifest/output
selection, mixed Cargo output, malformed JSON, and the shared target contract.

The branch was rebased onto fetched `origin/main` at `2bb0251`, using `af76081`
as the verified old-base boundary. All six branch commits were replayed. The
target's `hermetic_cargo` helper is retained and now receives the workspace
fixture target, so its removal of the caller's build-directory variables still
applies while dependencies remain shared. The target's reference-provenance
guidance and the installed-Kani harness check remain intact.

No test skip or timeout increase is part of the repair. The rebased candidate
`03e01f3` passed `make check-fmt`, `make typecheck`, `make lint`, `make test`
(834 of 834 tests, including doctests), `make markdownlint`, and `make nixie`.
