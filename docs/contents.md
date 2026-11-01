# Documentation contents

- [Documentation contents](contents.md) - Index of repository documentation.
- [Repository layout](repository-layout.md) - Guide to source, workspace
  crates, tests, fixtures, generated artefacts, and documentation locations.

## User and contributor guides

- [User's guide](users-guide.md) - Guide for library consumers loading
  `.theorem` files, interpreting schema types, and understanding value forms.
- [Developer's guide](developers-guide.md) - Maintainer manual for build,
  test, lint, release, extension, and contribution workflows.
- [Documentation style guide](documentation-style-guide.md) - Rules for
  spelling, Markdown structure, diagrams, document types, and Rust API comments.
- [`cargo theorem` user's guide](cargo-theorem-cli-users-guide.md) - Guide to
  installing, initialising, and using the CLI, including reports, JSON output,
  exit classes, and migration.
- [0.2.0 migration guide](v0-2-0-migration-guide.md) - Build-default changes
  that existing checkouts need to account for.

## Architecture and design

- [Theoremc design specification](theoremc-design.md) - Architecture and
  design rationale for theoremc's parser, schema, code generation, backend, and
  reporting.
- [Theorem file format](theorem-file-specification.md) - Normative schema and
  behavioural specification for `.theorem` documents.
- [Name mangling rules](name-mangling-rules.md) - Normative mappings for
  theorem action names, generated Kani harness symbols, and external theorem
  identifiers.
- [Theoremc development roadmap](roadmap.md) - Phased implementation roadmap
  with traceable tasks derived from design, schema, mangling, and ADR
  requirements.
- [`cargo theorem` CLI design](cargo-theorem-cli-design.md) - Cargo-native
  entry point for scaffolding, build integration, backend management,
  execution, reporting, and migration from `rust-prover-tools`.

## Decision records

- [ADR 001: theorem symbol stability and non-vacuity policy](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
  - Decision record for injective symbol mangling, explicit reference
    semantics, non-vacuity requirements, and theorem ID migration.
- [ADR 002: library-first internationalisation and localisation with Fluent](adr-002-library-first-internationalization-and-localization-with-fluent.md)
  - Decision record for library-safe localisation via injected localisers,
    structured diagnostic codes, and deterministic fallback policy.
- [ADR 003: architectural boundary enforcement for schema layers](adr-003-architectural-boundary-enforcement.md)
  - Decision record for schema layering, module boundaries, and incremental
    architecture enforcement.
- [ADR 004: theorem-side action signatures](adr-004-action-signature-specification.md)
  - Decision record for theorem-side signatures used by typed action probes
    and future argument shaping.
- [ADR 005: Rust build standard for development builds](adr-005-rust-build-standard.md)
  - Decision record for development-build compiler and linker flags.
- [ADR 005: vertical-slice-first roadmap sequencing](adr-005-vertical-slice-first-roadmap-sequencing.md)
  - Decision record for delivering a working Kani proof and counterexample
    before expanding semantic-law breadth or reporting.
- [ADR 006: backend-neutral harness planning boundary](adr-006-backend-neutral-harness-planning-boundary.md)
  - Decision record for a shared harness plan without direct schema-to-Kani
    coupling.
- [ADR 007: Verus backend preservation invariants](adr-007-verus-backend-preservation-invariants.md)
  - Decision record for preserving law meaning, primary identity, backend
    bindings, evidence, and Verus mode distinctions.
- [ADR 008: workspace build-support boundary](adr-008-workspace-build-support-boundary.md)
  - Decision record for the crate boundary around theorem discovery and
    generated-suite writing.

## Requests for comments

- [RFC 0001: semantic law templates over composable primaries](rfcs/0001-semantic-law-templates.md)
  - Accepted proposal for schema version 2 structured laws, composable
    primaries, semantic Law IR, and staged Kani lowering.
- [RFCs](rfcs/) - Proposed and accepted requests for comments on substantial
  language and architecture changes.

## Reference material

- [Navigating code complexity](complexity-antipatterns-and-refactoring-strategies.md)
  - Guide to complexity metrics, anti-patterns, and refactoring strategies.
- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md)
  - Patterns for testable Rust design without global-state coupling.
- [Localisable Rust libraries with Fluent](localizable-rust-libraries-with-fluent.md)
  - Reference material for library-safe localisation and Fluent integration.
- [A systematic guide to effective, ergonomic, and DRY doctests in Rust](rust-doctest-dry-guide.md)
  - Guide to maintainable Rust doctests with minimal duplication.
- [Mastering test fixtures in Rust with `rstest`](rust-testing-with-rstest-fixtures.md)
  - Reference for fixture design and parametrisation with `rstest`.
- [Scripting standards](scripting-standards.md) - Standards for project
  scripts, toolchain choices, structure, and operational expectations.

## Execution plans

- [Execution plans](execplans/) - Living implementation plans for roadmap
  tasks and audit findings.
  - [Step 1.1: `TheoremDoc` and subordinate schema types](execplans/1-1-1-theorem-doc-and-subordinate-schema-types.md)
    - ExecPlan for strict theorem document deserialisation.
  - [Step 1.2.1: Validate required fields and non-empty constraints](execplans/1-2-1-validate-required-fields-and-non-empty-constraints.md)
    - ExecPlan for post-deserialisation semantic validation rules.
  - [Step 1.2.2: Parse expression fields as `syn::Expr`](execplans/1-2-2-parse-assume-prove-and-witness-expressions.md)
    - ExecPlan for expression syntax validation and statement-block rejection.
  - [Step 1.2.3: Enforce `Step` and `LetBinding` shape rules](execplans/1-2-3-enforce-step-and-let-binding.md)
    - ExecPlan for step and let-binding structural validation.
  - [Step 1.2.4: Enforce non-vacuity defaults](execplans/1-2-4-enforce-non-vacuity-defaults.md)
    - ExecPlan for vacuity-policy defaults and override validation.
  - [Step 1.3.1: Structured diagnostics for parser failures](execplans/1-3-1-structured-diagnostics-for-parser-failures.md)
    - ExecPlan for source-located diagnostics and parser regression fixtures.
  - [Step 2.1.1: Canonical action-name validation](execplans/2-1-1-canonical-action-name-validation.md)
    - ExecPlan for dot-separated action-name grammar and keyword validation.
  - [Step 2.1.2: Action mangling](execplans/2-1-2-action-mangling.md)
    - ExecPlan for deterministic canonical action-name mangling.
  - [Step 2.1.3: Reject duplicate action names](execplans/2-1-3-fail-compilation-on-duplicate-action-names.md)
    - ExecPlan for detecting duplicate action names at compile time.
  - [Step 2.2.1: Per-file naming with path mangle and hash12](execplans/2-2-1-per-file-naming-using-path-mangle-and-hash12.md)
    - ExecPlan for deterministic per-file module naming.
  - [Step 2.2.2: Deterministic harness naming](execplans/2-2-2-deterministic-harness-naming.md)
    - ExecPlan for stable generated Kani harness identifiers.
  - [Step 2.3.1: Argument decoding for plain YAML strings](execplans/2-3-1-argument-decoding-for-plain-yaml-strings.md)
    - ExecPlan for argument-decoding semantics.
  - [Step 2.3.2: Optional literal text wrapper](execplans/2-3-2-optional-literal-text-wrapper.md)
    - ExecPlan for explicit literal-wrapper decoding.
  - [Step 2.3.3: Struct literal synthesis from YAML maps](execplans/2-3-3-struct-literal-synthesis-from-yaml-maps.md)
    - ExecPlan for map-backed argument lowering.
  - [Step 3.1.1: Build-script theorem scanning](execplans/3-1-1-build-rs-scanning-of-theorems.md)
    - ExecPlan for theorem discovery in the Cargo build script.
  - [Step 3.1.2: Generate the `OUT_DIR` theorem suite](execplans/3-1-2-generate-out-dir-theorem-suite-rs.md)
    - ExecPlan for generated theorem-suite wiring.
  - [Step 3.2.1: Stable per-file macro expansions](execplans/3-2-1-stable-per-file-module-macro-expansions.md)
    - ExecPlan for per-file `theorem_file!` expansion.
  - [Step 3.2.2: Gate generated Kani harnesses](execplans/3-2-2-gate-generated-kani-harnesses.md)
    - ExecPlan for cfg-gated Kani proof-harness generation.
  - [Step 3.3.1: Emit typed action probes](execplans/3-3-1-emit-typed-action-probes.md)
    - ExecPlan for typed theorem action probes.
  - [Step 3.3.2: Emit referenced-type probes](execplans/3-3-2-referenced-type-probes-for-generated-struct-literal-synthesis.md)
    - ExecPlan for generated struct-literal synthesis type probes.
  - [Resolve the 2026-06-14 codebase audit](execplans/codebase-audit-2026-06-14.md)
    - ExecPlan for findings across diagnostics, validation, tests,
      documentation, and dependency policy.
  - [Include provenance in action-signature conflict diagnostics](execplans/issue-51-include-provenance-in-macro-action-signature-conflict-diagnostics.md)
    - ExecPlan for adding theorem provenance to signature-conflict errors.
