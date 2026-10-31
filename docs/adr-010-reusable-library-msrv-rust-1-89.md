# Architectural Decision Record (ADR) 010: Reusable-library MSRV for serde-saphyr 2.0

## Status

Accepted. Adopt `serde-saphyr` 2.0.0 and declare Rust 1.89 as the minimum
supported Rust version (MSRV) for every workspace package. This supersedes the
mixed-MSRV boundary in [ADR-009](adr-009-ortho-config-cli-configuration.md);
ADR-009's application-only OrthoConfig configuration boundary remains accepted.

## Date

2026-10-31

## Context and problem statement

`theoremc-core` uses `serde-saphyr` for YAML deserialization. Issue #200
adopted a mixed-MSRV policy so that the reusable libraries could keep Rust 1.88
while the `cargo-theorem` application package declared Rust 1.89 for
`ortho_config` 0.10.0.

The original `serde-saphyr = "1.0.1"` caret requirement resolved to 1.3.0,
which declares Rust 1.89. To keep the library check on Rust 1.88, #200 pinned
the requirement to 1.1.0 and locked that release. `serde-saphyr` 2.0.0 is now
the current release and declares `rust-version = "1.89"`. Retaining 1.1.0 keeps
a stale parser line solely to preserve a Rust 1.88 floor, adds a second
compatibility axis to dependency updates, and prevents adopting the current
parser and diagnostic behaviour.

## Decision Drivers

- Keep the YAML parser on a supported current release.
- Remove a mixed-MSRV policy whose only remaining justification was the older
  parser.
- Keep the toolchain contract simple to test locally and in CI.
- Preserve the public YAML schema and structured diagnostic behaviour.

## Options considered

### Option A: retain serde-saphyr 1.1.0 and Rust 1.88 libraries

Rejected. It preserves a stale parser release, and any later dependency update
can silently reintroduce the Rust 1.89 requirement. Pinning also leaves a
split-MSRV policy that no longer reflects an application-only dependency
boundary.

### Option B: adopt serde-saphyr 2.0.0 and raise every package to Rust 1.89

Accepted. The workspace already ships a Rust 1.89 application package, so the
split no longer protects a single coherent consumer floor. Moving every package
to Rust 1.89 gives the workspace one testable contract and keeps the parser on
its current release line.

### Option C: adopt serde-saphyr 1.3.0 and keep Rust 1.88 libraries

Rejected. Version 1.3.0 already requires Rust 1.89, so it raises the MSRV
without providing 2.0's current release line.

## Decision

1. `theoremc-core` declares `serde-saphyr = "2.0.0"` with default features. The
   loader keeps using `from_multiple`, `Location`, `Spanned`, and error
   locations; the version bump is otherwise source-compatible.
2. Every workspace package declares `rust-version = "1.89"`: the root
   `theoremc` facade, `theoremc-core`, `theoremc-macros`,
   `theoremc-build-support`, `test-helpers`, and `cargo-theorem`.
3. `make msrv` runs `RUSTFLAGS="" cargo +1.89 check --workspace`. Continuous
   integration installs only the Rust 1.89 toolchain and runs that target. The
   previous Rust 1.88 library and Rust 1.89 application split is retired.
4. ADR-009's mixed-MSRV clauses are superseded. Its application-only
   OrthoConfig configuration boundary, discovery and precedence rules, profile
   support, and command metadata decisions remain in force.

## Consequences

- Library consumers now require Rust 1.89, the same floor as the CLI.
- One MSRV check replaces the split checks, and CI no longer installs Rust
  1.88.
- The MSRV gate covers library and binary targets. Test-only dependencies are
  outside the contract because the current `rstest-bdd` stack resolves a
  `cargo-platform` release that requires a newer toolchain than Rust 1.89.
- The workspace lockfile resolves `serde-saphyr` 2.0.0 and its
  `granit-parser` 2.0.0 dependency.
- The existing schema, loader, diagnostic, and behavioural tests remain the
  compatibility evidence for YAML parsing after the upgrade.
- Downstream crates that require Rust 1.88 must upgrade or remain on the last
  theoremc release that supported that floor.

## Related documents

- [ADR-002: library-first Fluent localization](adr-002-library-first-internationalization-and-localization-with-fluent.md)
- [ADR-009: OrthoConfig CLI configuration](adr-009-ortho-config-cli-configuration.md)
- [`cargo theorem` CLI design](cargo-theorem-cli-design.md)
- [Developer's guide](developers-guide.md)
- [Theoremc design](theoremc-design.md)
- [Theoremc development roadmap](roadmap.md)
