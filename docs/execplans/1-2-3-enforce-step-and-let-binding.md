# Enforce `Step` and `LetBinding` shape rules

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, `theoremc::schema::load_theorem_docs` validates the internal
structure of `Let` bindings and `Do` steps beyond what serde deserialization
alone can enforce. Specifically:

- Every `ActionCall.action` field (in both `Let` bindings and `Do` steps) must
  be non-empty after trimming.
- Every `MaybeBlock.because` field must be non-empty after trimming.
- Every `MaybeBlock.do_steps` list must contain at least one step (an empty
  `maybe` block is meaningless).
- Validation recurses into nested `maybe` blocks (a `maybe` containing another
  `maybe` with a blank `because` is caught).

This is Roadmap Phase 1, Step 1.2.3. It builds on Steps 1.2.1
(post-deserialization non-empty validation) and 1.2.2 (expression syntax
validation). The specification requirements are Theorem File Specification
(TFS) `TFS-4` sections 3.8, 3.9, and 4.2.3 (step and action schemas, maybe
block requirements) and Design document (DES) `DES-4` section 4.4 (step forms
in Let and Do).

Observable success: running `make test` passes with new tests that confirm (a)
valid documents with Let bindings and Do steps (including nested maybe blocks)
continue to parse successfully, and (b) documents with blank `action` fields,
blank `maybe.because` fields, and empty `maybe.do` lists are rejected with
clear, deterministic error messages identifying the theorem name, the section,
and the reason for rejection. Existing tests remain green (no regressions).

Note on `LetBinding` variant restriction: the `LetBinding` enum
(`src/schema/types.rs:151-159`) already has only `Call` and `Must` variants (no
`Maybe`). Serde's `#[serde(untagged)]` deserialization rejects any YAML that
doesn't match either variant shape. A `maybe:` block inside `Let` produces a
serde deserialization error. This is sufficient for v1 — no additional runtime
validation is needed to enforce "Let allows only call or must". The serde error
message is generic ("data did not match any variant") but is acceptable because
it correctly rejects the input; improving that message is a Step 1.3 concern
(diagnostics quality).

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
  and fixture files.
- Existing valid fixtures must continue to parse successfully.
- No new external dependencies required.

## Tolerances (exception triggers)

- Scope: if implementation requires more than 3 new source files or 400 net
  lines of code, stop and escalate.
- Dependencies: no new dependencies expected. If one is required, stop and
  escalate.
- Iterations: if a test or lint failure persists after 5 attempts, stop and
  escalate.
- Ambiguity: the normative spec is `docs/theorem-file-specification.md`. If
  the spec is ambiguous on a point that materially affects which step/binding
  forms are accepted or rejected, document the ambiguity in `Decision Log` and
  escalate.

## Risks

- Risk: adding step validation logic and integration to `validate.rs` would
  breach the 400-line limit. Severity: medium. Likelihood: certain
  (`validate.rs` is at 395 lines). Mitigation: create a new
  `src/schema/step.rs` module for step/let-binding validation, keeping
  `validate.rs` as the integration point with thin wrappers. This follows the
  established pattern from Step 1.2.2 where `expr.rs` was extracted.

- Risk: existing valid fixtures contain Let bindings and Do steps that must
  continue to pass. Severity: medium. Likelihood: low (audit shows
  `valid_full.theorem` has well-formed Let/Do/Maybe blocks with non-empty
  action names and because fields). Mitigation: audit all existing fixtures
  before implementing new validation.

- Risk: recursive validation of nested `maybe` blocks could increase cognitive
  complexity beyond the Clippy limit (9). Severity: low. Likelihood: low
  (recursion with early return is straightforward). Mitigation: keep the
  recursive function simple with clear match arms and early returns.

## Progress

- [x] (2026-02-17) Write ExecPlan document.
- [x] (2026-02-17) Milestone 0: audit existing fixtures for compliance.
- [x] (2026-02-17) Milestones 1+2: create `src/schema/step.rs` with validation
  logic and unit tests; integrate into `validate.rs`.
- [x] (2026-02-17) Milestone 3: create test fixtures and behaviour-driven
  development (BDD) tests.
- [x] (2026-02-17) Milestone 4: documentation updates.
- [x] (2026-02-17) Milestone 5: final quality gates (223 tests, 0 failures).

## Surprises & discoveries

- `validate.rs` exceeded 400 lines after integration. Resolved by
  consolidating three single-purpose expression validation helpers
  (`validate_assumption_exprs`, `validate_assertion_exprs`,
  `validate_witness_exprs`) into one `validate_expressions` function, and
  merging two rstest parameterized test groups into one. Final line count: 397.
- Clippy's `semicolon_if_nothing_returned` lint required adding semicolons
  inside match arm braces for the `?` expressions in `validate_step`.
- Clippy's `unwrap_used` lint rejected `unwrap_err()` in tests; replaced with
  `expect_err("should fail")` following the pattern in `identifier.rs`.

## Decision log

- D1: create `src/schema/step.rs` rather than extending `validate.rs`.
  `validate.rs` is at 395 lines. Adding step validation logic plus unit tests
  would push it past the 400-line limit. The step validation concern
  (structural shape checking for ActionCall, MaybeBlock, Step, and LetBinding)
  is distinct from existing validation (non-emptiness, expression syntax,
  evidence constraints), so a dedicated module improves separation of concerns.
  The new module exports `pub(crate)` functions consumed by `validate.rs`. This
  follows the pattern established in Step 1.2.2 with `expr.rs`. Date:
  2026-02-17.

- D2: return `Result<(), String>` from `step.rs` (not `SchemaError`).
  The caller in `validate.rs` has the context (theorem doc, section name)
  needed to construct `SchemaError::ValidationFailed`. Returning a plain reason
  string keeps `step.rs` decoupled from the error type. This matches the
  `expr.rs` precedent. Date: 2026-02-17.

- D3: no additional runtime check for `LetBinding` variant restriction.
  The `LetBinding` enum has only `Call` and `Must` variants. Serde's untagged
  deserialization already rejects `maybe` in Let blocks at parse time. Adding a
  redundant runtime check would be defensive but unnecessary. The serde error
  message quality is a Step 1.3 concern. Date: 2026-02-17.

- D4: use path-based error context for recursive validation.
  Error messages for nested steps use a path string (e.g., "Do step 2: maybe.do
  step 1") that builds up through recursion. This gives users a clear
  breadcrumb trail to the offending field without requiring source location
  tracking (which is a Step 1.3 concern). Date: 2026-02-17.

## Outcomes & retrospective

All milestones complete. Test count increased from 204 to 223 (19 new tests: 16
unit tests in `step.rs`, 5 BDD tests in `schema_bdd.rs`, minus 2 tests merged
during `validate.rs` consolidation). All quality gates pass. The existing
`valid_full.theorem` fixture (which exercises Let bindings and Do steps
including nested maybe blocks) continues to parse successfully. The module
extraction pattern (`step.rs` following `expr.rs`) kept `validate.rs` at 397
lines (under the 400-line limit).

## Context and orientation

The `theoremc` crate compiles human-readable `.theorem` YAML files into Kani
model-checking proof harnesses. The schema is defined in
`docs/theorem-file-specification.md` (`TFS-4`) and the design rationale in
`docs/theoremc-design.md` (`DES-4`).

The relevant type hierarchy for this task:

- `TheoremDoc.let_bindings: IndexMap<String, LetBinding>` — named bindings
  evaluated before Do steps.
- `TheoremDoc.do_steps: Vec<Step>` — ordered sequence of theorem steps.
- `LetBinding` (untagged enum): `Call(LetCall)` | `Must(LetMust)` — each
  wraps an `ActionCall`.
- `Step` (untagged enum): `Call(StepCall)` | `Must(StepMust)` |
  `Maybe(StepMaybe)` — StepCall/StepMust wrap `ActionCall`, StepMaybe wraps
  `MaybeBlock`.
- `MaybeBlock`: `because: String` + `do_steps: Vec<Step>` (recursive).
- `ActionCall`: `action: String` + `args: IndexMap<String, TheoremValue>` +
  `as_binding: Option<String>`.

Before this change, the validation pipeline (`validate_theorem_doc` in
`src/schema/validate.rs`) checks: About non-empty, Prove non-empty, assertion
fields non-empty, assumption fields non-empty, witness fields non-empty,
expression syntax, and evidence constraints. It does NOT validate the internal
structure of Let bindings, Do steps, or MaybeBlock fields.

Key files:

- `src/schema/mod.rs` (25 lines) — module declarations and public re-exports.
- `src/schema/types.rs` (312 lines) — schema struct and enum definitions.
- `src/schema/validate.rs` (395 lines) — post-deserialization validation. At
  the 400-line limit; new logic must be extracted.
- `src/schema/expr.rs` (145 lines) — expression syntax validation (Step 1.2.2
  pattern to follow).
- `src/schema/error.rs` (28 lines) — `SchemaError` enum.
- `src/schema/loader.rs` (336 lines) — YAML loading and validation
  orchestration.
- `tests/schema_bdd.rs` (278 lines) — BDD-style acceptance tests.
- `tests/schema_deser.rs` (211 lines) — happy-path deserialization tests.
- `tests/schema_deser_reject.rs` (241 lines) — unhappy-path rejection tests.
- `tests/fixtures/` — 33 fixture files (5 valid, 28 invalid).
- Current test count: 204 (79 unit + 86 integration + 15 deser + 21 reject +
  3 docs, 1 ignored).

## Plan of work

### Milestone 0: audit existing fixtures for compliance

Before implementing new validation, confirm that all `ActionCall.action` fields
in existing valid fixtures are non-empty, all `MaybeBlock.because` fields are
non-empty, and all `MaybeBlock.do_steps` lists are non-empty.

The fixtures to audit are:

- `tests/fixtures/valid_full.theorem`: contains Let with `must` (action:
  `account.params`) and `call` (action: `account.deposit`), Do with `must`
  (action: `account.validate`) and `maybe` (because: "optional second deposit",
  do: one `call` step). All fields are non-empty. No changes needed.
- `tests/fixtures/valid_minimal.theorem`: no Let or Do sections. No impact.
- `tests/fixtures/valid_multi.theorem`: check for Let/Do content.
- `tests/fixtures/valid_lowercase.theorem`: check for Let/Do content.
- `tests/fixtures/valid_vacuous.theorem`: check for Let/Do content.
- Inline YAML constants in `validate.rs` unit tests (`VALID_BASE`): no Let or
  Do sections. No impact.

Gate: visual audit only. No code changes.

### Milestone 1: create `src/schema/step.rs` with validation logic and unit tests

Create `src/schema/step.rs` following the `expr.rs` pattern. The module
provides `pub(crate)` validation functions that return `Result<(), String>`
(decoupled from `SchemaError`), allowing `validate.rs` to wrap them with
theorem-context error messages.

Module doc comment (`//!`) explaining its purpose: post-deserialization
structural validation for `Step`, `LetBinding`, `MaybeBlock`, and `ActionCall`
shapes.

Public functions:

    /// Validates that an action call's `action` field is non-empty after
    /// trimming.
    pub(crate) fn validate_action_call(
        action_call: &ActionCall,
    ) -> Result<(), String>

    /// Validates a single step's structural constraints.
    pub(crate) fn validate_step(
        step: &Step,
        path: &str,
        pos: usize,
    ) -> Result<(), String>

    /// Validates a list of steps, used for both top-level `Do` and
    /// nested `maybe.do` sequences.
    pub(crate) fn validate_step_list(
        steps: &[Step],
        path: &str,
    ) -> Result<(), String>

The `path` parameter provides context for error messages (e.g., "Do step", "Do
step 2: maybe.do step"). The `pos` parameter provides 1-based position within
the current list.

Implementation of `validate_action_call`:

1. If `action_call.action.trim().is_empty()`, return
   `Err("action must be non-empty after trimming".to_owned())`.
2. Otherwise, return `Ok(())`.

Implementation of `validate_step`:

1. Match on the step variant:
   - `Step::Call(c)` → call `validate_action_call(&c.call)` and map error
     with `format!("{path} {pos}: {reason}")`.
   - `Step::Must(m)` → call `validate_action_call(&m.must)` and map error
     with `format!("{path} {pos}: {reason}")`.
   - `Step::Maybe(m)` → validate the MaybeBlock:
     a. If `m.maybe.because.trim().is_empty()`, return
        `Err(format!("{path} {pos}: maybe.because must be non-empty after
        trimming"))`.
     b. If `m.maybe.do_steps.is_empty()`, return
        `Err(format!("{path} {pos}: maybe.do must contain at least one
        step"))`.
     c. Call `validate_step_list(&m.maybe.do_steps, &format!("{path} {pos}:
        maybe.do step"))` to recurse into nested steps.

Implementation of `validate_step_list`:

1. For each `(i, step)` in `steps.iter().enumerate()`, call
   `validate_step(step, path, i + 1)?`.
2. Return `Ok(())`.

Add `mod step;` to `src/schema/mod.rs`. Nothing is `pub use`-exported — the
module is `pub(crate)` only.

Unit tests in `#[cfg(test)] mod tests` within `step.rs`:

Use `rstest` parameterized tests. Build minimal `ActionCall`, `Step`, and
`MaybeBlock` values directly (no YAML parsing) to test each validation rule in
isolation.

Happy-path cases (all return `Ok(())`):

- ActionCall with non-empty action → Ok.
- Step::Call with valid ActionCall → Ok.
- Step::Must with valid ActionCall → Ok.
- Step::Maybe with non-empty because, non-empty do_steps containing valid
  steps → Ok.

Unhappy-path cases (all return `Err(...)`):

- ActionCall with empty action → Err containing "action must be non-empty".
- ActionCall with whitespace-only action → Err containing "action must be
  non-empty".
- Step::Maybe with empty because → Err containing "maybe.because must be
  non-empty".
- Step::Maybe with whitespace-only because → Err containing "maybe.because
  must be non-empty".
- Step::Maybe with empty do_steps → Err containing "maybe.do must contain
  at least one step".
- Nested maybe with empty because → Err containing "maybe.do step 1:
  maybe.because must be non-empty".

Gate: `make check-fmt && make lint && make test` (combined with Milestone 2 to
avoid dead-code lint, following the Step 1.2.2 precedent).

### Milestone 2: integrate step validation into `validate.rs`

Add `use super::step;` to `validate.rs`.

Add two thin wrapper functions:

    fn validate_let_bindings(doc: &TheoremDoc) -> Result<(), SchemaError>

Iterates `doc.let_bindings`. For each `(name, binding)`, extracts the
`ActionCall` (from `LetBinding::Call` or `LetBinding::Must`) and calls
`step::validate_action_call(action_call)`. Maps errors as
`fail(doc, format!("Let binding '{name}': {reason}"))`.

    fn validate_do_steps(doc: &TheoremDoc) -> Result<(), SchemaError>

Calls `step::validate_step_list(&doc.do_steps, "Do step")`. Maps errors as
`fail(doc, reason)` (the reason string already contains the full path context
from `step.rs`).

Insert both calls into `validate_theorem_doc` after
`validate_expressions(doc)?;` and before `validate_evidence(doc)?;`:

    validate_let_bindings(doc)?;
    validate_do_steps(doc)?;

Update the doc comment on `validate_theorem_doc` to include step/let-binding
validation in the list of applied checks.

Gate: `make check-fmt && make lint && make test` (combined with Milestone 1).

### Milestone 3: create test fixtures and BDD tests

Create 5 new fixture files in `tests/fixtures/`:

1. `invalid_maybe_empty_because.theorem` — `maybe` block with empty `because`
   field. Template: valid_full.theorem with `because: ""` on the maybe block.

2. `invalid_maybe_empty_do.theorem` — `maybe` block with empty `do` list.
   Template: valid_full.theorem with `do: []` on the maybe block.

3. `invalid_let_empty_action.theorem` — Let binding with blank action name.
   Template: valid_full.theorem with `action: ""` on a Let must binding.

4. `invalid_step_empty_action.theorem` — Do step with blank action name.
   Template: valid_full.theorem with `action: ""` on a Do must step.

5. `invalid_nested_maybe_empty_because.theorem` — nested maybe with blank
   `because`. Template: valid_full.theorem with a nested maybe inside the
   existing maybe, with `because: ""`.

Add a new BDD test group in `tests/schema_bdd.rs`:

    // -- Given invalid Step or LetBinding shapes, validation fails -----

    #[rstest]
    #[case::maybe_empty_because(
        "invalid_maybe_empty_because.theorem",
        "maybe.because must be non-empty"
    )]
    #[case::maybe_empty_do(
        "invalid_maybe_empty_do.theorem",
        "maybe.do must contain at least one step"
    )]
    #[case::let_empty_action(
        "invalid_let_empty_action.theorem",
        "action must be non-empty"
    )]
    #[case::step_empty_action(
        "invalid_step_empty_action.theorem",
        "action must be non-empty"
    )]
    #[case::nested_maybe_empty_because(
        "invalid_nested_maybe_empty_because.theorem",
        "maybe.because must be non-empty"
    )]
    fn given_invalid_step_or_let_shape_when_loaded_then_validation_fails(
        #[case] fixture: &str,
        #[case] expected_fragment: &str,
    ) {
        assert_fixture_err_contains(fixture, expected_fragment);
    }

Gate: `make check-fmt && make lint && make test`.

### Milestone 4: documentation updates

1. `docs/roadmap.md`: change `- [ ]` to `- [x]` for step 1.2.3 ("Enforce
   `Step` and `LetBinding` shape rules").

2. `docs/contents.md`: add entry for the new execplan under the "Execution
   plans" section.

3. `docs/theoremc-design.md`: add section recording Step 1.2.3 implementation
   decisions. Insert as a new §6.7 after the existing §6.6 ("Implementation
   decisions (Step 1.1)") and renumber the subsequent sections (current §6.7
   "Localized diagnostics contract" becomes §6.8, etc.). Update the `DES-6.5`
   anchor link in `docs/roadmap.md` to point to the correct new anchor.

4. `docs/users-guide.md`: add a "Step and Let binding validation" subsection
   after the "Expression syntax validation" section. Document the new
   validation rules: ActionCall.action non-empty, MaybeBlock.because non-empty,
   MaybeBlock.do non-empty, recursive validation of nested steps.

Gate: `make check-fmt` (for any Markdown formatting).

### Milestone 5: final quality gates

    set -o pipefail
    make check-fmt 2>&1 | tee /tmp/check-fmt.log
    make lint 2>&1 | tee /tmp/lint.log
    make test 2>&1 | tee /tmp/test.log

All three must exit 0. Test count should increase from 204.

## Concrete steps

All commands run from `/home/user/project`.

Milestone 0: visual audit of Let/Do/Maybe fields in existing valid fixtures and
inline YAML test constants. Confirmed all action names and because fields are
non-empty, and all maybe.do lists are non-empty. No changes needed.

Milestones 1+2 (combined to avoid dead-code lint):

1. Create `src/schema/step.rs` with module doc comment, validation functions,
   and unit tests.
2. Add `mod step;` to `src/schema/mod.rs`.
3. Add `use super::step;` and thin wrapper functions to
   `src/schema/validate.rs`.
4. Insert `validate_let_bindings(doc)?;` and `validate_do_steps(doc)?;` into
   `validate_theorem_doc`.
5. Update the doc comment on `validate_theorem_doc`.
6. Run `make fmt && make check-fmt && make lint && make test`.

Milestone 3:

1. Create 5 new fixture files in `tests/fixtures/`.
2. Add BDD test group to `tests/schema_bdd.rs`.
3. Run `make fmt && make check-fmt && make lint && make test`.

Milestone 4:

1. Update `docs/roadmap.md` checkbox.
2. Update `docs/contents.md` index.
3. Add implementation decisions section to `docs/theoremc-design.md`.
4. Update `docs/users-guide.md` with step/let-binding validation section.
5. Run `make check-fmt` (markdown validation).

Milestone 5: final quality gates.

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New tests cover: valid Let/Do/Maybe documents
  accepted, blank action names rejected, blank maybe.because rejected, empty
  maybe.do rejected, nested maybe validation errors caught.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Existing 204 tests continue to pass (no regressions).
- The valid_full.theorem fixture (which exercises Let bindings and Do steps
  including nested maybe) continues to parse successfully.

Quality method:

    set -o pipefail
    make check-fmt 2>&1 | tee /tmp/check-fmt.log
    make lint 2>&1 | tee /tmp/lint.log
    make test 2>&1 | tee /tmp/test.log

Expected: all three commands exit 0. Test count increases from 204.

## Idempotence and recovery

All steps are additive and re-runnable. No destructive operations. If a
milestone fails, fix the issue and re-run
`make check-fmt && make lint && make test` from the repo root.

## Artifacts and notes

Key file paths (all relative to repo root):

- `src/schema/step.rs` — step/let-binding validation module (NEW).
- `src/schema/mod.rs` — module declaration (modified: +1 line).
- `src/schema/validate.rs` — validation pipeline integration (modified).
- `tests/schema_bdd.rs` — BDD tests (modified).
- `tests/fixtures/invalid_maybe_empty_because.theorem` — fixture (NEW).
- `tests/fixtures/invalid_maybe_empty_do.theorem` — fixture (NEW).
- `tests/fixtures/invalid_let_empty_action.theorem` — fixture (NEW).
- `tests/fixtures/invalid_step_empty_action.theorem` — fixture (NEW).
- `tests/fixtures/invalid_nested_maybe_empty_because.theorem` — fixture (NEW).
- `docs/roadmap.md` — roadmap checkbox (modified).
- `docs/contents.md` — contents index (modified).
- `docs/theoremc-design.md` — design spec (modified: +1 section, renumbered).
- `docs/users-guide.md` — user guide (modified: +1 subsection).
- `docs/execplans/1-2-3-enforce-step-and-let-binding.md` — this ExecPlan
  (NEW).

## Interfaces and dependencies

No new external dependencies.

Internal interface added in `src/schema/step.rs`:

    /// Validates that an action call's `action` field is non-empty after
    /// trimming.
    pub(crate) fn validate_action_call(
        action_call: &ActionCall,
    ) -> Result<(), String>

    /// Validates a single step's structural constraints.
    pub(crate) fn validate_step(
        step: &Step,
        path: &str,
        pos: usize,
    ) -> Result<(), String>

    /// Validates a list of steps (top-level Do or nested maybe.do).
    pub(crate) fn validate_step_list(
        steps: &[Step],
        path: &str,
    ) -> Result<(), String>

Called from `validate_let_bindings` and `validate_do_steps` in
`src/schema/validate.rs`, which are called from `validate_theorem_doc` after
expression validation and before evidence validation.
