# Theorem file format

Decision context for symbol stability and vacuity policy is recorded in
[Architecture Decision Record (ADR) 0001](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md).

[RFC 0001](rfcs/0001-semantic-law-templates.md) is accepted and defines schema
version 2. This specification records that target contract as well as the
version 1 compatibility profile. Version 2 implementation is pending roadmap
step 4.4, after the working version 1 Kani vertical slice. The current public
model and raw adapter still use assertion-only proof lists; accepted syntax
below must not be read as a claim of current parser or backend support.

## 1. YAML (a human-readable data-serialization format) schema reference (v1)

### 1.1 Document model

A `.theorem` file contains either:

- a single YAML document, or
- multiple YAML documents separated by `---` (each document defines one
  theorem).

Implementation note: `serde-saphyr` supports deserializing multiple YAML
documents into a `Vec<T>` (and it aims to be panic-free on malformed input and
avoid `unsafe` in library code).[^1]

### 1.2 Conformance rules

These rules are *normative* for both versions unless explicitly qualified:

- Unknown top-level keys: **MUST error** (`deny_unknown_fields` behaviour).
- Required keys missing: **MUST error**.
- Scalar types wrong (e.g., `Tags: foo` instead of `Tags: [foo]`): **MUST
  error**.
- `Assume.expr` and `Prove.assert`:

  - **MUST parse as Rust expressions** (syntactic validation using `syn`).
  - **MUST be single expressions** (no statement blocks, no `let`, no `for`,
    etc.).
- Theorem names:

  - **MUST be unique per crate** (within the theorem suite included in that
    crate).
  - **MUST be a valid Rust identifier in the restricted ASCII subset** (details
    below). This matches the exploratory work’s “valid Rust identifier”
    validation rule, now made precise.

### 1.3 Lower-case alias recommended

Canonical keys use TitleCase. For author ergonomics, the parser **MAY** accept
these aliases:

- `Theorem` also as `theorem`
- `About` also as `about`
- `Tags` also as `tags`
- `Given` also as `given`
- `Forall` also as `forall`
- `Assume` also as `assume`
- `Let` also as `let`
- `Do` also as `do`
- `Prove` also as `prove`
- `Evidence` also as `evidence`

If aliases are implemented, they should remain shallow and predictable (avoid
multiple spellings for the same key beyond case).

______________________________________________________________________

## 2. Primitive schema types (building blocks)

These definitions make the rest unambiguous.

### 2.1 `Identifier`

An ASCII identifier string matching:

- Regex: `^[A-Za-z_][A-Za-z0-9_]*$`
- Additionally, it **MUST NOT** be a Rust reserved keyword as defined in the
  [Rust language reference](https://doc.rust-lang.org/reference/keywords.html)
  (e.g. `fn`, `match`, `type`). If it is, treat it as an error (don’t silently
  fix it in the author’s file).

Rationale: the exploratory spec already required theorem names to be valid
identifiers; this specification locks down the exact rule for predictable,
stable code generation. The ASCII restriction is intentional in v1 to keep
symbol generation deterministic across tooling and platform boundaries.

### 2.2 `Tag` (recommended format: lower-kebab or lower-snake)

- No semantic impact; it’s metadata.

### 2.3 `RustExpr`

- A string containing a Rust expression.
- It **MUST** parse as `syn::Expr`.
- It is assumed to typecheck to `bool` in places that require a boolean.

### 2.4 `RustType`

- A string containing a Rust type expression.
- It **MUST** parse as `syn::Type`.
- It **MUST NOT** contain a free named lifetime parameter such as `&'a T`.
  Use an owned type, an elided lifetime, or `'static` where appropriate.

Examples: `"u64"`, `"crate::account::Account"`, `"std::sync::Arc<MyType>"`,
`"Option<&'static str>"`.

______________________________________________________________________

## 3. Top-level schema: `TheoremDoc`

A theorem document is a YAML mapping with these fields.

### 3.1 `Schema`

- Type: integer
- Optional
- Omitted: **MUST** select the version 1 compatibility profile permanently.
  The domain model may preserve omission as `None`; this does not mean the
  newest supported version.
- `Schema: 1`: **MUST** select version 1 (opaque assertions only).
- `Schema: 2`: **MUST** select version 2 (opaque assertions and structured
  laws).
- Unsupported values: **MUST** produce a source-located unsupported-version
  diagnostic. A law in an omitted/version 1 document **MUST** produce a
  targeted version diagnostic rather than silently upgrading the document.
- Purpose: explicit opt-in to language evolution without changing the meaning
  of existing unversioned files.

These are the accepted version-selection requirements from RFC 0001; enforcing
version 2 and unsupported-version diagnostics remains implementation work in
step 4.4.

### 3.2 `Theorem` (required)

- Type: `Identifier`
- Uniqueness: must be unique across all theorem docs compiled into the crate.

Example: `BidirectionalLinksCommitPath3Nodes`

### 3.3 `About` (required)

- Type: string (can be YAML block string)
- Constraint: must be non-empty after trimming.

This is the behaviour-driven development (BDD) “scenario title + description”
equivalent; the field is mandatory to keep proofs socially legible. (This
mirrors the exploratory spec’s requirement that `about` is mandatory.)

### 3.4 `Tags` (list of `Tag`)

- Default: `[]`

### 3.5 `Given` (optional)

- Type: list of strings
- Default: `[]`
- Semantics: narrative only; no codegen.

### 3.6 `Forall` (optional)

- Type: mapping of `Identifier -> RustType`
- Default: `{}`

Example:

```yaml
Forall:
  a: Account
  amount: u64
```

Semantics (Kani backend): each entry becomes a symbolic input
`kani::any::<Ty>()`.

Semantics (ordinary Rust builds): each type is validated as `syn::Type` during
schema loading and participates in generated referenced-type probes during
`theorem_file!` expansion. If a type path is missing or moved in the theorem
owner crate, `cargo build` fails before any Kani harness is run.

Implementation note: preserve YAML map order by deserializing into an
insertion-ordered map (e.g., `IndexMap`) so generated harnesses stay stable
under diffs.

### 3.7 `Assume` (optional)

- Type: list of `Assumption`
- Default: `[]`

Each `Assumption` is a mapping:

- `expr` (required): `RustExpr` (must parse)
- `because` (required): non-empty string explanation

Example:

```yaml
Assume:
  - expr: "amount <= (u64::MAX - a.balance)"
    because: "prevent overflow in deposit"
```

Semantics (Kani): each becomes `kani::assume(<expr>);` (with optional
tracing/report metadata).

### 3.7.1 `Witness` (required unless vacuity is explicitly allowed)

- Type: list of `WitnessCheck`
- Default: `[]`
- Constraint: non-empty unless `Evidence.kani.allow_vacuous: true`

Each `WitnessCheck` is a mapping:

- `cover` (required): `RustExpr` (must parse)
- `because` (required): non-empty string explanation

Semantics (Kani): each witness emits `kani::cover!(<expr>)`, so successful runs
must exercise at least one non-vacuous path unless vacuity is explicitly
accepted.

### 3.8 `Let` (optional)

- Type: mapping of `Identifier -> LetBinding`
- Default: `{}`

Each binding defines a named value computed before `Do:` executes. Use it for
fixtures and derived constants.

A `LetBinding` is **exactly one** of:

- `{ call: ActionCall }`
- `{ must: ActionCall }`

The v1 schema explicitly disallows `maybe` in `Let`, because “binding exists
only in some paths” creates scoping complexity that harms readability and makes
later steps hard to validate.

Example (from the settled syntax):

```yaml
Let:
  params:
    must:
      action: hnsw.params
      args: { max_connections: 1, max_level: 2 }
  graph:
    call:
      action: hnsw.graph_with_capacity
      args: { params: { ref: params }, capacity: 3 }
```

Semantics:

- `call`: evaluate the action; bind the result to the `Let` key.
- `must`: evaluate the action; prove it cannot fail (Result/Option handling);
  bind its unwrapped success value.

### 3.9 `Do` (optional, but practically required)

- Type: list of `Step`
- Default: `[]`

Each `Step` is exactly one of:

- `{ call: ActionCall }`
- `{ must: ActionCall }`
- `{ maybe: MaybeBlock }`

(Details below.)

### 3.9.1 `Actions` (required when actions are referenced)

- Type: mapping of `ActionName -> ActionSignature`
- Default: `{}`

Each entry declares the expected Rust function signature for an action named by
`Let` or `Do`. The theorem document, not the current Rust implementation, is
the source of this expected contract. This follows
[ADR 004](adr-004-action-signature-specification.md).

Example:

```yaml
Actions:
  hnsw.attach_node:
    params:
      graph: "&mut crate::hnsw::Graph"
      node: "crate::hnsw::NodeId"
    returns: "Result<(), crate::hnsw::AttachError>"
```

Semantics:

- Every action referenced by this document's `Let` and `Do` sections must have
  a matching `Actions` entry. In version 2, this also applies to every law
  primary and custom relation.
- `params` order is significant and defines the generated `fn(...)` probe
  parameter order.
- Parameter names must match `args` keys used by calls to the action.
- Parameter and return type strings must parse as Rust type expressions.
- Parameter and return type strings must not contain free named lifetime
  parameters such as `&'a T`.
- `returns` defaults to `()` when omitted.
- The generated probe checks the declared signature against the resolved
  `crate::theorem_actions::{mangled_identifier}` export.
- Every declared parameter and return type also participates in the
  referenced-type probe, even before Phase 4 action execution wiring exists.

### 3.10 `Prove` (required)

- Type: non-empty list of `ProofObligation`.
- Version 1 permits only `Assert(Assertion)`.
- Version 2 permits `Assert(Assertion)` and `Law(LawObligation)` in the same
  list.
- Each item **MUST** contain exactly one of `assert` or `law`, with sibling
  `because` (required, non-empty after trimming). Unknown fields and ambiguous
  items **MUST** error.

An opaque `Assertion` is a mapping:

- `assert` (required): `RustExpr` (must parse)
- `because` (required): non-empty string

Example:

```yaml
Prove:
  - assert: "hnsw.is_bidirectional(&graph)"
    because: "bidirectional invariant holds after commit-path reconciliation"
```

Semantics (Kani): emits `assert!(<expr>, "<because>");`. Opaque assertions
remain valid in version 2 and explicitly backend-constrained in the shared plan.

A `LawObligation` has `{ law: Law, because: String }`. The `law` mapping
**MUST** contain exactly one catalogue kind from section 3.10.2.

### 3.10.1 Primary invocations and relations (version 2)

Laws reuse `Actions`; there is no top-level `Primaries` registry. A primary
invocation contains:

- `action`: canonical `ActionName`, with a theorem-owned `Actions` signature;
- `bind`: mapping of law-defined roles to actual action parameter identifiers;
- `args`: fixed arguments using section 5's existing value forms;
- `mode`: `call` or `must`, defaulting to `call`.

Unary operations, invariants, and mappings require the role `input`. Binary
combiners and relations require both `left` and `right`. A fully specified
invocation has no dynamic roles. Each role **MUST** name a declared parameter;
missing or unexpected roles, absent parameters, and parameters supplied by both
`bind` and `args` **MUST** error.

Dynamically bound parameters **MUST** use shared references; by-value or
mutable bound parameters are rejected for the initial catalogue. Primaries
should return owned semantic results. No implicit cloning, default
construction, or semantic adapter may be inserted. Signature checks cannot
establish purity or exclude hidden state, I/O, randomness, or interior mutation.

`call` preserves the declared return type, including fallible values. `must`
reuses section 4.2.2: require successful `Result` or present `Option`, then
yield the inner value; infallible returns pass through. Unsupported unwrapping
shapes **MUST** error.

The comparison laws default to equality except `refines`, which requires a
custom relation. A custom relation is a binary primary binding `left` and
`right`, whose final unwrapped return type **MUST** be `bool`. Its signature
does not prove that the relation is adequate; weak or always-true relations
require independent scrutiny.

### 3.10.2 Law catalogue (version 2)

In the following table, `R` denotes equality or the selected custom relation.
Inputs and states use the existing section 5 value forms; primary fields use
section 3.10.1.

| Kind           | Required fields                                               | Meaning                                                     |
| -------------- | ------------------------------------------------------------- | ----------------------------------------------------------- |
| `idempotent`   | `operation`, `input`                                          | `f(f(x)) R f(x)`                                            |
| `round_trip`   | `forward`, `backward`, `input`                                | `backward(forward(x)) R x`                                  |
| `equivalent`   | `left`, `right`, `input`                                      | `left(x) R right(x)`                                        |
| `refines`      | `implementation`, `specification`, `relation`, `input`        | `relation(implementation(x), specification(x))`             |
| `preserves`    | `invariant`, `transition`, `state`                            | `invariant(state)` implies `invariant(transition(state))`   |
| `homomorphism` | `mapping`, `input_combine`, `output_combine`, `left`, `right` | `map(combine_input(x, y)) R combine_output(map(x), map(y))` |

*Table 1: Version 2 law kinds, fields, and semantic propositions.*

`idempotent`, `round_trip`, `equivalent`, and `homomorphism` accept an optional
`relation` primary. Round trip is directional; a bidirectional codec needs two
instances. For `preserves`, the invariant yields `bool` and the model
transition yields a new state. Kani lowering checks precondition reachability,
assumes the precondition, invokes the transition, and asserts the invariant on
its result.

Example fragment (accepted version 2 syntax, pending implementation):

```yaml
Schema: 2
Actions:
  config.normalize:
    params:
      config: "&crate::config::Config"
    returns: "crate::config::Config"
Prove:
  - assert: "config.is_valid()"
    because: "the existing opaque assertion remains available"
  - law:
      idempotent:
        operation:
          action: config.normalize
          bind: { input: config }
          args: {}
        input: { ref: config }
    because: "normalization reaches a fixed point after one application"
```

The fragment requires the usual theorem metadata, a `config` binding, evidence,
and explicit witnesses to form a complete document. Full examples for every law
are in
[RFC 0001's catalogue](rfcs/0001-semantic-law-templates.md#initial-law-catalogue).

### 3.10.3 Validation, lowering, and evidence (version 2)

Validation **MUST** reject unknown or multiple law kinds, missing signatures,
invalid action names, role/parameter conflicts, invalid reference shapes,
invalid modes or relations, obvious composition mismatches, and empty
rationales. Diagnostics retain source spans and precise field breadcrumbs. Law
action references extend signature conflict checks, action probes,
referenced-type probes, and non-executed law-composition probes. rustc handles
aliases, associated types, coercions, and generic normalization beyond
theoremc's structural validation.

Validated laws lower into semantic Law IR, then the shared `HarnessPlan` from
[ADR 006](adr-006-backend-neutral-harness-planning-boundary.md). Opaque
obligations join at the same plan. Neither representation contains Kani syntax
or token streams. Laws reuse the existing renderer and runner; backends
**MUST** check the complete plan's capabilities before emitting partial output.

Each law preserves its source rationale and parent identity, with deterministic
child identities such as `prove[2].law.idempotent.relation`. These identities
must not depend on generated local names. Longhand version 1 and corresponding
version 2 laws require semantically equivalent plan fixtures.

Law-generated precondition covers supplement explicit theorem witnesses; they
**MUST NOT** replace them. Unreachable obligations are not successful proofs.
Run records distinguish `proved`, `falsified`, `unreachable`, `undetermined`,
`unsupported`, and `execution_error`. Resource exhaustion is `undetermined`
with reason `resource_exhaustion`. Evidence remains backend-indexed, retaining
bounds and provenance; a Kani law result remains bounded model-checking
evidence.

Future Verus support **MUST** preserve version 2 law meaning and add explicit
bindings for executable, specification, and proof symbols and mathematical
views. Ordinary action signatures do not imply those bindings. Opaque Rust
expressions remain backend-constrained. These requirements follow
[ADR 007](adr-007-verus-backend-preservation-invariants.md).

### 3.11 `Evidence` (required)

- Type: `Evidence`
- Must specify at least one backend configuration.

For v1, Kani is the MVP backend, so `Evidence.kani` is the primary required
config for Kani-targeted theorems.

The exploratory work already defined “Evidence must specify at least one
backend” and “unwind required for kani”; this specification now locks that down
in schema form.

______________________________________________________________________

## 4. Step and action schemas

### 4.1 `ActionCall`

An `ActionCall` is a mapping:

- `action` (required): `ActionName` (see below)
- `args` (required): mapping of `Identifier -> Value`
- `as` (optional): `Identifier`

Example:

```yaml
call:
  action: account.deposit
  args: { account: { ref: a }, amount: { ref: amount } }
  as: b
```

Binding rules:

- In `Do:`:

  - If `as` exists, the call’s return value is bound to that name.
  - If `as` is absent:

    - Allowed only if the return type is `()` (infallible, no value).
    - Otherwise, **error** (prevents accidentally discarding important results
      or failures).
- In `Let:`:

  - The `Let` key is the binding name; `as` is ignored (and should error if
    present, to prevent confusion).

### 4.1.1 `ActionSignature`

An `ActionSignature` is a mapping:

- `params` (optional): ordered mapping of `Identifier -> RustType`
- `returns` (optional): `RustType`, defaulting to `()`

Examples:

```yaml
Actions:
  account.deposit:
    params:
      account: "&mut crate::account::Account"
      amount: "u64"
    returns: "Result<(), crate::account::DepositError>"

  audit.flush:
    params: {}
```

`params` is optional only for zero-argument actions. If present, it may be an
empty mapping. The implementation must preserve the order of `params`, because
Rust function pointer types are positional even when their parameters are
written with names.

The `ActionSignature` declaration does not resolve or call the action. It is
input to generated compile-time probes and later Kani action lowering.

During ordinary `theorem_file!` expansion, all distinct types declared in
`Forall` and `Actions` are probed in the theorem owner crate:

```rust
const _: () = {
    fn __theoremc_assert_referenced<T: ?Sized>() {}
    let _ = __theoremc_assert_referenced::<crate::account::Account>;
};
```

The probe forces Rust to resolve the declared type path. It does not validate
struct fields or lower YAML maps into struct literals; those checks occur when
later generated harness code uses the type in executable Rust.

### 4.2 `Step` variants

#### 4.2.1 `call`

```yaml
- call:
    action: hnsw.add_bidirectional_edge
    args: { graph: { ref: graph }, origin: 0, target: 2, level: 1 }
```

Semantics: invokes the action.

#### 4.2.2 `must`

```yaml
- must:
    action: hnsw.attach_node
    args: { graph: { ref: graph }, node: 2, level: 1, sequence: 2 }
```

Semantics: invokes the action and proves it cannot fail under current
assumptions.

More precisely:

- If the action returns `Result<T, E>`:

  - `must` generates an obligation `assert!(res.is_ok(), "...")` then unwraps
    to `T`.
- If the action returns `Option<T>`:

  - `must` generates `assert!(opt.is_some(), "...")` then unwraps to `T`.
- If the action returns `T` or `()`:

  - `must` simply calls it; no additional obligation is created.

These semantics match the settled design conversation and the exploratory
specification’s description of `must`.

#### 4.2.3 `maybe`

- `because` (required): non-empty string explanation
- `do` (required): list of `Step`

Example:

```yaml
- maybe:
    because: "optional baseline edge for state space exploration"
    do:
      - call:
          action: hnsw.add_bidirectional_edge
          args: { graph: { ref: graph }, origin: 0, target: 1, level: 0 }
```

Semantics: symbolic branching.

In Kani, this compiles to something morally equivalent to:

```rust
let b: bool = kani::any();
if b { /* nested steps */ }
```

So the model checker explores both branches. The exploratory spec also states
this interpretation of `maybe`.

______________________________________________________________________

## 5. Value forms and how they compile

`args:` values are YAML values with a small amount of interpretation to
preserve friendliness while staying typecheckable.

### 5.1 `Value` forms

A `Value` is one of:

1. YAML integer → Rust integer literal (`0`, `1`, `32`, …)
2. YAML boolean → Rust boolean literal (`true`/`false`)
3. YAML string → Rust string literal
4. YAML list → Rust `vec![...]`
5. YAML map → either a struct literal or an explicit wrapper form

### 5.2 Explicit variable references and string literals

For YAML argument values in `args:`:

- Plain YAML strings are always string literals.
- Variable references must use `{ ref: <Identifier> }`.

String literals may also use `{ literal: <String> }` when explicitness helps.

```yaml
label: { literal: "graph" }
```

This rule avoids accidental meaning changes when new bindings are introduced.

### 5.3 Explicit wrappers (map values)

A YAML map value with one of these sentinel keys takes a special meaning:

- `{ ref: <Identifier> }` → force variable reference
- `{ literal: <String> }` → force string literal

Any other map value is treated as a candidate **struct literal**.

### 5.4 Struct literal synthesis

If an arg value is a YAML map passed into an action parameter of some struct
type `T`, the generator emits:

```rust
T { field1: ..., field2: ..., }
```

using the map keys as field names.

If this does not typecheck, Rust produces a hard error at build time, providing
the desired “always connected” feedback loop.

______________________________________________________________________

## 6. Evidence schema

### 6.1 `Evidence`

`Evidence` is a mapping of backend name → backend config.

For v1, define:

- `kani` (optional but required if the theorem declares `Tags`/intent for kani
  or if the Kani suite is run)
- `verus` (optional; placeholder config only for now; real Verus semantics land
  post-MVP)
- `stateright` (optional; placeholder)

The exploratory spec includes multi-backend intent and a Kani-first ordering;
this schema supports that while keeping Kani MVP crisp.

### 6.2 `Evidence.kani`

- `unwind` (required): positive integer
  Compiles to `#[kani::unwind(<n>)]`. Kani documents `#[kani::unwind]` as the
  mechanism to control loop unwinding bounds.[^2]

- `expect` (required): enum string:

  - `SUCCESS`
  - `FAILURE`
  - `UNREACHABLE`
  - `UNDETERMINED`

`expect` exists for report gating (and for negative tests where a
counterexample is expected).

- `allow_vacuous` (optional): boolean, default `false`
- `vacuity_because` (required when `allow_vacuous: true`): non-empty string
  rationale

If `allow_vacuous` is `false`, `Witness` must contain at least one item.

### 6.3 `Evidence.verus` (placeholder)

A mapping (not required for MVP):

- `mode` (optional): enum string such as `proof`

This exists only to keep the schema stable for the multi-backend story already
outlined in exploratory work and examples.

______________________________________________________________________

## 7. Precise mangling rules

Now the “no surprises” bit: how strings in `.theorem` map to Rust symbol
identifiers.

Normative mangling definitions live in `docs/name-mangling-rules.md`. This
section summarizes those rules for convenience within the schema document.

### 7.1 Action name grammar (`ActionName`)

An action name is a dot-separated path:

- Grammar: `Segment ("." Segment)+`
- Each `Segment` must match `Identifier`.

Examples:

- `account.deposit`
- `hnsw.commit_apply`
- `hnsw.graph_with_capacity`

Enforcing *at least one dot* is recommended so authors naturally namespace
their actions and avoid collisions.

### 7.2 Action resolution rule (string → Rust function path)

**Canonical action module:** `crate::theorem_actions`

**Mangled Rust function identifier:**
`{action_slug(canonical_name)}__h{hash12(canonical_name)}`.

Definitions:

- `segment_escape(segment)`:
  1. Replace `_` with `_u`.
  2. Leave ASCII letters and digits unchanged.
- `action_slug(canonical_name)`:
  1. Split `canonical_name` on `.`.
  2. Apply `segment_escape` to each segment.
  3. Join escaped segments with `__`.
- `hash12(value)`:
  1. Compute `blake3(value.as_bytes())`.
  2. Take the first 12 lowercase hex characters.

So:

- `hnsw.attach_node` →
  `crate::theorem_actions::hnsw__attach_unode__h3f6b2a80c9d1`
- `account.withdraw` →
  `crate::theorem_actions::account__withdraw__h5a197f4ee18c`
- `hnsw.graph.with_capacity` →
  `crate::theorem_actions::hnsw__graph__with_ucapacity__h8d6a20f2c44e`

Reserved keywords:

- If any segment equals a Rust keyword, error out (don’t try to “fix” it). This
  keeps author intent explicit and avoids spooky collisions.

Collision checks:

- Detect duplicate canonical action names and fail with source locations.
- Detect duplicate mangled action identifiers and fail with all colliding names.

This rule gives compile-time binding without relying on link-time registries;
`inventory` remains useful for reporting but is not required for resolution.
This directly supports the “always connected” pattern described in the
exploratory work.

### 7.3 Generated module naming (file path → Rust module)

Each `.theorem` file expands into a generated Rust module name that must be:

- stable across builds
- uniqueness across files
- human-recognizable names
- no collisions from sanitization

Define:

- Input: the literal path string `P` passed to `theorem_file!("P")` (relative
  to crate root).

- `path_stem(P)`: remove the trailing `.theorem` extension if present.

- `path_mangle(P)`:

  1. Replace `/` and `\` with `__`
  2. Replace any character not in `[A-Za-z0-9_]` with `_`
  3. Collapse consecutive `_` into a single `_`
  4. Lowercase
  5. If it starts with a digit, prefix `_`

- `hash12(P)`: compute `blake3(P.as_bytes())`, take the first 12 hex chars of
  the digest.

Generated module name:

```plaintext
__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}
```

Example:

- `P = "theorems/bidirectional.theorem"`
- `path_stem(P) = "theorems/bidirectional"`
- `path_mangle(...) = "theorems__bidirectional"`
- `hash12(P) = "a1b2c3d4e5f6"` (illustrative)

Module:

```plaintext
mod __theoremc__file__theorems__bidirectional__a1b2c3d4e5f6 { ... }
```

Why the hash: if both `my-file.theorem` and `my_file.theorem` exist, both
mangle to the same identifier; the hash prevents collision while still keeping
names readable.

### 7.4 Generated harness function naming (theorem name → Kani harness)

Kani supports selecting a single proof harness using `--harness <name>`, and it
supports using the *full module-qualified harness name* when needed (e.g.
`ptr::unique::verify::check_new`), with Kani output printing the “full name” it
is checking.[^3]

This should be exploited while also keeping the simple name unique so
`--harness theorem__foo` usually works.

Define:

- `theorem_id` = the `Theorem` field (an `Identifier`).
- `theorem_key` = `{P}#{theorem_id}`.

- `theorem_snake(theorem_id)`:

  - If `theorem_id` already matches `^[a-z_][a-z0-9_]*$`, keep it.
  - Else convert UpperCamelCase → snake_case with this deterministic rule:

    - Insert `_` between a lower/digit and an upper (e.g. `Path3` → `path_3`)
    - Split acronym runs before the last capital when followed by a lowercase
      (e.g. `HNSWInvariant` → `hnsw_invariant`)
    - Lowercase everything

- Harness function identifier:

```plaintext
theorem__{theorem_snake(theorem_id)}__h{hash12(theorem_key)}
```

Collision checks:

- Theorem names must still be unique across the crate theorem suite.
- The generator must detect duplicate theorem keys and fail compilation with
  source locations.

### 7.5 Full harness path layout

Within the per-file module, generate backend submodules. For Kani:

```rust
#[cfg(kani)]
mod kani {
    #[kani::proof]
    #[kani::unwind(N)]
    pub fn theorem__...() { ... }
}
```

So the *full harness name* Kani can use is:

```plaintext
__theoremc__file__...::kani::theorem__...__h{hash12(P#T)}
```

And the *short harness name* (what engineers usually type) is:

```plaintext
theorem__...
```

If there’s ever ambiguity (it shouldn’t happen if theorem names are unique, but
bugs happen), Kani still supports the full name path.[^3]

### 7.6 Stable external theorem IDs and migration

Reporting uses a stable external theorem ID independent of Rust symbols:

- Canonical ID: `{normalized_path(P)}#{theorem_id}`
- `normalized_path(P)`:
  1. Use `/` as separator.
  2. Remove leading `./`.
  3. Preserve case.

When files or theorem names move, maintain aliases in
`theorems/theorem-id-aliases.yaml`:

- Each entry maps an old canonical ID to a new canonical ID.
- Alias graphs must be acyclic.
- Every deprecated ID must resolve to exactly one canonical ID.

______________________________________________________________________

## 8. Minimal Rust struct skeleton (to make implementation straightforward)

This sketch describes the accepted target model after step 4.4, not the current
assertion-only public API or raw adapter. Version-aware raw decoding and
semantic validation enforce the version rules before domain conversion. Raw
adapters remain separate from domain types and lowering.

The public change from `Vec<Assertion>` to `Vec<ProofObligation>` is
source-breaking for constructing, destructuring, and iterating consumers. It
must land at a documented release boundary; a compatibility accessor must not
hide law obligations. Existing version 1 files require no migration. Adding
`Schema: 2` opts into laws while preserving existing assertion syntax.

This can be implemented with `serde` derives and deserialized using
`serde-saphyr` (e.g., `serde_saphyr::from_str`, or multi-doc APIs), as
required.[^1]

```rust
// Pseudocode-level skeleton; fields omitted for brevity.

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoremDoc {
    #[serde(default)]
    pub schema: Option<u32>, // "Schema"

    #[serde(rename = "Theorem", alias = "theorem")]
    pub theorem: String,

    #[serde(rename = "About", alias = "about")]
    pub about: String,

    #[serde(rename = "Tags", alias = "tags", default)]
    pub tags: Vec<String>,

    #[serde(rename = "Given", alias = "given", default)]
    pub given: Vec<String>,

    #[serde(rename = "Forall", alias = "forall", default)]
    pub forall: indexmap::IndexMap<String, String>, // Identifier -> RustType

    #[serde(rename = "Assume", alias = "assume", default)]
    pub assume: Vec<Assumption>,

    #[serde(rename = "Witness", alias = "witness", default)]
    pub witness: Vec<WitnessCheck>,

    #[serde(rename = "Let", alias = "let", default)]
    pub let_bindings: indexmap::IndexMap<String, LetBinding>,

    #[serde(rename = "Do", alias = "do", default)]
    pub do_steps: Vec<Step>,

    #[serde(rename = "Actions", alias = "actions", default)]
    pub actions: indexmap::IndexMap<String, ActionSignature>,

    #[serde(rename = "Prove", alias = "prove")]
    pub prove: Vec<ProofObligation>,

    #[serde(rename = "Evidence", alias = "evidence")]
    pub evidence: Evidence,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assumption {
    pub expr: String,
    pub because: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    #[serde(rename = "assert")]
    pub assert_expr: String,
    pub because: String,
}

// Domain sketch only: raw adapters enforce exclusive keys and version gating.
pub enum ProofObligation {
    Assert(Assertion),
    Law(LawObligation),
}

pub struct LawObligation {
    pub law: Law, // Dedicated law domain types from section 3.10.2.
    pub because: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WitnessCheck {
    pub cover: String,
    pub because: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum LetBinding {
    Call { call: ActionCall },
    Must { must: ActionCall },
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Step {
    Call { call: ActionCall },
    Must { must: ActionCall },
    Maybe { maybe: MaybeBlock },
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaybeBlock {
    pub because: String,
    #[serde(rename = "do")]
    pub do_steps: Vec<Step>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCall {
    pub action: String,
    // Or your own Value type.
    pub args: indexmap::IndexMap<String, serde_saphyr::Value>,
    #[serde(rename = "as", default)]
    pub as_binding: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSignature {
    #[serde(default)]
    pub params: indexmap::IndexMap<String, String>, // Identifier -> RustType
    #[serde(default = "unit_type")]
    pub returns: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    #[serde(default)]
    pub kani: Option<KaniEvidence>,
    #[serde(default)]
    pub verus: Option<serde_saphyr::Value>,
    #[serde(default)]
    pub stateright: Option<serde_saphyr::Value>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KaniEvidence {
    pub unwind: u32,
    pub expect: String,
    #[serde(default)]
    pub allow_vacuous: bool,
    #[serde(default)]
    pub vacuity_because: Option<String>,
}

fn unit_type() -> String {
    "()".to_owned()
}
```

(A wrapper around `serde_saphyr::Value` is likely useful so a project-specific
`Value` enum can enforce “no nulls”, implement `{ref:}` and `{literal:}`
wrappers, and support typed struct-literal emission cleanly.)

[^1]: <https://docs.rs/serde-saphyr?utm_source=chatgpt.com> "serde_saphyr -
      Rust"
[^2]: <https://model-checking.github.io/kani/reference/attributes.html?utm_source=chatgpt.com>
       "Attributes - The Kani Rust Verifier"
[^3]: <https://model-checking.github.io/verify-rust-std/tools/kani.html?utm_source=chatgpt.com>
       "Kani - Verify Rust Std Lib"
