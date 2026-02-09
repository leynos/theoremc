# User's guide

This guide covers the behaviour and API of the `theoremc` library from the
perspective of a library consumer.

## Theorem document schema

A `.theorem` file is a UTF-8 text file containing one or more YAML documents.
Multiple documents within a single file are separated by `---`. Each document
describes one theorem.

### Loading theorem documents

Use `theoremc::schema::load_theorem_docs` to parse a `.theorem` file's
contents into a vector of `TheoremDoc` structs:

    use theoremc::schema::load_theorem_docs;

    let yaml = std::fs::read_to_string("theorems/my_theorem.theorem")?;
    let docs = load_theorem_docs(&yaml)?;

The function:

- Deserializes one or more YAML documents from the input string.
- Rejects unknown keys (any key not defined in the schema causes an error).
- Validates theorem identifiers and `Forall` keys against the identifier
  rules (see below).
- Returns `Err(SchemaError)` with an actionable message on failure.

### Top-level fields

Every theorem document is a YAML mapping with the following fields. Keys
use `TitleCase` canonically, but lowercase aliases are also accepted (e.g.,
`Theorem` or `theorem`).

| Field | Type | Required | Default | Notes |
| --- | --- | --- | --- | --- |
| `Schema` | integer | no | `None` (treated as 1) | Forwards compatibility. |
| `Theorem` | string | **yes** | — | Must be a valid identifier (see below). |
| `About` | string | **yes** | — | Human-readable description of intent. |
| `Tags` | list of strings | no | `[]` | Metadata for filtering and reporting. |
| `Given` | list of strings | no | `[]` | Narrative context (no codegen impact). |
| `Forall` | map (identifier → type) | no | `{}` | Symbolic quantified variables. |
| `Assume` | list of `Assumption` | no | `[]` | Constraints on symbolic inputs. |
| `Witness` | list of `WitnessCheck` | no | `[]` | Non-vacuity witnesses. |
| `Let` | map (identifier → `LetBinding`) | no | `{}` | Named fixtures. |
| `Do` | list of `Step` | no | `[]` | Theorem step sequence. |
| `Prove` | list of `Assertion` | **yes** | — | Proof obligations. |
| `Evidence` | `Evidence` | **yes** | — | Backend configuration. |

### Identifier rules

Theorem names and `Forall` map keys must satisfy:

- Match the ASCII pattern `^[A-Za-z_][A-Za-z0-9_]*$`.
- Must **not** be a Rust reserved keyword (`fn`, `let`, `match`, `type`,
  `self`, `Self`, `async`, `yield`, etc.).

Invalid identifiers produce an `InvalidIdentifier` error with a message
explaining why the identifier was rejected.

### Subordinate types

**Assumption**: a constraint on symbolic inputs.

    Assume:
      - expr: "amount <= u64::MAX"
        because: "prevent overflow"

**Assertion**: a proof obligation.

    Prove:
      - assert: "balance == expected"
        because: "deposit adds to balance"

**WitnessCheck**: a non-vacuity witness.

    Witness:
      - cover: "amount == 50"
        because: "mid-range deposit is exercised"

**LetBinding**: a named value binding. Must be one of `call` or `must`.

    Let:
      params:
        must:
          action: account.params
          args: { max_balance: 1000 }
      result:
        call:
          action: account.deposit
          args: { account: { ref: a }, amount: { ref: amount } }

**Step**: an element of the `Do` sequence. Must be one of `call`, `must`, or
`maybe`.

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

**ActionCall**: an invocation of a theorem action.

- `action` (required): dot-separated action name (e.g., `account.deposit`).
- `args` (required): mapping of parameter name to value.
- `as` (optional): binding name for the return value.

**Evidence**: backend configuration. Currently supports `kani`, with
`verus` and `stateright` as placeholders.

    Evidence:
      kani:
        unwind: 10
        expect: SUCCESS

**KaniEvidence** fields:

- `unwind` (required): positive integer (loop unwinding bound).
- `expect` (required): one of `SUCCESS`, `FAILURE`, `UNREACHABLE`, or
  `UNDETERMINED`.
- `allow_vacuous` (optional, default `false`): whether vacuous success is
  permitted.
- `vacuity_because` (required when `allow_vacuous` is `true`): human-readable
  justification.

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
`SchemaError` has two variants:

- `Deserialize(String)` — YAML parsing or schema mismatch error.
- `InvalidIdentifier { identifier, reason }` — identifier validation failure.

Both variants produce actionable error messages suitable for display to
theorem authors.

### Minimal example

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
