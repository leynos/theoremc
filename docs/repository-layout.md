# Repository layout

This document maps the current theoremc repository structure and identifies the
owning component for code, tests, and documentation. Architecture rationale
lives in [the design document](theoremc-design.md); contributor workflows live
in the [developer's guide](developers-guide.md).

## Top-level files

**Table:** Repository root responsibilities

| Path                     | Responsibility                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------- |
| `Cargo.toml`             | Workspace membership, root package metadata, feature flags, and workspace lint policy |
| `Cargo.lock`             | Locked dependency graph for reproducible workspace builds                             |
| `Makefile`               | Canonical local quality gates and formatting commands                                 |
| `build.rs`               | Root package build script that discovers theorem files and writes the generated suite |
| `src/lib.rs`             | Public facade that re-exports `theoremc-core` and `theoremc-macros` APIs              |
| `src/main.rs`            | Placeholder command entrypoint for the root package binary                            |
| `src/build_discovery.rs` | Theorem file discovery used by the root build script                                  |
| `src/build_suite.rs`     | Generated-suite rendering for build-script output                                     |
| `src/arg_lowering.rs`    | Test-gated prototype for future argument-expression lowering                          |

## Workspace crates

**Table:** Workspace crate responsibilities

| Path                      | Responsibility                                                                                |
| ------------------------- | --------------------------------------------------------------------------------------------- |
| `crates/theoremc-core/`   | Schema loading, validation, diagnostics, name mangling, collision checks, and theorem-file IO |
| `crates/theoremc-macros/` | `theorem_file!` proc-macro expansion, generated Kani harnesses, and typed action probes       |
| `crates/test-helpers/`    | Shared test support crate for integration tests that need reusable helpers                    |

The root `theoremc` package remains the consumer-facing facade and owns Cargo
build integration. It should not duplicate core schema or macro expansion logic
owned by the workspace crates.

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

| Path                                 | Responsibility                                                          |
| ------------------------------------ | ----------------------------------------------------------------------- |
| `docs/contents.md`                   | Index of repository documentation                                       |
| `docs/users-guide.md`                | User-facing API and theorem-file behaviour guide                        |
| `docs/developers-guide.md`           | Maintainer workflows, quality gates, and internal conventions           |
| `docs/theorem-file-specification.md` | Normative `.theorem` schema and semantics reference                     |
| `docs/theoremc-design.md`            | Architecture, design rationale, and current versus planned system shape |
| `docs/adr-*.md`                      | Architecture Decision Records (ADRs)                                    |
| `docs/execplans/`                    | Living implementation plans and milestone records                       |

Update this file and `docs/contents.md` when a directory gains a new durable
responsibility or when documentation moves.
