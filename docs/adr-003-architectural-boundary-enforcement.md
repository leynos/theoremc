# Architectural Decision Record (ADR) 003: architectural boundary enforcement for schema layers

- Status: proposed
- Date: 2026-02-23
- Deciders: theoremc maintainers
- Technical story: enforce layered schema boundaries and anti-corruption
  constraints

## Context

Theoremc's schema subsystem follows a layered design with hexagonal influences:

- public domain API types (`TheoremDoc`, related schema types, and newtypes),
- loader orchestration,
- semantic validation,
- a raw anti-corruption adapter between YAML/serde and domain types,
- diagnostics as a shared cross-cutting concern.

This shape improves readability and change safety, but only if layer
dependencies remain unidirectional. In a single-crate codebase, accidental
`use` edges and broad re-exports can silently erode these boundaries.

The project needs enforceable, automated architectural checks that are strong
enough to catch drift while remaining pragmatic for local development and
continuous integration (CI).

## Decision

### 1. Adopt an explicit schema layer contract

Schema modules must follow this dependency direction:

```plaintext
domain API/types
    ↑
raw adapter (YAML/span bridge)
    ↑
validator (domain rules)
    ↑
loader (orchestration)

diagnostics are shared across layers and stay infrastructure-only
```

Rules:

- Domain API/types must not import from raw, validator, or loader modules.
- The raw adapter is the only layer that may depend on YAML deserialization
  implementation details.
- Validator modules may depend on domain types, raw adapter types, and
  diagnostics, but not on loader orchestration logic.
- Loader modules may orchestrate raw adapter and validator modules, but must
  not become the place where domain invariants are implemented.

### 2. Use module visibility as the first enforcement layer

The crate's public API surface remains curated through `schema::mod` re-exports
and explicit `pub(crate)` boundaries:

- raw adapter and diagnostics internals are not publicly re-exported,
- only stable domain and loader entry points are exposed to consumers,
- compile-fail contract tests/doc tests are used to guard non-public internals.

### 3. Enforce baseline architecture checks in CI

The project adopts three high-priority baseline checks:

- Rust visibility boundaries (via module visibility and re-export discipline),
- `cargo modules graph --acyclic --lib` for acyclicity,
- Clippy gate with `-D clippy::wildcard_imports` to keep dependencies explicit.

### 4. Add targeted custom linting for layer-edge violations

The project introduces a `theoremc_arch_lint` Dylint library for
medium-priority rules that Rust visibility cannot express directly, for example:

- `schema::types` importing `schema::raw`,
- validator modules importing loader-only orchestration logic.

### 5. Add optional dependency-policy enforcement

The project adds a low-priority `cargo-deny` policy for architectural
dependency constraints, starting with architecture-sensitive crates such as
YAML adapters, and extending as backend surface area grows.

## Consequences

Positive consequences:

- Architectural drift is detected automatically instead of by code review only.
- Public API boundaries stay clear and intentional.
- The anti-corruption boundary remains explicit as the parser stack evolves.
- Teams can adopt enforcement incrementally without an immediate crate split.

Costs and trade-offs:

- CI setup becomes broader (`cargo-modules`, Dylint, and optional `cargo-deny`).
- Custom lints require maintenance as module paths evolve.
- Some checks (forbidden import edges) need explicit fixture crates/tests to
  prevent regressions.

## Alternatives considered

### Rely on conventions and code review only

Rejected because boundary erosion is subtle in single-crate module graphs and
is often discovered late.

### Split schema layers into separate crates immediately

Rejected for now. This is stronger isolation but introduces packaging and build
overhead before the current architecture settles.

### Use only Dylint and skip baseline checks

Rejected. Module visibility and acyclicity checks provide simpler, lower-cost
protection and should run even when custom lint infrastructure is unavailable.

## Related documents

- `docs/theoremc-design.md`
- `docs/roadmap.md`
- `docs/theorem-file-specification.md`
