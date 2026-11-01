# Documentation contents

- [Documentation contents](contents.md) - Index for the repository
  documentation set.
- [Repository layout](repository-layout.md) - Canonical guide to the source
  tree, generated artefacts, fixtures, and long-lived documentation locations.

## User and contributor guides

- [User's guide](users-guide.md) - Guide for library consumers covering schema
  types, loading API, identifier rules, and value forms.
- [Developer's guide](developers-guide.md) - Maintainer manual for build,
  test, lint, release, extension, and contribution workflows.
- [Documentation style guide](documentation-style-guide.md) - Authoring rules
  for spelling, formatting, Markdown structure, diagrams, and documentation
  consistency.
- [0.2.0 migration guide](v0-2-0-migration-guide.md) - What the new build
  defaults change for an existing checkout.
- [Scripting standards](scripting-standards.md) - Standards for project
  scripts, covering toolchain choices, structure, and operational expectations.
- [`cargo theorem` CLI design](cargo-theorem-cli-design.md) - Cargo-native,
  agent-readable entry point for project scaffolding, build integration,
  backend management, theorem execution, reporting, and migration from
  `rust-prover-tools`.
- [`cargo theorem` user's guide](cargo-theorem-cli-users-guide.md) -
  Task-oriented guide for callers of the planned CLI, covering installation,
  project initialization, theorem creation, checking, backend management,
  execution, reports, JSON output, exit classes, and migration.

## Architecture and design

- [Theoremc design specification](theoremc-design.md) - End-to-end
  architecture and design rationale for theoremc, covering parser, code
  generation, backend, and reporting design.
- [Theorem file format](theorem-file-specification.md) - Normative schema and
  behavioural specification for `.theorem` documents and their compile-time
  semantics.
- [Name mangling rules](name-mangling-rules.md) - Normative mapping rules for
  theorem action names, generated Kani harness symbols, and external theorem
  identifiers.
- [Theoremc development roadmap](roadmap.md) - Phased implementation roadmap
  with traceable tasks derived from design, schema, mangling, and ADR
  requirements.
- [Request for Comments (RFC) 0001: semantic law templates over composable
  primaries](rfcs/0001-semantic-law-templates.md) - Accepted proposal for
  schema version 2 structured laws, composable primaries, semantic Law IR, and
  staged Kani lowering.
- [RFCs](rfcs/) - Accepted and proposed requests for comments that define
  substantial language and architecture changes before implementation.

## Decision records

- [Architecture Decision Record (ADR) 001: theorem symbol stability and
  non-vacuity
  policy](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
  - Decision record for injective symbol mangling, explicit reference
    semantics, non-vacuity requirements, and theorem ID migration.
- [Architecture Decision Record (ADR) 002: library-first internationalization
  and localization with
  Fluent](adr-002-library-first-internationalization-and-localization-with-fluent.md)
  - Decision record for Fluent-based, library-safe localization via injected
    localizers, structured diagnostic codes, and deterministic fallback policy.
- [Architecture Decision Record (ADR) 003: architectural boundary enforcement
  for schema layers](adr-003-architectural-boundary-enforcement.md)
  - Decision record for schema layering constraints, module boundary policy,
    and an incremental architecture-enforcement stack.
- [Architecture Decision Record (ADR) 004: theorem-side action
  signatures](adr-004-action-signature-specification.md)
  - Decision record for explicit theorem-side action signatures used by typed
    action probes and future argument shaping.
- [Architecture Decision Record (ADR) 005: Rust build
  standard](adr-005-rust-build-standard.md) - Decision record for the shared
  Rust build, test, and release conventions adopted by the workspace.
- [Architecture Decision Record (ADR) 005: vertical-slice-first roadmap
  sequencing](adr-005-vertical-slice-first-roadmap-sequencing.md)
  - Decision record for making a real Kani proof and counterexample the next
    release gate, before semantic laws or rich reporting.
- [Architecture Decision Record (ADR) 006: backend-neutral harness planning
  boundary](adr-006-backend-neutral-harness-planning-boundary.md)
  - Decision record for the shared harness plan that supports the Kani
    vertical slice and later RFC 0001 law lowering without direct
    schema-to-Kani coupling.
- [Architecture Decision Record (ADR) 007: Verus backend preservation
  invariants](adr-007-verus-backend-preservation-invariants.md)
  - Decision record for keeping law meaning, primary identity, backend
    bindings, evidence, and Verus mode distinctions separate.
- [Architecture Decision Record (ADR) 008: workspace build-support
  boundary](adr-008-workspace-build-support-boundary.md)
  - Decision record for the narrow crate boundary around theorem discovery
    and generated-suite writing.

## Reference material

- [Navigating code complexity: a guide for implementers and
  maintainers](complexity-antipatterns-and-refactoring-strategies.md)
  - Practical guide to complexity metrics, anti-patterns, and refactoring
    strategies for long-lived codebases.
- [Reliable testing in Rust via dependency
  injection](reliable-testing-in-rust-via-dependency-injection.md)
  - Patterns for testable Rust design using dependency injection to avoid
    global-state coupling.
- [A systematic guide to effective, ergonomic, and "don't repeat yourself"
  (DRY) doctests in Rust](rust-doctest-dry-guide.md)
  - Detailed reference for writing maintainable Rust doctests with minimal
    duplication.
- [Mastering test fixtures in Rust with
  `rstest`](rust-testing-with-rstest-fixtures.md)
  - Comprehensive reference for fixture design, parametrization, and best
    practices with `rstest`.
- [Localizable Rust libraries with Fluent](localizable-rust-libraries-with-fluent.md)
  - Reference material for library-safe localization patterns and Fluent
    integration.

## Execution plans

- [Execution plans](execplans/) - Living implementation plans for roadmap
  tasks.
  - [Code base audit findings from
    2026-06-05](execplans/code-base-audit-2026-06-05.md)
    - ExecPlan for addressing canonical action names, typed validation
      diagnostics, schema serde boundaries, macro separation, reusable test
      helpers, and documentation gaps.
  - [Step 1.1: `TheoremDoc` and subordinate schema
    types](execplans/1-1-1-theorem-doc-and-subordinate-schema-types.md)
    - ExecPlan for strict theorem document deserialization.
  - [Step 1.2.1: Validate required fields and non-empty
    constraints](execplans/1-2-1-validate-required-fields-and-non-empty-constraints.md)
    - ExecPlan for post-deserialization semantic validation rules.
  - [Step 1.2.2: Parse expression fields as
    `syn::Expr`](execplans/1-2-2-parse-assume-prove-and-witness-expressions.md)
    - ExecPlan for expression syntax validation and statement block
      rejection.
  - [Step 1.2.3: Enforce `Step` and `LetBinding` shape
    rules](execplans/1-2-3-enforce-step-and-let-binding.md)
    - ExecPlan for step and let-binding structural validation.
  - [Step 1.2.4: Enforce non-vacuity
    defaults](execplans/1-2-4-enforce-non-vacuity-defaults.md)
    - ExecPlan for vacuity policy defaults and override validation.
  - [Step 1.3.1: Structured diagnostics for parser
    failures](execplans/1-3-1-structured-diagnostics-for-parser-failures.md)
    - ExecPlan for source-located diagnostics and parser regression fixtures.
  - [Step 2.1.1: Canonical action-name
    validation](execplans/2-1-1-canonical-action-name-validation.md)
    - ExecPlan for dot-separated action-name grammar and keyword validation.
  - [Step 2.1.2: Action
    mangling](execplans/2-1-2-action-mangling.md)
    - ExecPlan for deterministic canonical action-name mangling.
  - [Step 2.1.3: Fail compilation on duplicate action
    names](execplans/2-1-3-fail-compilation-on-duplicate-action-names.md)
    - ExecPlan for duplicate action-name detection.
  - [Step 2.2.1: Per-file naming using path mangle and
    hash12](execplans/2-2-1-per-file-naming-using-path-mangle-and-hash12.md)
    - ExecPlan for deterministic per-file module naming.
  - [Step 2.2.2: Deterministic harness
    naming](execplans/2-2-2-deterministic-harness-naming.md)
    - ExecPlan for stable generated Kani harness identifiers.
  - [Step 2.3.1: Argument decoding for plain YAML
    strings](execplans/2-3-1-argument-decoding-for-plain-yaml-strings.md)
    - ExecPlan for argument decoding semantics.
  - [Step 2.3.2: Optional literal text
    wrapper](execplans/2-3-2-optional-literal-text-wrapper.md)
    - ExecPlan for explicit literal wrapper decoding.
  - [Step 2.3.3: Struct literal synthesis from YAML
    maps](execplans/2-3-3-struct-literal-synthesis-from-yaml-maps.md)
    - ExecPlan for map-backed argument lowering.
  - [Step 3.1.1: Build.rs scanning of
    theorems](execplans/3-1-1-build-rs-scanning-of-theorems.md)
    - ExecPlan for Cargo build-script theorem discovery.
  - [Step 3.1.2: Generate OUT_DIR theorem
    suite](execplans/3-1-2-generate-out-dir-theorem-suite-rs.md)
    - ExecPlan for generated theorem suite wiring.
  - [Step 3.2.1: Stable per-file module macro
    expansions](execplans/3-2-1-stable-per-file-module-macro-expansions.md)
    - ExecPlan for proc-macro per-file module expansion.
  - [Step 3.2.2: Gate generated Kani
    harnesses](execplans/3-2-2-gate-generated-kani-harnesses.md)
    - ExecPlan for cfg-gated Kani proof harness generation.
  - [Step 3.3.1: Emit typed action
    probes](execplans/3-3-1-emit-typed-action-probes.md)
    - ExecPlan for typed theorem action probes.
  - [Step 3.3.2: Referenced-type probes for generated struct literal
    synthesis](execplans/3-3-2-referenced-type-probes-for-generated-struct-literal-synthesis.md)
    - ExecPlan for compile-time referenced-type probes.
  - [Include provenance in macro action-signature conflict
    diagnostics](execplans/issue-51-include-provenance-in-macro-action-signature-conflict-diagnostics.md)
    - ExecPlan for issue 51.
  - [Resolve the 2026-06-14 codebase
    audit](execplans/codebase-audit-2026-06-14.md)
    - ExecPlan for resolving audit findings across diagnostics, validation,
      tests, documentation, and dependency policy.
