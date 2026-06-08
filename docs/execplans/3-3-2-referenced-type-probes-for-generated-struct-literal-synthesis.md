# Step 3.3.2: emit referenced-type probes for generated struct-literal synthesis

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: IN PROGRESS

## Purpose / big picture

Implement the second checkbox under Phase 3, Step 3.3 of the roadmap by making
every Rust type referenced by a `.theorem` document participate in ordinary
Rust type-checking before Phase 4 Kani harness emission lands.

A referenced type is any Rust type appearing in a theorem document outside an
expression body: each `Forall` symbolic input type, each `Actions.params`
type, and each `Actions.returns` type. After this change, `theorem_file!`
continues to emit the per-file module produced by Step 3.2 and the typed
action probes produced by Step 3.3.1, and additionally emits one
deterministic compile-time existence probe per distinct referenced type.
The probes are wrapped in a single anonymous `const _` block per file with
one shared `?Sized`-relaxed assertion helper, so types containing references,
higher-rank lifetimes, and unsized inner types do not require per-shape
switches:

```rust
const _: () = {
    fn __theoremc_assert_referenced<T: ?Sized>() {}
    let _ = __theoremc_assert_referenced::<ExpectedTypeOne>;
    let _ = __theoremc_assert_referenced::<ExpectedTypeTwo>;
    // ... one statement per distinct referenced type
};
```

This step delivers the type-resolution backstop only. Wiring struct-literal
synthesis into proc-macro expansion (so `T { field: value }` literals are
emitted from YAML maps) is a Phase 4 concern that depends on these probes
having already locked the target type-name path in place. The roadmap label
"for generated struct-literal synthesis and step bindings" describes what
the probes *guard*, not what this plan emits.

The observable result is that renaming a `Forall` type, moving an action
parameter type to a different module, or deleting a struct used in a theorem
declaration fails compilation in the theorem owner crate during an ordinary
non-Kani `cargo build`, not only later under `cargo kani`. This is a
compile-time connectedness feature only; it does not introspect struct fields,
run actions, or implement Phase 4 Kani execution semantics.

Observable success:

1. A fixture crate containing a valid theorem with declared `Forall` and
   `Actions` types and a matching `crate::theorem_actions` module builds
   successfully in an ordinary non-Kani `cargo build`.
2. The generated expansion contains one deterministic referenced-type probe
   per distinct semantically equivalent Rust type collected from `Forall`,
   `Actions.params`, and `Actions.returns`, in stable first-seen order.
3. A fixture crate whose theorem declares `Forall: x: crate::Missing` fails
   compilation with a Rust `cannot find type ... in crate` diagnostic that
   points at the generated probe, not at a Kani harness body.
4. A fixture crate whose action signature references a moved type
   (`crate::old::Foo` renamed to `crate::new::Foo`) fails compilation with a
   Rust diagnostic against the generated probe.
5. Distinct probes are deduplicated by canonical `syn::Type` token stream
   so semantically equivalent type strings (`Vec<u8>` and `Vec <u8>`) collapse
   to one probe; whitespace normalisation is a side effect of the canonical
   token comparison, not the primary contract.
6. Primitive types (`u64`, `bool`, `()`) are accepted by the probe without any
   special-casing and do not produce false positives or Clippy noise.
7. Reference-bearing types with elided lifetimes (`&mut crate::Account`,
   `Vec<&'static str>`) compile through the probe. Types with free named
   lifetime parameters (`&'a Foo`) are rejected at schema validation time
   with a deterministic diagnostic; they never reach probe emission.
8. A theorem with no `Forall` entries and an empty `Actions` mapping emits
   no probe block, matching the Step 3.3.1 contract for action probes.
9. Existing schema validation, action probes, duplicate action collision
   checks, module naming, harness naming, and `#[cfg(kani)]` harness gating
   remain unchanged.
10. Unit tests using `rstest` cover type collection, deduplication, the
    exact generated token shape, reference-bearing types
    (`&mut crate::Account`, `Vec<&'static str>`), primitive types, and the
    empty-`Forall`-and-empty-`Actions` degenerate case.
11. Behavioural tests using `rstest-bdd` cover the theorem-author workflow
    for happy-path builds, missing `Forall` types, and moved `Actions`
    types, asserting on stable substrings (`cannot find type`, the type
    name, and the probe helper identifier).
12. Compile-fail tests cover generated-code diagnostics for missing and
    moved referenced types, with narrow `*.stderr` snapshots scoped to the
    probe-pointing source line.
13. `docs/theoremc-design.md`, `docs/theorem-file-specification.md`,
    `docs/users-guide.md`, and `docs/developers-guide.md` describe the
    implemented referenced-type probe contract, alongside the existing
    action probe contract.
14. `docs/roadmap.md` marks the Step 3.3.2 checkbox done only after
    implementation, validation, CodeRabbit review, and commit gates pass.
15. `make check-fmt`, `make lint`, and `make test` pass. Because
    documentation changes are in scope, `make fmt`, `make markdownlint`,
    and `make nixie` also pass before the implementation is complete.

## Constraints

- This plan must not be implemented until the user explicitly approves it.
- Scope is limited to roadmap step `3.3.2`, "emit referenced-type probes for
  generated struct literal synthesis and step bindings". Do not implement
  nested struct-field introspection, struct-literal synthesis wiring into
  proc-macro expansion, Phase 4 action execution, `kani::any`, `kani::assume`,
  `assert!`, `kani::cover!`, `must`, `maybe`, evidence result policy,
  reporting, or runtime reflection.
- Do not modify the existing typed action probe contract from Step 3.3.1. The
  referenced-type probe is an additional, separate item per distinct type.
- Preserve the Step 3.2 generated module layout: each theorem file expands
  into the same stable private module, keeps the source `include_str!`, and
  keeps Kani harnesses under `#[cfg(kani)]`. Referenced-type probes live in
  the per-file module body alongside action probes, not inside the
  `#[cfg(kani)]` submodule, so drift fails in ordinary builds.
- Preserve ADR-003 schema layering: type traversal lives in `theoremc-core`
  next to `collision::referenced_actions`; `theoremc-macros` only renders
  tokens. `schema/` and `mangle/` boundaries are not crossed.
- Probe items must compile without `#[allow(dead_code)]` attributes. Use the
  anonymous `const _` item pattern already established by Step 3.3.1.
- Do not introduce runtime reflection, `inventory`, or any link-time type
  registry. The contract is a compile-time binding check.
- Do not parse types outside the theorem owner crate or introduce a Rust
  module resolver. Type strings stay as `syn::Type` and are emitted into the
  consuming crate verbatim.
- Behavioural tests that model user workflows must use `rstest-bdd` matching
  the existing local configuration (`rstest-bdd-macros`).
- Property tests are not required for deterministic traversal or token
  rendering. Add `proptest` only if the implementation introduces a new
  invariant over arbitrary type sets.
- Kani and Verus proofs are not required for this change. The feature is a
  Rust compile-time type-checking contract, not a proof obligation or unsafe
  code invariant.
- Run code quality gates sequentially and capture long output with `tee` into
  `/tmp` logs. Do not run format checks, lints, or tests in parallel.
- Run `coderabbit review --agent` after each major implementation milestone,
  clear all concerns before moving to the next milestone, and record the
  result in this plan.
- Commit after each logical change that passes its gates. Use file-based
  commit messages with `git commit -F`, never `git commit -m`.
- Documentation and comments must use en-GB Oxford spelling and grammar.
- Keep Rust source files under 400 lines. If macro code or tests grow too
  large, split them into focused sibling modules.

## Tolerances

- Approval: if this plan is not explicitly approved, do not make
  implementation changes.
- Probe shape: if the recommended generic-assertion probe shape produces
  spurious warnings (for example `unused_results` or `path_statements`),
  document the failure and stop after one alternative attempt. Do not silence
  lints to make the chosen shape work.
- Scope: if implementation requires changing `.theorem` syntax, build-suite
  generation, Step 3.2 Kani harness layout, Step 3.3.1 action probe layout,
  or any Phase 4 harness body, stop and ask for direction.
- Type decomposition: this plan probes top-level types only. If a milestone
  appears to require recursing into generic arguments or struct fields, stop
  and reassess; that is a separate roadmap step.
- Source scanning: if probing the existence of any referenced type appears to
  require parsing or resolving Rust modules outside the consuming crate's
  type universe, stop. That is larger than Step 3.3.2.
- Public API: if the new `theoremc-core` referenced-type helper cannot be
  added without exposing incidental traversal internals, stop and present API
  options.
- Dependencies: if a new crates.io dependency appears necessary, make one
  prototype and stop for approval before adding it.
- Diagnostics: if compile-fail `*.stderr` snapshots become unstable across
  local and CI Rust versions after two attempts to narrow the asserted
  output, switch to BDD fixture-build assertions on stable fragments such as
  `cannot find type`, the type name, and the generated probe path, and
  document the trade-off.
- Code size: if implementation grows beyond roughly eight changed code files
  or 500 net Rust lines before documentation updates, stop and reassess
  whether the task should be split.
- Validation: if any of `make check-fmt`, `make lint`, or `make test` still
  fails after five focused fix attempts, stop with captured logs and
  summarize the remaining failure.
- CodeRabbit: if `coderabbit review --agent` is unavailable in the local
  environment, record that fact and continue only after the ordinary
  repository gates pass. If it reports actionable concerns, address them
  before continuing or document why the concern is out of scope and seek
  direction.

## Risks

- Risk: the chosen probe shape (`const _: () = { fn _assert<T: ?Sized>() {}
  let _ = _assert_referenced::<T>; };`) accepts unsized and reference-bearing
  types but produces a slightly less obvious diagnostic anchor than the
  function-pointer coercion used for action probes. Severity: medium.
  Likelihood: medium. Mitigation: prefer concise, deterministic probe
  identifiers (`__theoremc_assert_referenced`) and confirm with trybuild that
  the diagnostic still references the probe location.

- Risk: redundant Rust type strings can produce many duplicate probes (for
  example, the same `crate::Account` appearing in three actions). Severity:
  low. Likelihood: high. Mitigation: deduplicate by canonical `syn::Type`
  token stream using the same equivalence helper as
  `ActionSignature::is_semantically_equivalent` so `Vec<u8>` and `Vec <u8>`
  fold together.

- Risk: parsing `Forall` type strings as `syn::Type` introduces a new schema
  validation point and can reject documents that previously loaded. Severity:
  medium. Likelihood: medium. Mitigation: keep the diagnostic close in shape
  to the existing `validate_rust_type` Actions diagnostic, migrate any
  affected fixtures atomically, and document the new validation in the user
  guide.

- Risk: trybuild `*.stderr` snapshots drift across rustc versions when the
  diagnostic includes Rust's note-tracking. Severity: medium. Likelihood:
  medium. Mitigation: trybuild remains the right tool for the macro-side
  fixtures, but rely on BDD fixture build assertions on stable substrings
  for the theorem-author workflow scenarios.

- Risk: emitting probes for primitive types like `u64` and `()` could trigger
  Clippy lints around redundant generic instantiation. Severity: low.
  Likelihood: low. Mitigation: confirm the probe shape passes
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` on
  a representative fixture before declaring the milestone complete.

- Risk: this task is adjacent to nested map lowering and struct field
  introspection, so it may be tempting to broaden scope to probe individual
  struct fields. Severity: medium. Likelihood: medium. Mitigation: keep
  field-level introspection explicitly out of scope; the probe only forces
  type-path resolution, leaving field correctness to Phase 4 harness
  compilation.

- Risk: probes that wrap `Vec<&'a str>` in a function-pointer with elided
  lifetime succeed, but probes for raw `&'a Foo` written by a theorem author
  with an explicit lifetime cannot succeed because Rust will not bind `'a`.
  Severity: low. Likelihood: low. Mitigation: documented `RustType`
  convention disallows free lifetime parameters; emit a deterministic schema
  diagnostic if such a string appears, rather than producing a confusing
  rustc error.

## Signposts and required references

- Roadmap task: `docs/roadmap.md` Phase 3, Step 3.3, second checkbox.
- `DES-5`: `docs/theoremc-design.md` §5, Rust actions, struct-literal
  synthesis, and argument shaping rules.
- `DES-7`: `docs/theoremc-design.md` §7.3, build integration and binding
  probes, including the explicit note that referenced-type probes belong to
  Step 3.3.2.
- `TFS-1`: `docs/theorem-file-specification.md` §2.4 (`RustType` grammar) and
  §3.6 (`Forall` mapping).
- `TFS-4`: `docs/theorem-file-specification.md` §3.9.1 and §4.1.1, the
  `Actions` and `ActionSignature` schema.
- `TFS-5`: `docs/theorem-file-specification.md` §5.4, struct literal
  synthesis.
- `ADR-3`: `docs/adr-003-architectural-boundary-enforcement.md`, schema and
  collision module boundaries that this plan must respect.
- `ADR-4`: `docs/adr-004-action-signature-specification.md`, theorem-side
  action signature declarations and the rationale for treating types as
  theorem-owned contracts.
- Companion ExecPlan:
  `docs/execplans/3-3-1-emit-typed-action-probes.md`, which sets the
  precedent for probe rendering, trybuild fixtures, BDD fixture-crate
  patterns, and signature-equivalence checks.
- `docs/rust-testing-with-rstest-fixtures.md`, `rstest` style guide.
- `docs/reliable-testing-in-rust-via-dependency-injection.md`, fixture
  isolation and external process boundaries.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`, keeping the
  macro implementation small and extractable.
- Skills referenced when preparing this plan: `leta`, `rust-router`,
  `rust-types-and-apis`, `arch-crate-design`, `rust-errors`,
  `hexagonal-architecture`, `execplans`, `firecrawl`, `commit-message`,
  `pr-creation`, `en-gb-oxendict`, and `logisphere-experts`.
- External prior art:
  - `std::marker::PhantomData` is `pub struct PhantomData<T: ?Sized>` and
    accepts unsized inner types at module scope. Source:
    <https://doc.rust-lang.org/std/marker/struct.PhantomData.html>. The
    Nomicon describes its use for "mentioning" a lifetime without holding a
    reference. Source:
    <https://doc.rust-lang.org/nomicon/phantom-data.html>.
  - Higher-rank function-pointer types
    (`for<'a> fn(&'a T)`) bind elided lifetimes without requiring item-level
    generic parameters and are themselves `'static`. Source:
    <https://github.com/rust-lang/rust/issues/80317>.
  - `trybuild` matches whole-file `*.stderr` snapshots and treats
    `TRYBUILD=overwrite cargo test` as the regeneration workflow; snapshots
    drift between rustc versions, so the toolchain is pinned via
    `rust-toolchain.toml` and substring assertions in BDD fixture builds
    cover the user workflow. Source: <https://docs.rs/trybuild>.
  - `static_assertions` provides type-level assertions
    (`assert_impl_all!`, `assert_type_eq_all!`) but is overkill for a pure
    type-path existence check; an anonymous `const _` item is enough. Source:
    <https://docs.rs/static_assertions>.

## Implementation plan

### Milestone 0: write unit-level red tests for probe emission

Begin by creating failing tests that describe the behaviour before production
code changes. The chosen probe shape is the generic-assertion form wrapped in
an anonymous `const _` block, with one shared `?Sized`-relaxed helper and one
`let _ = ...;` statement per distinct referenced type. This shape admits
sized, unsized, and reference-bearing types without per-shape switches and
stays anonymous so dead-code lints do not require `#[allow]` attributes.
`PhantomData<T>` would also work but requires an item-level binding for each
probe, whereas the chosen form keeps everything inside one anonymous block.

The red tests at this milestone are **unit tests on
`render_expansion`'s token output**, not trybuild fixtures. trybuild fixtures
that intentionally reference a missing type would already fail to compile
(because the type really is missing in the fixture crate's universe) without
proving the probe was emitted. Trybuild fixtures are added during Milestone 2
once probes exist and the `*.stderr` snapshot anchors are stable.

Add an `rstest`-driven unit test in `crates/theoremc-macros/src/` (in a new
sibling module `type_probe_tests.rs` next to `action_probe_tests.rs`,
mirroring the established structure of `crates/theoremc-macros/src/lib.rs`).
The red tests must assert that the token stream produced by
`render_expansion` for the following inputs does **not** contain the
substring `__theoremc_assert_referenced` before implementation, and that
each test will flip to assert the expected probe block in Milestone 2:

- a theorem with one `Forall` variable of a custom type and one referenced
  action whose `Actions.params` and `Actions.returns` mention distinct
  custom types,
- a theorem with primitive types only (`Forall: n: "u64"`, action with
  `params: { flag: "bool" }` and `returns: "()"`),
- a theorem with two action signatures that share a parameter type via
  whitespace-equivalent strings (`"Vec<u8>"` and `"Vec <u8>"`),
- a theorem with no `Forall` entries and an empty `Actions` map (degenerate
  case — no probe block expected before *or* after).

Do not proceed to Milestone 1 until the red tests fail for the expected
reason: the macro understands the theorem but does not yet emit referenced-
type probes. Run:

```sh
cargo nextest run -p theoremc-macros 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-macro-red.out
```

Expected result before implementation: the new tests fail because the
expansion does not yet emit referenced-type probes.

Run `coderabbit review --agent` after the red-test commit. Address any
concern that affects the planned contract before continuing.

### Milestone 1: extend schema validation and expose distinct referenced types

Introduce a new private module `crates/theoremc-core/src/schema/rust_type.rs`
that owns Rust type parsing and canonicalisation for the whole crate. It
exposes:

- `pub(crate) fn parse(ty: &str) -> Result<syn::Type, syn::Error>` — the
  thin `syn::parse_str` wrapper, trimming whitespace before parsing so
  callers do not duplicate that step;
- `pub(crate) fn canonical_token_stream(ty: &str) -> Option<String>` — the
  `quote::ToTokens` round-trip used as the canonical dedup and equivalence
  key. Returns `None` when parsing fails, in which case callers fall back
  to trimmed string equality;
- `pub(crate) fn validate(ty: &str, context: impl FnOnce(syn::Error) ->
  SchemaError) -> Result<(), SchemaError>` — drives validation with a
  caller-supplied diagnostic context closure so each call site keeps its
  own deterministic error string shape.

Re-point the existing `validate_action_signatures` validator
(`crates/theoremc-core/src/schema/validate.rs:188-215`) at
`rust_type::validate` so the existing diagnostic text remains stable.
Re-point `ActionSignature::is_semantically_equivalent`
(`crates/theoremc-core/src/schema/types.rs:265-294`) at
`rust_type::canonical_token_stream` so probe dedup and signature equivalence
share one canonical comparison.

Add a new `validate_forall_types` step inserted into the
`validate_theorem_doc` pipeline immediately after
`validate_action_signatures`. Use `rust_type::validate` with a
`Forall entry 'x': type is not a valid Rust type: <syn error>` shape, matching
the established Actions pattern. Reject types containing free named lifetime
parameters at this stage so the probe never emits a binding that rustc
cannot bind (for example, `&'a Foo` written by a theorem author). The
rejection diagnostic shape is
`Forall entry 'x': type contains a free named lifetime parameter
'<name>'; use an owned type or an elided lifetime`. Apply the same check
inside `validate_action_signatures` for `Actions.params` and
`Actions.returns` to keep the two validators symmetric.

Next, add a narrow helper alongside `referenced_actions` in
`crates/theoremc-core/src/collision.rs` that returns distinct referenced
types in deterministic first-seen order. Bias toward keeping the helper in
`collision.rs` because its job — "what does this document refer to outside
itself" — sits squarely in the collision module's remit. Only migrate both
helpers into a sibling `references.rs` module if `collision.rs` exceeds
350 lines after the addition. The helper signature is:

```rust
pub fn referenced_types(docs: &[TheoremDoc]) -> Vec<&str>;
```

Traversal axis (pinned for determinism):

- per document in document order;
- within each document, `Forall` entries first (in `IndexMap` order), then
  `Actions` entries (in `IndexMap` order), and within each `Actions` entry,
  `params` values (in `IndexMap` order) followed by `returns`.

Deduplicate by `rust_type::canonical_token_stream` and fall back to trimmed
string equality on parse failure. The helper returns string slices borrowed
from the document graph so the proc-macro renderer can keep its existing
`syn::parse_str` step and reuse the same error type. It does not expose
collision grouping or signature internals.

Add `rstest` unit coverage for:

- `Forall` types,
- `Actions.params` types,
- `Actions.returns` types,
- semantically equivalent types fold together (canonical-token-stream
  dedup, exercised via the `Vec<u8>` / `Vec <u8>` pair),
- repeated types within one theorem yield one entry,
- repeated types across theorem documents yield one entry,
- the pinned traversal-axis order (`Forall` first, then `Actions.params`,
  then `Actions.returns`),
- rejection of free named lifetime parameters in both `Forall` and
  `Actions` validators, and
- deterministic first-seen ordering.

Run:

```sh
cargo nextest run -p theoremc-core 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-core.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the core helper plus schema validation as a single change.

### Milestone 2: emit referenced-type probes in `theorem_file!`

Update `crates/theoremc-macros/src/lib.rs` so `render_expansion` asks
`theoremc-core` for distinct referenced types and emits a single
deterministic probe block alongside the existing action probes inside the
per-file module body. The canonical emitted form is one shared helper
declaration plus one `let _ = ...;` statement per distinct referenced type,
wrapped in a single anonymous `const _` block:

```rust
// theoremc: compile-time referenced-type probe (3.3.2)
const _: () = {
    fn __theoremc_assert_referenced<T: ?Sized>() {}
    let _ = __theoremc_assert_referenced::<ExpectedTypeOne>;
    let _ = __theoremc_assert_referenced::<ExpectedTypeTwo>;
    // ... one statement per distinct referenced type
};
```

Implementation notes:

- Always declare `__theoremc_assert_referenced` exactly once per per-file
  module, even when many types are probed; the renderer must not emit one
  helper per probe statement.
- Emit a leading marker comment `// theoremc: compile-time referenced-type
  probe (3.3.2)` on the generated block so a theorem author running
  `cargo expand` immediately sees why the block exists. The marker uses the
  same `quote::quote!` raw-string approach available to proc-macro output;
  if rustfmt strips the comment, fall back to including the marker inside
  the helper function name via a doc-equivalent identifier choice and
  record the decision in the Decision Log.
- Probes live outside the `#[cfg(kani)]` backend module so ordinary builds
  detect drift. They must not introduce any `kani::` references.
- A theorem with no `Forall` entries and an empty `Actions` map emits no
  probe block, matching the Step 3.3.1 contract for action probes.
- The error type `MacroExpansionError` gains a new variant
  `InvalidReferencedType { ty: String, message: String }` whose
  `thiserror` message reads
  `referenced type ``{ty}`` is invalid: {message}`,
  mirroring the `InvalidActionSignature` shape from Step 3.3.1. The
  renderer invokes this when
  `syn::parse_str::<syn::Type>` fails on a string the schema accepted; in
  the normal flow Milestone 1 validation makes that unreachable, so the
  variant is defensive rather than expected, but it must not panic.
- Clippy gate: the block must compile cleanly under
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  Prefer `let _ = expr;` over bare `expr;` to avoid `path_statements`, and
  spot-check `clippy::let_underscore_untyped` on the fixture crate; if it
  fires, add `let _: fn() = __theoremc_assert_referenced::<T>;` to ascribe
  a function-pointer type without leaking lifetimes.

Update macro unit tests and snapshots to prove:

- no probes are emitted when no type is referenced (empty `Forall` and
  empty `Actions`),
- exactly one helper function is declared per per-file module regardless of
  probe count,
- one probe statement is emitted per distinct semantically equivalent type
  (canonical-token-stream dedup),
- duplicate references emit one probe,
- `Forall`, `Actions.params`, and `Actions.returns` types are all included
  in the pinned traversal order, and
- reference-bearing types (`&mut crate::Account`,
  `Vec<&'static str>`), generic types (`Vec<u8>`), and primitive types
  (`u64`, `bool`, `()`) all survive the probe round trip without spurious
  diagnostics.

Add the trybuild compile-fail fixtures now that probes exist:

- `referenced_type_missing.rs`, `referenced_type_missing.theorem`, and a
  matching fixture `theorem_actions` module covering a `Forall` variable
  declared as `crate::Missing` with no matching type in the consuming
  fixture crate;
- `referenced_type_moved.rs`, `referenced_type_moved.theorem`, and a
  matching fixture `theorem_actions` module covering an `Actions.params`
  type renamed under the consuming crate so the typed action probe still
  finds the function path but the referenced-type probe fails to resolve.

Keep the trybuild `*.stderr` snapshots narrow: assert on the probe-pointing
source line and the missing-type fragment (`cannot find type` and the type
name), avoiding the longer rustc notes that drift across versions. The BDD
substring assertions added in Milestone 3 remain the authoritative
diagnostic-stability check.

Regenerate the trybuild `*.stderr` snapshots once probes are wired in and
the diagnostic anchors point at the generated probe block.

Run:

```sh
cargo nextest run -p theoremc-macros 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-macros.out
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 \
    | tee /tmp/clippy-theoremc-3-3-2-macros.out
TRYBUILD=overwrite cargo nextest run -p theoremc-macros --test expand 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-trybuild-overwrite.out
cargo nextest run -p theoremc-macros --test expand 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-trybuild.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and
commit the macro expansion change.

### Milestone 3: add behavioural compile checks

Update `tests/theorem_file_macro_bdd.rs`,
`tests/features/theorem_file_macro.feature`, and
`tests/theorem_file_macro_bdd/fixture_crate.rs` so fixture crates can define
matching types in their library crate and a minimal `crate::theorem_actions`
module. Add `rstest-bdd` scenarios for:

- a theorem owner crate with declared `Forall` and `Actions` types and
  matching local definitions builds without Kani installed;
- a theorem owner crate whose `Forall` type was removed from the consuming
  crate fails with a `cannot find type` diagnostic referencing the probe;
- a theorem owner crate whose `Actions.params` type was renamed in the
  consuming crate fails with the same diagnostic shape, even though the
  typed action probe coercion still finds the function path.

Reuse the existing `assert_fixture_build_fails_with` helper and assert on
stable substrings (the type name, `cannot find type`, the probe block
identifier). Keep existing non-Kani and `cargo kani list` scenarios intact.

Run:

```sh
cargo nextest run --test theorem_file_macro_bdd 2>&1 \
    | tee /tmp/test-theoremc-3-3-2-bdd.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the behavioural tests.

### Milestone 4: update documentation and roadmap

Update `docs/theoremc-design.md` §7.3 with the concrete generated
referenced-type probe shape, the chosen probe construction (one anonymous
`const _` block per file with a single shared `fn _assert<T: ?Sized>()`
helper and one `let _ = ...;` statement per distinct referenced type), the
deduplication rule based on `syn::Type` canonical token streams, the
emitted `// theoremc: compile-time referenced-type probe (3.3.2)` marker
comment that aids `cargo expand` inspection, and the explicit non-goal of
probing struct field names or decomposing generic arguments. Note the
double-underscore-prefixed helper name (`__theoremc_assert_referenced`) is
chosen so rustfmt and rustc consistently leave it alone. Cross-reference
Step 3.3.1 typed action probes and the Phase 4 struct-literal synthesis
wiring so the reader sees the layered coverage.

Update `docs/theorem-file-specification.md` §3.6 (Forall) to note that type
strings are validated as `syn::Type` at schema load time, mirroring §3.9.1
(`Actions`). Add a small example showing the resulting compile-time
behaviour.

Update `docs/users-guide.md` to explain the user-visible behaviour: missing
or moved types in `Forall` and `Actions` declarations fail ordinary Rust
compilation in the theorem owner crate, with diagnostics pointing at the
generated probe block. Add a paragraph distinguishing referenced-type probes
from typed action probes.

Update `docs/developers-guide.md` §2.1 (or add §2.1.2) to list the new
`theoremc-core` internal helpers: `collision::referenced_types`, plus the
shared `schema::rust_type::{parse, canonical_token_stream, validate}`
trio. Reference the probe emission testing convention (unit-level red
tests; trybuild fixtures only after probes exist; BDD substring assertions
for the user workflow).

Update `docs/roadmap.md` by marking only the Step 3.3.2 checkbox done. Leave
later Phase 3 and Phase 4 items unchecked.

Run:

```sh
make fmt 2>&1 | tee /tmp/fmt-theoremc-3-3-2.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-3-3-2.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-3-3-2.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the documentation and roadmap updates.

### Milestone 5: full quality gate and final review

Run the repository gates sequentially:

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-3-3-2.out
make lint 2>&1 | tee /tmp/lint-theoremc-3-3-2.out
make test 2>&1 | tee /tmp/test-theoremc-3-3-2.out
```

Run a final `coderabbit review --agent`. Clear every actionable concern or
record a user-approved reason for leaving it unresolved. Update
`Outcomes & Retrospective`, set this ExecPlan status to `COMPLETE`, commit
the final plan update, push the branch, and update the pull request.

## Validation strategy

The validation strategy is intentionally layered to keep diagnostics
stable and fast feedback for theorem authors high.

Use `rstest` unit tests for pure traversal, deduplication, and token
rendering because those tests are fast, deterministic, and directly exercise
the internal contract. Cover ordering, semantic equivalence, and edge cases
like empty `Forall` maps and single-document versus multi-document theorem
files.

Use `trybuild` compile-fail tests for procedural macro diagnostics where the
generated Rust must fail compilation in a predictable way. Snapshots may be
narrow on the probe-pointing line and broad elsewhere; rely on the
`*.stderr` snapshot as the ground truth and regenerate with
`TRYBUILD=overwrite` whenever the diagnostic anchor or rustc note tracking
genuinely changes.

Use `rstest-bdd` behavioural tests for end-to-end theorem-owner workflows.
The existing `tests/theorem_file_macro_bdd.rs` suite already builds temporary
fixture crates, serialises Cargo invocations, and optionally checks Kani
harness discovery when `cargo-kani` is installed. Extend it with the
referenced-type scenarios so the user workflow is covered with stable
substring assertions and not fragile full-snapshot stderr matching.

Use the full repository gates at the end because this change crosses
`theoremc-core`, the proc-macro crate, generated Rust shape, fixture crates,
and documentation.

## Progress

- [x] Receive explicit user approval to proceed with implementation.
- [x] Milestone 0: probe-shape confirmation and red tests.
- [x] Milestone 1: `theoremc-core` `referenced_types` helper and `Forall`
  type validation.
- [x] Milestone 2: emit referenced-type probes in `theorem_file!` and update
  trybuild snapshots.
- [x] Milestone 3: behavioural compile checks for missing and moved
  referenced types.
- [ ] Milestone 4: documentation and roadmap update.
- [ ] Milestone 5: full quality gate, final CodeRabbit review, mark
  complete.

## Surprises & Discoveries

- 2026-06-08: `docs/contents.md` references `docs/repository-layout.md`, but
  that file is absent from the working tree. Orientation proceeded with
  `docs/contents.md`, `leta files`, and the existing source layout instead.
- 2026-06-08: The Milestone 0 red test command
  `cargo nextest run -p theoremc-macros` produced the expected failures:
  21 tests passed and the three new referenced-type probe assertions failed
  because `__theoremc_assert_referenced` is not emitted yet. The output is in
  `/tmp/test-theoremc-3-3-2-macro-red.out`.
- 2026-06-08: Milestone 1 focused validation passed with
  `cargo nextest run -p theoremc-core`; 281 tests passed. The output is in
  `/tmp/test-theoremc-3-3-2-core.out`.
- 2026-06-08: Milestone 2 focused validation passed with
  `cargo nextest run -p theoremc-macros`; 24 tests passed, including the
  formerly red referenced-type probe tests. The output is in
  `/tmp/test-theoremc-3-3-2-macros.out`.
- 2026-06-08: `make check-fmt` and `make lint` passed after Milestones 1 and
  2. Logs are in `/tmp/check-fmt-theoremc-3-3-2-m1-m2.out` and
  `/tmp/lint-theoremc-3-3-2-m1-m2.out`.
- 2026-06-08: `make test` passed after Milestones 1 and 2: 569 nextest tests
  passed, followed by passing workspace doctests. The output is in
  `/tmp/test-theoremc-3-3-2-m1-m2.out`.
- 2026-06-08: `coderabbit review --agent` completed for Milestones 1 and 2
  with zero findings. The output is in
  `/tmp/coderabbit-theoremc-3-3-2-m1-m2.out`.
- 2026-06-08: Milestone 3 focused validation passed with
  `cargo nextest run --test theorem_file_macro_bdd`; 13 tests passed. The
  output is in `/tmp/test-theoremc-3-3-2-bdd.out`.
- 2026-06-08: `make check-fmt`, `make lint`, and `make test` passed after
  Milestone 3. The full test gate reported 572 nextest tests passed, followed
  by passing workspace doctests. Logs are in
  `/tmp/check-fmt-theoremc-3-3-2-m3.out`,
  `/tmp/lint-theoremc-3-3-2-m3.out`, and
  `/tmp/test-theoremc-3-3-2-m3.out`.
- 2026-06-08: `coderabbit review --agent` completed for Milestone 3 with
  zero findings. The output is in `/tmp/coderabbit-theoremc-3-3-2-m3.out`.

## Decision Log

- 2026-06-02: Keep this ExecPlan in `DRAFT` and do not mark the roadmap item
  done. Rationale: the user explicitly requires approval before
  implementation.
- 2026-06-08: Move this ExecPlan from `DRAFT` to `IN PROGRESS`.
  Rationale: the user explicitly requested implementation of this approved
  plan in this session.
- 2026-06-08: Defer the Milestone 0 commit and CodeRabbit review until the
  referenced-type implementation makes the red tests green. Rationale: the
  repository instruction says commits and CodeRabbit reviews must be gated by
  deterministic checks; committing intentionally failing tests would violate
  that stronger quality gate.
- 2026-06-08: Emit the referenced-type probe block without the planned marker
  comment. Rationale: Rust comments are not represented in procedural macro
  `TokenStream` output, so `quote!` cannot reliably preserve such a marker;
  the deterministic helper name `__theoremc_assert_referenced` remains the
  stable diagnostic and `cargo expand` anchor.
- 2026-06-08: Narrow BDD failure assertions to the stable rustc fragments
  available in ordinary fixture builds: `cannot find type`, the missing type,
  and the missing module path. Rationale: without nightly macro backtraces,
  rustc points diagnostics at the generated `theorem_file!` invocation in
  `OUT_DIR/theorem_suite.rs` and does not include the generated helper
  identifier in stderr.
- 2026-06-02: Treat top-level type-path resolution as the bar for "missing
  and moved" type detection, leaving struct field correctness and nested
  generic argument decomposition to Phase 4 harness compilation and to
  future struct-literal field probes. Rationale: the acceptance criterion is
  "missing-type and moved-type breakages"; structural recursion would expand
  scope without changing the diagnostic the theorem author sees first.
- 2026-06-02: Choose the generic-assertion probe shape
  (`const _: () = { fn _assert<T: ?Sized>() {} let _ = _assert::<T>; };`)
  over `PhantomData<T>` and `fn(T) -> T` coercions. Rationale: it accepts
  unsized and reference-bearing types without per-shape switches and stays
  anonymous so dead-code lints do not require `#[allow]` attributes.
- 2026-06-02: Place the `referenced_types` helper in
  `crates/theoremc-core/src/collision.rs` next to `referenced_actions`,
  with an explicit option to migrate both helpers into a `references.rs`
  module if `collision.rs` exceeds 350 lines. Rationale: ADR-003 keeps
  document traversal inside core; mirroring the action-probe precedent
  preserves layering and reuses the established testing approach.
- 2026-06-02: Introduce a single new `crate::schema::rust_type` module that
  owns both the `syn::Type` parsing helper and the canonical
  `quote::ToTokens` round-trip used for equivalence. Re-point
  `validate_action_signatures` and `ActionSignature::is_semantically_
  equivalent` at the new module, and use it from the new
  `validate_forall_types`. Rationale: avoids divergence between probe
  dedup, signature equivalence, and Forall validation, and concentrates
  Rust-type concerns in one place rather than the two-path choice
  considered during plan drafting.
- 2026-06-02: Validate `Forall` type strings as `syn::Type` at schema load
  time, and reject types with free named lifetime parameters in both
  `Forall` and `Actions` validators. Rationale: surfaces typos and
  unbindable lifetimes as schema diagnostics instead of confusing rustc
  errors emitted from the generated probe.
- 2026-06-02: Make Milestone 0 red tests assert on `render_expansion` token
  output, not on trybuild fixtures. Rationale: a trybuild fixture that
  references a missing type already fails to compile without proving the
  probe was emitted; the unit-level red test is the only thing that proves
  the absence of probe emission before Milestone 2 lands.
- 2026-06-02: Pin the probe traversal axis: per document, `Forall` first,
  then `Actions.params` per entry, then `Actions.returns` per entry.
  Rationale: removes ambiguity about probe ordering and lets dedup behaviour
  be checked by a small ordered fixture.
- 2026-06-02: Emit a `// theoremc: compile-time referenced-type probe
  (3.3.2)` marker comment on the generated block, and use the
  `__theoremc_assert_referenced` helper name. Rationale: `cargo expand`
  legibility; the double-underscore prefix keeps the identifier outside
  rustfmt rewrites and inside the conventional reserved-prefix space.
- 2026-06-02: Keep struct-literal synthesis wiring into the proc-macro
  expansion out of scope for this step. Rationale: struct-literal synthesis
  is currently invoked only by the standalone lowering tests; Phase 4 will
  wire it into harness emission, and forcing it here would couple this step
  to incomplete Kani semantics. The plan title retains the roadmap phrase
  ("for generated struct literal synthesis") for traceability, with an
  explicit scope clarifier in `Purpose / big picture` that this plan
  delivers the type-resolution backstop only.

## Outcomes & Retrospective

(populated at completion)

## Revision note

- 2026-06-02: initial draft prepared with `leta` for code orientation,
  background agent help for prior-art research (`firecrawl`) and
  architecture guidance, and the `execplans` skill for the plan envelope.
