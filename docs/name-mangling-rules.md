# Name mangling rules

This document defines the current mangling rules for action resolution,
generated Kani harness naming, and stable external theorem identifiers.
Decision context is recorded in
[Architecture Decision Record (ADR) 0001](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md).

## Action name mangling

An action name in a `.theorem` file is a dot-separated path.

- Grammar: `Segment ("." Segment)+`
- Each `Segment` must match the ASCII identifier regex
  `^[A-Za-z_][A-Za-z0-9_]*$`.
- No segment may be a Rust reserved keyword. Reserved-keyword segments are an
  error. The keyword set follows the
  [Rust language reference](https://doc.rust-lang.org/reference/keywords.html).

### Resolution target

- Canonical action module: `crate::theorem_actions`
- Canonical action name: the original dot path, for example
  `hnsw.attach_node`
- Mangled Rust function identifier:
  `{action_slug(canonical_name)}__h{hash12(canonical_name)}`

### Mangling algorithm

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

Examples:

- `action: hnsw.attach_node` resolves to
  `crate::theorem_actions::hnsw__attach_unode__h3f6b2a80c9d1` (illustrative
  hash).
- `action: account.deposit` resolves to
  `crate::theorem_actions::account__deposit__h5a197f4ee18c` (illustrative hash).
- `action: hnsw.graph.with_capacity` resolves to
  `crate::theorem_actions::hnsw__graph__with_ucapacity__h8d6a20f2c44e`
  (illustrative hash).

This rule is intentionally mechanical and injective for valid input names.

### Collision checks

- Build-time generation must detect duplicate canonical action names and report
  each source location.
- Build-time generation must detect duplicate mangled identifiers, report all
  colliding canonical names, and fail compilation.
- Build-time binding must verify that every resolved identifier exists in
  `crate::theorem_actions`.

## Harness name mangling (Kani MVP)

Each theorem document generates a Kani proof harness function with a stable
name, inside a stable per-file module, inside a `kani` submodule.

Inputs:

- `P`: the literal path string passed to `theorem_file!("P")`, relative to the
  crate root
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
- `hash12(P)`: `blake3(P.as_bytes())`, taking the first 12 lowercase hex
  characters of the digest

Per-file module name:

- `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}`

Example shape:

- `theorems/bidirectional.theorem` maps to
  `__theoremc__file__theorems__bidirectional__a1b2c3d4e5f6` (illustrative hash).

### 2. Backend submodule

Inside the per-file module, the Kani backend is emitted under `mod kani`, gated
by `#[cfg(kani)]`.

### 3. Harness function name

Within the Kani submodule, each theorem `T` generates one harness function:

- Harness function identifier:
  `theorem__{theorem_slug(T)}__h{hash12(theorem_key(P, T))}`

Definitions:

- `theorem_key(P, T)`: the exact string `{P}#{T}`
- `theorem_slug(T)`:
  - If `T` matches `^[a-z_][a-z0-9_]*$`, keep it unchanged.
  - Otherwise, convert UpperCamelCase to `snake_case` deterministically:
    1. Insert `_` between a lower-case letter or digit and an upper-case
       letter.
    2. Split acronym runs before the last capital when followed by lower-case
       text.
    3. Lowercase the final result.

Examples:

- `T = DepositWithdrawInverse` maps to
  `theorem__deposit_withdraw_inverse__h4cd2a0b6157e` (illustrative hash).
- `T = BidirectionalLinksCommitPath3Nodes` maps to
  `theorem__bidirectional_links_commit_path_3_nodes__h8a912bd34fc1`
  (illustrative hash).
- `T = hnsw_smoke` maps to
  `theorem__hnsw_smoke__h7f11e8ddab40` (illustrative hash).

### 4. Full harness name (fully qualified)

The fully qualified harness path is:

- Path template:

```plaintext
__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}::kani::theorem__{theorem_slug(T)}__h{hash12(theorem_key(P, T))}
```

### 5. Uniqueness and collision checks

- `Theorem:` identifiers must still be unique across the crate theorem suite.
- Harness symbol uniqueness does not rely on snake-case uniqueness; hash suffix
  enforces distinct symbols for distinct theorem keys.
- Build-time generation must still detect and fail on duplicate
  `theorem_key(P, T)` values.

## Stable external theorem identifiers

Reporting and CI use a stable external theorem ID that is distinct from Rust
symbol names.

### Canonical external ID

- Canonical form: `{normalized_path(P)}#{T}`
- `normalized_path(P)` rules:
  1. Use `/` as the separator.
  2. Remove any leading `./`.
  3. Preserve case.

Example:

- `theorems/bidirectional.theorem#BidirectionalLinksCommitPath3Nodes`

### Migration rules for moves and renames

When a theorem file path or theorem name changes, add alias entries to
`theorems/theorem-id-aliases.yaml`.

- File format: mapping of old canonical ID to new canonical ID.
- Alias targets must be canonical IDs or other aliases that eventually resolve
  to a canonical ID.
- Alias graphs must be acyclic; cycles are a build-time error.
- Every deprecated ID must resolve deterministically to exactly one canonical
  ID.

Example:

```yaml
theorems/old/bidirectional.theorem#BidirectionalLinksCommitPath3Nodes: theorems/bidirectional.theorem#BidirectionalLinksCommitPath3Nodes
theorems/bidirectional.theorem#BidirectionalLinksPath3: theorems/bidirectional.theorem#BidirectionalLinksCommitPath3Nodes
```
