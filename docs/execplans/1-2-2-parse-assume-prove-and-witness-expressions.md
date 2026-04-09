# Parse `Assume.expr`, `Prove.assert`, and `Witness.cover` as `syn::Expr`

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, `theoremc::schema::load_theorem_docs` validates that every
expression-bearing field (`Assume.expr`, `Prove.assert`, `Witness.cover`)
contains syntactically valid Rust and is a single expression rather than a
statement block, loop, or flow-control construct. A document with
`assert: "{ let x = 1; x }"` or `expr: "for i in 0..10 { }"` is rejected with a
deterministic, actionable error message identifying the theorem name, the
field, and the reason for rejection.

This is Roadmap Phase 1, Step 1.2.2. It builds on Step 1.2.1
(post-deserialization non-empty validation) by adding syntactic expression
validation via the `syn` crate. The specification requirements are Theorem File
Specification (TFS) `TFS-1` sections 1.2 and 2.3 ("`RustExpr` MUST parse as
`syn::Expr`; MUST be single expressions") and Design document (DES) `DES-6`
section 6.2 ("Expression syntax: `Assume.expr` and `Prove.assert` parse as
`syn::Expr` (syntax only)").

Observable success: running `make test` passes with new tests that confirm (a)
valid expressions like `"x > 0"` and `"result.is_valid()"` are accepted, and
(b) statement blocks like `"{ let x = 1; x }"`, loops like
`"for i in 0..10 { }"`, and syntactic garbage like `"not rust %%"` are rejected
with clear error messages. Existing tests remain green (no regressions).

## Constraints

- All code must pass `make check-fmt`, `make lint`, and `make test`.
- Clippy lints are aggressive (see `Cargo.toml` `[lints.clippy]`): no
  `unwrap`, no `expect`, no indexing, no panics in result functions, no missing
  docs, cognitive complexity <= 9.
- No `unsafe` code.
- No file longer than 400 lines.
- Module-level (`//!`) doc comments on every module.
- Public APIs documented with rustdoc (`///`).
- Comments in en-GB-oxendict spelling.
- Use `thiserror` for error enums (not `eyre` in library code).
- Edition 2024, nightly-2026-01-30 toolchain.
- This plan must not modify paths outside `src/schema/`, `tests/`, `docs/`,
  `Cargo.toml`, and fixture files.
- Existing valid fixtures must continue to parse successfully.
- The `syn` crate must be a production dependency (not dev-only) because
  expression validation is core library functionality invoked by
  `load_theorem_docs`.

## Tolerances (exception triggers)

- Scope: if implementation requires more than 4 new source files or 600 net
  lines of code, stop and escalate.
- Dependencies: `syn` is the only new dependency expected. If another is
  required, stop and escalate.
- Iterations: if a test or lint failure persists after 5 attempts, stop and
  escalate.
- Ambiguity: the normative spec is `docs/theorem-file-specification.md`. If
  the spec is ambiguous on a point that materially affects which expression
  forms are accepted or rejected, document the ambiguity in `Decision Log` and
  escalate.

## Risks

- Risk: adding expression validation logic and tests inline to `validate.rs`
  would breach the 400-line limit. Severity: medium. Likelihood: certain
  (`validate.rs` was at 327 lines). Mitigation: create a new
  `src/schema/expr.rs` module for expression parsing validation, keeping
  `validate.rs` as the integration point with thin wrappers.

- Risk: existing valid fixtures or inline YAML test constants might contain
  expression strings that `syn` cannot parse or that trip the statement-block
  denylist. Severity: medium. Likelihood: low (audit shows simple expressions
  like `"true"`, `"x < 100"`, `"result.balance() >= amount"`). Mitigation:
  audit all existing expression strings before implementing new validation.

- Risk: `syn::parse_str` behaviour for edge cases (e.g., `"let x = 5"`)
  could vary across syn versions. Severity: low. Likelihood: low (`Expr::Let`
  has been stable since syn 2.0). Mitigation: pin to `^2.0.115` (already in
  lockfile) and test exact strings.

## Progress

- [x] (2026-02-12) Write ExecPlan document.
- [x] (2026-02-12) Milestone 0: audit existing fixtures for expression
  compliance.
- [x] (2026-02-12) Milestone 1: add `syn` dependency.
- [x] (2026-02-12) Milestone 2: create `src/schema/expr.rs` with unit tests.
- [x] (2026-02-12) Milestone 3: integrate expression validation into
  `validate.rs`.
- [x] (2026-02-12) Milestone 4: create test fixtures and behaviour-driven
  development (BDD) tests.
- [x] (2026-02-12) Milestone 5: documentation updates.
- [x] (2026-02-12) Milestone 6: final quality gates (198 tests passing).

## Surprises & discoveries

- Observation: `cargo fmt` reformats the `format!` call in `validate_rust_expr`
  and the `concat!` call for the denylist message onto single lines. One
  format-fix cycle was needed. Evidence: `make check-fmt` failure after initial
  write of `expr.rs`. Impact: always run `make fmt` before `make check-fmt`
  when writing new code (consistent with Step 1.2.1 discovery).

- Observation: Clippy's `missing_const_for_fn` lint fires on `is_statement_like`
  because the `matches!` macro with `syn::Expr` variants can be evaluated at
  compile time. Evidence: `make lint` failure after initial write. Impact:
  `is_statement_like` was made `const fn`.

- Observation: Milestones 2 and 3 cannot pass `make lint` independently because
  `expr.rs` functions are dead code until wired into `validate.rs`. Evidence:
  `dead_code` lint error when only `expr.rs` exists without integration.
  Impact: milestones 2 and 3 were combined into a single quality gate
  checkpoint.

- Observation: `validate.rs` reached 398 lines after integration (just under
  the 400-line limit). Evidence: `wc -l` count. Impact: the line budget is very
  tight; future additions to validation should consider extracting more logic
  into dedicated modules.

## Decision log

- D1: create `src/schema/expr.rs` rather than extending `validate.rs`.
  `validate.rs` was at 327 lines. Adding expression parsing logic plus unit
  tests would push it past the 400-line limit. The expression parsing concern
  (syntactic validation via `syn`) is distinct from structural validation
  (non-emptiness, field presence), so a dedicated module improves separation of
  concerns. The new module exports a single `pub(crate)` function consumed by
  `validate.rs`. Date: 2026-02-12.

- D2: denylist approach for rejected expression forms (not allowlist).
  The spec says "no statement blocks, no `let`, no `for`, etc." A denylist of
  14 specific `syn::Expr` variants plus compound assignment operators (10
  `BinOp::*Assign` variants) covers the "etc." while allowing legitimate
  expression forms (closures, `if`, `match`, method calls). An allowlist would
  risk being too restrictive and rejecting valid expressions that theorem
  authors need. Date: 2026-02-12.

- D3: `syn` features `parsing` + `full` with `default-features = false`.
  The `parsing` feature provides `syn::parse_str::<Expr>`. The `full` feature
  provides all `Expr` variant types needed for the denylist check. Without
  `full`, only a minimal subset of `Expr` variants is available. Other features
  (`derive`, `printing`, `clone-impls`) are not needed. Date: 2026-02-12.

- D4: expression validation runs after non-blank validation.
  Blank strings would produce confusing `syn` parse errors. Running non-blank
  checks first ensures empty/whitespace fields produce the clearer "must be
  non-empty after trimming" message. Date: 2026-02-12.

- D5: `validate_rust_expr` returns `Result<(), String>` (not `SchemaError`).
  The caller in `validate.rs` has the context (section name, 1-based index,
  theorem doc) needed to construct `SchemaError::ValidationFailed`. Returning a
  plain reason string keeps `expr.rs` decoupled from the error type. Date:
  2026-02-12.

## Outcomes & retrospective

All milestones completed successfully. The implementation delivers:

- 1 new source module (`src/schema/expr.rs`, 145 lines including tests)
  containing expression syntax validation via `syn`.
- `src/schema/validate.rs` grew from 327 to 398 lines with expression
  validation integration (4 new functions, 4 new unit test cases).
- 6 new fixture files in `tests/fixtures/` covering statement block rejection
  and invalid syntax rejection.
- 6 new BDD tests in `tests/schema_bdd.rs` covering expression validation via
  fixtures.
- Total test count increased from 167 to 198 (31 new tests: 26 unit tests in
  `expr.rs`, 4 unit tests in `validate.rs`, and 6 BDD tests).
- 1 new production dependency: `syn` 2.0.115 with `parsing` + `full` features.
- Documentation updated: `users-guide.md` (expression validation section),
  `roadmap.md` (checkbox), `contents.md` (execplan entry), `theoremc-design.md`
  (implementation decisions section 6.5).
- All quality gates pass: `make check-fmt`, `make lint`, `make test`.

Lessons learned:

- Extracting expression validation into a dedicated module (`expr.rs`) was
  essential. `validate.rs` reached 398 lines even with thin wrappers, leaving
  almost no headroom. Future validation features will need similar extraction.
- The denylist approach to rejected expression forms is conservative and
  extensible. If future requirements mandate stricter expression rules, the
  denylist in `is_statement_like` can be extended without changing the public
  API.
- Combining milestones 2 and 3 was necessary because dead code lints prevent
  independent quality gates for a module that exists but is not yet called. In
  future, plan milestones around lint-passable increments.

## Context and orientation

The `theoremc` crate compiles human-readable `.theorem` YAML (YAML Ain't Markup
Language) files into Kani model-checking proof harnesses. The schema is defined
in `docs/theorem-file-specification.md` (`TFS-1`) and the design rationale in
`docs/theoremc-design.md` (`DES-6`).

Three struct types carry expression fields as plain `String` values:

- `Assumption.expr` (`src/schema/types.rs:110`) -- Rust expression for an
  `Assume` constraint.
- `Assertion.assert_expr` (`src/schema/types.rs:126`) -- Rust boolean
  expression for a `Prove` assertion (serde-renamed from `assert`).
- `WitnessCheck.cover` (`src/schema/types.rs:139`) -- Rust expression for a
  `Witness` coverage marker.

Before this change, these fields were only validated for non-emptiness (Step
1.2.1) in `src/schema/validate.rs`. The validation pipeline
(`validate_theorem_doc`) called: `validate_about` -> `validate_prove_non_empty`
-> `validate_assertions` -> `validate_assumptions` -> `validate_witnesses` ->
`validate_evidence`. Each uses
`SchemaError::ValidationFailed { theorem, reason }` from `src/schema/error.rs`.

Key files:

- `src/schema/mod.rs` -- module declarations and public re-exports.
- `src/schema/expr.rs` -- expression syntax validation (NEW).
- `src/schema/validate.rs` -- post-deserialization validation (398 lines).
- `src/schema/error.rs` -- `SchemaError` enum (27 lines).
- `src/schema/types.rs` -- schema structs (313 lines).
- `src/schema/loader.rs` -- YAML loading and validation orchestration.
- `tests/schema_bdd.rs` -- BDD-style integration tests (287 lines).
- `tests/fixtures/` -- 33 fixture files (5 valid, 28 invalid).
- `Cargo.toml` -- dependencies and lint configuration (83 lines).

## Plan of work

### Milestone 0: audit existing fixtures for expression compliance

Before implementing new validation, confirm that all expression strings in
existing valid fixtures and inline YAML test constants parse as `syn::Expr` and
are not statement-like. The expressions found are: `"true"`, `"x < 100"`,
`"x == 50"`, `"amount <= 100"`, `"amount == 50"`,
`"result.balance() >= amount"`, `"result.is_valid()"`. All are simple single
expressions. No changes needed.

Gate: visual audit only. No code changes.

### Milestone 1: add `syn` dependency

Edit `Cargo.toml` to add `syn` as a direct dependency:

```toml
syn = { version = "2.0.115", default-features = false, features = ["parsing", "full"] }
```

This goes in `[dependencies]` (not `[dev-dependencies]`) because expression
validation is core library functionality.

Gate: `cargo check` compiles successfully.

### Milestone 2: create `src/schema/expr.rs` with unit tests

Create `src/schema/expr.rs` with the following structure:

Module doc comment (`//!`) explaining its purpose: syntactic validation of Rust
expression strings using `syn`.

Public function:

```rust
/// Validates that `input` is a syntactically valid Rust expression
/// and is not a statement-like form (block, loop, assignment, or
/// flow-control construct).
///
/// Returns `Ok(())` if the input is a valid single expression.
/// Returns `Err(reason)` with a human-readable reason string if
/// parsing fails or a disallowed form is detected.
pub(crate) fn validate_rust_expr(input: &str) -> Result<(), String>
```

Implementation:

1. Call `syn::parse_str::<syn::Expr>(input)`. On parse failure, return
   `Err(format!("is not a valid Rust expression: {err}"))`.
2. On success, check the parsed `Expr` against the denylist using
   `is_statement_like()`. If it matches, return
   `Err("must be a single expression, not a statement or block".to_owned())`.
3. Otherwise, return `Ok(())`.

Private predicate:

```rust
fn is_statement_like(expr: &syn::Expr) -> bool
```

Uses `matches!` to check for these 14 denied `syn::Expr` variants, plus a
helper `is_compound_assignment` that detects compound assignment operators
(`+=`, `-=`, etc.) which `syn` 2.x represents as `Expr::Binary` with
`BinOp::*Assign` variants:

| Variant                   | Why rejected                                         |
| ------------------------- | ---------------------------------------------------- |
| `Expr::Assign`            | Assignment is a side effect, not a value expression. |
| `Expr::Async`             | `async { ... }` block. Spec: "no statement blocks".  |
| `Expr::Block`             | Explicit `{ ... }` block with statements.            |
| `Expr::Break`             | Flow control, not a value-producing expression.      |
| `Expr::Const`             | `const { ... }` block. Analogous to async/unsafe.    |
| `Expr::Continue`          | Flow control.                                        |
| `Expr::ForLoop`           | `for` loop. Spec: "no `for`".                        |
| `Expr::Let`               | `let` guard/binding. Spec: "no `let`".               |
| `Expr::Loop`              | Unconditional `loop { ... }`.                        |
| `Expr::Return`            | Flow control.                                        |
| `Expr::TryBlock`          | `try { ... }` block.                                 |
| `Expr::Unsafe`            | `unsafe { ... }` block. Spec: "no statement blocks". |
| `Expr::While`             | `while` loop. Analogous to `for`.                    |
| `Expr::Yield`             | `yield` expression. Only meaningful in generators.   |
| `Expr::Binary` (compound) | `+=`, `-=`, `*=`, etc. Mutating side effect.         |

Allowed forms include: `if`, `match`, closures, method calls, function calls,
binary/unary operations, literals, paths, field access, indexing, casts,
references, ranges, tuples, arrays, struct literals, macros, and `try` (`?`)
operator.

Add `mod expr;` to `src/schema/mod.rs`. Nothing is `pub use`-exported -- the
module is `pub(crate)` only.

Unit tests in `#[cfg(test)] mod tests` within `expr.rs`:

Happy-path rstest parameterized cases (all return `Ok(())`):

- `"true"` -- boolean literal.
- `"x > 0"` -- binary comparison.
- `"result.is_valid()"` -- method call.
- `"result.balance() >= amount"` -- method call with comparison.
- `"hnsw.is_bidirectional(&graph)"` -- function call with reference arg.
- `"!hnsw.edge_present(&graph, 2, 0, 1)"` -- unary + function call.
- `"amount <= (u64::MAX - a.balance)"` -- parenthesized arithmetic.
- `"x"` -- plain identifier.
- `"if x > 0 { a } else { b }"` -- if expression (allowed).
- `"match x { 1 => true, _ => false }"` -- match expression (allowed).
- `"|x| x > 0"` -- closure (allowed).

Unhappy-path rstest parameterized cases (all return `Err(...)`):

- `"{ let x = 1; x > 0 }"` -- block expression.
- `"for i in 0..10 { }"` -- for loop.
- `"while true { }"` -- while loop.
- `"loop { break 42; }"` -- infinite loop.
- `"let x = 5"` -- let expression.
- `"unsafe { x }"` -- unsafe block.
- `"async { x }"` -- async block.
- `"const { 42 }"` -- const block.
- `"return 42"` -- return expression.
- `"break 42"` -- break expression.
- `"continue"` -- continue expression.
- `"x = 5"` -- assignment.
- `"not rust code %%"` -- parse failure.
- `"x >"` -- parse failure.
- `"if { }"` -- parse failure.

Gate: `make check-fmt && make lint && make test` (combined with Milestone 3 to
avoid dead-code lint).

### Milestone 3: integrate expression validation into `validate.rs`

Add `use super::expr;` to `validate.rs`.

Add a new orchestrator function:

```rust
fn validate_expressions(doc: &TheoremDoc) -> Result<(), SchemaError>
```

This calls three section-specific helpers:

```rust
fn validate_assumption_exprs(doc: &TheoremDoc) -> Result<(), SchemaError>
```

Iterates `doc.assume`, calling `expr::validate_rust_expr(a.expr.trim())` and
wrapping errors as
`fail(doc, format!("Assume constraint {}: expr {reason}", i + 1))`.

```rust
fn validate_assertion_exprs(doc: &TheoremDoc) -> Result<(), SchemaError>
```

Iterates `doc.prove`, calling `expr::validate_rust_expr(a.assert_expr.trim())`
and wrapping errors as
`fail(doc, format!("Prove assertion {}: assert {reason}", i + 1))`.

```rust
fn validate_witness_exprs(doc: &TheoremDoc) -> Result<(), SchemaError>
```

Iterates `doc.witness`, calling `expr::validate_rust_expr(w.cover.trim())` and
wrapping errors as `fail(doc, format!("Witness {}: cover {reason}", i + 1))`.

Insert `validate_expressions(doc)?;` into `validate_theorem_doc` after
`validate_witnesses(doc)?;` and before `validate_evidence(doc)?;`. This ensures
non-blank checks run first with clearer messages.

Update the doc comment on `validate_theorem_doc` to include expression
validation in the list of applied checks.

Add unit tests to the existing `#[cfg(test)] mod tests` block in `validate.rs`:

- `block_assume_expr`: fragment:
  `"Assume constraint 1: expr must be a single expression"`.
- `for_loop_assert`: fragment:
  `"Prove assertion 1: assert must be a single expression"`.
- `block_witness_cover`: fragment:
  `"Witness 1: cover must be a single expression"`.
- `invalid_syntax_assume`: fragment:
  `"Assume constraint 1: expr is not a valid Rust expression"`.

Gate: `make check-fmt && make lint && make test`.

### Milestone 4: create test fixtures and BDD tests

Create 6 new fixture files in `tests/fixtures/`:

1. `invalid_block_assume_expr.theorem` -- `expr: "{ let x = 1; x }"`
2. `invalid_for_loop_assert.theorem` -- `assert: "for i in 0..10 { }"`
3. `invalid_while_witness_cover.theorem` -- `cover: "while true { }"`
4. `invalid_syntax_assume_expr.theorem` -- `expr: "not valid %%"`
5. `invalid_syntax_assert.theorem` -- `assert: "not valid %%"`
6. `invalid_syntax_witness_cover.theorem` -- `cover: "not valid %%"`

Add a new BDD test group in `tests/schema_bdd.rs` with 6 cases covering all
three expression fields (block rejection and syntax rejection).

Gate: `make check-fmt && make lint && make test`.

### Milestone 5: documentation updates

1. `docs/roadmap.md`: change `- [ ]` to `- [x]` for step 1.2.2.
2. `docs/contents.md`: add entry for the new execplan.
3. `docs/theoremc-design.md`: add section 6.5 recording implementation
   decisions.
4. `docs/users-guide.md`: add "Expression syntax validation" subsection.
5. Create this ExecPlan document.

### Milestone 6: final quality gates

```shell
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint 2>&1 | tee /tmp/lint.log
make test 2>&1 | tee /tmp/test.log
```

All three must exit 0.

## Concrete steps

All commands run from `/home/user/project`.

Milestone 0: visual audit of expression strings in existing fixtures and inline
YAML. Confirmed all are simple single expressions.

Milestone 1: added `syn` to `[dependencies]` in `Cargo.toml`:

```toml
syn = { version = "2.0.115", default-features = false,
        features = ["parsing", "full"] }
```

`cargo check` passed.

Milestone 2+3 (combined): created `src/schema/expr.rs`, added `mod expr;` to
`src/schema/mod.rs`, added expression validation integration to
`src/schema/validate.rs`. `make check-fmt && make lint && make test` passed.

Milestone 4: created 6 fixture files, added 6 BDD test cases.
`make check-fmt && make lint && make test` passed (198 total tests).

Milestone 5: updated `roadmap.md`, `contents.md`, `theoremc-design.md`,
`users-guide.md`, created this ExecPlan document.

Milestone 6: final quality gates passed.

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New tests cover: valid expressions accepted
  (boolean, comparison, method call, if, match, closure), statement blocks
  rejected (block, for, while, loop, let, unsafe, async, const, return, break,
  continue, assignment), and invalid syntax rejected.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Existing 167 tests continue to pass (no regressions).

Quality method:

```shell
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint 2>&1 | tee /tmp/lint.log
make test 2>&1 | tee /tmp/test.log
```

Expected: all three commands exit 0. Test count increased from 167 to 198.

## Idempotence and recovery

All steps are additive and re-runnable. No destructive operations. If a
milestone fails, fix the issue and re-run
`make check-fmt && make lint && make test` from the repo root.

## Artifacts and notes

Key file paths (all relative to repo root):

- `Cargo.toml` -- dependency addition (modified).
- `src/schema/expr.rs` -- expression validation module (NEW, 145 lines).
- `src/schema/mod.rs` -- module declaration (modified: +1 line).
- `src/schema/validate.rs` -- validation pipeline integration (modified:
  327 -> 398 lines).
- `tests/schema_bdd.rs` -- BDD tests (modified: 254 -> 287 lines).
- `tests/fixtures/invalid_block_assume_expr.theorem` -- fixture (NEW).
- `tests/fixtures/invalid_for_loop_assert.theorem` -- fixture (NEW).
- `tests/fixtures/invalid_while_witness_cover.theorem` -- fixture (NEW).
- `tests/fixtures/invalid_syntax_assume_expr.theorem` -- fixture (NEW).
- `tests/fixtures/invalid_syntax_assert.theorem` -- fixture (NEW).
- `tests/fixtures/invalid_syntax_witness_cover.theorem` -- fixture (NEW).
- `docs/roadmap.md` -- roadmap checkbox (modified).
- `docs/contents.md` -- contents index (modified).
- `docs/theoremc-design.md` -- design spec (modified: +1 section, renumbered
  6.5-6.9).
- `docs/users-guide.md` -- user guide (modified: +1 subsection).
- `docs/execplans/1-2-2-parse-assume-prove-and-witness-expressions.md` --
  this ExecPlan (NEW).

## Interfaces and dependencies

New dependency:

```toml
syn = { version = "2.0.115", default-features = false, features = ["parsing", "full"] }
```

Internal interface added in `src/schema/expr.rs`:

```rust
/// Validates that `input` is a syntactically valid Rust expression
/// and is not a statement-like form.
pub(crate) fn validate_rust_expr(input: &str) -> Result<(), String>
```

Called from `validate_expressions` in `src/schema/validate.rs`, which is called
from `validate_theorem_doc` after non-blank validation and before evidence
validation.
