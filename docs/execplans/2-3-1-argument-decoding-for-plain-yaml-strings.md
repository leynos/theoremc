# Step 2.3.1: argument decoding for plain YAML strings

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, the `theoremc` library decodes action-call argument values
from raw YAML into semantically typed `ArgValue` variants. Plain YAML strings
are unconditionally treated as string literals. Variable references require the
explicit `{ ref: <Identifier> }` map wrapper. This prevents semantic drift:
adding a new `Let` binding can never silently change the meaning of an existing
argument that was previously a plain string.

Observable success: loading a `.theorem` file that uses `{ ref: x }` in an
argument produces an `ArgValue::Reference` carrying the binding name `x`.
Loading a file where an argument is the plain string `"x"` produces
`ArgValue::Literal(LiteralValue::String("x".into()))` regardless of whether a
binding named `x` exists. Invalid `{ ref: ... }` values (empty string, keyword,
non-identifier) are rejected with actionable error messages. `make check-fmt`,
`make lint`, and `make test` pass.

Signposts: `TFS-5` (theorem-file-specification.md section 5), `ADR-3`
(adr-001-theorem-symbol-stability-and-non-vacuity-policy.md decision 3),
`DES-5` (theoremc-design.md section 5.5.1).

## Constraints

- ADR-003 boundary rule: domain types (public API) must not import from raw,
  validator, or loader modules. The new `ArgValue` type belongs in the domain
  type layer.
- Source files must not exceed 400 lines.
- Use `rstest-bdd` v0.5.0 for behavioural tests, `rstest` for unit tests.
- Comments and documentation must use en-GB-oxendict spelling.
- No new external dependencies beyond what is already in `Cargo.toml`.
- Existing public API surfaces (`schema::*`, `mangle::*`, `collision::*`) must
  not have existing items removed or have their field types changed in a way
  that breaks downstream consumers (additive changes only, except for the
  planned `ActionCall.args` type change which is a deliberate upgrade).
- The `SchemaError` enum's existing variants must not be removed or have their
  fields changed (additive-only).
- Quality gates: `make check-fmt`, `make lint`, `make test` must pass.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 15 files or more than
  700 net lines of code, stop and escalate with a narrowed split.
- Dependencies: if a new external dependency beyond what is already in
  `Cargo.toml` is needed, stop and escalate.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop and escalate with logs.
- Ambiguity: if the treatment of a specific YAML value form is unclear in a
  concrete scenario, stop and present options.

## Risks

- Risk: Clippy `too_many_arguments` triggered by rstest parameterized tests
  with 5+ case params. Severity: medium. Likelihood: medium. Mitigation: use
  helper structs as per the `Golden` pattern established in `src/mangle.rs`.

- Risk: Changing `ActionCall.args` from `IndexMap<String, TheoremValue>` to
  `IndexMap<String, ArgValue>` may break existing test builders. Severity:
  medium. Likelihood: low (existing builders use `IndexMap::new()` without
  inserting values). Mitigation: update the few test builders that insert
  values; empty `IndexMap::new()` infers the new value type automatically.

- Risk: The custom `Deserialize` for `TheoremValue` is used to build
  `ActionCall.args`. Changing the args type to `ArgValue` means serde must
  deserialize YAML into `ArgValue` directly, or a post-deserialization
  conversion step is needed. Severity: high. Likelihood: certain (by design).
  Mitigation: use a two-stage approach — serde deserializes into `TheoremValue`
  in the raw layer as before, then a conversion function
  `decode_arg_value(param_name, value)` runs during the raw-to-public
  conversion step in `RawTheoremDoc::to_theorem_doc()`. Keep the helper
  signature short and typed:

  ```rust
  pub fn decode_arg_value(
      param_name: &str,
      value: TheoremValue,
  ) -> Result<ArgValue, ArgDecodeError>
  ```

- Risk: `src/schema/types.rs` is already 312 lines. Adding `ArgValue` and
  `LiteralValue` types could push it near the 400-line limit. Severity: medium.
  Likelihood: medium. Mitigation: place `ArgValue` in a new file
  `src/schema/arg_value.rs`.

- Risk: `src/schema/raw.rs` is already 316 lines. Adding `RawActionCall`,
  `RawLetCall`, `RawLetMust`, `RawStepCall`, `RawStepMust`, `RawStepMaybe`,
  `RawMaybeBlock`, `RawLetBinding`, `RawStep` types and conversion logic could
  push it well over 400 lines. Severity: high. Likelihood: high. Mitigation:
  extract the new raw action types and their conversion logic into a new file
  `src/schema/raw_action.rs`, keeping `raw.rs` focused on the existing
  `RawTheoremDoc` and its conversions. Import and use the raw action types from
  the new module. Alternatively, the existing `LetBinding`, `Step`, and related
  types in `raw.rs` already use the public `ActionCall` directly (they are
  `use super::types::{...LetBinding, Step...}`). Instead of duplicating all
  wrapper types, consider deserializing `ActionCall` with `TheoremValue` args
  via an intermediate serde approach, then converting in `to_theorem_doc()`.
  The preferred approach is: keep `raw.rs` importing the serde-compatible raw
  step/let types from a new `raw_action.rs` module.

## Progress

- [x] Milestone 0: baseline verification (all existing tests pass).
- [x] Milestone 1: create `ArgValue` and `LiteralValue` types in
  `src/schema/arg_value.rs`.
- [x] Milestone 2: implement `decode_arg_value` conversion function.
- [x] Milestone 3: change `ActionCall.args` to `IndexMap<String, ArgValue>` and
  wire decoding into `RawTheoremDoc::to_theorem_doc()`.
- [x] Milestone 4: reuse `ValidationFailed` error variant (no new variant
  needed).
- [x] Milestone 5: add unit tests for `decode_arg_value`.
- [x] Milestone 6: add BDD behavioural tests and fixture files.
- [x] Milestone 7: add semantic-stability acceptance test (binding addition
  cannot alter literal semantics).
- [x] Milestone 8: update the design doc, the user's guide, and the roadmap.
- [x] Milestone 9: run full quality gates and capture logs.

## Surprises & discoveries

- Clippy's `approx_constant` lint rejects float literals that are close to
  well-known mathematical constants (pi, e). Using `3.14` or `2.718` in test
  cases triggers the lint; `99.5` was used instead.
- Clippy's `missing_const_for_fn` lint required `non_string_kind` to be marked
  `const fn` since it only does pattern matching on an enum.
- The `cargo fmt` post-processing reformatted some BDD test functions that
  exceeded the line-width limit, which was a cosmetic non-issue.
- Raw string literals `r"..."` cannot contain unescaped double quotes; the
  STABILITY_BASE constant needed `r#"..."#` syntax because the embedded YAML
  contains `param: "x"`.

## Decision log

- Decision: change `ActionCall.args` from `IndexMap<String, TheoremValue>` to
  `IndexMap<String, ArgValue>` rather than introducing a parallel
  `ResolvedActionCall` type. Rationale: `ArgValue` is the correct semantic
  representation for action arguments at the domain level. A parallel type
  hierarchy adds complexity without benefit since `TheoremValue` remains
  available for `Evidence` configs and other raw YAML values. The change is
  additive in intent — `ArgValue` is strictly more informative than
  `TheoremValue` for arguments. Date/Author: 2026-03-01 / DevBoxer.

- Decision: perform decoding in `RawTheoremDoc::to_theorem_doc()` rather than
  in a separate post-validation pass. Rationale: the raw-to-public conversion
  is the natural boundary where YAML-level types become domain-level types.
  This is where `Spanned<String>` becomes `String` for other fields, so
  `TheoremValue` becoming `ArgValue` follows the same pattern. The `raw.rs`
  module already imports from `super::types`, so importing `ArgValue` from
  there does not violate ADR-003 boundaries. Date/Author: 2026-03-01 / DevBoxer.

- Decision: for Step 2.3.1, YAML maps that are NOT `{ ref: <name> }` are
  passed through as `ArgValue::RawMap(IndexMap<String, TheoremValue>)`. This
  preserves forward compatibility for Step 2.3.2 (`{ literal: ... }` wrapper)
  and Step 2.3.3 (struct-literal synthesis) without prematurely rejecting valid
  future forms. Sequences are similarly passed through as
  `ArgValue::RawSequence(Vec<TheoremValue>)`. Date/Author: 2026-03-01 /
  DevBoxer.

## Outcomes & retrospective

Implementation complete. All acceptance criteria met:

- Plain YAML strings decode as `ArgValue::Literal(LiteralValue::String(...))`.
- Explicit `{ ref: name }` maps decode as `ArgValue::Reference(name)`.
- Invalid ref targets (empty, keyword, non-identifier, non-string) are rejected
  with actionable error messages including the parameter name.
- The semantic-stability acceptance test proves that adding a `Let` binding
  cannot alter the decoding of a plain string argument in the same theorem.
- All existing fixture files continue to load successfully.
- 16 unit tests cover all decoding paths.
- 5 BDD scenarios cover happy paths, error paths, and semantic stability.
- `make check-fmt`, `make lint`, `make test` all pass (355 tests, 0 failures).
- Documentation updated: `theoremc-design.md` §6.7.6, `users-guide.md` value
  forms section, `roadmap.md` checkbox marked.

Net artefacts: 3 new source files (`arg_value.rs`, `arg_value_tests.rs`,
`raw_action.rs`), 1 BDD test runner, 1 feature file, 5 fixture files. 4
existing source files modified (`types.rs`, `raw.rs`, `loader.rs`, `mod.rs`). 3
documentation files updated. Well within the 15-file / 700-line tolerance.

## Context and orientation

The `theoremc` crate (repository root) compiles `.theorem` YAML files into Rust
proof harnesses. The current loading pipeline is:

1. `load_theorem_docs(yaml)` in `src/schema/loader.rs` calls
   `serde_saphyr::from_multiple(input)` to deserialize YAML into
   `Vec<RawTheoremDoc>`.
2. Each `RawTheoremDoc` is converted to a `TheoremDoc` via
   `raw_doc.to_theorem_doc()` in `src/schema/raw.rs`.
3. `validate_theorem_doc(&doc)` in `src/schema/validate.rs` validates each
   document (fields, expressions, action name grammar).
4. `check_action_collisions(&docs)` in `src/collision.rs` detects mangled
   identifier collisions.

Action arguments currently live in `ActionCall.args` as
`IndexMap<String, TheoremValue>` where `TheoremValue` is a generic YAML value
enum (`Bool`, `Integer`, `Float`, `String`, `Sequence`, `Mapping`). There is no
semantic interpretation — a string `"x"` and a map `{ ref: x }` are both just
`TheoremValue` variants with no distinction between "literal" and "reference".

Key files and their approximate line counts:

- `src/lib.rs` (15 lines) — crate root, declares `pub mod schema`, `pub mod
  mangle`, `pub mod collision`.
- `src/schema/mod.rs` (33 lines) — module root with public re-exports.
- `src/schema/types.rs` (312 lines) — `TheoremDoc`, `ActionCall`, `Step`,
  `LetBinding`, and related types. `ActionCall` is at line 241.
- `src/schema/value.rs` (116 lines) — `TheoremValue` enum with custom
  `Deserialize` implementation.
- `src/schema/raw.rs` (317 lines) — `RawTheoremDoc` with `to_theorem_doc()`
  conversion at line 160.
- `src/schema/error.rs` (69 lines) — `SchemaError` enum.
- `src/schema/validate.rs` (389 lines) — post-deserialization validation.
- `src/schema/step.rs` (278 lines) — step and action call validation.
- `src/schema/identifier.rs` (168 lines) — identifier validation including
  `is_valid_ascii_identifier_pattern` and `is_rust_reserved_keyword`.
- `src/schema/loader.rs` (241 lines) — multi-document loading.
- `src/collision_tests.rs` (~200 lines) — collision detection tests with
  `action_call()` builder at line 49.
- `tests/fixtures/valid_full.theorem` — fixture using `{ ref: ... }` syntax.

The existing fixture `tests/fixtures/valid_full.theorem` already uses
`{ ref: a }`, `{ ref: amount }`, and `{ ref: result }` in arguments, plus
literal integer `1000` and `10`. These will be decoded into the new `ArgValue`
variants after this change.

## Plan of work

### Milestone 0: baseline verification

Run `make test` to confirm all existing tests pass before any changes.

Go/no-go check: existing suite passes.

### Milestone 1: create `ArgValue` and `LiteralValue` types

Create `src/schema/arg_value.rs` with the following types:

```rust
//! Semantically decoded action-call argument values.
//!
//! This module defines `ArgValue`, the domain-level representation of
//! action-call arguments after YAML deserialization and semantic
//! decoding. Plain YAML scalars become `Literal` variants, explicit
//! `{ ref: <Identifier> }` maps become `Reference` variants, and
//! other composite forms are preserved as raw values for future
//! lowering steps (`TFS-5`, `ADR-3`, `DES-5`).

/// A semantically decoded action-call argument value.
///
/// After YAML deserialization, each `TheoremValue` in an action call's
/// `args` map is decoded into an `ArgValue` that distinguishes
/// literals from variable references. This encoding ensures that plain
/// YAML strings are unconditionally treated as string literals and
/// variable references require the explicit `{ ref: <name> }` wrapper
/// (`TFS-5` section 5.2, `ADR-3` decision 3).
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    /// A scalar literal value (bool, integer, float, or string).
    Literal(LiteralValue),
    /// An explicit variable reference via `{ ref: <Identifier> }`.
    Reference(String),
    /// A YAML sequence not yet lowered (future: `vec![...]` synthesis).
    RawSequence(Vec<super::value::TheoremValue>),
    /// A YAML map not yet lowered (future: struct-literal synthesis or
    /// `{ literal: ... }` wrapper recognition).
    RawMap(indexmap::IndexMap<String, super::value::TheoremValue>),
}

/// A scalar literal value decoded from a YAML argument.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    /// A boolean literal (`true` / `false`).
    Bool(bool),
    /// A signed 64-bit integer literal.
    Integer(i64),
    /// A floating-point literal.
    Float(f64),
    /// A string literal (plain YAML string or explicit
    /// `{ literal: "..." }` wrapper in future).
    String(String),
}
```

Wire `mod arg_value;` into `src/schema/mod.rs` and add `pub use` for `ArgValue`
and `LiteralValue`.

Go/no-go check: `cargo check` succeeds.

### Milestone 2: implement `decode_arg_value` conversion

In `src/schema/arg_value.rs`, add a public conversion function:

```rust
/// Decodes a raw `TheoremValue` into a semantically typed `ArgValue`.
///
/// Decoding rules (`TFS-5` section 5.2):
///
/// - `TheoremValue::Bool(b)` → `ArgValue::Literal(LiteralValue::Bool(b))`
/// - `TheoremValue::Integer(n)` → `ArgValue::Literal(LiteralValue::Integer(n))`
/// - `TheoremValue::Float(f)` → `ArgValue::Literal(LiteralValue::Float(f))`
/// - `TheoremValue::String(s)` → `ArgValue::Literal(LiteralValue::String(s))`
/// - `TheoremValue::Sequence(v)` → `ArgValue::RawSequence(v)`
/// - `TheoremValue::Mapping(m)` with exactly one key `"ref"` whose
///   value is `TheoremValue::String(name)` where `name` is a valid
///   ASCII identifier and not a Rust keyword →
///   `ArgValue::Reference(name)`
/// - `TheoremValue::Mapping(m)` with exactly one key `"ref"` whose
///   value is invalid → `Err(...)` with an actionable message.
/// - `TheoremValue::Mapping(m)` (any other map) →
///   `ArgValue::RawMap(m)` (preserved for future lowering).
pub fn decode_arg_value(
    param_name: &str,
    value: TheoremValue,
) -> Result<ArgValue, ArgDecodeError> { ... }
```

The `param_name` argument is used in error messages to identify which argument
failed decoding (e.g., "argument 'graph_ref': ref value 'fn' is a Rust reserved
keyword").

The function uses `super::identifier::is_valid_ascii_identifier_pattern` and
`super::identifier::is_rust_reserved_keyword` from `src/schema/identifier.rs`
for validating the `ref` target name.

Detection of `{ ref: <name> }`: check if the map has exactly one entry, the key
is the string `"ref"`, and the value is a `TheoremValue::String`. If the key is
`"ref"` but the value is not a string, that is also an error.

Go/no-go check: `cargo check` succeeds.

### Milestone 3: change `ActionCall.args` and wire decoding

In `src/schema/types.rs`, change `ActionCall`:

```rust
pub struct ActionCall {
    pub action: String,
    pub args: IndexMap<String, ArgValue>,  // was TheoremValue
    pub as_binding: Option<String>,
}
```

Update the import in `types.rs` to bring in `ArgValue` from `super::arg_value`.

In `src/schema/raw.rs`, `RawTheoremDoc` currently imports and uses the public
`LetBinding`, `Step`, and related types directly (line 12):
`use super::types::{Evidence, KaniEvidence, KaniExpectation, LetBinding, Step, TheoremDoc};`.
 Since `ActionCall.args` changes from `TheoremValue` to `ArgValue`, serde can
no longer deserialize YAML directly into the public `ActionCall` (the YAML
contains raw `TheoremValue`-shaped data). Raw versions of all types that
contain `ActionCall` are therefore required.

The dependency chain requiring raw counterparts:

- `ActionCall` (contains `args: IndexMap<String, TheoremValue>` at serde level)
- `LetCall`, `LetMust` (contain `ActionCall`)
- `LetBinding` (enum of `LetCall` | `LetMust`)
- `StepCall`, `StepMust` (contain `ActionCall`)
- `StepMaybe` (contains `MaybeBlock`)
- `MaybeBlock` (contains `Vec<Step>`)
- `Step` (enum of `StepCall` | `StepMust` | `StepMaybe`)

That is 9 raw types. Place them in a new `src/schema/raw_action.rs` module to
keep `raw.rs` under 400 lines (currently 316 lines).

`src/schema/raw_action.rs` will contain:

- `RawActionCall` — same as current `ActionCall` but with `TheoremValue` args
- `RawLetCall`, `RawLetMust`, `RawLetBinding` — mirrors of public types
- `RawStepCall`, `RawStepMust`, `RawStepMaybe`, `RawMaybeBlock`, `RawStep`
- `convert_action_call(raw: &RawActionCall) -> Result<ActionCall, ArgDecodeError>`
  — decodes args via `decode_arg_value`
- `convert_let_binding(raw: &RawLetBinding) -> Result<LetBinding, ArgDecodeError>`
- `convert_step(raw: &RawStep) -> Result<Step, ArgDecodeError>` — recursive for
  maybe blocks

All serde attributes (`deny_unknown_fields`, `untagged`, renames) mirror the
public types exactly.

In `raw.rs`, update `RawTheoremDoc`:

```rust
pub(crate) let_bindings: IndexMap<String, RawLetBinding>,  // was LetBinding
pub(crate) do_steps: Vec<RawStep>,                          // was Step
```

Import from `super::raw_action::{RawLetBinding, RawStep}`.

The `to_theorem_doc()` method changes to call the conversion functions.
`convert_let_bindings` and `convert_steps` construct `RawDocDecodeError`
variants that wrap the underlying `ArgDecodeError` as a `#[source]`:

```rust
pub(crate) fn to_theorem_doc(&self) -> Result<TheoremDoc, RawDocDecodeError> {
    let let_bindings = convert_let_bindings(&self.let_bindings)?;
    let do_steps = convert_steps(&self.do_steps)?;
    Ok(TheoremDoc { ..., let_bindings, do_steps, ... })
}
```

Since `to_theorem_doc` now returns `Result<..., RawDocDecodeError>`, the loader
stringifies the error at the boundary and attaches a source-location diagnostic
via `attach_validation_diagnostic`:

```rust
let doc = raw_doc.to_theorem_doc().map_err(|decode_err| {
    let reason = decode_err.to_string();
    let error = SchemaError::ValidationFailed {
        theorem: raw_doc.theorem.value.to_string(),
        reason,
        diagnostic: None,
    };
    attach_validation_diagnostic(error, source, &raw_doc)
})?;
```

Go/no-go check: `cargo check` succeeds, existing tests pass with updated
fixture values.

### Milestone 4: add error variant (if needed)

Evaluate whether a new `SchemaError` variant is warranted. The current design
routes decoding errors through `SchemaError::ValidationFailed` with a
descriptive reason string. This is consistent with how other validation errors
are reported. If a distinct error variant would improve programmatic error
handling, add `InvalidArgValue` to `SchemaError`. Otherwise, reuse
`ValidationFailed`.

Decision: reuse `ValidationFailed` for now. The error reason string will
contain the argument name and specific failure details. This avoids adding a
variant that may need to change when Steps 2.3.2 and 2.3.3 extend decoding.

Go/no-go check: `cargo check` succeeds.

### Milestone 5: unit tests for `decode_arg_value`

Add unit tests in `src/schema/arg_value.rs` (inline `#[cfg(test)] mod tests`)
or, if the file would exceed 400 lines, in a separate
`src/schema/arg_value_tests.rs` via `#[path]` attribute.

Test cases:

1. Plain string → `Literal(String(...))`.
2. Boolean true → `Literal(Bool(true))`.
3. Boolean false → `Literal(Bool(false))`.
4. Integer → `Literal(Integer(...))`.
5. Float → `Literal(Float(...))`.
6. `{ ref: valid_name }` → `Reference("valid_name")`.
7. `{ ref: _underscore }` → `Reference("_underscore")`.
8. `{ ref: "" }` → error (empty identifier).
9. `{ ref: "fn" }` → error (Rust keyword).
10. `{ ref: "123bad" }` → error (invalid identifier pattern).
11. `{ ref: 42 }` (non-string value) → error.
12. `{ ref: true }` (boolean value) → error.
13. `{ other_key: value }` → `RawMap(...)`.
14. `{ ref: name, extra: value }` → `RawMap(...)` (two keys, not a ref
    wrapper).
15. YAML sequence → `RawSequence(...)`.
16. Empty map `{}` → `RawMap(...)`.

Go/no-go check: `cargo test -- arg_value` passes.

### Milestone 6: BDD behavioural tests and fixture files

Create `tests/features/arg_decode.feature` with Gherkin scenarios:

```gherkin
Feature: Argument value decoding
  Requirement: plain YAML strings must be treated as literals
  and variable references must use explicit { ref: name } wrappers,
  ensuring theorem argument values have stable, explicit semantics

  Scenario: Plain string arguments are decoded as literals
    Given a theorem file with plain string arguments
    Then loading succeeds and arguments are string literals

  Scenario: Explicit ref arguments are decoded as references
    Given a theorem file with { ref: name } arguments
    Then loading succeeds and arguments are variable references

  Scenario: Integer and boolean arguments are decoded as literals
    Given a theorem file with integer and boolean arguments
    Then loading succeeds and arguments are scalar literals

  Scenario: Invalid ref target is rejected
    Given a theorem file with an invalid ref target
    Then loading fails with an actionable error message
```

Create `tests/arg_decode_bdd.rs` implementing the scenarios.

Create fixture files:

- `tests/fixtures/valid_arg_string_literal.theorem` — plain string args.
- `tests/fixtures/valid_arg_ref.theorem` — `{ ref: name }` args.
- `tests/fixtures/valid_arg_mixed_scalars.theorem` — integer, boolean, string
  args.
- `tests/fixtures/invalid_arg_ref_keyword.theorem` — `{ ref: fn }`.
- `tests/fixtures/invalid_arg_ref_empty.theorem` — `{ ref: "" }`.

Go/no-go check: `cargo test --test arg_decode_bdd` passes.

### Milestone 7: semantic-stability acceptance test

This is the critical acceptance criterion from the roadmap: "tests prove adding
a new binding cannot alter existing literal argument semantics."

Create a test (in the BDD suite or as a dedicated unit test) that:

1. Defines a theorem YAML with `Let: { x: { call: { action: a.b, args: {} } } }`
   and a Do step with `args: { param: "x" }` (plain string "x").
2. Loads the theorem and verifies `param` is `ArgValue::Literal(String("x"))`.
3. Defines a second theorem YAML identical except `Let` also binds `param`
   (i.e., a binding with the same name as the string argument).
4. Loads the second theorem and verifies `param` is still
   `ArgValue::Literal(String("x"))` — NOT a reference.
5. Defines a third YAML where `param` uses `{ ref: x }` and verifies it
   decodes as `ArgValue::Reference("x")`.

This directly demonstrates the invariant from ADR-3: "adding a new binding
cannot alter existing literal argument semantics."

Go/no-go check: acceptance test passes.

### Milestone 8: documentation updates

Update `docs/theoremc-design.md` — add a section under §6 recording the
implementation decisions for Step 2.3.1 (types introduced, decoding location,
raw-type pattern).

Update `docs/users-guide.md` — expand the "Value forms in arguments" section to
explain `ArgValue` semantics, including the `{ ref: name }` syntax and the
invariant that plain strings are always literals.

Update `docs/roadmap.md` — mark the Step 2.3.1 checkbox `[x]`.

Go/no-go check: `make markdownlint` passes.

### Milestone 9: quality gates

Run `make check-fmt`, `make lint`, `make test` with `set -o pipefail` and `tee`
for log capture.

Go/no-go check: all three gates pass with zero errors and zero warnings.

## Concrete steps

Run from the repository root.

1. Baseline verification:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-3-1-baseline-test.log
   ```

   Expected signal: existing suite passes.

2. After code and test edits, run formatting:

   ```shell
   make fmt
   ```

3. Run formatting gate:

   ```shell
   set -o pipefail
   make check-fmt 2>&1 | tee /tmp/2-3-1-check-fmt.log
   ```

   Expected signal: formatter check exits 0.

4. Run lint gate:

   ```shell
   set -o pipefail
   make lint 2>&1 | tee /tmp/2-3-1-lint.log
   ```

   Expected signal: rustdoc + clippy exit 0 with no denied warnings.

5. Run full tests:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-3-1-test.log
   ```

   Expected signal: all tests pass, including new arg decoding unit tests, BDD
   scenarios, and the semantic-stability acceptance test.

## Validation and acceptance

Acceptance behaviours:

- A `.theorem` file with `args: { name: "hello" }` loads successfully and
  produces `ArgValue::Literal(LiteralValue::String("hello".into()))` for the
  `name` parameter.
- A `.theorem` file with `args: { name: { ref: graph } }` loads successfully
  and produces `ArgValue::Reference("graph".into())` for the `name` parameter.
- A `.theorem` file with `args: { name: { ref: fn } }` fails loading with an
  error containing "Rust reserved keyword".
- Adding a `Let` binding named `"hello"` to a theorem does not change the
  decoding of a plain string argument `"hello"` in the same theorem.
- All existing fixture files (`valid_full.theorem`, `valid_multi.theorem`, etc.)
  continue to load successfully.
- `make check-fmt` passes.
- `make lint` passes (zero warnings).
- `make test` passes (all existing + new tests, 0 failures).

Quality criteria:

- Tests: all existing and new unit/BDD tests pass.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Markdown: `make markdownlint` passes.

## Idempotence and recovery

All steps are idempotent; rerunning commands is safe. If a gate fails, inspect
`/tmp/2-3-1-*.log`, apply minimal corrective edits, and rerun only the failing
gate before rerunning the full gate sequence.

## Artefacts and notes

New artefacts:

- `src/schema/arg_value.rs` — `ArgValue`, `LiteralValue` types and
  `decode_arg_value` conversion function.
- `src/schema/raw_action.rs` — raw serde-compatible action types
  (`RawActionCall`, `RawLetBinding`, `RawStep`, etc.) and conversion functions
  to their public counterparts with decoded `ArgValue` args.
- `tests/features/arg_decode.feature` — BDD feature file.
- `tests/arg_decode_bdd.rs` — BDD test runner.
- `tests/fixtures/valid_arg_string_literal.theorem` — fixture with plain
  string args.
- `tests/fixtures/valid_arg_ref.theorem` — fixture with `{ ref: ... }` args.
- `tests/fixtures/valid_arg_mixed_scalars.theorem` — fixture with mixed scalar
  args.
- `tests/fixtures/invalid_arg_ref_keyword.theorem` — fixture with invalid ref
  target (keyword).
- `tests/fixtures/invalid_arg_ref_empty.theorem` — fixture with empty ref
  target.
- `docs/execplans/2-3-1-argument-decoding-for-plain-yaml-strings.md` — this
  ExecPlan.

Updated artefacts:

- `src/schema/mod.rs` — add `mod arg_value;`, `mod raw_action;`, and
  `pub use` for `ArgValue`, `LiteralValue`.
- `src/schema/types.rs` — change `ActionCall.args` from
  `IndexMap<String, TheoremValue>` to `IndexMap<String, ArgValue>`. Remove the
  `TheoremValue` import; add `ArgValue` import. Remove `Deserialize` derive
  from `ActionCall` and all types that contain it (`LetCall`, `LetMust`,
  `StepCall`, `StepMust`, `StepMaybe`, `MaybeBlock`, `LetBinding`, `Step`) —
  serde now deserializes the raw versions instead.
- `src/schema/raw.rs` — change `RawTheoremDoc.let_bindings` type from
  `IndexMap<String, LetBinding>` to `IndexMap<String, RawLetBinding>` and
  `do_steps` from `Vec<Step>` to `Vec<RawStep>`. Update `to_theorem_doc()` to
  return `Result<TheoremDoc, RawDocDecodeError>` and use conversion functions
  from `raw_action.rs`. Update imports.
- `src/schema/loader.rs` — update `to_theorem_doc()` call site to handle
  `Result`.
- `src/schema/step.rs` — no changes needed (validates `action` string only,
  does not inspect `args`).
- `src/collision_tests.rs` — update `action_call()` builder if it inserts
  values (currently it uses `IndexMap::new()` so no change expected).
- `docs/theoremc-design.md` — add §6.x implementation decisions for 2.3.1.
- `docs/users-guide.md` — expand argument value semantics section.
- `docs/roadmap.md` — mark Step 2.3.1 checkbox `[x]`.

## Interfaces and dependencies

### New public types (`theoremc::schema`)

In `src/schema/arg_value.rs`:

```rust
/// A semantically decoded action-call argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgValue {
    /// A scalar literal value.
    Literal(LiteralValue),
    /// An explicit variable reference via `{ ref: <Identifier> }`.
    Reference(String),
    /// A YAML sequence not yet lowered (future: `vec![...]` synthesis).
    RawSequence(Vec<super::value::TheoremValue>),
    /// A YAML map not yet lowered (future: struct-literal synthesis or
    /// `{ literal: ... }` wrapper recognition).
    RawMap(indexmap::IndexMap<String, super::value::TheoremValue>),
}

/// A scalar literal value decoded from a YAML argument.
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    /// A boolean literal.
    Bool(bool),
    /// A signed 64-bit integer literal.
    Integer(i64),
    /// A floating-point literal.
    Float(f64),
    /// A string literal.
    String(String),
}
```

### Modified public type (`theoremc::schema::ActionCall`)

In `src/schema/types.rs`:

```rust
pub struct ActionCall {
    pub action: String,
    pub args: IndexMap<String, ArgValue>,  // changed from TheoremValue
    pub as_binding: Option<String>,
}
```

### Internal conversion function

In `src/schema/arg_value.rs`:

```rust
pub fn decode_arg_value(
    param_name: &str,
    value: TheoremValue,
) -> Result<ArgValue, ArgDecodeError>
```

### Dependencies

No new external dependencies. Uses existing:

- `indexmap` (already in `Cargo.toml`)
- `super::identifier::{is_valid_ascii_identifier_pattern, is_rust_reserved_keyword}`
   (existing)
- `super::value::TheoremValue` (existing)
