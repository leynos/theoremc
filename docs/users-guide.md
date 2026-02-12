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
- At least one `Witness` entry is required unless
  `Evidence.kani.allow_vacuous` is `true`.

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
  permitted.
- `vacuity_because` (required when `allow_vacuous` is `true`): human-readable
  justification. Must be non-empty after trimming.

### Value forms in arguments

Action arguments accept:

- YAML booleans → Rust boolean literals.
- YAML integers → Rust integer literals.
- YAML strings → Rust string literals (plain strings are always literals).
- YAML lists → `vec![...]`.
- YAML maps → struct literals or explicit wrappers.
- `{ ref: name }` → variable reference (explicit).
- `{ literal: "text" }` → explicit string literal.

### Error handling

`load_theorem_docs` returns `Result<Vec<TheoremDoc>, SchemaError>` where
`SchemaError` has three variants:

- `Deserialize(String)` — YAML parsing or schema mismatch error.
- `InvalidIdentifier { identifier, reason }` — identifier validation failure.
- `ValidationFailed { theorem, reason }` — structural constraint violation
  (e.g., empty `Prove` section or no Evidence backend).

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
