# Documentation contents

- [Documentation contents](contents.md) - Index for the repository
  documentation set.
- [Repository layout](repository-layout.md) - Canonical guide to the source
  tree, generated artefacts, fixtures, and long-lived documentation locations.

## User and contributor guides

- [User's guide](users-guide.md) - Task-oriented guide for library consumers
  loading `.theorem` files, interpreting schema types, and understanding value
  forms.
- [Developer's guide](developers-guide.md) - Maintainer manual for build,
  test, lint, release, extension, and contribution workflows.
- [Documentation style guide](documentation-style-guide.md) - Authoring rules
  for spelling, Markdown structure, diagrams, document types, and Rust API
  comments.

## Architecture and design

- [Theoremc design specification](theoremc-design.md) - Living architecture
  document for theoremc, covering parser, schema, code generation, backend, and
  reporting design.
- [Theorem file format](theorem-file-specification.md) - Normative schema and
  behavioural specification for `.theorem` documents.
- [Name mangling rules](name-mangling-rules.md) - Normative mapping rules for
  theorem action names, generated Kani harness symbols, and external theorem
  identifiers.
- [Theoremc development roadmap](roadmap.md) - Phased implementation roadmap
  with traceable tasks derived from design, schema, mangling, and ADR
  requirements.

## Decision records

- [ADR 001: theorem symbol stability and non-vacuity policy](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
  - Decision record for injective symbol mangling, explicit reference
    semantics, non-vacuity requirements, and theorem ID migration.
- [ADR 002: library-first internationalization and localization with Fluent](adr-002-library-first-internationalization-and-localization-with-fluent.md)
  - Decision record for Fluent-based, library-safe localization via injected
    localizers, structured diagnostic codes, and deterministic fallback policy.
- [ADR 003: architectural boundary enforcement for schema layers](adr-003-architectural-boundary-enforcement.md)
  - Decision record for schema layering constraints, module boundary policy,
    and an incremental architecture-enforcement stack.
- [ADR 004: theorem-side action signatures](adr-004-action-signature-specification.md)
  - Decision record for explicit theorem-side action signatures used by typed
    action probes and future argument shaping.

## Reference material

- [Navigating code complexity: a guide for implementers and maintainers](complexity-antipatterns-and-refactoring-strategies.md)
  - Practical guide to complexity metrics, anti-patterns, and refactoring
    strategies for long-lived codebases.
- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md)
  - Patterns for testable Rust design using dependency injection to avoid
    global-state coupling.
- [A systematic guide to effective, ergonomic, and DRY doctests in Rust](rust-doctest-dry-guide.md)
  - Detailed reference for writing maintainable Rust doctests with minimal
    duplication.
- [Mastering test fixtures in Rust with `rstest`](rust-testing-with-rstest-fixtures.md)
  - Comprehensive reference for fixture design, parametrization, and best
    practices with `rstest`.
- [Scripting standards](scripting-standards.md) - Standards for project scripts,
  covering toolchain choices, structure, and operational expectations.
- [Localizable Rust libraries with Fluent](localizable-rust-libraries-with-fluent.md)
  - Reference material for library-safe localization patterns and Fluent
    integration.

## Execution plans

- [Execution plans](execplans/) - Living implementation plans for roadmap
  tasks.
  - [Code base audit findings from 2026-06-05](execplans/code-base-audit-2026-06-05.md)
    - ExecPlan for addressing canonical action names, typed validation
      diagnostics, schema serde boundaries, macro separation, reusable test
      helpers, and documentation gaps.
  - [Step 1.1: `TheoremDoc` and subordinate schema types](execplans/1-1-1-theorem-doc-and-subordinate-schema-types.md)
    - ExecPlan for strict theorem document deserialization.
  - [Step 1.2.1: Validate required fields and non-empty constraints](execplans/1-2-1-validate-required-fields-and-non-empty-constraints.md)
    - ExecPlan for post-deserialization semantic validation rules.
  - [Step 1.2.2: Parse expression fields as `syn::Expr`](execplans/1-2-2-parse-assume-prove-and-witness-expressions.md)
    - ExecPlan for expression syntax validation and statement block
      rejection.
  - [Step 1.2.3: Enforce `Step` and `LetBinding` shape rules](execplans/1-2-3-enforce-step-and-let-binding.md)
    - ExecPlan for step and let-binding structural validation.
  - [Step 1.2.4: Enforce non-vacuity defaults](execplans/1-2-4-enforce-non-vacuity-defaults.md)
    - ExecPlan for vacuity policy defaults and override validation.
  - [Step 1.3.1: Structured diagnostics for parser failures](execplans/1-3-1-structured-diagnostics-for-parser-failures.md)
    - ExecPlan for source-located diagnostics and parser regression fixtures.
  - [Step 2.1.1: Canonical action-name validation](execplans/2-1-1-canonical-action-name-validation.md)
    - ExecPlan for dot-separated action-name grammar and keyword validation.
  - [Step 2.1.2: Action mangling](execplans/2-1-2-action-mangling.md)
    - ExecPlan for deterministic canonical action-name mangling.
  - [Step 2.1.3: Fail compilation on duplicate action names](execplans/2-1-3-fail-compilation-on-duplicate-action-names.md)
    - ExecPlan for duplicate action-name detection.
  - [Step 2.2.1: Per-file naming using path mangle and hash12](execplans/2-2-1-per-file-naming-using-path-mangle-and-hash12.md)
    - ExecPlan for deterministic per-file module naming.
  - [Step 2.2.2: Deterministic harness naming](execplans/2-2-2-deterministic-harness-naming.md)
    - ExecPlan for stable generated Kani harness identifiers.
  - [Step 2.3.1: Argument decoding for plain YAML strings](execplans/2-3-1-argument-decoding-for-plain-yaml-strings.md)
    - ExecPlan for argument decoding semantics.
  - [Step 2.3.2: Optional literal text wrapper](execplans/2-3-2-optional-literal-text-wrapper.md)
    - ExecPlan for explicit literal wrapper decoding.
  - [Step 2.3.3: Struct literal synthesis from YAML maps](execplans/2-3-3-struct-literal-synthesis-from-yaml-maps.md)
    - ExecPlan for map-backed argument lowering.
  - [Step 3.1.1: Build.rs scanning of theorems](execplans/3-1-1-build-rs-scanning-of-theorems.md)
    - ExecPlan for Cargo build-script theorem discovery.
  - [Step 3.1.2: Generate OUT_DIR theorem suite](execplans/3-1-2-generate-out-dir-theorem-suite-rs.md)
    - ExecPlan for generated theorem suite wiring.
  - [Step 3.2.1: Stable per-file module macro expansions](execplans/3-2-1-stable-per-file-module-macro-expansions.md)
    - ExecPlan for proc-macro per-file module expansion.
  - [Step 3.2.2: Gate generated Kani harnesses](execplans/3-2-2-gate-generated-kani-harnesses.md)
    - ExecPlan for cfg-gated Kani proof harness generation.
  - [Step 3.3.1: Emit typed action probes](execplans/3-3-1-emit-typed-action-probes.md)
    - ExecPlan for typed theorem action probes.
