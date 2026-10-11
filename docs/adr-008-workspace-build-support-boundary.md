# Architectural Decision Record (ADR) 008: workspace build-support boundary

## Status

Accepted. Keep theorem discovery and generated-suite writing behind the
workspace-internal `theoremc-build-support` crate boundary.

## Date

2026-10-11

## Decision metadata

- Deciders: theoremc maintainers
- Technical story: replace build-script source sharing by file path for the
  existing discovery and suite-generation implementation

## Context

The root `build.rs` previously included `src/build_discovery.rs` and
`src/build_suite.rs` with `#[path]`. This made the build script and root
library/test compilation depend on those implementation files remaining at
specific paths. Moving or reorganizing a module could therefore break a
separate Cargo compilation context even when the implementation itself was
unchanged.

[ADR-005](adr-005-vertical-slice-first-roadmap-sequencing.md) deferred
build-support crate extraction as noncritical to its Kani vertical-slice gate.
The issue addressed here is narrower than a general extraction programme: the
existing discovery and suite-generation code was already shared across
build-script and library/test compilation by source placement rather than by a
Cargo crate boundary.

## Decision

Place the existing build-script preparation, theorem discovery, and generated
suite writing in `crates/theoremc-build-support`. The root package declares
that crate as its direct build dependency. Root `build.rs` remains the Cargo
adapter: it calls `prepare_build_script()`, then emits the cfg and rerun
metadata using the returned `BuildScriptOutput`.

The support crate's API is `pub` where the build script or fixture crates must
call across a crate boundary. This visibility does not make the package part of
the public `theoremc` library API or establish a downstream dependency
contract. Reuse is limited to workspace build integration, the support crate's
own tests, and fixture crates that verify Cargo build-script behaviour. Do not
include support implementation source with `#[path]`.

This decision supersedes only the build-support extraction deferral in
[ADR-005](adr-005-vertical-slice-first-roadmap-sequencing.md), and only for the
existing discovery and suite-writing implementation. It does not make broad
build-support extraction a project policy or reopen any other ADR-005
sequencing decision.

## Consequences

- The root build script, support crate, and fixture crates compile through the
  same explicit Cargo dependency boundary rather than sharing source by path.
- Build-support implementation dependencies belong to the support crate;
  root test dependencies remain owned by the root package's tests.
- Unit tests stay with the support crate. Behavioural fixture-crate tests
  continue to verify Cargo rerun metadata and generated-suite wiring.
- Internal refactoring of discovery or suite-writing modules no longer
  requires preserving their former `src/` paths for build-script compilation.
