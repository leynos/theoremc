# Architectural Decision Record (ADR) 005: vertical-slice-first roadmap sequencing

- Status: accepted
- Date: 2026-08-19
- Deciders: theoremc maintainers
- Technical story: move from generated Kani stubs to a working proof path

## Context

Theoremc has completed the first three roadmap phases. It can discover theorem
files, validate their schema, generate stable per-file modules and Kani harness
names, attach Kani proof metadata, and emit compile-time action and type probes.
The generated proof functions still have empty bodies.

The current roadmap completes the remaining work horizontally:

1. emit all major schema sections;
2. implement every `call`, `must`, and `maybe` case;
3. implement evidence-driven result policy;
4. build reporting, stable identifiers, and playback;
5. add end-to-end examples later.

That order delays the first answer to theoremc's central product question:

> Can an engineer place a readable `.theorem` file beside real Rust code and
> have Kani prove or falsify its claim through the normal build integration?

RFC 0001 also proposes schema version 2, structured semantic laws, a public
proof-obligation API change, semantic validation, and law lowering. Those
features are valuable, but placing them before the first working proof path
would increase the distance to a vertical slice and make it harder to tell
whether failures belong to the basic verifier pipeline or the law layer.

A vertical slice must therefore come first, but it must not hard-code an
assertion-only, Kani-specific architecture that RFC 0001 or a future Verus
backend would immediately have to dismantle.

## Goals

- Produce the shortest credible route from a schema version 1 theorem to a real
  Kani result.
- Exercise every architectural seam that later semantic laws need.
- Prove both success and failure, not merely harness discovery.
- Preserve non-vacuity guarantees in the first slice.
- Avoid pulling reporting, enforcement, localization, or the full law catalogue
  onto the critical path.
- Leave unsupported constructs fail-closed until their semantics are complete.

## Non-goals

- Complete every Phase 4 construct before proving one useful theorem.
- Implement RFC 0001 before the existing language works end to end.
- Build rich Markdown, HTML, JUnit, or Cucumber reports for the first slice.
- Implement stable theorem ID aliases or concrete playback for the first slice.
- Add a generic multi-backend framework before a second backend exists.
- Change the settled `.theorem` version 1 source syntax.

## Decision

### 1. Make the working vertical slice the next release gate

The next implementation milestone is not "all Kani semantics". It is one
complete, representative route through the system:

```text
.theorem source
    -> version 1 schema validation
    -> normalized harness plan
    -> generated Kani Rust
    -> cargo kani execution
    -> proved or falsified result
```

The slice must include:

- at least one typed `Forall` binding;
- at least one `Assume` clause where useful;
- ordered `Let` evaluation;
- an infallible action invocation with a bound return value;
- a `Prove` assertion with its rationale;
- an explicit `Witness` cover;
- a passing theorem;
- a deliberately false theorem that produces a counterexample.

A small idempotence theorem expressed longhand is the preferred example. It
exercises the same composable-primary shape that RFC 0001 will later compress
into an `idempotent` law, without requiring schema version 2.

### 2. Recast roadmap Step 4.1 around the shared harness plan

Roadmap Step 4.1 becomes "deliver the Kani vertical slice through the shared
harness-planning boundary". It includes:

- introduce the internal, backend-neutral harness plan from ADR 006;
- lower the supported schema version 1 subset into that plan;
- promote the existing argument-lowering prototype into the owned macro
  rendering path;
- render symbolic bindings, opaque assumptions, invocations, assertions, and
  covers into Kani Rust;
- add positive and negative fixture crates that run Kani;
- reject every accepted source construct that lacks implemented lowering.

The first slice may support only infallible `call` invocations. It must reject
fallible calls, `must`, or `maybe` if their semantics have not landed.

### 3. Keep semantic breadth in Step 4.2

After the slice works, Step 4.2 expands operational semantics in small,
independently testable increments:

1. value-returning and unit-returning `call`;
2. bound fallible `call` where the result remains explicit;
3. `must` over `Result`;
4. `must` over `Option`;
5. infallible documentary `must`;
6. symbolic `maybe` with nested steps.

Every increment lowers through the same resolved-invocation representation.
No construct receives a second, direct Kani renderer.

### 4. Split execution policy from rich reporting

Roadmap Step 4.3 becomes a minimal Kani runner and evidence-policy milestone.
It may invoke Kani, associate outcomes with stable obligation identities, and
compare actual status with `Evidence.kani.expect`.

It does not need to render Markdown, HTML, JUnit XML, Cucumber JSON, or concrete
playback artefacts. Those remain in Phase 5.

The minimal runner must distinguish:

- proved;
- falsified;
- unreachable;
- undetermined;
- unsupported;
- execution error.

`UNREACHABLE` and `UNDETERMINED` continue to fail by default unless explicitly
expected and justified.

### 5. Place RFC 0001 after the vertical slice and before rich reporting

Add a new roadmap Step 4.4 for RFC 0001:

1. freeze omitted `Schema` as the version 1 compatibility profile;
2. add schema version 2 and `ProofObligation`;
3. add the semantic law domain model and validation;
4. lower laws into semantic Law IR;
5. lower Law IR into the existing harness plan;
6. reuse the existing Kani renderer and minimal runner;
7. prove equivalence between longhand version 1 examples and corresponding
   version 2 laws.

This order makes RFC 0001 a new frontend into a proven execution seam rather
than a simultaneous redesign of the frontend, middle end, backend, and runner.

### 6. Pull one example forward and leave broad examples later

The small vertical-slice example moves from the late examples phase into Step
4.1. It should use a pure, shared-reference primary with an owned result, such
as normalization of a bounded integer or compact configuration value.

The richer `account` and HNSW examples remain later. They should validate
stateful workflows, realistic data shaping, reporting, and developer guidance
once the minimal execution model has stabilized.

### 7. Keep noncritical work off the vertical-slice path

The following work remains deferred unless it directly blocks the slice:

- rich report rendering;
- stable external ID aliases;
- concrete playback orchestration;
- Dylint enforcement;
- localization and Fluent integration;
- build-support crate extraction;
- placeholder CLI resolution;
- schema module housekeeping unrelated to touched code;
- the full six-law RFC 0001 catalogue;
- Verus bindings or a Verus runner.

These items remain valuable. They simply do not define whether theoremc can
prove a theorem.

## Required roadmap amendments

The roadmap should make the following textual and dependency changes.

| Existing area | Amendment |
| --- | --- |
| Phase 4 outcome | Name a working Kani proof path as the primary outcome |
| Step 4.1 | Add the harness plan, real fixture execution, and pass/fail slice |
| Step 4.2 | Retain breadth work for `call`, `must`, and `maybe` |
| Step 4.3 | Add a minimal runner; move rich formats to Phase 5 |
| New Step 4.4 | Integrate RFC 0001 only after the slice works |
| Step 5.1 | Model generic obligations and backend-indexed evidence |
| Step 6.3 | Keep broad examples, but remove the first-slice dependency |
| Sequencing summary | Make the vertical slice the gate before laws or reports |

The implementation plan for Step 4.1 must cite ADR 006. Work on Step 5.1 and
RFC 0001 must cite ADR 007.

## Vertical-slice acceptance criteria

The slice is complete only when all of the following hold:

- a fixture crate discovers a theorem through the normal build script;
- ordinary `cargo build` remains free of Kani-only dependencies;
- `cargo kani list` discovers the stable generated harness;
- Kani proves one meaningful theorem;
- Kani falsifies one deliberately incorrect theorem;
- the failure preserves the source theorem and proof-obligation identity;
- an explicit witness is emitted and checked;
- unsupported source constructs fail with deterministic diagnostics;
- code-generation snapshots show the harness-plan-to-Kani mapping;
- the longhand example is suitable for a later schema version 2 law-equivalence
  fixture.

A mocked Kani command or token snapshot alone does not satisfy the milestone.
Environments without Kani may skip the executable integration test, but at
least one continuous integration path or maintainer gate must execute it.

## Consequences

Positive consequences:

- Product risk is retired earlier.
- The first useful proof arrives before report and policy ornamentation.
- RFC 0001 builds on a working compiler path instead of an empty scaffold.
- The future Verus boundary receives real execution experience before its
  binding design is fixed.
- Failures become easier to localize because fewer new layers land at once.

Costs and trade-offs:

- Step numbering and roadmap prose require revision.
- Some Phase 4 semantics remain temporarily unsupported after the first slice.
- The minimal runner may later be absorbed into `theoremd`.
- One small example appears before the polished example suite.
- Maintainers must enforce fail-closed behaviour to avoid partial semantics.

## Alternatives considered

### Complete the current Phase 4 horizontally

Rejected. It delays end-to-end evidence and encourages direct schema-to-Kani
rendering before a reusable middle boundary has proven necessary.

### Implement RFC 0001 before the slice

Rejected. It combines a schema break, six law forms, semantic validation, IR
work, and backend lowering before the existing language has proved one theorem.

### Build reporting before executable integration

Rejected. A polished report over an unproven execution model would optimize the
least certain part of the system last.

### Use HNSW as the first slice

Rejected for the first milestone. HNSW remains an excellent realistic example,
but its state, bounds, graph mutation, and counterexample complexity would make
basic pipeline defects harder to isolate.

### Emit Kani directly from `TheoremDoc` for speed

Rejected. It appears shorter locally, but it creates the exact assertion-only,
Kani-specific coupling that ADR 006 is designed to prevent.

## Related documents

- [RFC 0001: semantic law templates](rfcs/0001-semantic-law-templates.md)
- [ADR 006: backend-neutral harness planning
  boundary](adr-006-backend-neutral-harness-planning-boundary.md)
- [ADR 007: Verus backend preservation
  invariants](adr-007-verus-backend-preservation-invariants.md)
- [Development roadmap](roadmap.md)
- [Theoremc design specification](theoremc-design.md)
