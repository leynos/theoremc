# Architectural Decision Record (ADR) 007: Verus backend preservation invariants

- Status: accepted
- Date: 2026-08-19
- Deciders: theoremc maintainers
- Technical story: prevent Kani and RFC 0001 decisions from blocking Verus

## Context

Theoremc's first backend is Kani. Kani verifies ordinary executable Rust proof
harnesses under bounded symbolic execution. It can invoke normal Rust actions,
use `kani::any`, assume predicates, cover reachable states, and check Rust
assertions.

Verus has a materially different semantic model. It distinguishes executable
(`exec`), specification (`spec`), and proof (`proof`) code, with restrictions on
which modes may call which functions. An executable Rust action and its
function-pointer signature do not automatically identify:

- the mathematical view of its input and output types;
- the corresponding specification function;
- the refinement argument connecting implementation to specification;
- required preconditions and postconditions;
- reusable proof lemmas.

RFC 0001 improves the situation by making common laws explicit. A structured
`idempotent` or `round_trip` obligation carries more semantic information than
an opaque Rust expression. It still does not prove that an arbitrary executable
primary can be called from Verus specification or proof code.

The project needs invariants that constrain current Kani work and schema version
2 law design without forcing premature Verus implementation. These invariants
should ensure that adopting Verus after theoremc 0.2.0 requires additive
bindings and backend work rather than reinterpretation of existing laws.

## Goals

- Keep schema version 2 law meaning stable across backends.
- Separate semantic primary identity from backend symbols.
- Preserve Verus mode distinctions and mathematical views.
- Keep opaque Rust constructs explicitly nonportable.
- Make backend capability failure explicit and atomic.
- Preserve stable obligation identity, provenance, and non-vacuity semantics.
- Distinguish bounded Kani evidence from deductive Verus evidence.
- Permit additive Verus bindings after theoremc 0.2.0.

## Non-goals

- Choose the final Verus binding syntax.
- Implement a Verus runner or renderer.
- Claim that all stateful version 1 workflows will become Verus-compatible.
- Translate arbitrary Rust expressions into Verus specifications.
- Require Verus-specific fields in schema version 2 law declarations.
- Standardize a language-neutral proof IR.

## Decision

Future work must preserve the following invariants.

### Invariant 1: semantic laws have backend-independent meaning

A schema version 2 law defines a semantic proposition. For example,
`idempotent` means that applying the selected operation twice is related to
applying it once.

Adding Verus must not change that proposition or require authors to rewrite a
valid law into a Verus-specific law form. Backend-specific configuration may
select evidence, bindings, bounds, or proof support, but it must not redefine
the law.

### Invariant 2: semantic primary identity is separate from backend binding

The canonical action name identifies the semantic primary in theorem source and
Law IR. It is not itself a complete backend binding.

Backend bindings map that identity to backend artefacts. A future Verus binding
may include:

```text
semantic primary
    -> executable implementation
    -> specification function
    -> implementation-to-specification refinement proof
    -> mathematical input/output views
    -> optional supporting lemmas
```

Kani may need only the executable implementation. Verus may need all or part of
the richer tuple. The absence of a Verus binding produces an unsupported
capability diagnostic, not guessed behaviour.

### Invariant 3: theorem-side action signatures remain executable contracts

ADR 004 `Actions` declarations specify the executable Rust action contract used
for rustc probes and Kani invocation shaping.

They do not imply:

- that the action is pure;
- that its type has a mathematical view;
- that it may be called from Verus `spec` or `proof` code;
- that its executable body is its specification;
- that equality of executable outputs is the intended semantic relation.

Verus support must add explicit bindings or metadata rather than extending the
meaning of an existing action signature implicitly.

### Invariant 4: Verus mode distinctions remain explicit

Theoremc must preserve the distinction between `exec`, `spec`, and `proof`
artefacts. A backend adapter may not silently treat an executable function as a
specification function or call across Verus mode boundaries that Verus forbids.

Binding validation must identify the expected mode of each symbol and reject
incompatible compositions before emission.

### Invariant 5: mathematical views are explicit and additive

Executable Rust types often require mathematical views for deductive proof.
Examples include sequences, maps, graphs, indices, bounded integers, and
mutation-heavy domain structures.

A future Verus design must represent those views explicitly. It may use:

- additive schema version 3 metadata;
- a separate backend binding registry;
- Rust traits or generated adapters;
- another accepted binding mechanism.

Whichever mechanism is chosen, schema version 2 law documents remain valid and
retain their original meaning. The binding layer supplies the view; it does not
reinterpret the theorem source.

### Invariant 6: opaque Rust expressions remain backend-constrained

`Assume.expr`, opaque `Prove.assert`, and `Witness.cover` contain Rust
expressions. Theoremc must label their harness-plan nodes as opaque Rust.

A Verus backend may support a documented subset through explicit parsing or
binding, but it must not claim general portability. Unsupported expressions
fail capability checking before any partial Verus output is emitted.

Authors who require cross-backend portability should prefer structured laws and
explicit semantic primaries.

### Invariant 7: semantic IR and harness plans contain no Kani syntax

Law IR and the shared harness plan may contain semantic operations such as:

- acquire a symbolic value;
- invoke a resolved primary;
- require successful `Result` or present `Option`;
- assume a predicate;
- compare values through equality or a relation;
- assert a proposition;
- record reachability;
- preserve source and obligation identity.

They must not contain:

- `kani::any` tokens;
- `kani::assume` or `kani::cover` macro syntax;
- Kani attribute tokens;
- generated Rust local variable names as semantic identity;
- Kani-specific status enums as the universal result model.

Kani rendering remains a projection from the plan.

### Invariant 8: capability checking is complete before emission

A backend must inspect the complete theorem plan and reject unsupported
operations before generating partial output.

Capability errors are distinct from:

- schema errors;
- proof falsification;
- unreachable obligations;
- verifier resource exhaustion;
- backend execution failure.

This distinction must survive into machine-readable run records.

### Invariant 9: obligation identity is stable across renderers

Theorem, law, and generated sub-obligation identities originate before backend
rendering. They must not depend on:

- generated local names;
- Kani harness line numbers;
- Verus proof function names;
- backend output ordering;
- translated English rationale text.

The same unchanged schema version 2 law verified by Kani and Verus should report
the same semantic obligation identity, with backend-specific evidence attached.

### Invariant 10: evidence is backend-indexed and trust-aware

A successful Kani run and a successful Verus proof do not carry identical
epistemic meaning.

Run records must identify at least:

- backend and backend version;
- theorem and obligation identity;
- evidence configuration;
- bounds or unwind limits where applicable;
- assumptions and reachability status;
- success, falsification, unreachable, undetermined, or unsupported status;
- trusted or admitted components where the backend exposes them;
- generated artefact references where available.

User-facing reports must not flatten both backends into an unqualified
"proved" label without preserving this evidence context.

### Invariant 11: non-vacuity is semantic, mechanisms are backend-specific

The theorem author's witness intent and vacuity policy remain part of theorem
semantics. Kani may implement reachability through `cover`. Verus may require a
constructive witness, a satisfiability lemma, an executable witness function,
or another accepted mechanism.

The shared model must preserve:

- the source witness identity and rationale;
- whether the relevant state is reachable or constructible;
- whether vacuity was explicitly allowed;
- the backend mechanism used as evidence.

A backend-specific generated reachability check does not silently replace an
explicit source witness.

### Invariant 12: no hidden semantic adapters

Theoremc may not insert implicit clones, defaults, mathematical views,
coercions, or model transitions merely to make a law compile under a backend.

Adapters belong in explicit, linted, reviewable Rust or Verus code and receive a
stable binding identity. Where an adapter models a production operation,
reports should distinguish proof about the model from evidence that the model
matches production behaviour.

### Invariant 13: backend-specific proof support is additive

A future Verus binding may add:

- specification symbols;
- executable/specification correspondence proofs;
- preconditions and postconditions;
- mathematical views;
- proof lemmas;
- trusted assumptions;
- backend-specific evidence policy.

These additions must not make existing Kani bindings invalid unless the source
contract itself was unsound or ambiguous. Backend support is indexed by backend
rather than stored in one mutually exclusive action mode.

### Invariant 14: reporting models obligations, not Kani output

The canonical run model should contain generic theorem and obligation records
with backend-indexed evidence. Kani JSON, Verus diagnostics, JUnit XML, and
human reports are adapters around that model.

The model must support one source law with several generated sub-obligations and
must preserve parent-child relationships.

### Invariant 15: schema evolution is explicit

The intended release sequence is:

- theoremc 0.2: schema version 2, semantic Law IR, shared harness plan, and Kani
  law lowering;
- theoremc 0.3: additive Verus bindings, capability validation, renderer, and
  runner;
- theoremc 0.4: richer lemmas, state-transition support, and structured witness
  forms where experience justifies them.

Version numbers remain planning guidance rather than a promise. The invariant
is that Verus support does not silently alter schema version 2 semantics.

If a future requirement genuinely cannot be represented additively, theoremc
must introduce an explicit new schema version and migration guidance. It must
not reinterpret an old document under a new backend.

## What may break after theoremc 0.2

The following changes may require an ordinary pre-1.0 Rust API break:

- public types for backend binding registries;
- public run-record types while they are still stabilizing;
- renderer or runner extension APIs;
- internal Law IR or harness-plan modules if they were exposed experimentally.

The following should not need to break:

- the meaning of schema version 2 law declarations;
- canonical semantic primary identity;
- stable source obligation IDs;
- the distinction between structured and opaque obligations;
- existing Kani backend bindings;
- theorem-side executable action signatures.

This is the principal architectural promise of this ADR.

## Validation obligations for future Verus work

A Verus backend proposal must demonstrate at least:

1. one unchanged schema version 2 law verified by both Kani and Verus;
2. explicit mapping from semantic primary to executable and specification
   symbols;
3. an implementation-to-specification correspondence obligation;
4. capability rejection for an opaque Rust assertion;
5. stable obligation identity across both backend reports;
6. distinct evidence metadata for bounded Kani and deductive Verus outcomes;
7. non-vacuity handling appropriate to both backends;
8. no Kani tokens or Kani status types in shared semantic modules.

A prototype that verifies only a separately rewritten Verus theorem does not
satisfy the cross-backend adoption goal.

## Consequences

Positive consequences:

- Schema version 2 laws retain durable meaning.
- Kani implementation choices cannot quietly become universal semantics.
- Verus receives the explicit bindings its mode system requires.
- Reports can compare backend evidence without conflating it.
- Unsupported cross-backend cases fail honestly.
- The project can add Verus after 0.2 without a forced law-language rewrite.

Costs and trade-offs:

- Verus adoption requires a real binding design rather than automatic magic.
- Some version 1 stateful theorems may remain Kani-only.
- Backend-indexed run records are richer than a single pass/fail enum.
- Authors may need model adapters and correspondence proofs.
- Mathematical views and trusted assumptions create additional review surface.

## Alternatives considered

### Treat executable Rust actions as Verus specifications

Rejected. Verus mode separation and mathematical views make this technically
false for general Rust code.

### Put Verus symbols directly in schema version 2 laws

Rejected. It would make the law syntax backend-specific and force authors to
rewrite otherwise portable semantic claims.

### Translate opaque Rust expressions automatically

Rejected. General translation is not reliable, and partial translation would
create a dangerous illusion of proof portability.

### Build a Verus-only theorem language

Rejected as the primary path. Backend-specific escape hatches may be necessary,
but structured laws should preserve one semantic source where both backends can
support it.

### Flatten all successful evidence into `SUCCESS`

Rejected. Bounded model checking and deductive verification have different
bounds, assumptions, failure modes, and trust surfaces.

### Delay all Verus thinking until implementation starts

Rejected. A small set of invariants now prevents Kani-specific choices from
hardening into expensive compatibility constraints, without putting Verus work
on the vertical-slice path.

## Related documents

- [RFC 0001: semantic law templates](rfcs/0001-semantic-law-templates.md)
- [ADR 005: vertical-slice-first roadmap
  sequencing](adr-005-vertical-slice-first-roadmap-sequencing.md)
- [ADR 006: backend-neutral harness planning
  boundary](adr-006-backend-neutral-harness-planning-boundary.md)
- [ADR 004: theorem-side action
  signatures](adr-004-action-signature-specification.md)
- [Theoremc design specification](theoremc-design.md)
- [Development roadmap](roadmap.md)
