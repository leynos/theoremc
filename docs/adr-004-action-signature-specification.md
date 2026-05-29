# Architectural Decision Record (ADR) 004: theorem-side action signatures

- Status: accepted
- Date: 2026-05-25
- Deciders: theoremc maintainers
- Technical story: compile-time action probes and action argument typing

## Context

The theoremc design requires generated Rust code to check that every action
named in a `.theorem` file still resolves to a compatible function in
`crate::theorem_actions`. The intended probe shape is:

```rust
let _: fn(ExpectedArg1, ExpectedArg2) -> ExpectedReturn =
    crate::theorem_actions::mangled_action_identifier;
```

This requires theoremc to know the expected parameter order, parameter types,
and return type before Rust type checking. The current schema records action
names and argument values, but not action signatures.

Rust procedural macros operate on token streams and emit token streams during
compilation. They do not receive type-checked information about arbitrary paths
such as `crate::theorem_actions::hnsw__attach_unode__h...`.[^1] Link-time
registries such as `inventory` and `linkme` are useful for collecting metadata
or typed static elements after compilation and linking, but they are not a
source of data that a `theorem_file!` macro can inspect while expanding.[^2][^3]

Local experiments confirmed the practical boundary:

- `let _: fn(_) -> _ = action;` compiles, but Rust infers the placeholders from
  the current action function, so the probe does not detect signature drift.
- `let _: fn(graph: &mut Graph, node: NodeId) -> Result<(), AttachError> =
  action;` compiles when the function matches and fails with `E0308
  ` when a parameter or return type drifts.

The missing design decision is therefore not the probe mechanism. The missing
decision is where the expected signature comes from.

## Goals and Non-Goals

### Goals

- Detect action signature drift at ordinary Rust compile time.
- Give theorem documents a stable, theorem-owned type contract without relying
  on proc-macro access to rustc type information.
- Provide parameter names, parameter order, and parameter types for future
  argument shaping and Kani action lowering.
- Keep `theorem_file!` bounded and deterministic: parse theorem data, mangle
  action names, and emit Rust probes without scanning owner-crate source code.

### Non-Goals

- Do not infer signatures from Rust action implementations during macro
  expansion.
- Do not make `#[theorem_action]` mandatory; it remains an optional metadata
  and reporting hook.
- Do not design shared signature manifests, imports, or cross-file signature
  de-duplication in this ADR.
- Do not introduce runtime reflection or runtime action registries.

## Decision

### 1. Do not infer theorem action signatures from Rust action implementations

Theoremc will not infer action signatures by scanning Rust source, resolving
`pub use` graphs, reading generated attribute output, or asking rustc for type
information during `theorem_file!` expansion.

Inference from the current Rust implementation cannot prove drift from a stable
theorem contract. If the generated probe's expected type is derived from the
function being checked, the probe merely restates the current code.

### 2. Add theorem-side action signature declarations

Each theorem document may declare the expected signatures for the actions it
references in a top-level `Actions` mapping. For Step 3.3.1 and later action
lowering work, every action referenced by that document's `Let` and `Do`
sections must have an `Actions` entry.

The declaration is keyed by canonical action name:

```yaml
Actions:
  hnsw.attach_node:
    params:
      graph: "&mut crate::hnsw::Graph"
      node: "crate::hnsw::NodeId"
    returns: "Result<(), crate::hnsw::AttachError>"
```

Rules:

- `params` is an insertion-ordered mapping from parameter identifier to
  `RustType`.
- The order of `params` is the generated function-pointer parameter order.
- Parameter identifiers must be valid theorem identifiers and must match the
  keys accepted in `args`.
- Each parameter type and the return type must parse as `syn::Type`.
- `returns` defaults to `()` when omitted.
- The action name key must satisfy the canonical action-name grammar already
  defined for `ActionCall.action`.
- If the same action is declared more than once within one
  `theorem_file!` expansion, all declarations must be identical.
- Different `.theorem` files may temporarily declare different expectations
  for the same action. Ordinary Rust type checking will then reject any file
  whose expected signature no longer matches the exported function.

The theorem document is the source of the expected contract. The Rust action
implementation remains the executable behaviour.

### 3. Generate probes from declared signatures

For every distinct referenced action, `theorem_file!` will build a bare
function pointer type from the corresponding `Actions` declaration and assign
the resolved action function to it:

```rust
const _: fn(graph: &mut crate::hnsw::Graph, node: crate::hnsw::NodeId)
    -> Result<(), crate::hnsw::AttachError> =
    crate::theorem_actions::hnsw__attach_unode__h3f6b2a80c9d1;
```

Each probe is emitted as an anonymous `const _` item rather than a wrapper
function. The compiler still type-checks the coercion, but the `_` name skips
dead-code lints without an `#[allow]` attribute.

Rust permits names in bare function pointer parameter types and coerces
function items to compatible `fn` pointers.[^4] This gives deterministic
diagnostics for missing exports and `E0308` type mismatches without runtime
reflection.

### 4. Keep `#[theorem_action]` optional

The optional `#[theorem_action("...")]` macro remains a documentation,
metadata, and future reporting hook. It is not the source of expected action
signatures for `theorem_file!`.

A future enhancement may let the attribute macro check that an action's
canonical name matches its mangled export or submit metadata through
`inventory`, but that does not replace theorem-side signatures.

## Consequences

Positive consequences:

- Signature drift is checked against a stable theorem-owned contract.
- The macro implementation remains bounded; it parses theorem data and emits
  Rust, but does not become a Rust module resolver.
- The same signature data supports future argument shaping because parameter
  names, order, and types are available before code generation.
- Missing action signatures become schema diagnostics instead of weak generated
  Rust.

Costs and trade-offs:

- `.theorem` files become more explicit and slightly less storyboard-only.
- Authors must keep signature declarations synchronized with intentional Rust
  API changes.
- Type paths in theorem files may need qualification, for example
  `crate::hnsw::Graph`, to compile from the generated module context.
- Shared actions referenced by many files may duplicate signature declarations
  until a later shared manifest or import mechanism is designed.

## Alternatives considered

### Infer signatures from `crate::theorem_actions`

Rejected. A function-like procedural macro cannot obtain rustc's resolved type
for an arbitrary path during expansion. Source scanning would also need to
follow `pub use` graphs, module layout, conditional compilation, generics, and
imports. More importantly, deriving the expected type from the current action
implementation would not detect drift.

### Use `fn(_) -> _` placeholder probes

Rejected. Rust accepts the syntax, but infers the placeholders from the
right-hand side. This proves reachability of the current action function, not
compatibility with a stable expected signature.

### Generate signature aliases from `#[theorem_action]`

Rejected as the source for Step 3.3.1. If the alias is generated from the
current function signature, it changes when the function changes and cannot
detect drift. If the attribute requires an explicit signature, the macro still
cannot inspect that alias during `theorem_file!` expansion for argument
shaping. It may remain useful as an additional implementation-side assertion.

### Use `inventory` or `linkme`

Rejected for compile-time probe generation. These crates provide useful
distributed registration patterns. `inventory` collects plugin values from
linked source files, and `linkme` gathers static slice elements through the
linker. They are not readable by an expanding `theorem_file!` macro, and they
do not supply theorem-owned expected signatures.

### Generate a manifest in `build.rs`

Deferred. A build-generated manifest could become a future ergonomics layer,
especially if it validates explicit action declarations and writes a
`theorem-actions.generated.yaml` file. It is not the Step 3.3.1 source because
generating the manifest from current Rust actions repeats the inference
problem, while adding a checked-in manifest introduces another file discovery
and synchronization design.

### Follow Cucumber-style executable step definitions only

Rejected for theoremc's compile-time drift contract. Cucumber step definitions
bind prose steps to executable methods and transform captures into method
arguments.[^5] That is good prior art for keeping behaviour in ordinary code,
but theoremc also needs a document-owned type contract before generating Rust
harnesses and probes.

## Related documents

- `docs/theoremc-design.md`
- `docs/theorem-file-specification.md`
- `docs/name-mangling-rules.md`
- `docs/execplans/3-3-1-emit-typed-action-probes.md`

[^1]: <https://doc.rust-lang.org/reference/procedural-macros.html>
[^2]: <https://docs.rs/inventory>
[^3]: <https://docs.rs/linkme/latest/linkme/struct.DistributedSlice.html>
[^4]: <https://doc.rust-lang.org/reference/types/function-pointer.html>
[^5]: <https://cucumber.io/docs/cucumber/step-definitions/>
