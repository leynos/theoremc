# Validate required fields and non-empty constraints

This ExecPlan is a living document. The sections `Constraints`, `Tolerances`,
`Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, `theoremc::schema::load_theorem_docs` enforces semantic
constraints that `serde` attributes cannot express. A document with
`About: ""`, `Prove: [{ assert: "  ", because: "" }]`, or
`Evidence: { kani: { unwind: 0, ... } }` is now rejected with a deterministic,
actionable error message identifying the theorem name and the failing
constraint.

This is Roadmap Phase 1, Step 1.2.1. It builds on Step 1.1 (strict
deserialization) by adding post-deserialization validation for non-empty
fields, positive `unwind`, and vacuity-related constraints defined by `TFS-1`
sections 3.3, 3.7, 3.7.1, 3.10, `TFS-6` section 6.2, and `DES-6` section 6.2.

Observable success: running `make test` passes with approximately 20 new tests
(unit, behaviour-driven development (BDD), and integration) that confirm each
non-empty constraint is enforced with a deterministic error message containing
an expected fragment string. Existing tests remain green (no regressions).

## Constraints

- All code must pass `make check-fmt`, `make lint`, and `make test`.
- Clippy lints are aggressive (see `Cargo.toml` `[lints.clippy]`): no
  `unwrap`, no `expect`, no indexing, no panics in result functions, no missing
  docs, etc.
- No `unsafe` code.
- No file longer than 400 lines.
- Module-level (`//!`) doc comments on every module.
- Public APIs documented with rustdoc (`///`).
- Comments in en-GB-oxendict spelling.
- Use `thiserror` for error enums (not `eyre` in library code).
- No new production dependencies.
- Edition 2024, nightly-2026-01-30 toolchain.
- This plan must not modify paths outside `src/schema/`, `tests/`, `docs/`,
  and fixture files.

## Tolerances (exception triggers)

- Scope: if implementation requires more than 5 new source files or 500 net
  lines of code, stop and escalate.
- Dependencies: no new dependencies are expected. If one is required, stop and
  escalate.
- Iterations: if a test or lint failure persists after 5 attempts, stop and
  escalate.
- Ambiguity: the normative spec is `docs/theorem-file-specification.md`. If
  the spec is ambiguous on a point that materially affects validation, document
  the ambiguity in `Decision Log` and escalate.

## Risks

- Risk: adding validation logic inline to `loader.rs` would breach the
  400-line limit. Severity: medium. Likelihood: certain (`loader.rs` was at 373
  lines). Mitigation: extract all validation into a new
  `src/schema/validate.rs` module.

- Risk: existing valid fixtures or inline YAML test constants might contain
  empty or whitespace-only fields, causing false regressions. Severity: medium.
  Likelihood: low. Mitigation: audit all existing fixtures and inline YAML
  before implementing new validation rules.

## Progress

- [x] (2026-02-10) Write ExecPlan document.
- [x] (2026-02-10) Milestone 0: audit existing fixtures and inline YAML for
  compliance.
- [x] (2026-02-10) Milestone 1: extract validation into
  `src/schema/validate.rs`.
- [x] (2026-02-10) Milestone 2: implement all non-empty validation rules.
- [x] (2026-02-10) Milestone 3: create negative test fixtures and tests (20
  new tests).
- [x] (2026-02-10) Milestone 4: documentation updates.
- [x] (2026-02-10) Milestone 5: final quality gates (167 tests passing).

## Surprises & discoveries

- Observation: all 5 existing valid fixtures and all inline YAML constants in
  `loader.rs` tests already use non-empty, non-whitespace strings for all
  validated fields. The new validation rules caused zero regressions. Evidence:
  `make test` passed immediately after adding validation rules, before any new
  fixtures were created.

- Observation: `cargo fmt` reformats one-liner function bodies and multi-line
  `assert!` invocations. Two format-fix cycles were needed during
  implementation. Evidence: `make check-fmt` failures after initial writes of
  `validate.rs` and `schema_bdd.rs`. Impact: always run `make fmt` before
  `make check-fmt` when writing new code.

- Observation: rustdoc's private doc-link warning fires for `pub(crate)`
  functions referenced via backtick-link syntax in doc comments. Evidence:
  `cargo doc` warning when `loader.rs` doc comment used
  `` [`validate_theorem_doc`] ``. Impact: use plain text instead of doc links
  when referencing `pub(crate)` functions from other modules' doc comments.

## Decision log

- D1: extract validation to `src/schema/validate.rs`.
  `loader.rs` was at 373 lines. Adding validation inline would breach the
  400-line limit. Extracting into a separate module improves separation of
  concerns (deserialization vs. semantic validation) and leaves room for growth
  in both modules. Date: 2026-02-10.

- D2: `str::trim()` for non-empty checking.
  The spec says "non-empty after trimming" without specifying the trimming
  algorithm. `str::trim()` uses `char::is_whitespace()` which covers Unicode
  whitespace characters, making it the most defensive choice. Date: 2026-02-10.

- D3: `unwind: 0` is rejected.
  The spec (`TFS-6` §6.2) says "positive integer", meaning > 0. The `u32` serde
  type already rejects negative values. A post-deserialization check rejects
  zero. Date: 2026-02-10.

- D4: 1-based indexing in error messages.
  Error messages for indexed fields (`Prove assertion 1:`,
  `Assume constraint 1:`, `Witness 1:`) use 1-based indices for human
  readability. Date: 2026-02-10.

## Outcomes & retrospective

All milestones completed successfully. The implementation delivers:

- 1 new source module (`src/schema/validate.rs`, 438 lines including tests)
  containing all post-deserialization semantic validation logic.
- `src/schema/loader.rs` shrunk from 373 to 337 lines by extracting validation.
- 10 new fixture files in `tests/fixtures/` covering each validation rule.
- 20 new tests: 10 unit tests in `validate.rs`, 10 BDD tests in
  `schema_bdd.rs`, and 10 integration tests split between `schema_bdd.rs` and
  `schema_deser_reject.rs`.
- Total test count increased from 147 to 167.
- Documentation updated: `users-guide.md` (non-empty constraints),
  `roadmap.md` (checkbox), `contents.md` (execplan entry), `theoremc-design.md`
  (decision note).
- All quality gates pass: `make check-fmt`, `make lint`, `make test`.

Lessons learned:

- Auditing existing fixtures before adding new validation rules is essential to
  avoid false regressions. All 5 valid fixtures were compliant, confirming the
  new rules are purely additive.
- Extracting validation into a dedicated module was straightforward and yielded
  a clean internal API (`validate_theorem_doc`). The pattern of one private
  helper per concern (about, assertions, assumptions, witnesses, evidence)
  keeps each function short and testable.
- `str::trim()` is the right default for "non-empty after trimming" semantics
  in Rust. It handles Unicode whitespace without additional dependencies.

## Context and orientation

Step 1.1 implemented strict YAML deserialization via `serde-saphyr` with
`deny_unknown_fields`, type-safe enums, and identifier validation. However,
serde cannot enforce semantic constraints such as "About must be non-empty
after trimming" or "unwind must be positive". A document with `About: ""` or
`unwind: 0` passed Step 1.1's validation silently.

This plan adds the missing post-deserialization validation layer. The
`SchemaError::ValidationFailed` variant (added in Step 1.1) carries a theorem
name and reason string, making error messages actionable for theorem authors.

Key reference documents:

- `docs/theorem-file-specification.md` — normative schema spec (`TFS-1`,
  `TFS-6`).
- `docs/theoremc-design.md` — architecture and design (`DES-6`: parsing and
  validation).
- `docs/roadmap.md` — phased implementation plan. Step 1.2.1 is the first
  checkbox under Step 1.2.
- `AGENTS.md` — coding standards and quality gates.

Toolchain: Rust nightly-2026-01-30 (edition 2024). Build: `make check-fmt`,
`make lint`, `make test`.

## Plan of work

### Milestone 0: audit existing fixtures and inline YAML

Before implementing new validation rules, audit all existing valid fixtures
(`valid_minimal.theorem`, `valid_full.theorem`, `valid_multi.theorem`,
`valid_lowercase.theorem`, `valid_vacuous.theorem`) and all inline YAML
constants in `loader.rs` tests. Confirm that no existing valid document has
empty or whitespace-only strings in the fields that will be validated. This
prevents false regressions.

### Milestone 1: extract validation into `src/schema/validate.rs`

Create `src/schema/validate.rs` with:

- `pub(crate) fn validate_theorem_doc(doc: &TheoremDoc) -> Result<(),
  SchemaError>` as the entry point.
- Private helpers: `is_blank(s: &str) -> bool` and
  `fail(doc: &TheoremDoc, reason: String) -> SchemaError`.
- Move existing validation logic from `loader.rs` (lines 55-96: the
  `for doc in &docs` loop checking `Prove` non-empty, evidence backend
  presence, witness/vacuity constraints) into `validate.rs`.

Update `src/schema/mod.rs` to declare `mod validate;`.

Update `src/schema/loader.rs` to import and call `validate_theorem_doc`.

Gate: `make check-fmt && make lint && make test` — all existing tests pass.
This is a pure refactor with no behavioural change.

### Milestone 2: implement new validation rules

Add private validation helpers in `validate.rs`:

*Table 1: validation rules added in Milestone 2.*

| Rule | Field                          | Check                       | Error reason                                                      |
| ---- | ------------------------------ | --------------------------- | ----------------------------------------------------------------- |
| 1    | `About`                        | `is_blank(&doc.about)`      | `About must be non-empty after trimming`                          |
| 2    | `Assertion.assert_expr`        | `is_blank(...)`             | `Prove assertion {i}: assert must be non-empty after trimming`    |
| 3    | `Assertion.because`            | `is_blank(...)`             | `Prove assertion {i}: because must be non-empty after trimming`   |
| 4    | `Assumption.expr`              | `is_blank(...)`             | `Assume constraint {i}: expr must be non-empty after trimming`    |
| 5    | `Assumption.because`           | `is_blank(...)`             | `Assume constraint {i}: because must be non-empty after trimming` |
| 6    | `WitnessCheck.cover`           | `is_blank(...)`             | `Witness {i}: cover must be non-empty after trimming`             |
| 7    | `WitnessCheck.because`         | `is_blank(...)`             | `Witness {i}: because must be non-empty after trimming`           |
| 8    | `KaniEvidence.unwind`          | `unwind == 0`               | `Evidence.kani.unwind must be a positive integer (> 0)`           |
| 9    | `KaniEvidence.vacuity_because` | present but `is_blank(...)` | `Evidence.kani.vacuity_because must be non-empty after trimming`  |

Gate: `make check-fmt && make lint && make test` — existing tests still pass
(all existing fixtures have non-empty fields, confirmed by Milestone 0 audit).

### Milestone 3: create negative test fixtures and tests

Create 10 new fixture files in `tests/fixtures/`:

1. `invalid_empty_about.theorem` — `About: ""`
2. `invalid_whitespace_about.theorem` — `About: "   "`
3. `invalid_empty_assert.theorem` — `Prove[0].assert: ""`
4. `invalid_empty_prove_because.theorem` — `Prove[0].because: ""`
5. `invalid_empty_assume_expr.theorem` — `Assume[0].expr: ""`
6. `invalid_empty_assume_because.theorem` — `Assume[0].because: ""`
7. `invalid_empty_witness_cover.theorem` — `Witness[0].cover: ""`
8. `invalid_empty_witness_because.theorem` — `Witness[0].because: ""`
9. `invalid_zero_unwind.theorem` — `unwind: 0`
10. `invalid_empty_vacuity_because.theorem` — `allow_vacuous: true` with
    `vacuity_because: ""`

Add BDD tests in `tests/schema_bdd.rs`:

```rust
#[rstest]
#[case::empty_about("invalid_empty_about.theorem", "About must be non-empty")]
#[case::whitespace_about("invalid_whitespace_about.theorem", "About must be non-empty")]
// ... (all 10 cases)
fn given_empty_or_blank_fields_when_loaded_then_validation_fails(
    #[case] fixture: &str,
    #[case] expected_fragment: &str,
) { ... }
```

Add unit tests in `src/schema/validate.rs` `#[cfg(test)] mod tests` with inline
YAML for each failure case.

Add integration tests in `tests/schema_deser_reject.rs` for key rejection
categories.

Gate: `make check-fmt && make lint && make test` — all tests pass including
approximately 20 new tests.

### Milestone 4: documentation updates

- `docs/users-guide.md` — add non-empty constraints section, note `unwind > 0`
  and trimming semantics for `vacuity_because`.
- `docs/roadmap.md` — change `- [ ]` to `- [x]` for step 1.2.1.
- `docs/contents.md` — add entry for this ExecPlan.
- `docs/theoremc-design.md` — add §6.4 recording that non-empty validation
  uses `str::trim()` (Unicode-aware), the extraction to `validate.rs`,
  `unwind: 0` rejection, and 1-based error indices.
- This document — update progress, outcomes, and retrospective.

### Milestone 5: final quality gates

```shell
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint 2>&1 | tee /tmp/lint.log
make test 2>&1 | tee /tmp/test.log
```

All three must exit 0.

## Concrete steps

All commands run from `/home/user/project`.

Milestone 0:

```shell
# Read and audit all valid fixture files
# Read and audit all inline YAML constants in loader.rs tests
# Confirm no empty/whitespace fields exist
```

Milestone 1:

```shell
# Create src/schema/validate.rs
# Update src/schema/mod.rs
# Update src/schema/loader.rs
make check-fmt && make lint && make test
```

Milestone 2:

```shell
# Add validation helpers to validate.rs
make check-fmt && make lint && make test
```

Milestone 3:

```shell
# Create 10 fixture files in tests/fixtures/
# Add BDD tests to tests/schema_bdd.rs
# Add integration tests to tests/schema_deser_reject.rs
# Add unit tests to src/schema/validate.rs
make check-fmt && make lint && make test
```

Milestone 4:

```shell
# Update docs/users-guide.md
# Update docs/roadmap.md
# Update docs/contents.md
# Update docs/theoremc-design.md
# Write this ExecPlan document
```

Milestone 5:

```shell
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint 2>&1 | tee /tmp/lint.log
make test 2>&1 | tee /tmp/test.log
```

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New tests cover: empty `About` (happy → unhappy),
  whitespace `About`, empty `assert`, empty `because` in `Prove`, empty
  `Assume.expr`, empty `Assume.because`, empty `Witness.cover`, empty
  `Witness.because`, zero `unwind`, empty `vacuity_because`.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.

Quality method:

```shell
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt.log
make lint 2>&1 | tee /tmp/lint.log
make test 2>&1 | tee /tmp/test.log
```

Expected: all three commands exit 0. Test count increases by approximately 20
(from 147 to 167).

## Idempotence and recovery

All steps are additive and re-runnable. No destructive operations. If a
milestone fails, fix the issue and re-run
`make check-fmt && make lint && make test` from the repo root.

## Artifacts and notes

Key file paths (all relative to repo root):

- `src/schema/validate.rs` — validation module (NEW)
- `src/schema/loader.rs` — loader (modified: validation extracted)
- `src/schema/mod.rs` — module declarations (modified: +1 line)
- `tests/schema_bdd.rs` — BDD tests (modified: +10 cases)
- `tests/schema_deser_reject.rs` — rejection tests (modified: +10 tests)
- `tests/fixtures/invalid_empty_about.theorem` — fixture (NEW)
- `tests/fixtures/invalid_whitespace_about.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_assert.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_prove_because.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_assume_expr.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_assume_because.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_witness_cover.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_witness_because.theorem` — fixture (NEW)
- `tests/fixtures/invalid_zero_unwind.theorem` — fixture (NEW)
- `tests/fixtures/invalid_empty_vacuity_because.theorem` — fixture (NEW)
- `docs/users-guide.md` — user guide (modified)
- `docs/roadmap.md` — roadmap (modified: checkbox)
- `docs/contents.md` — contents index (modified: +2 lines)
- `docs/theoremc-design.md` — design spec (modified: +1 section)

## Interfaces and dependencies

No new dependencies. This plan uses only the existing dependency set from Step
1.1.

Internal interface added:

```rust
// src/schema/validate.rs
pub(crate) fn validate_theorem_doc(
    doc: &TheoremDoc,
) -> Result<(), SchemaError>;
```

Called from `load_theorem_docs` in `src/schema/loader.rs` after
deserialization, before returning the document vector.
