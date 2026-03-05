# User's guide

This guide covers the behaviour and application programming interface (API) of
the `theoremc` library from the perspective of a library consumer.

## Theorem document schema

A `.theorem` file is a UTF-8 text file containing one or more YAML (YAML Ain't
Markup Language) documents. Multiple documents within a single file are
separated by `---`. Each document describes one theorem.

### Loading theorem documents

Use `theoremc::schema::load_theorem_docs` to parse a `.theorem` file's contents
into a vector of `TheoremDoc` structs:

```rust
use theoremc::schema::load_theorem_docs;

let yaml = std::fs::read_to_string("theorems/my_theorem.theorem")?;
let docs = load_theorem_docs(&yaml)?;
```

The function:

- Deserializes one or more YAML documents from the input string.
- Rejects unknown keys (any key not defined in the schema causes an error).
- Validates theorem identifiers and `Forall` keys against the identifier
  rules (see below).
- Enforces non-empty constraints on string fields (see below).
- Returns `Err(SchemaError)` with an actionable message on failure.

When a concrete source path is available (for example, a fixture path or
project file path), prefer `load_theorem_docs_with_source` so diagnostics
include that source identifier:

```rust
use theoremc::schema::{SourceId, load_theorem_docs_with_source};

let source = "theorems/my_theorem.theorem";
let yaml = std::fs::read_to_string(source)?;
let docs = load_theorem_docs_with_source(&SourceId::new(source), &yaml)?;
```

### Top-level fields

Every theorem document is a YAML mapping with the following fields. Keys use
`TitleCase` canonically, but lowercase aliases are also accepted (e.g.,
`Theorem` or `theorem`).

| Field      | Type                              | Required | Default              | Notes                                                                   |
| ---------- | --------------------------------- | -------- | -------------------- | ----------------------------------------------------------------------- |
| `Schema`   | integer                           | no       | `None` (unspecified) | Forwards compatibility.                                                 |
| `Theorem`  | string                            | **yes**  | —                    | Must be a valid identifier (see below).                                 |
| `About`    | string                            | **yes**  | —                    | Human-readable description of intent. Must be non-empty after trimming. |
| `Tags`     | list of strings                   | no       | `[]`                 | Metadata for filtering and reporting.                                   |
| `Given`    | list of strings                   | no       | `[]`                 | Narrative context (no codegen impact).                                  |
| `Forall`   | map (identifier → type)           | no       | `{}`                 | Symbolic quantified variables.                                          |
| `Assume`   | list of `Assumption`              | no       | `[]`                 | Constraints on symbolic inputs.                                         |
| `Witness`  | list of `WitnessCheck`            | no       | `[]`                 | Non-vacuity witnesses.                                                  |
| `Let`      | map (identifier → `LetBinding`)   | no       | `{}`                 | Named fixtures.                                                         |
| `Do`       | list of `Step`                    | no       | `[]`                 | Theorem step sequence.                                                  |
| `Prove`    | list of `Assertion`               | **yes**  | —                    | Proof obligations.                                                      |
| `Evidence` | `Evidence`                        | **yes**  | —                    | Backend configuration.                                                  |

### Identifier rules

Theorem names and `Forall` map keys must satisfy:

- Match the ASCII pattern `^[A-Za-z_][A-Za-z0-9_]*$`.
- Must **not** be a Rust reserved keyword (`fn`, `let`, `match`, `type`,
  `self`, `Self`, `async`, `yield`, etc.).

Invalid identifiers produce an `InvalidIdentifier` error with a message
explaining why the identifier was rejected.

### Non-empty constraints

All string fields that carry semantic content must be non-empty after trimming
(leading and trailing whitespace removed using Unicode-aware `str::trim()`).
The loader rejects documents where any of the following fields are empty or
contain only whitespace:

- `About`
- `Assumption.expr` and `Assumption.because`
- `Assertion.assert` and `Assertion.because`
- `WitnessCheck.cover` and `WitnessCheck.because`
- `KaniEvidence.vacuity_because` (when present)

The loader also enforces these structural constraints:

- The `Prove` section requires at least one assertion.
- `Evidence.kani.unwind` accepts only positive integers (> 0).
- At least one `Witness` entry is required when `allow_vacuous` is omitted or
  explicitly `false`.
- `allow_vacuous: true` is accepted only with a non-empty
  `vacuity_because` rationale.

### Expression syntax validation

The expression fields `Assumption.expr`, `Assertion.assert`, and
`WitnessCheck.cover` must contain syntactically valid Rust expressions. The
loader parses each expression using `syn::Expr` and rejects any expression that
is not a single, value-producing form.

Accepted forms include comparisons, function and method calls, boolean
literals, identifiers, arithmetic, `if` expressions, `match` expressions,
closures, field access, and other standard Rust expressions:

```yaml
Assume:
  - expr: "amount <= (u64::MAX - balance)"
    because: prevent overflow
Prove:
  - assert: "result.is_valid()"
    because: account invariants hold
Witness:
  - cover: "if x > 0 { x } else { 1 }"
    because: positive branch is exercised
```

Rejected forms include statement blocks, loops (`for`, `while`, `loop`), `let`
bindings, `unsafe`/`async`/`const` blocks, assignments, and flow-control
statements (`return`, `break`, `continue`):

```yaml
# These will be rejected:
Assume:
  - expr: "{ let x = 1; x > 0 }"    # block expression
  - expr: "for i in 0..10 { }"       # for loop
  - expr: "x = 5"                    # assignment
```

### Step and Let binding validation

The loader validates the structural constraints of `Let` bindings and `Do`
steps:

- Every `ActionCall.action` field (in both `Let` bindings and `Do` steps) must
  be non-empty after trimming. Blank action names are rejected.
- Every non-blank `ActionCall.action` must follow canonical action-name
  grammar: `Segment ("." Segment)+` (at least one `.` separator).
- Each canonical action-name segment must match
  `^[A-Za-z_][A-Za-z0-9_]*$` and must not be a Rust reserved keyword.
- Every `MaybeBlock.because` field must be non-empty after trimming.
- Every `MaybeBlock.do` list must contain at least one step (an empty `maybe`
  block is meaningless).
- Validation recurses into nested `maybe` blocks. A `maybe` containing another
  `maybe` with a blank `because` is caught with a full path context (e.g.,
  `"Do step 2: maybe.do step 1: maybe.because must be non-empty"`).
- `Let` bindings accept only `call` or `must` variants. A `maybe` block inside
  `Let` is rejected at the deserialization level.

### Subordinate types

**Assumption**: a constraint on symbolic inputs. Both `expr` and `because` are
required and must be non-empty after trimming.

```yaml
Assume:
  - expr: "amount <= u64::MAX"
    because: "prevent overflow"
```

**Assertion**: a proof obligation. Both `assert` and `because` are required and
must be non-empty after trimming.

```yaml
Prove:
  - assert: "balance == expected"
    because: "deposit adds to balance"
```

**WitnessCheck**: a non-vacuity witness. Both `cover` and `because` are
required and must be non-empty after trimming.

```yaml
Witness:
  - cover: "amount == 50"
    because: "mid-range deposit is exercised"
```

**LetBinding**: a named value binding. Must be one of `call` or `must`.

```yaml
Let:
  params:
    must:
      action: account.params
      args: { max_balance: 1000 }
  result:
    call:
      action: account.deposit
      args: { account: { ref: a }, amount: { ref: amount } }
```

**Step**: an element of the `Do` sequence. Must be one of `call`, `must`, or
`maybe`.

```yaml
Do:
  - call:
      action: account.deposit
      args: { account: { ref: a }, amount: 100 }
  - must:
      action: account.validate
      args: { account: { ref: result } }
  - maybe:
      because: "optional second deposit"
      do:
        - call:
            action: account.deposit
            args: { account: { ref: result }, amount: 10 }
```

**ActionCall**: an invocation of a theorem action.

- `action` (required): dot-separated action name (e.g., `account.deposit`).
- `action` must use canonical grammar (`Segment ("." Segment)+`), where each
  segment is an ASCII identifier and not a Rust reserved keyword.
- `args` (required): mapping of parameter name to value.
- `as` (optional): binding name for the return value.

**Evidence**: backend configuration. Currently, supports `kani`, with `verus`
and `stateright` as placeholders.

```yaml
Evidence:
  kani:
    unwind: 10
    expect: SUCCESS
```

**KaniEvidence** fields:

- `unwind` (required): positive integer, must be > 0 (loop unwinding bound).
- `expect` (required): one of `SUCCESS`, `FAILURE`, `UNREACHABLE`, or
  `UNDETERMINED`.
- `allow_vacuous` (optional, default `false`): whether vacuous success is
  permitted. When omitted, behaviour is identical to `allow_vacuous: false`.
- `vacuity_because` (required when `allow_vacuous` is `true`): human-readable
  justification. Must be non-empty after trimming.

### Value forms in arguments

After YAML deserialization, each action argument value is decoded into an
`ArgValue` that distinguishes literals from variable references. This encoding
ensures that plain YAML strings are unconditionally treated as string literals
and variable references require the explicit `{ ref: <name> }` wrapper.

**Decoded argument types (`ArgValue`):**

- `ArgValue::Literal(LiteralValue::Bool(b))` — a YAML boolean (`true`/`false`).
- `ArgValue::Literal(LiteralValue::Integer(n))` — a YAML integer.
- `ArgValue::Literal(LiteralValue::Float(f))` — a YAML float.
- `ArgValue::Literal(LiteralValue::String(s))` — a plain YAML string. Plain
  strings are **always** string literals, regardless of whether a `Let` binding
  with the same name exists in the same theorem.
- `ArgValue::Reference(name)` — an explicit variable reference via
  `{ ref: <name> }`. The `name` must be a valid ASCII identifier
  (`^[A-Za-z_][A-Za-z0-9_]*$`) and must not be a Rust reserved keyword.
- `ArgValue::RawSequence(values)` — a YAML sequence (future: `vec![...]`
  synthesis).
- `ArgValue::RawMap(map)` — a YAML map that is not a recognized wrapper (future:
  struct-literal synthesis or `{ literal: ... }` wrapper).

**Semantic stability invariant:** adding a new `Let` binding can never silently
change the meaning of an existing argument that was previously a plain string.
A plain string `"x"` always decodes as `ArgValue::Literal(String("x"))`, even
if a binding named `x` exists. To reference a binding, use `{ ref: x }`.

**Examples:**

```yaml
args:
  name: "hello"           # → ArgValue::Literal(String("hello"))
  count: 42               # → ArgValue::Literal(Integer(42))
  enabled: true           # → ArgValue::Literal(Bool(true))
  graph_ref: { ref: graph }  # → ArgValue::Reference("graph")
```

**Invalid reference targets** produce actionable error messages:

- `{ ref: "" }` — "ref value must not be empty"
- `{ ref: fn }` — "ref value 'fn' is a Rust reserved keyword"
- `{ ref: 123bad }` — "ref value '123bad' is not a valid identifier"
- `{ ref: 42 }` — "ref value must be a string identifier, not an integer"

**Supported YAML value forms** (summary):

- YAML booleans → Rust boolean literals.
- YAML integers → Rust integer literals.
- YAML floats → Rust float literals.
- YAML strings → Rust string literals (plain strings are always literals).
- YAML lists → `vec![...]` (future lowering).
- YAML maps → struct literals or explicit wrappers (future lowering).
- `{ ref: name }` → variable reference (explicit).
- `{ literal: "text" }` → explicit string literal (future).

### Error handling

`load_theorem_docs` and `load_theorem_docs_with_source` return
`Result<Vec<TheoremDoc>, SchemaError>`, where `SchemaError` has five variants:

- `Deserialize { message, diagnostic }` — YAML parsing or schema mismatch
  error.
- `InvalidIdentifier { identifier, reason }` — identifier validation failure.
- `InvalidActionName { action, reason }` — action name grammar or keyword
  validation failure.
- `ValidationFailed { theorem, reason, diagnostic }` — structural constraint
  violation (e.g., empty `Prove` section or no Evidence backend).
- `MangledIdentifierCollision { message }` — two or more different canonical
  action names produce the same mangled Rust identifier.

For parse and validation failures, `diagnostic` includes structured location
metadata when available:

- stable code (`schema.parse_failure` or `schema.validation_failure`),
- source identifier,
- line and column,
- deterministic fallback message.

Use `SchemaError::diagnostic()` to access this payload for custom rendering,
snapshot assertions, or editor integration.

All variants produce actionable error messages suitable for display to theorem
authors.

### Minimal example

```yaml
Theorem: DepositInvariant
About: Depositing into an account preserves the balance invariant.
Forall:
  amount: u64
Assume:
  - expr: "amount <= u64::MAX - balance"
    because: "prevent overflow"
Witness:
  - cover: "amount == 50"
    because: "mid-range deposit is exercised"
Prove:
  - assert: "new_balance == balance + amount"
    because: "deposit adds exactly the deposited amount"
Evidence:
  kani:
    unwind: 10
    expect: SUCCESS
```

## Action name mangling

The `theoremc::mangle` module provides deterministic, injective transformation
of canonical action names into Rust identifiers. Each canonical action name
(e.g., `account.deposit`) is mangled into a unique identifier that resolves
into the `crate::theorem_actions` module.

### Mangling a canonical action name

Use `mangle_action_name` to transform a validated canonical action name:

```rust
use theoremc::mangle::mangle_action_name;

let mangled = mangle_action_name("account.deposit");
assert_eq!(mangled.slug(), "account__deposit");
assert_eq!(mangled.hash(), "05158894bfb4");
assert_eq!(mangled.identifier(), "account__deposit__h05158894bfb4");
assert_eq!(
    mangled.path(),
    "crate::theorem_actions::account__deposit__h05158894bfb4",
);
```

The function assumes its input has already passed canonical action-name
validation. It does not re-validate.

### Mangling algorithm

The algorithm follows `docs/name-mangling-rules.md`:

1. **Segment escape**: replace each `_` in a segment with `_u`. ASCII letters
   and digits are unchanged.
2. **Action slug**: split the canonical name on `.`, escape each segment, and
   join the escaped segments with `__`.
3. **Hash suffix**: compute `blake3(canonical_name.as_bytes())` and take the
   first 12 lowercase hex characters.
4. **Mangled identifier**: `{slug}__h{hash12}`.
5. **Resolution path**: `crate::theorem_actions::{identifier}`.

### Building-block functions

The individual building blocks are also public for reuse:

- `segment_escape(segment)` — escapes underscores in a single segment.
- `action_slug(canonical_name)` — produces the escaped slug.
- `hash12(value)` — computes the 12-character blake3 hash prefix.

### Injectivity guarantee

The escaping rule ensures that different canonical action names always produce
different mangled identifiers. For example, `a.b_c` (slug: `a__b_uc`) and
`a_b.c` (slug: `a_ub__c`) produce distinct slugs because `_` is escaped to `_u`
while segment boundaries use `__`.

## Mangled-identifier collision detection

The `theoremc::collision` module provides build-time collision detection for
mangled action-name identifiers across loaded theorem documents. The check runs
automatically as part of `load_theorem_docs` and
`load_theorem_docs_with_source`.

### What is checked

The check detects **mangled-identifier collisions**: two or more different
canonical action names that produce the same mangled Rust identifier. This is a
defensive safety net; the mangling algorithm is injective by design, so a
collision should never occur with well-formed input.

Multiple theorems referencing the same canonical action name is expected and
accepted — only distinct canonical names that collide after mangling trigger an
error.

When a collision is detected, the loader returns
`Err(SchemaError::MangledIdentifierCollision { message })` with a
human-readable report listing all colliding canonical names per mangled
identifier.

### Calling the check directly

The collision check can also be called independently:

```rust
use theoremc::collision::check_action_collisions;
use theoremc::schema::load_theorem_docs;

let docs = load_theorem_docs(yaml)?;
// The check already ran inside load_theorem_docs, but it can be
// re-run after combining documents from multiple files:
check_action_collisions(&docs)?;
```
