# Theorem file format

Decision context for symbol stability and vacuity policy is recorded in
[Architecture Decision Record (ADR) 0001](adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md).

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

These rules are *normative* for v1:

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

Examples: `"u64"`, `"crate::account::Account"`, `"std::sync::Arc<MyType>"`,
`"Option<&'static str>"`.

______________________________________________________________________

## 3. Top-level schema: `TheoremDoc`

A theorem document is a YAML mapping with these fields.

### 3.1 `Schema`

- Type: integer
- Optional
- Default: `1`
- Purpose: forwards compatibility for future schema changes.

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

### 3.10 `Prove` (required)

- Type: list of `Assertion`
- Must be non-empty.

Each `Assertion` is a mapping:

- `assert` (required): `RustExpr` (must parse)
- `because` (required): non-empty string

Example:

```yaml
Prove:
  - assert: "hnsw.is_bidirectional(&graph)"
    because: "bidirectional invariant holds after commit-path reconciliation"
```

Semantics (Kani): emits `assert!(<expr>, "<because>");`.

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
section summarises those rules for convenience within the schema document.

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

This rule gives compile-time binding without relying on link-time registries
(`inventory` remains useful for reporting, but is not required for resolution).
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

This is not “extra design”; it is the schema expressed in the shape actually
deserialized into.

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

    #[serde(rename = "Prove", alias = "prove")]
    pub prove: Vec<Assertion>,

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
    pub args: indexmap::IndexMap<String, serde_saphyr::Value>, // or your own Value type
    #[serde(default)]
    pub as_: Option<String>,
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
