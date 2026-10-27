# Repository layout

This document maps the current theoremc repository structure and identifies the
owning component for code, tests, and documentation. Architecture rationale
lives in [the design document](theoremc-design.md); contributor workflows live
in the [developer's guide](developers-guide.md).

## Top-level files

**Table:** Repository root responsibilities

| Path                  | Responsibility                                                                        |
| --------------------- | ------------------------------------------------------------------------------------- |
| `Cargo.toml`          | Workspace membership, root package metadata, feature flags, and workspace lint policy |
| `Cargo.lock`          | Locked dependency graph for reproducible workspace builds                             |
| `Makefile`            | Canonical local quality gates and formatting commands                                 |
| `build.rs`            | Root build-script adapter that emits cfg and rerun metadata from build-support output |
| `src/lib.rs`          | Public facade that re-exports `theoremc-core` and `theoremc-macros` APIs              |
| `src/arg_lowering.rs` | Test-gated prototype for future argument-expression lowering                          |

## Workspace crates

**Table:** Workspace crate responsibilities

| Path                             | Responsibility                                                                                |
| -------------------------------- | --------------------------------------------------------------------------------------------- |
| `crates/cargo-theorem/`          | Cargo CLI entry point, application configuration, command metadata, and process boundary      |
| `crates/theoremc-build-support/` | Workspace-internal theorem discovery, generated-suite writing, and build-script preparation   |
| `crates/theoremc-core/`          | Schema loading, validation, diagnostics, name mangling, collision checks, and theorem-file IO |
| `crates/theoremc-macros/`        | `theorem_file!` proc-macro expansion, generated Kani harnesses, and typed action probes       |
| `crates/test-helpers/`           | Shared test support crate for integration tests that need reusable helpers                    |

The root `theoremc` package remains the consumer-facing library facade and owns
the Cargo build-script adapter. The `cargo-theorem` package owns the separate
CLI application boundary and is the only package that depends on OrthoConfig.
`theoremc-build-support` owns discovery and suite writing behind a
workspace-internal Cargo crate boundary. The root package should not duplicate
core schema or macro expansion logic owned by the workspace crates.

## Tests and fixtures

**Table:** Test layout

| Path                            | Responsibility                                                                     |
| ------------------------------- | ---------------------------------------------------------------------------------- |
| `tests/*.rs`                    | Integration and behaviour-driven development (BDD) test entrypoints                |
| `tests/features/*.feature`      | `rstest-bdd` feature specifications                                                |
| `tests/fixtures/*.theorem`      | Valid and invalid theorem documents used by schema and macro tests                 |
| `tests/common/`                 | Shared integration-test support for fixture crates, schema loading, and assertions |
| `crates/*/src/*_tests.rs`       | Unit tests colocated with the crate that owns the implementation                   |
| `crates/theoremc-macros/tests/` | Proc-macro compile-pass and compile-fail fixtures                                  |

## Documentation

**Table:** Documentation entrypoints

| Path                                                                               | Responsibility                                                           |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `docs/contents.md`                                                                 | Index of repository documentation                                        |
| `docs/users-guide.md`                                                              | User-facing API and theorem-file behaviour guide                         |
| `docs/developers-guide.md`                                                         | Maintainer workflows, quality gates, and internal conventions            |
| `docs/theorem-file-specification.md`                                               | Normative `.theorem` schema and semantics reference                      |
| `docs/theoremc-design.md`                                                          | Architecture, design rationale, and current versus planned system shape  |
| `docs/cargo-theorem-cli-design.md`                                                 | `cargo theorem` CLI contract, boundaries, and implementation sequence    |
| `docs/cargo-theorem-cli-users-guide.md`                                            | User guide for the implemented `context` foundation and planned commands |
| `docs/adr-009-ortho-config-cli-configuration.md`                                   | Application-only OrthoConfig decision and MSRV boundary                  |
| `docs/ortho-config/`                                                               | Imported OrthoConfig guides and local redirects to pinned upstream docs  |
| `docs/ortho-config/users-guide.md`                                                 | Imported OrthoConfig v0.10.0 users' guide                                |
| `docs/ortho-config/v0-10-0-migration-guide.md`                                     | Imported v0.10.0 migration reference                                     |
| `docs/ortho-config-v0-10-0-provenance.md`                                          | Immutable source commit and imported-file checksums                      |
| `docs/ortho-config/adrs/`                                                          | Redirect to the pinned OrthoConfig external-subcommand ADR               |
| `docs/examples/hello_world/`                                                       | Redirect to the pinned OrthoConfig Hello World example                   |
| `docs/ortho_l10n/`                                                                 | Redirect to the pinned OrthoConfig localization crate README             |
| `docs/execplans/issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli.md` | Living implementation and validation plan                                |
| `docs/adr-*.md`                                                                    | Architecture Decision Records (ADRs)                                     |
| `docs/rfcs/`                                                                       | Language and architecture Requests for Comments (RFCs)                   |
| `docs/execplans/`                                                                  | Living implementation plans and milestone records                        |

An RFC defines a substantial proposed or accepted language and architecture
change. An ADR records the decision and constraints that govern implementation.
An ExecPlan translates accepted decisions into executable milestones.

Update this file and `docs/contents.md` when a directory gains a new durable
responsibility or when documentation moves.
