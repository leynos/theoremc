# Architectural Decision Record (ADR) 001: theorem symbol stability and non-vacuity policy

- Status: accepted
- Date: 2026-02-08
- Deciders: theoremc maintainers
- Technical story: theorem action binding, harness naming, and vacuity safety

## Context

The theoremc design establishes compile-time correlation between `.theorem`
documents, Rust action exports, and generated Kani harnesses. During design
review, several structural risks were identified:

- Action-name mangling based only on joining segments with `__` is not
  injective when segments may contain underscores.
- Harness names based only on snake-casing theorem identifiers are not
  injective across mixed naming styles.
- Implicit interpretation of plain YAML strings as either literals or variable
  references can change theorem semantics when new bindings are introduced.
- Vacuity prevention was optional, which allows green-but-meaningless proof
  results to pass default workflows.
- External theorem identities were not explicitly separated from generated Rust
  symbol names, making path/name migrations operationally brittle.

The system requires deterministic compile-time behaviour and stable operational
identities for reporting and CI.

## Decision

### 1. Injective action mangling

Action resolution remains anchored to `crate::theorem_actions`, but generated
Rust identifiers now use:

- escaped slug (`_` in segments is escaped as `_u`)
- plus a deterministic hash suffix derived from the canonical action name

Format:

```plaintext
{action_slug(canonical_name)}__h{hash12(canonical_name)}
```

Build-time generation must fail on duplicate canonical names or duplicate
mangled names.

### 2. Injective harness naming

Harness functions retain human-readable slugs but are made unique by hashing
`{P}#{T}` (`P` is theorem file path, `T` is theorem ID):

```plaintext
theorem__{theorem_slug(T)}__h{hash12(P#T)}
```

The fully qualified name includes per-file module hashing with `hash12(P)`.
Build-time generation must fail on duplicate theorem keys.

### 3. Explicit reference semantics in YAML args

Plain YAML strings are always string literals.

Variable references must be explicit:

```yaml
{ ref: binding_name }
```

Literal wrappers remain supported:

```yaml
{ literal: "text" }
```

This removes ambiguity and prevents semantic drift caused by newly introduced
bindings.

### 4. Non-vacuity witness policy

`Witness` checks are required by default, and each check compiles to
`kani::cover!`.

Vacuity is only permitted when explicitly acknowledged:

- `Evidence.kani.allow_vacuous: true`
- `Evidence.kani.vacuity_because: <non-empty rationale>`

Default policy remains that `UNREACHABLE` and `UNDETERMINED` are failures
unless explicitly expected and justified.

### 5. Stable external theorem identifiers and migration

Operational identity is separated from Rust symbols.

Canonical external theorem ID format:

```plaintext
{normalized_path(P)}#{T}
```

Renames and moves must be mapped via `theorems/theorem-id-aliases.yaml`, with
acyclic aliases and deterministic resolution to a single canonical ID.

## Consequences

Positive consequences:

- Compile-time bindings are robust against identifier-collision edge cases.
- Harness selection remains human-readable while becoming deterministic.
- Reporting and CI identities are stable and migration-aware.
- Vacuity risks are surfaced as default failures rather than optional quality
  work.

Costs and trade-offs:

- Generated Rust symbols are longer and less aesthetically minimal.
- Theorem authors must use explicit `{ ref: ... }` wrappers for bindings.
- Alias-map governance is required for theorem moves and renames.

## Alternatives considered

### Keep delimiter-only mangling

Rejected because delimiter-only schemes are not injective with underscore-rich
segments and mixed naming styles.

### Keep optional witness policy

Rejected because optional non-vacuity checks are commonly omitted, reducing
trust in green CI signals.

### Use path-only or theorem-only external IDs

Rejected because either axis alone is insufficient for long-term stability and
rename clarity.

## Related documents

- `docs/name-mangling-rules.md`
- `docs/theorem-file-specification.md`
- `docs/theoremc-design.md`
