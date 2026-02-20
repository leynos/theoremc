# [Documentation contents](contents.md)

- [Documentation contents](contents.md) - Index of all documentation files in
  this directory.
- [Architecture Decision Record (ADR) 001: theorem symbol stability and non-vacuity policy](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
  - Decision record for injective symbol mangling, explicit reference
  semantics, non-vacuity requirements, and theorem ID migration.
- [Architecture Decision Record (ADR) 002: library-first internationalization and localization with Fluent](adr-002-library-first-internationalization-and-localization-with-fluent.md)
  - Decision record for Fluent-based, library-safe localization via injected
  localizers, structured diagnostic codes, and deterministic fallback policy.
- [Navigating code complexity: a guide for implementers and maintainers](complexity-antipatterns-and-refactoring-strategies.md)
  - Practical guide to complexity metrics, anti-patterns, and refactoring
  strategies for long-lived codebases.
- [Documentation style guide](documentation-style-guide.md) - Authoring rules
  for spelling, formatting, Markdown structure, diagrams, and documentation
  consistency.
- [Name mangling rules](name-mangling-rules.md) - Normative mapping rules for
  theorem action names, generated Kani harness symbols, and external theorem
  identifiers.
- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md)
  - Patterns for testable Rust design using dependency injection to avoid
  global-state coupling.
- [A systematic guide to effective, ergonomic, and "don't repeat yourself" (DRY) doctests in Rust](rust-doctest-dry-guide.md)
  - Detailed reference for writing maintainable Rust doctests with minimal
  duplication.
- [Mastering test fixtures in Rust with `rstest`](rust-testing-with-rstest-fixtures.md)
  - Comprehensive reference for fixture design, parametrization, and best
  practices with `rstest`.
- [Scripting standards](scripting-standards.md) - Standards for project
  scripts, covering toolchain choices, structure, and operational expectations.
- [Theorem file format](theorem-file-specification.md) - Schema and behavioural
  specification for `.theorem` documents and their compile-time semantics.
- [Theoremc development roadmap](roadmap.md) - Phased
  implementation roadmap with traceable tasks derived from design, schema,
  mangling, and ADR requirements.
- [Theoremc design specification](theoremc-design.md) - End-to-end architecture
  and design rationale for theoremc, including parser, code generation,
  backend, and reporting design.
- [User's guide](users-guide.md) - Guide for library consumers covering
  schema types, loading API, identifier rules, and value forms.
- [Execution plans](execplans/) - Implementation plans for roadmap steps.
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
