# User's guide

This guide covers the behaviour and application programming interface (API) of
the `theoremc` library from the perspective of a library consumer.

## Build discovery

`theoremc` now uses a root-level `build.rs` script to discover theorem files
under `theorems/**/*.theorem`.

The build-time contract is:

- theorem files are discovered recursively from the crate-root `theorems/`
  directory,
- discovered theorem paths are normalized to forward-slash crate-relative form
  and sorted deterministically, and
- editing a discovered `.theorem` file causes Cargo to rerun the build script
  on the next build.

The repository does not need a pre-seeded `theorems/` directory. On the
supported toolchain, theoremc watches the root `theorems` path even when it is
absent, so creating the directory and adding the first theorem later still
causes the next build to rerun the build script.

Only files ending in `.theorem` are treated as theorem inputs. However, the
root `theorems/` directory is watched so Cargo can notice newly created theorem
trees. As a result, changes elsewhere under that watched directory may still
rerun the build script even though non-`.theorem` files are not parsed or fed
into later theorem compilation steps.

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
- `ArgValue::RawSequence(values)` — a YAML sequence. During proof harness
  generation (Phase 3), sequences are recursively lowered to `vec![...]` macro
  expressions. Nested sequences, scalars, and references are supported.
- `ArgValue::RawMap(map)` — any YAML map that is not a single-key sentinel
  wrapper. During proof harness generation (Phase 3), maps are lowered to
  struct literals using the expected parameter type name. Field values are
  lowered recursively. Multi-key maps are never treated as wrappers, even when
  one of their keys is `ref` or `literal`.

**Semantic stability invariant:** adding a new `Let` binding can never silently
change the meaning of an existing argument that was previously a plain string.
A plain string `"x"` always decodes as `ArgValue::Literal(String("x"))`, even
if a binding named `x` exists. To reference a binding, use `{ ref: x }`.

**Examples:**

```yaml
args:
  name: "hello"              # → ArgValue::Literal(String("hello"))
  count: 42                  # → ArgValue::Literal(Integer(42))
  enabled: true              # → ArgValue::Literal(Bool(true))
  graph_ref: { ref: graph }  # → ArgValue::Reference("graph")
  label: { literal: "graph" }  # → ArgValue::Literal(String("graph"))
  opts: { timeout: 30 }     # → ArgValue::RawMap (future: struct literal)
```

**Invalid reference targets** produce actionable error messages:

- `{ ref: "" }` — "ref value must not be empty"
- `{ ref: fn }` — "ref value 'fn' is a Rust reserved keyword"
- `{ ref: 123bad }` — "ref value '123bad' is not a valid identifier"
- `{ ref: 42 }` — "ref value must be a string identifier, not an integer"

**Explicit literal wrappers** produce string literals when the value is a
string. Non-string values are rejected:

- `{ literal: "graph" }` —
  `ArgValue::Literal(LiteralValue::String("graph"))`.
- `{ literal: "" }` — `ArgValue::Literal(LiteralValue::String(""))` (empty
  string is valid).
- `{ literal: 42 }` — "literal value must be a string, not an integer".
- `{ literal: true }` — "literal value must be a string, not a boolean".

**Lowering limitations** (current implementation):

- **Nested maps** within composite values (maps inside lists, or maps as field
  values within other maps) are not yet supported because field type
  information requires Phase 3 compile-time type probes. Attempts to lower
  nested maps produce a clear error directing users to use explicit
  let-bindings for nested struct construction. Top-level map arguments and
  lists of scalars/references are fully supported.
- **Type shape restrictions**: Only simple type paths (`MyStruct`,
  `module::Type`) are supported as expected parameter types. Generic types,
  references, and tuple types require explicit handling and may produce
  unsupported type errors during lowering.

**Supported YAML value forms** (summary):

- YAML booleans → Rust boolean literals (`true`, `false`).
- YAML integers → Rust unsuffixed integer literals (`42`, not `42i64`).
- YAML floats → Rust unsuffixed float literals (`99.5`, not `99.5f64`).
- YAML strings → Rust string literals (`"hello"`). Plain strings are always
  literals.
- YAML lists → `vec![...]` macro expressions (lowered recursively during Phase
  3 harness generation). Nested lists, scalars, and references are supported.
  Empty lists are allowed (`vec![]`).
- YAML maps → Rust struct literals (lowered during Phase 3 harness generation
  using the expected parameter type name). Field values are lowered
  recursively. Unknown fields, missing fields, and type mismatches surface as
  Rust compilation errors, not theoremc validation errors.
- Single-key sentinel wrappers: `{ ref: name }` → `ArgValue::Reference`,
  `{ literal: "text" }` → `ArgValue::Literal`. All other YAML maps (including
  multi-key maps such as `{ literal: "x", other: 1 }`) pass through as
  `ArgValue::RawMap` for struct-literal lowering.

### Error handling

`load_theorem_docs` and `load_theorem_docs_with_source` return
`Result<Vec<TheoremDoc>, SchemaError>`, where `SchemaError` has six variants:

- `Deserialize { message, diagnostic }` — YAML parsing or schema mismatch
  error.
- `InvalidIdentifier { identifier, reason }` — identifier validation failure.
- `InvalidActionName { action, reason }` — action name grammar or keyword
  validation failure.
- `ValidationFailed { theorem, reason, diagnostic }` — structural constraint
  violation (e.g., empty `Prove` section or no Evidence backend).
- `MangledIdentifierCollision { message }` — two or more different canonical
  action names produce the same mangled Rust identifier.
- `DuplicateTheoremKey { theorem_key, collisions, diagnostic }` — two theorem
  documents loaded from the same source produce the same literal theorem key
  `{P}#{T}`, with structured collision diagnostics for each duplicate key.

For parse failures, validation failures, and duplicate theorem-key failures,
`diagnostic` includes structured location metadata when available:

- stable code (`schema.parse_failure` or `schema.validation_failure`),
- source identifier,
- line and column,
- deterministic fallback message.

Use `SchemaError::diagnostic()` to access this payload for custom rendering,
snapshot assertions, or editor integration. For duplicate theorem-key errors,
callers can also inspect
`SchemaError::DuplicateTheoremKey { theorem_key, collisions, diagnostic }`
directly to enumerate every colliding theorem key in stable order.

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

## Per-file module naming

The `theoremc::mangle` module provides per-file module naming for `.theorem`
file paths. Each path is transformed into a deterministic, collision-resistant
Rust module name of the form:

```plaintext
__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}
```

### Path mangling functions

- `path_stem(path) -> PathStem` — removes a trailing `.theorem` extension if
  present; otherwise returns `path` unchanged.
- `path_mangle(&PathStem)` — sanitizes a path stem into a Rust-identifier-safe
  fragment using the five-step algorithm from `docs/name-mangling-rules.md` §1:
  1. Replace `/` and `\` with `__`.
  2. Replace any character not in `[A-Za-z0-9_]` with `_`.
  3. Collapse consecutive `_` to a single `_`.
  4. Lowercase the result.
  5. If the result starts with a digit, prefix `_`.
- `hash12(path)` — computes the first 12 lowercase hex characters of the blake3
  digest of the **original** path string (not the mangled stem).

### Composite entry point

`mangle_module_path(path)` combines the building blocks and returns a
`MangledModule` struct with accessors:

```rust
use theoremc::mangle::mangle_module_path;

let m = mangle_module_path("theorems/bidirectional.theorem");
assert_eq!(m.stem(), "theorems/bidirectional");
assert_eq!(m.mangled_stem(), "theorems_bidirectional");
assert_eq!(m.hash(), "1fc14bdf614f");
assert_eq!(
    m.module_name(),
    "__theoremc__file__theorems_bidirectional__1fc14bdf614f",
);
```

### Collision resistance

Paths that differ only in characters lost during sanitization (e.g.,
`theorems/my-file.theorem` and `theorems/my_file.theorem`) produce the same
mangled stem but different module names because `hash12` operates on the
original path string. The 12-character blake3 hash suffix provides the real
disambiguator.

## Theorem harness naming

The `theoremc::mangle` module also provides deterministic theorem harness
naming for Kani proof functions. Each theorem document maps to a harness
identifier of the form:

```plaintext
theorem__{theorem_slug(T)}__h{hash12(P#T)}
```

Where:

- `P` is the literal theorem file path string supplied by the caller.
- `T` is the theorem identifier from the `Theorem` field.
- `theorem_key(P, T)` is the exact string `{P}#{T}`.
- `theorem_slug(T)` preserves identifiers already matching
  `^[a-z_][a-z0-9_]*$` and otherwise converts CamelCase deterministically,
  including acronym and digit boundaries.

### Harness naming helpers

- `theorem_key(path, theorem)` — returns the exact theorem key `{P}#{T}`.
- `theorem_slug(theorem)` — returns the deterministic harness slug.
- `mangle_theorem_harness(path, theorem)` — returns a `MangledHarness` with
  `theorem()`, `slug()`, `theorem_key()`, `hash()`, and `identifier()`
  accessors.

```rust
use theoremc::mangle::{hash12, mangle_theorem_harness, theorem_key, theorem_slug};

assert_eq!(
    theorem_key(
        "theorems/bidirectional.theorem",
        "BidirectionalLinksCommitPath3Nodes",
    ),
    "theorems/bidirectional.theorem#BidirectionalLinksCommitPath3Nodes",
);
assert_eq!(
    theorem_slug("BidirectionalLinksCommitPath3Nodes"),
    "bidirectional_links_commit_path_3_nodes",
);

let harness = mangle_theorem_harness(
    "theorems/bidirectional.theorem",
    "BidirectionalLinksCommitPath3Nodes",
);
assert_eq!(
    harness.identifier(),
    format!(
        "theorem__bidirectional_links_commit_path_3_nodes__h{}",
        hash12(&theorem_key(
            "theorems/bidirectional.theorem",
            "BidirectionalLinksCommitPath3Nodes",
        )),
    ),
);
```

### Duplicate theorem-key rejection

`load_theorem_docs_with_source` now rejects duplicate theorem keys before code
generation. In the current loader boundary this means a multi-document
`.theorem` source cannot declare the same `Theorem` identifier twice, because
both documents would produce the same literal theorem key `{source}#{Theorem}`.

The loader returns `SchemaError::DuplicateTheoremKey` with:

- the exact colliding theorem key,
- structured collision diagnostics naming every duplicate theorem-key
  occurrence in deterministic order, and
- a structured diagnostic pointing at the duplicate theorem field.
