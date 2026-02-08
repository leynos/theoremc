# Name mangling rules

This document defines the current mangling rules for action resolution and
generated Kani harness naming.

## Action name mangling

An action name in a `.theorem` file is a dot-separated path.

- Grammar: `Segment ("." Segment)+`
- Each `Segment` must match the ASCII identifier regex
  `^[A-Za-z_][A-Za-z0-9_]*$`.
- No segment may be a Rust reserved keyword. Reserved-keyword segments are an
  error.

### Resolution target

- Canonical action module: `crate::theorem_actions`
- Mangled Rust function identifier: join the segments with `__` (double
  underscore), without further transformation

The mapping is mechanical:

- `action: hnsw.attach_node` resolves to
  `crate::theorem_actions::hnsw__attach_node`.
- `action: account.deposit` resolves to
  `crate::theorem_actions::account__deposit`.
- `action: hnsw.graph.with_capacity` resolves to
  `crate::theorem_actions::hnsw__graph__with_capacity`.

No additional inference is applied during mangling. Compile-time binding and
typechecking provide the behavioural guarantees.

## Harness name mangling (Kani MVP)

Each theorem document generates a Kani proof harness function with a stable
name, inside a stable per-file module, inside a `kani` submodule.

Inputs:

- `P`: the literal path string passed to `theorem_file!("P")` (relative to the
  crate root)
- `T`: the theorem `Theorem:` identifier in the YAML document

### 1. Per-file module name

A per-file Rust module scopes all theorems defined in that file.

Definitions:

- `path_stem(P)`: `P` with a trailing `.theorem` removed, if present
- `path_mangle(S)`:
  1. Replace `/` and `\` with `__`.
  2. Replace any character not in `[A-Za-z0-9_]` with `_`.
  3. Collapse consecutive `_` to a single `_`.
  4. Lowercase the result.
  5. If the result starts with a digit, prefix `_`.
- `hash8(P)`: `blake3(P.as_bytes())`, taking the first eight hex characters of
  the digest

Per-file module name:

- `__theoremc__file__{path_mangle(path_stem(P))}__{hash8(P)}`

Example shape:

- `theorems/bidirectional.theorem` maps to
  `__theoremc__file__theorems__bidirectional__a1b2c3d4` (illustrative hash)

### 2. Backend submodule

Inside the per-file module, the Kani backend is emitted under `mod kani`, gated
by `#[cfg(kani)]`.

### 3. Harness function name

Within the Kani submodule, each theorem `T` generates one harness function:

- Harness function identifier: `theorem__{theorem_snake(T)}`

`theorem_snake` is defined as follows:

- If `T` already matches `^[a-z_][a-z0-9_]*$`, it is unchanged.
- Otherwise, convert UpperCamelCase to `snake_case` deterministically:
  1. Insert `_` between a lower-case letter or digit and an upper-case letter
     (`Path3` becomes `path_3`).
  2. Split acronym runs before the last capital when followed by lower-case
     text (`HNSWInvariant` becomes `hnsw_invariant`).
  3. Lowercase the final result.

Examples:

- `T = DepositWithdrawInverse` maps to
  `theorem__deposit_withdraw_inverse`.
- `T = BidirectionalLinksCommitPath3Nodes` maps to
  `theorem__bidirectional_links_commit_path_3_nodes`.
- `T = hnsw_smoke` maps to `theorem__hnsw_smoke`.

### 4. Full harness name (fully qualified)

The fully qualified harness path is:

- `__theoremc__file__{path_mangle(path_stem(P))}__{hash8(P)}::kani::theorem__{theorem_snake(T)}`

Example shape:

- `__theoremc__file__theorems__bidirectional__a1b2c3d4::kani::theorem__bidirectional_links_commit_path_3_nodes`

### 5. Uniqueness rule

- `Theorem:` identifiers must be unique across the crate theorem suite, so
  harness function identifiers remain unique in practice, even across multiple
  `.theorem` files.
- The per-file module hash exists to prevent collisions caused by path
  sanitization. It does not resolve theorem-name collisions.
