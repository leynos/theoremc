# Request for Comments (RFC) 0001: semantic law templates over composable primaries

- Status: accepted
- Date: 2026-08-19
- Schema version: 2
- Initial backend: Kani
- Future backend considered: Verus
- Related decisions:
  [ADR 005](../adr-005-vertical-slice-first-roadmap-sequencing.md),
  [ADR 006](../adr-006-backend-neutral-harness-planning-boundary.md), and
  [ADR 007](../adr-007-verus-backend-preservation-invariants.md)
- Originating proposal:
  [GitHub issue 80](https://github.com/leynos/theoremc/issues/80)

## Summary

Add a structured semantic-law layer to `.theorem` files. Authors may continue
to write an explicit Rust assertion, or may instantiate a named law such as
`idempotent`, `round_trip`, `equivalent`, `refines`, `preserves`, or
`homomorphism`.

A law remains a proof obligation inside `Prove`:

```yaml
Schema: 2

Prove:
  - assert: "config.is_valid()"
    because: "the existing assertion form remains available"

  - law:
      idempotent:
        operation:
          action: config.normalize
          bind: { input: config }
          args:
            policy: { ref: policy }
        input: { ref: config }
    because: "normalization reaches a fixed point after one application"
```

Law instances lower into a typed semantic law intermediate representation
(IR), then into the backend-neutral harness plan defined by ADR 006. The first
implementation targets Kani. A later Verus backend may consume the same law
semantics through explicit Verus bindings, subject to ADR 007.

The proposal reuses the existing `Actions` mapping. An action used as a leaf of
a structured law is a **composable primary**. Good primaries are small,
deterministic with respect to their declared inputs, observationally pure in
ordinary use, accept shared-reference semantic inputs, and return owned
semantic values.

Existing conforming `.theorem` documents remain valid without source changes.
Law syntax requires `Schema: 2`. The public Rust schema API changes from
`Vec<Assertion>` to `Vec<ProofObligation>` at an explicit pre-1.0 breaking
boundary.

## Problem

The current `Prove` form contains an opaque Rust boolean expression:

```yaml
Prove:
  - assert: "decode(encode(value)) == value"
    because: "encoding and decoding preserve the value"
```

That escape hatch remains useful, but it has four limitations.

First, repeated semantic shapes remain repeated source code. Round trips,
idempotence, refinement, invariant preservation, and homomorphisms recur across
projects, but every theorem restates their plumbing.

Second, theoremc can validate only the syntax of an opaque assertion before
generated Rust reaches rustc. It cannot distinguish the intended law from a
plausible but subtly different assertion.

Third, opaque Rust expressions weaken the planned multi-backend architecture.
A Kani renderer can embed a Rust expression in `assert!`, but a Verus renderer
cannot generally reinterpret arbitrary executable Rust as a `spec` expression
or proof obligation.

Fourth, raw assertions do not create useful design pressure. A structured law
can require a small operation, relation, invariant, or combiner with a precise
signature. That encourages production code and proof adapters to expose
composable semantic units rather than one large, stateful procedure.

The problem is not a shortage of assertion syntax. It is the absence of a
small semantic vocabulary between human intent and backend mechanics.

## Goals

- Preserve all existing conforming `.theorem` source syntax.
- Add a concise, reviewable vocabulary for common semantic laws.
- Keep explicit assertions as an escape hatch.
- Reuse theorem-side action signatures and argument shaping.
- Encourage small, deterministic, composable proof primaries.
- Validate law shape, action roles, and signature compatibility before backend
  emission.
- Lower laws through a semantic law IR and the shared harness plan.
- Implement Kani lowering without baking Kani syntax into the law schema.
- Preserve a technically honest path to a future Verus backend.
- Preserve source-located, actionable diagnostics.
- Keep generated behaviour explicit enough for counterexample reporting and
  review.

## Non-goals

- Replace arbitrary assertions with a complete theorem language.
- Infer mathematical laws from Rust implementations.
- Prove that a primary is pure merely because its signature looks pure.
- Add inline Rust blocks or backend-specific proof code to `.theorem`.
- Automatically translate arbitrary Rust expressions into Verus `spec fn`
  expressions.
- Implement the Verus backend in this RFC.
- Implement a property-testing backend in this RFC.
- Introduce named theorem imports, lemma dependency graphs, or a package
  registry of user-defined laws.
- Make every useful algebraic law a built-in template.
- Delay the first working Kani vertical slice. ADR 005 places this RFC after
  that slice.

## Terminology

### Law template

A built-in semantic shape with named roles and defined meaning. For example,
`idempotent` means that applying an operation twice is related to applying it
once.

### Law instance

One use of a law template in a theorem, with concrete actions, fixed arguments,
inputs, and a human rationale.

### Composable primary

A theorem action used as a leaf operation, invariant, relation, or combiner in
a law instance. The term **primary** avoids conflict with Rust's language-level
primitive types.

### Bound role

A semantic input supplied by the law template rather than directly by the
author's `args` mapping. Unary primaries bind `input`. Binary primaries bind
`left` and `right`.

### Fixed argument

An ordinary theorem argument supplied through the existing `ArgValue`
mechanism. Fixed arguments may contain literals, explicit `{ ref: ... }`
references, lists, and structured mappings.

### Relation

A predicate used to compare two law results. Equality is the default for most
comparison laws. A custom relation is itself a binary composable primary.

### Opaque assertion

The existing `{ assert, because }` proof obligation. It remains legal, but its
semantics are available only to backends that understand the contained Rust
expression.

## Schema versioning

Structured laws require an explicit schema declaration:

```yaml
Schema: 2
```

The version rules are:

| Source form | Interpretation |
| --- | --- |
| `Schema` omitted | Parse using the version 1 compatibility profile |
| `Schema: 1` | Parse using schema version 1 |
| `Schema: 2` | Parse using schema version 2 |
| Any unsupported value | Emit a source-located unsupported-version diagnostic |

Omission permanently means version 1. An unchanged file must not acquire new
semantics because a newer theoremc happens to read it.

Version 1 permits opaque assertions only. Version 2 permits both opaque
assertions and structured laws. A law inside an unversioned or `Schema: 1`
document produces a targeted diagnostic rather than silently upgrading the
document.

A future Verus binding schema must be additive. ADR 007 permits a later schema
version, such as version 3, or an external binding registry, but it forbids
changing the meaning of a valid schema version 2 law declaration.

## Proof-obligation model

Version 2 changes the public domain model approximately as follows:

```rust,no_run
pub enum ProofObligation {
    Assert(Assertion),
    Law(LawObligation),
}

pub struct LawObligation {
    pub law: Law,
    pub because: String,
}

pub enum Law {
    Idempotent(IdempotentLaw),
    RoundTrip(RoundTripLaw),
    Equivalent(EquivalentLaw),
    Refines(RefinementLaw),
    Preserves(PreservationLaw),
    Homomorphism(HomomorphismLaw),
}
```

The exact module split remains an implementation detail. New law types must not
inflate the existing schema type module. The recommended shape is a dedicated
proof-obligation domain module, a corresponding raw adapter, semantic
validation outside the raw layer, and lowering outside schema deserialization.

`because` remains a sibling of `assert` or `law`. Reports can therefore treat
every `Prove` item uniformly.

## Reuse `Actions` for primaries

This RFC does not add a top-level `Primaries` mapping. Every action referenced
by a law must already have a theorem-owned signature in `Actions`:

```yaml
Actions:
  config.normalize:
    params:
      config: "&crate::config::Config"
      policy: "&crate::config::Policy"
    returns: "crate::config::Config"
```

The action declaration remains the theorem-owned executable contract. The Rust
function remains its executable implementation. ADR 007 makes clear that this
contract does not, by itself, identify a Verus specification or proof.

A law reference extends the set of actions considered referenced for:

- missing-signature validation;
- canonical action-name validation;
- signature conflict detection;
- generated function-pointer probes;
- referenced-type probes;
- law-composition probes;
- report cross-references.

## Primary invocation shape

Every primary invocation contains:

```yaml
action: config.normalize
bind:
  input: config
args:
  policy: { ref: policy }
mode: call
```

The fields mean:

- `action` identifies the canonical theorem action;
- `bind` maps law-defined semantic roles to action parameter names;
- `args` supplies fixed arguments through the existing argument-value syntax;
- `mode` is `call` or `must` and defaults to `call`.

The selected law and primary position define the permitted roles:

| Primary shape | Required roles |
| --- | --- |
| Unary operation, invariant, or mapping | `input` |
| Binary combiner or relation | `left`, `right` |
| Fully specified invocation | none |

The values of `bind` are actual parameter identifiers from the corresponding
`Actions` declaration. A bound parameter must not also appear in `args`.

`mode: must` reuses the existing `must` semantics:

- `Result<T, E>` must be successful and yields `T`;
- `Option<T>` must be present and yields `T`;
- an infallible return value passes through unchanged.

`mode: call` preserves the declared return type and never silently unwraps a
fallible value.

## Composability contract

For the initial law catalogue, dynamically bound parameters must use shared
references. Binary primaries must use shared references for both bound
parameters. Mutable-reference bound parameters are rejected.

Primaries should return owned semantic results. A thin, explicit Rust adapter
may clone or construct an owned model when a production API mutates in place:

```rust,no_run
pub fn apply_update_model(graph: &Graph, update: &Update) -> Graph {
    let mut next = graph.clone();
    next.apply(update);
    next
}
```

That clone is visible, linted, reviewable Rust. theoremc must not insert an
implicit clone merely to make a law compose.

The compiler can enforce parameter roles, shared-reference shape, absence of
mutable bound references, and obvious return-shape constraints. It cannot prove
the absence of hidden I/O, global state, clock reads, randomness, or interior
mutation. Authoring guidance must state that limitation plainly.

Existing stateful workflows remain valid. Authors may continue to use `Let`,
`Do`, and opaque assertions for mutation-heavy scenarios.

## Initial law catalogue

### Idempotence

Meaning:

```text
f(f(x)) R f(x)
```

`R` defaults to equality.

```yaml
- law:
    idempotent:
      operation:
        action: config.normalize
        bind: { input: config }
        args:
          policy: { ref: policy }
      input: { ref: config }
  because: "normalization reaches a fixed point"
```

### Round trip

Meaning:

```text
backward(forward(x)) R x
```

This law is deliberately one directional. A bidirectional codec requires two
law instances.

```yaml
- law:
    round_trip:
      forward:
        action: codec.encode
        bind: { input: value }
        args:
          format: { ref: format }
      backward:
        action: codec.decode
        bind: { input: bytes }
        args:
          format: { ref: format }
        mode: must
      input: { ref: document }
  because: "encoding and decoding preserve every representable document"
```

### Equivalence

Meaning:

```text
left(x) R right(x)
```

`R` defaults to equality.

```yaml
- law:
    equivalent:
      left:
        action: search.reference
        bind: { input: haystack }
        args:
          needle: { ref: needle }
      right:
        action: search.optimized
        bind: { input: haystack }
        args:
          needle: { ref: needle }
      input: { ref: haystack }
  because: "the optimized search preserves reference behaviour"
```

### Refinement

Meaning:

```text
relation(implementation(x), specification(x))
```

Refinement is directional and requires a custom relation.

```yaml
- law:
    refines:
      implementation:
        action: planner.optimized
        bind: { input: request }
        args: {}
      specification:
        action: planner.reference
        bind: { input: request }
        args: {}
      relation:
        action: planner.result_refines
        bind:
          left: implementation
          right: specification
        args: {}
      input: { ref: request }
  because: "the optimized plan satisfies the reference guarantees"
```

### Invariant preservation

Meaning:

```text
invariant(state) implies invariant(transition(state))
```

The transition is a model operation that returns a new state.

```yaml
- law:
    preserves:
      invariant:
        action: graph.is_bidirectional
        bind: { input: graph }
        args: {}
      transition:
        action: graph.apply_update_model
        bind: { input: graph }
        args:
          update: { ref: update }
        mode: must
      state: { ref: graph }
  because: "every accepted graph update preserves bidirectionality"
```

For Kani, lowering records reachability of the precondition, assumes the
precondition, invokes the transition, and asserts the invariant on the result.
The generated reachability check supplements, but does not replace, the
explicit theorem witness requirement.

### Homomorphism

Meaning:

```text
map(combine_input(x, y)) R combine_output(map(x), map(y))
```

`R` defaults to equality.

```yaml
- law:
    homomorphism:
      mapping:
        action: participants.count_free
        bind: { input: participants }
        args: {}
      input_combine:
        action: participants.concat
        bind:
          left: left
          right: right
        args: {}
      output_combine:
        action: arithmetic.add_u64
        bind:
          left: left
          right: right
        args: {}
      left: { ref: xs }
      right: { ref: ys }
  because: "free-participant counts compose over concatenation"
```

## Equality and custom relations

`idempotent`, `round_trip`, `equivalent`, and `homomorphism` use equality by
default. They may supply a custom binary relation:

```yaml
relation:
  action: config.same_meaning
  bind:
    left: actual
    right: expected
  args:
    policy: { ref: policy }
```

A custom relation must bind both roles. Its final unwrapped return type must be
`bool`. `refines` always requires a custom relation.

A custom relation may be weak or even always true. Signature checking cannot
establish semantic adequacy. Equality remains the preferred relation, and
important custom relations should receive independent verification.

## Lowering architecture

The pipeline is:

```text
versioned raw schema
        |
        v
validated domain model
        |
        +-------------------------+
        |                         |
        v                         v
opaque schema 1/2 obligation    semantic Law IR
        |                         |
        +------------+------------+
                     |
                     v
             shared HarnessPlan
                     |
             +-------+--------+
             |                |
             v                v
         Kani renderer     future Verus renderer
```

ADR 006 owns the harness-plan boundary. The plan contains semantic operations,
resolved invocations, stable obligation identities, source provenance, and
explicit opaque-expression nodes. It contains no Kani token streams or Kani
syntax.

A schema version 1 theorem may express an idempotence claim longhand through
`Let` and `Prove`. A schema version 2 `idempotent` law should lower to a
semantically equivalent harness plan. Plan-equivalence fixtures are required
for the first implementation.

Opaque assertions lower to an explicitly backend-constrained node. They do not
become portable merely because they share a container with law obligations.

## Backend capability checks

A backend must reject unsupported obligations before emitting partial output.
The initial capability expectations are:

| Obligation | Kani MVP | Future Verus |
| --- | --- | --- |
| Structured laws in this RFC | required | intended after binding design |
| Opaque Rust assertion | required | not guaranteed |
| Opaque Rust assumption | required | not guaranteed |
| Opaque Rust witness | required | not guaranteed |
| Stateful `Do` sequence | required | requires separate design |
| `must` over `Result` or `Option` | required | requires explicit mapping |

Kani lowers structured laws into proof-harness calls, assumptions, covers, and
assertions. Kani remains bounded by its configured unwind and resource limits.
A template does not change the epistemic status of a bounded model-checking
result.

Verus separates executable, specification, and proof code. An ordinary theorem
action export and function-pointer probe cannot identify the `spec fn` or proof
lemma required by Verus. ADR 007 therefore requires explicit backend bindings
and forbids inference from executable action signatures.

## Non-vacuity policy

Structured laws must not bypass the existing witness policy.

For Kani:

- invariant preservation emits a reachability check for its precondition;
- law-generated assumptions remain visible in the run record;
- explicit theorem witnesses remain required unless vacuity is justified;
- unreachable sub-obligations are reported as unreachable, not successful.

A generated cover for a law precondition does not satisfy the document witness
requirement. The witness records the theorem author's intended nontrivial path;
the generated cover checks an operational precondition.

A future backend may implement reachability differently, but the semantic run
record must preserve the distinction between proved, falsified, unreachable,
undetermined, and unsupported obligations.

## Stable sub-obligation identities

A law is one source assertion but may lower into several operational checks.
Generated sub-obligations receive deterministic identities derived from the
theorem and `Prove` index:

```text
prove[2].law.idempotent.first_application
prove[2].law.idempotent.second_application
prove[2].law.idempotent.relation
```

Reports lead with the source law and its `because` text. Generated code remains
debugging detail rather than the primary interface.

Counterexample reporting should identify:

- source theorem and `Prove` item;
- law kind;
- bound inputs;
- fixed arguments where representable;
- the primary invocation that failed under `must`;
- left and right comparison results where representable;
- reachability and assumption status;
- backend and evidence configuration.

Stable report identifiers must not depend on generated local variable names.

## Validation and diagnostics

Schema and semantic validation must reject at least:

- a law in a version 1 or unversioned document;
- an unknown law kind;
- more than one law kind in one `law` mapping;
- a missing primary action signature;
- an invalid canonical action name;
- a missing or unexpected bound role;
- a role bound to an absent action parameter;
- the same action parameter supplied by both `bind` and `args`;
- a dynamically bound mutable-reference parameter;
- a bound parameter that is not a shared reference;
- a custom relation without both `left` and `right`;
- a relation whose declared unwrapped return is not `bool`;
- an obviously incompatible law composition visible from declarations;
- `mode: must` on a return shape theoremc cannot unwrap;
- a law unsupported by the selected evidence backend;
- an empty `because` value.

Diagnostics retain source spans and precise breadcrumbs, for example:

```text
Prove item 2: law.homomorphism.output_combine.bind.right
```

Theorem-side type strings cannot provide full Rust type equivalence. Type
aliases, associated types, coercions, and generic normalization remain rustc's
job. Theoremc rejects clear structural errors, then emits non-executed
law-composition probes so rustc type-checks the actual composition before a
verifier runs.

## Compatibility and migration

### Existing `.theorem` files

No migration is required for conforming current documents. Omitted `Schema` and
`Schema: 1` both select the version 1 compatibility profile.

Existing files may opt into version 2 by adding `Schema: 2`. Their existing
assertions remain legal.

Documents that declare unsupported future version numbers begin to fail. This
is an intentional tightening of previously undefined syntax.

### Public Rust API

The change from:

```rust,no_run
pub prove: Vec<Assertion>
```

to:

```rust,no_run
pub prove: Vec<ProofObligation>
```

is source-breaking for consumers that construct, destructure, or iterate over
the public schema model as assertions.

Theoremc is pre-1.0. This break lands at an explicitly documented release
boundary, naturally version 0.2.0 if no intervening release changes the
sequence. A compatibility accessor may ease read-only migration, but it must
not conceal law obligations or present assertions as the complete proof list.

### Future Verus bindings

Schema version 2 law declarations remain valid. A future binding layer adds the
executable implementation, specification function, refinement proof,
mathematical view, and optional lemma bindings required by Verus. It must not
reinterpret the existing law syntax.

## Implementation sequence

ADR 005 places the work after the first working Kani vertical slice.

### Stage 1: versioned proof-obligation schema

- Freeze omitted `Schema` as version 1.
- Reject unsupported schema versions.
- Introduce raw and public `ProofObligation` variants.
- Preserve current assertion parsing exactly.
- Add version 1 and version 2 parser fixtures.
- Add source-located diagnostics for law shape and version errors.

### Stage 2: law model and validation

- Add the six law domain types and common primary invocation type.
- Resolve law action references through theorem-side signatures.
- Validate roles, parameters, modes, reference shape, and relations.
- Extend action conflict and referenced-type probes.
- Lower validated law instances into semantic Law IR.

### Stage 3: shared-plan lowering and Kani reuse

- Lower each law into the existing harness plan.
- Reuse the Kani renderer built for the vertical slice.
- Preserve `must`, assumption, assertion, and cover semantics.
- Add positive and negative integration cases for each law.
- Demonstrate a counterexample for every comparison law.
- Demonstrate unreachable-precondition detection for `preserves`.
- Add longhand-versus-law plan-equivalence fixtures.

### Stage 4: authoring guidance and examples

- Add examples for normalization, codecs, reference equivalence, state
  transitions, and collection homomorphisms.
- Document composable-primary design.
- Show explicit adapters from mutation-heavy APIs to model operations.
- Document custom-relation weakness and witness requirements.
- Explain when an opaque assertion is more honest than a forced template.

### Stage 5: Verus binding RFC

Before implementing a Verus renderer:

- define executable-to-specification symbol binding;
- define which bindings are `exec`, `spec`, or `proof`;
- define mathematical views of executable Rust types;
- define preconditions, postconditions, and proof-lemma representation;
- define backend capability diagnostics;
- test at least one unchanged schema version 2 law through Kani and Verus.

ADR 007 supplies the invariants that this follow-up design must preserve.

## Alternatives considered

### Add a top-level `Laws` section

Rejected. It would split proof obligations between `Prove` and `Laws`, create
ordering and reporting questions, and imply that a law is not a proposition to
prove.

### Replace `Prove.assert` with laws

Rejected. The catalogue cannot express every useful theorem. Opaque assertions
remain necessary for unusual predicates, transition histories, and incremental
migration.

### Encode laws as Rust macros inside `assert`

Rejected. This is Kani- and Rust-specific, exposes generated plumbing to
authors, weakens source diagnostics, and leaves a future Verus backend without
structured semantic information.

### Add a generic expression-and-hole template language

Rejected. It would become a second programming language inside YAML and would
recreate the review, typing, and portability problems that theoremc's action
model avoids.

### Add a separate `Primaries` registry

Deferred. `Actions` already supplies canonical names, signatures, probes, and
argument parameter names. Optional metadata may later annotate purity claims,
Verus bindings, or backend capabilities without replacing `Actions`.

### Infer roles from action signatures

Rejected. Parameter position and conventional names do not establish semantic
roles. Explicit `bind` mappings are longer but safer.

### Generate clones for by-value primaries

Rejected. Hidden cloning obscures cost, adds trait requirements, and makes
generated code repair a poorly composable interface.

### Complete RFC 0001 before the Kani vertical slice

Rejected by ADR 005. The existing assertion language must first prove that the
shared execution pipeline works end to end. RFC 0001 then adds a second
frontend into a proven harness-planning and rendering seam.

## Consequences

Positive consequences:

- Common semantic laws become concise and reviewable.
- Theoremc can validate intended law shape before backend emission.
- Law semantics are reusable across backends.
- Existing assertion syntax remains available.
- The initial Kani renderer is reused rather than dismantled.
- A future Verus backend receives explicit semantic structure without false
  promises about automatic executable-to-specification translation.

Costs and trade-offs:

- Schema version 2 and the public proof-obligation enum create a documented
  pre-1.0 API break.
- Authors of composable primaries accept stricter interface guidance.
- The law catalogue becomes long-lived public surface area.
- Custom relations and model adapters may still be semantically weak.
- Backend-neutral law syntax does not make every theorem construct portable.

## Related documents

- [ADR 005: vertical-slice-first roadmap
  sequencing](../adr-005-vertical-slice-first-roadmap-sequencing.md)
- [ADR 006: backend-neutral harness planning
  boundary](../adr-006-backend-neutral-harness-planning-boundary.md)
- [ADR 007: Verus backend preservation
  invariants](../adr-007-verus-backend-preservation-invariants.md)
- [Theorem file specification](../theorem-file-specification.md)
- [Theoremc design specification](../theoremc-design.md)
- [Development roadmap](../roadmap.md)
- [ADR 004: theorem-side action
  signatures](../adr-004-action-signature-specification.md)
