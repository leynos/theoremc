# Architectural Decision Record (ADR) 006: backend-neutral harness planning boundary

- Status: accepted
- Date: 2026-08-19
- Deciders: theoremc maintainers
- Technical story: accommodate RFC 0001 without delaying the Kani vertical slice

## Context

The generated `theorem_file!` expansion already owns stable modules, Kani proof
attributes, action probes, and referenced-type probes. Its harness bodies are
empty. The most direct implementation of roadmap Phase 4 would make the proc
macro walk `TheoremDoc` and emit Kani tokens for every `Forall`, `Assume`,
`Let`, `Do`, `Prove`, and `Witness` item.

That approach would deliver code quickly, but it would bind three concerns
inside one renderer:

- schema version 1 source shape;
- proof semantics;
- Kani syntax and execution mechanics.

RFC 0001 adds a second proof-obligation source shape. A schema version 2 law is
not merely another assertion string. It may expand into several invocations,
comparisons, success obligations, assumptions, and reachability checks.

A future Verus backend introduces a different pressure. Opaque Rust expressions
are not generally portable to Verus, while structured laws may be portable only
when explicit specification and proof bindings exist.

The project therefore needs a seam between validated theorem meaning and
backend rendering. That seam must be small enough not to become an abstract
compiler cathedral before the first vertical slice.

## Goals

- Prevent the Kani renderer from depending directly on schema version 1 types.
- Give RFC 0001 a second lowering path into an already working backend.
- Reuse one invocation model for `call`, `must`, and law primaries.
- Preserve source locations and stable obligation identities.
- Keep opaque Rust expressions explicitly backend-constrained.
- Avoid `proc_macro2::TokenStream`, `quote!`, or Kani syntax in the semantic
  plan.
- Keep the first vertical slice narrow and implementable.

## Non-goals

- Define a universal theorem-prover IR.
- Predict every operation required by Verus or Stateright.
- Introduce a public IR stability commitment.
- Add a new workspace crate before ownership pressure requires one.
- Implement schema version 2 or any semantic law before the vertical slice.
- Add a generic backend trait hierarchy in the first implementation.

## Decision

### 1. Introduce an internal harness plan

Theoremc will introduce an internal, backend-neutral `HarnessPlan` between
validated theorem semantics and generated backend code.

The exact Rust names may evolve, but the conceptual shape is:

```rust,no_run
struct HarnessPlan {
    theorem: TheoremIdentity,
    evidence: EvidencePlan,
    operations: Vec<Operation>,
}

enum Operation {
    Symbolic(SymbolicBinding),
    Assume(PredicateObligation),
    Invoke(ResolvedInvocation),
    Branch(SymbolicBranch),
    Compare(Comparison),
    Assert(PredicateObligation),
    Cover(PredicateObligation),
}
```

The plan records meaning needed by renderers:

- stable theorem and obligation identities;
- source provenance;
- declared types;
- binding order and use;
- resolved action identities and signatures;
- semantically shaped arguments;
- invocation mode;
- comparisons and relations;
- assumptions, assertions, and reachability checks;
- backend evidence requirements;
- explicit opaque-expression nodes.

The plan contains no Kani attributes, Kani macro names, generated Rust local
names, or token streams.

### 2. Keep rendering in the proc-macro crate

The Kani renderer remains in `theoremc-macros`, where `syn`, `quote`,
`proc_macro2`, generated module structure, and Rust token emission already
belong.

The initial harness-plan model and schema version 1 lowering belong in an
internal module owned by `theoremc-core`, because they operate on validated
domain values and do not emit Rust. This remains an internal API.

If the plan or backend set later creates an undesirable dependency boundary,
the project may extract an emitter crate. That extraction is deferred until a
concrete second owner exists.

### 3. Normalize schema version 1 before rendering

The Kani renderer must not consume `Assertion`, `Step`, `LetBinding`, or
`TheoremDoc` directly.

Schema version 1 lowering converts them into normalized operations. An opaque
assertion becomes conceptually:

```rust,no_run
Predicate::OpaqueRust {
    expression: ParsedRustExpression,
    source: SourceSpan,
}
```

The explicit variant matters. It prevents the project from claiming that an
arbitrary Rust expression is backend-neutral.

This normalization does not require the public `TheoremDoc.prove` API break
before the vertical slice. The public schema version 1 model may remain
unchanged until RFC 0001 lands.

### 4. Make resolved invocation the common unit

All action use lowers through one representation:

```rust,no_run
struct ResolvedInvocation {
    action: ResolvedAction,
    arguments: Vec<ResolvedArgument>,
    mode: InvocationMode,
    binding: Option<BindingId>,
    source: SourceSpan,
}

enum InvocationMode {
    Call,
    Must,
}
```

This representation serves:

- `Let.call`;
- `Let.must`;
- `Do.call`;
- `Do.must`;
- nested steps inside `maybe`;
- every RFC 0001 composable primary;
- custom relations used by semantic laws.

Schema-specific role binding and fixed arguments are resolved before backend
rendering. The renderer does not need to understand `bind`, YAML maps, or law
kinds.

`must` remains a semantic invocation policy, not a special Kani rendering case
attached to `Step::Must`.

### 5. Promote argument lowering into the owned rendering path

The existing test-gated argument-lowering prototype provides useful behaviour
for literals, references, lists, and struct literals. The vertical-slice work
will promote or adapt it into the proc-macro rendering path.

The semantic plan retains decoded argument values, expected parameter types,
and borrowing intent. The Kani renderer converts those values into Rust
expressions. Token streams do not leak back into the plan.

The renderer may adapt argument passing according to declared parameter type:

- `&mut T` receives `&mut binding`;
- `&T` receives `&binding`;
- `T` receives the value by move.

No renderer may insert implicit cloning, default construction, or semantic
adapters.

### 6. Give every generated check a stable obligation identity

The plan assigns deterministic structural identities before backend rendering.
Schema version 1 examples include:

```text
assume[0]
let[1].must
prove[0]
witness[0]
```

RFC 0001 may add children such as:

```text
prove[2].law.idempotent.first_application
prove[2].law.idempotent.second_application
prove[2].law.idempotent.relation
```

Backend-generated local names must not define report identity. Renderers may
include obligation identities in assertion messages or backend metadata, but
the semantic identity exists first.

### 7. Lower laws through semantic Law IR into the same plan

RFC 0001 adds this path:

```text
schema version 2 law
    -> validated law domain model
    -> semantic Law IR
    -> HarnessPlan
    -> Kani renderer
```

The Law IR retains the law kind, role bindings, relation choice, source
rationale, and primary invocations. It does not contain Kani syntax.

The schema version 1 path remains:

```text
schema version 1 theorem
    -> normalized version 1 lowering
    -> HarnessPlan
    -> Kani renderer
```

The Kani renderer is therefore completed once for the vertical slice and
reused by RFC 0001.

### 8. Require longhand-versus-law equivalence fixtures

The first law implementation must pair at least one schema version 1 theorem
written out longhand with the equivalent schema version 2 law.

For example, version 1 may invoke normalization twice and assert equality. The
version 2 theorem may instantiate `idempotent`. Tests must show equivalent
semantic plans after normalization, apart from source-form metadata and
law-specific parent obligation identity.

This proves that RFC 0001 compresses and validates existing semantics rather
than creating a separate proof engine.

### 9. Check backend capability before partial emission

A backend renderer must validate that it supports every operation in the plan
before emitting output. It must not emit a partial harness and fail midway.

For the first Kani slice, unsupported operations produce deterministic
compile-time diagnostics. For a future Verus backend, opaque Rust predicates or
stateful sequences may be rejected before code generation.

Capability failure is not proof failure. The run and report model must preserve
that distinction.

### 10. Avoid a premature backend trait hierarchy

The first implementation may use ordinary functions or modules:

```rust,no_run
fn lower_v1(doc: &TheoremDoc) -> Result<HarnessPlan, LoweringError>;
fn validate_kani_capabilities(plan: &HarnessPlan) -> Result<(), CapabilityError>;
fn render_kani(plan: &HarnessPlan) -> Result<TokenStream, RenderError>;
```

A common backend trait becomes justified when a second renderer demonstrates
shared lifecycle behaviour. Until then, a trait would freeze guessed methods
and error boundaries.

## Minimum plan repertoire for the vertical slice

The first plan implementation needs only:

- typed symbolic binding;
- opaque Rust assumption;
- resolved infallible invocation;
- result binding;
- opaque Rust assertion;
- opaque Rust cover;
- stable obligation identity;
- Kani unwind evidence.

`must`, `maybe`, structured comparison, custom relation, law parent-child
records, and backend-generic run evidence may land incrementally after that
slice. Their eventual shape must follow this ADR's boundaries.

## Consequences

Positive consequences:

- The vertical slice remains small.
- Kani rendering is reusable by RFC 0001.
- Schema evolution no longer forces backend rewrites.
- Opaque Rust constructs remain honestly backend-constrained.
- Stable identities and provenance exist before reporting.
- Verus can reject unsupported constructs without parsing Kani-flavoured IR.
- Invocation semantics have one implementation path.

Costs and trade-offs:

- The first slice adds a small middle layer before emitting tokens.
- Some concepts appear in both source-domain and normalized-plan types.
- Internal IR types require focused tests and documentation.
- The plan will evolve as real backend requirements emerge.
- The project must resist exposing the internal plan as a stable public API too
  early.

## Alternatives considered

### Render Kani directly from `TheoremDoc`

Rejected. It couples schema version 1, proof semantics, and Kani syntax and
makes RFC 0001 retrofit a new source form through an assertion-only renderer.

### Implement the full semantic Law IR first

Rejected. The vertical slice does not need six laws or their validation. The
small harness plan can be proven through schema version 1 first.

### Introduce a universal multi-backend compiler IR

Rejected. The project lacks enough backend evidence to design one responsibly.
The harness plan covers the common operational seam now known to exist.

### Move all lowering into `theoremc-macros`

Rejected. Semantic normalization and action-resolution policy should remain
testable without procedural-macro token emission. Only Rust rendering belongs
in the macro crate.

### Make `ProofObligation` public before the slice

Deferred. Doing so would absorb some RFC 0001 API shock, but it would touch
already working schema code on the critical path. Internal normalization gives
most of the architectural protection without requiring that public break.

### Add a backend trait immediately

Rejected. One backend does not provide evidence for the right trait surface.
Capabilities and rendering functions remain explicit until Verus work begins.

## Related documents

- [ADR 005: vertical-slice-first roadmap
  sequencing](adr-005-vertical-slice-first-roadmap-sequencing.md)
- [ADR 007: Verus backend preservation
  invariants](adr-007-verus-backend-preservation-invariants.md)
- [RFC 0001: semantic law templates](rfcs/0001-semantic-law-templates.md)
- [ADR 004: theorem-side action
  signatures](adr-004-action-signature-specification.md)
- [Theoremc design specification](theoremc-design.md)
- [Development roadmap](roadmap.md)
