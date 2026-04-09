# Step 2.1.3: fail compilation on duplicate action names

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, the `theoremc` library detects **mangled-identifier
collisions** in loaded `.theorem` documents and fails with actionable error
messages before any code generation occurs.

A mangled-identifier collision occurs when two or more *different* canonical
action names produce the same mangled Rust identifier via `mangle_action_name`.
Because the mangling algorithm is injective by design (proven in Step 2.1.2
tests), this should never happen for valid inputs. The check is a defensive
safety net that protects against algorithm regressions or hash collisions.

Multiple theorems referencing the same canonical action name is expected and
accepted — only distinct canonical names that collide after mangling are
reported as hard errors per `NMR-1` and `DES-5`.

Observable success: calling `check_action_collisions(&[TheoremDoc])` returns
`Ok(())` when no collisions exist, or returns
`Err(SchemaError::MangledIdentifierCollision { .. })` listing all colliding
names with their source theorems. Integration tests prove both collision
classes are detected. `make check-fmt`, `make lint`, and `make test` pass.

## Constraints

- ADR-003 boundary rule: the `schema` module and the `mangle` module must not
  cross-depend. Collision detection requires both, so it must live in a
  separate top-level module that imports from both.
- Source files must not exceed 400 lines.
- Use `rstest-bdd` v0.5.0 for behavioural tests, `rstest` for unit tests.
- Comments and documentation must use en-GB-oxendict spelling.
- No new external dependencies beyond what is already in `Cargo.toml`.
- Existing public API surfaces (`schema::*`, `mangle::*`) must not change
  their existing items (additive changes only).
- The `SchemaError` enum's existing variants must not be removed or have their
  fields changed (additive-only).
- Quality gates: `make check-fmt`, `make lint`, `make test` must pass.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 12 files or more than
  600 net lines of code, stop and escalate with a narrowed split.
- Interface: if a public API signature change to existing `schema` or `mangle`
  items is required, stop and escalate.
- Dependencies: if a new external dependency beyond what is already in
  `Cargo.toml` is needed, stop and escalate.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop and escalate with logs.
- Ambiguity: if the meaning of "duplicate canonical action name" is unclear in
  a concrete scenario, stop and present options.

## Risks

- Risk: Clippy `too_many_arguments` triggered by test helper functions with
  many parameters. Severity: medium. Likelihood: medium. Mitigation: use helper
  structs as per the `Golden` pattern established in `src/mangle.rs`.

- Risk: `src/schema/validate.rs` is already 388 lines, close to the 400-line
  limit. Severity: medium. Likelihood: low (collision detection will not be
  placed in this file). Mitigation: collision detection lives in a new
  top-level module `src/collision.rs`.

- Risk: Clippy `indexing_slicing = "deny"` may be triggered by collection
  access patterns. Severity: low. Likelihood: low. Mitigation: use iterators
  and `.get()` methods exclusively.

## Progress

- [x] (2026-02-27 00:00Z) Draft ExecPlan for Step 2.1.3.
- [x] (2026-02-27 00:05Z) Milestone 0: baseline verification (all existing
  tests pass).
- [x] (2026-02-27 00:10Z) Milestone 1: add `MangledIdentifierCollision` variant
      to
  `SchemaError` and create `src/collision.rs` module scaffold.
- [x] (2026-02-27 00:15Z) Milestone 2: implement action-name collection
  (traversal of `TheoremDoc`).
- [x] (2026-02-27 00:18Z) Milestone 3: implement collision detection logic
  (canonical and mangled).
- [x] (2026-02-27 00:20Z) Milestone 4: wire collision detection into the
  loader.
- [x] (2026-02-27 00:22Z) Milestone 5: add unit tests for collision
  detection.
- [x] (2026-02-27 00:28Z) Milestone 6: add test fixtures and BDD behavioural
  tests.
- [x] (2026-02-27 00:33Z) Milestone 7: update documentation and roadmap.
- [x] (2026-02-27 00:38Z) Milestone 8: run full quality gates and capture
  logs.

## Surprises & discoveries

- Observation: Clippy lint `elidable_lifetime_names` triggered on
  `collect_all_occurrences<'a>(docs: &'a [TheoremDoc])`. Evidence: `make lint`
  failed with this warning denied. Impact: replaced with elided lifetime
  `fn collect_all_occurrences(docs: &[TheoremDoc]) -> Vec<ActionOccurrence<'_>>`
   per Clippy's suggestion. Minimal impact.

- Observation: `TheoremName::new` takes `String` not `&str`. Evidence: compile
  error when constructing `TheoremDoc` in unit tests. Impact: used
  `.to_owned()` in test helper. No architectural impact.

## Decision log

- Decision: place collision detection in a new top-level module
  `src/collision.rs`, not inside `schema` or `mangle`. Rationale: collision
  detection requires importing from both `schema::types` (to traverse
  `TheoremDoc`) and `mangle` (to compute mangled identifiers). ADR-003
  prohibits cross-dependency between `schema` and `mangle`. A new top-level
  module that wires both together is the cleanest architectural choice and
  follows the same pattern used when `mangle` was created as a peer of
  `schema`. Date/Author: 2026-02-27 / DevBoxer.

- Decision: define "duplicate canonical action names" as identical canonical
  action strings in the collected set of unique names. Rationale: within a
  single loaded file, the set of unique canonical names is inherently
  deduplicated. The same action called from multiple theorems (e.g.,
  `account.deposit` in both a `Let` binding and a `Do` step across different
  theorem documents) is normal and expected. The canonical-name check builds
  the unique set and confirms cleanliness. The real defensive value is the
  mangled-identifier collision check. Both checks exist for completeness per
  NMR-1 and to support future cross-file collision detection (Step 3.x).
  Date/Author: 2026-02-27 / DevBoxer.

- Decision: use `BTreeMap` and `BTreeSet` rather than `HashMap`/`HashSet` for
  all collision grouping. Rationale: deterministic iteration order ensures
  error messages are stable and reproducible, which is important for snapshot
  tests and deterministic diagnostics. Date/Author: 2026-02-27 / DevBoxer.

- Decision: the collision check is called from `schema::loader` in
  `load_theorem_docs_with_source` after per-document validation. Rationale: the
  loader already orchestrates parse, validate, and return. Adding collision
  detection as a final pass before returning `Ok(docs)` is a natural extension.
  The loader imports `crate::collision::check_action_collisions`, which is a
  top-level module import — this does not violate ADR-003 because the schema
  module's types and validation logic do not themselves depend on mangle. Only
  the loader orchestration calls into collision, which in turn calls into
  mangle. Date/Author: 2026-02-27 / DevBoxer.

## Outcomes & retrospective

Step 2.1.3 is complete.

Implemented outcomes:

- Added `MangledIdentifierCollision { message: String }` variant to
  `SchemaError` in `src/schema/error.rs`.
- Created `src/collision.rs` (new top-level module) with:
  - `check_action_collisions(docs: &[TheoremDoc]) -> Result<(), SchemaError>`
    public entry point.
  - Internal traversal functions for `TheoremDoc` (let bindings, do steps,
    recursive maybe blocks).
  - `BTreeMap`/`BTreeSet` grouping for deterministic collision reporting.
  - Mangled-identifier collision detection via `mangle_action_name`.
  - Human-readable error message formatting.
- Wired module into `src/lib.rs` with `pub mod collision;`.
- Integrated collision check into `load_theorem_docs_with_source` in
  `src/schema/loader.rs`, after per-document validation.
- Added 13 unit tests in `src/collision.rs` covering: traversal (4 tests),
  collision detection (5 tests), grouping (2 tests), and formatting (2 tests).
- Added BDD behavioural coverage with `rstest-bdd` v0.5.0:
  - `tests/features/collision.feature` (3 scenarios).
  - `tests/collision_bdd.rs` (3 scenario runners).
- Added test fixture
  `tests/fixtures/valid_shared_action_across_theorems.theorem`.
- Updated documentation:
  - `docs/theoremc-design.md` — added §6.7.5 implementation decisions.
  - `docs/users-guide.md` — added "Action name collision detection" section.
  - `docs/roadmap.md` — marked Step 2.1.3 checkbox done.

Quality-gate result:

- `make check-fmt` passed.
- `make lint` passed (zero warnings).
- `make test` passed (327 tests: 165 unit + 162 integration, 0 failures).
- `make markdownlint` passed (0 errors).

Retrospective:

- Placing collision detection in its own top-level module (`src/collision.rs`)
  preserved ADR-003 boundaries cleanly. The module depends on both `schema` and
  `mangle` without either depending on the other.
- The mangled-identifier collision check is a defensive safety net. No real
  collision could be triggered through the loader because the mangling
  algorithm is injective. The unit test exercises the detection path directly
  using crafted data.
- The canonical-name check within a single file is trivially satisfied (set
  deduplication). The infrastructure is ready for cross-file collision
  detection when multi-file loading arrives in Step 3.x.

## Context and orientation

The `theoremc` crate (`/home/user/project`) compiles `.theorem` YAML files into
proof harnesses. The current pipeline is:

1. `load_theorem_docs(yaml)` in `src/schema/loader.rs` deserializes YAML into
   `Vec<TheoremDoc>`.
2. `validate_theorem_doc(doc)` in `src/schema/validate.rs` validates each
   document (fields, expressions, action name grammar).
3. `mangle_action_name(canonical_name)` in `src/mangle.rs` transforms a
   canonical dot-separated name into a `MangledAction` struct containing slug,
   hash, identifier, and resolution path.

Steps 2.1.1 (canonical action-name grammar validation) and 2.1.2 (action name
mangling) are complete. Step 2.1.3 adds collision detection as a new concern
that wires `schema` and `mangle` together.

The `TheoremDoc` struct contains action names in two locations:

- `let_bindings: IndexMap<String, LetBinding>` — each `LetBinding` is either
  `Call(LetCall { call: ActionCall })` or `Must(LetMust { must: ActionCall })`.
- `do_steps: Vec<Step>` — each `Step` is
  `Call(StepCall { call: ActionCall })`, `Must(StepMust { must: ActionCall })`,
  or `Maybe(StepMaybe { maybe: MaybeBlock { do_steps: Vec<Step> } })`.

An `ActionCall` has field `action: String` containing the canonical
dot-separated action name (e.g., `"account.deposit"`).

Key files and their approximate line counts:

- `src/lib.rs` (11 lines) — crate root, declares `pub mod schema;` and
  `pub mod mangle;`.
- `src/schema/mod.rs` (32 lines) — schema module root with public re-exports.
- `src/schema/types.rs` (312 lines) — `TheoremDoc`, `ActionCall`, `Step`,
  `LetBinding`, `MaybeBlock`, and related types.
- `src/schema/error.rs` (58 lines) — `SchemaError` enum.
- `src/schema/loader.rs` (239 lines) — `load_theorem_docs` and
  `load_theorem_docs_with_source`.
- `src/mangle.rs` (397 lines) — action name mangling with `MangledAction`.
- `tests/common/mod.rs` (12 lines) — `load_fixture` helper.

## Plan of work

### Milestone 0: baseline verification

Run `make test` to confirm all existing tests pass before any changes.

Go/no-go check: existing suite passes.

### Milestone 1: module scaffold and error variant

Add a new variant to `SchemaError` in `src/schema/error.rs`:

```rust
/// Two or more different canonical action names produce the same
/// mangled Rust identifier.
#[error("mangled identifier collision: {message}")]
MangledIdentifierCollision {
    /// Human-readable collision report listing all colliding
    /// canonical names per mangled identifier.
    message: String,
},
```

Update the `diagnostic()` method on `SchemaError` to handle the new variant
(returns `None`, same as `InvalidIdentifier` and `InvalidActionName`).

Create `src/collision.rs` with module-level doc comment and stub public
function.

Wire `pub mod collision;` into `src/lib.rs`.

Go/no-go check: `cargo check` succeeds.

### Milestone 2: implement action-name collection

In `src/collision.rs`, implement traversal functions that walk a `TheoremDoc`
and collect all canonical action names paired with their theorem name.

- `collect_doc_actions(doc) -> Vec<(&str, &str)>` — returns
  `(canonical_name, theorem_name)` pairs.
- `collect_step_actions(steps, theorem, out)` — recursive helper for step
  lists including nested `maybe` blocks.

Go/no-go check: unit tests for collection pass.

### Milestone 3: implement collision detection

Implement the public entry point and internal helpers:

- `check_action_collisions(docs: &[TheoremDoc]) -> Result<(), SchemaError>` —
  the public function.
- `group_by_canonical(occurrences)` — groups action occurrences by canonical
  name into `BTreeMap<&str, BTreeSet<&str>>`.
- `find_mangled_collisions(canonical_names)` — mangles each unique canonical
  name and groups by mangled identifier. Filters groups with more than one
  canonical name.
- `format_collision_message(mangled_collisions)` — builds a human-readable
  error string.

Go/no-go check: `cargo check` succeeds.

### Milestone 4: wire into the loader

In `src/schema/loader.rs`, function `load_theorem_docs_with_source`, add the
collision check after the per-document validation loop, before `Ok(docs)`:

```rust
crate::collision::check_action_collisions(&docs)?;
```

Go/no-go check: existing tests still pass.

### Milestone 5: unit tests

Add `#[cfg(test)] mod tests` to `src/collision.rs` with unit tests covering:

1. Let binding traversal.
2. Do step traversal.
3. Nested maybe block traversal.
4. Distinct canonical names pass.
5. Mangled collision detection (using the internal helper directly with crafted
   data).
6. Error message formatting.

Go/no-go check: `cargo test -- collision` passes.

### Milestone 6: test fixtures and BDD tests

Create fixture `tests/fixtures/valid_shared_action_across_theorems.theorem` — a
multi-document file where two theorems reference the same canonical action
names.

Create `tests/features/collision.feature` with BDD scenarios:

1. Distinct action names across theorems are accepted.
2. Same action name reused within one theorem is accepted.
3. Mangled identifier collision is detected.

Create `tests/collision_bdd.rs` following the pattern in
`tests/schema_action_name_bdd.rs`.

Go/no-go check: `cargo test --test collision_bdd` passes.

### Milestone 7: documentation and roadmap

Update `docs/theoremc-design.md` — add §6.7.5 implementation decisions.

Update `docs/users-guide.md` — add "Action name collision detection" section.

Update `docs/roadmap.md` — mark Step 2.1.3 checkbox `[x]`.

Go/no-go check: `make markdownlint` passes.

### Milestone 8: quality gates

Run `make check-fmt`, `make lint`, `make test` with `set -o pipefail` and `tee`
for log capture.

Go/no-go check: all three gates pass with zero errors and zero warnings.

## Concrete steps

Run from repository root: `/home/user/project`.

1. Baseline verification:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-1-3-baseline-test.log
   ```

   Expected signal: existing suite passes.

2. After code and test edits, run formatting gate:

   ```shell
   set -o pipefail
   make check-fmt 2>&1 | tee /tmp/2-1-3-check-fmt.log
   ```

   Expected signal: formatter check exits 0.

3. Run lint gate:

   ```shell
   set -o pipefail
   make lint 2>&1 | tee /tmp/2-1-3-lint.log
   ```

   Expected signal: rustdoc + clippy exit 0 with no denied warnings.

4. Run full tests:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-1-3-test.log
   ```

   Expected signal: all tests pass, including new collision unit tests and BDD
   scenarios.

## Validation and acceptance

Acceptance behaviours:

- `check_action_collisions(&docs)` returns `Ok(())` for documents with no
  collisions (e.g., `valid_full.theorem`, `valid_multi.theorem`).
- `check_action_collisions(&docs)` returns
  `Err(SchemaError::MangledIdentifierCollision { .. })` when two different
  canonical names produce the same mangled identifier.
- The error message lists all colliding canonical names.
- All existing tests continue to pass (no regressions).
- BDD scenarios in `tests/features/collision.feature` pass.

Quality criteria:

- Tests: all existing and new unit/BDD tests pass.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Final verification: `make test` passes after docs updates and roadmap tick.

## Idempotence and recovery

All steps are idempotent; rerunning commands is safe. If a gate fails, inspect
`/tmp/2-1-3-*.log`, apply minimal corrective edits, and rerun only the failing
gate before rerunning the full gate sequence.

## Artefacts and notes

New artefacts:

- `src/collision.rs` — collision detection module (new top-level module).
- `tests/fixtures/valid_shared_action_across_theorems.theorem` — fixture
  with duplicate canonical action names across theorems.
- `tests/collision_bdd.rs` — BDD test runner.
- `tests/features/collision.feature` — BDD feature file.

Updated artefacts:

- `src/lib.rs` — add `pub mod collision;`.
- `src/schema/error.rs` — add `MangledIdentifierCollision` variant.
- `src/schema/loader.rs` — add collision check call after validation loop.
- `docs/theoremc-design.md` — add section 6.7.5.
- `docs/users-guide.md` — add collision detection section.
- `docs/roadmap.md` — mark Step 2.1.3 done.

## Interfaces and dependencies

### Public API (`theoremc::collision`)

In `src/collision.rs`, define:

```rust
/// Checks for mangled-identifier collisions across loaded theorem
/// documents.
///
/// Collects all canonical action names, mangles each one, and reports
/// an error when two or more different canonical names produce the
/// same mangled Rust identifier. Multiple theorems referencing the
/// same canonical name is accepted and does not trigger a collision.
///
/// # Errors
///
/// Returns [`SchemaError::MangledIdentifierCollision`] listing all
/// colliding canonical names per mangled identifier.
pub fn check_action_collisions(docs: &[TheoremDoc]) -> Result<(), SchemaError>;
```

### New `SchemaError` variant

In `src/schema/error.rs`, add:

```rust
/// Two or more different canonical action names produce the same
/// mangled Rust identifier.
#[error("mangled identifier collision: {message}")]
MangledIdentifierCollision {
    /// Human-readable collision report listing all colliding
    /// canonical names per mangled identifier.
    message: String,
},
```

### Dependencies

No new external dependencies required. The module uses:

- `crate::schema::{TheoremDoc, Step, LetBinding, SchemaError}` (public types).
- `crate::mangle::mangle_action_name` (public function).
- `std::collections::{BTreeMap, BTreeSet}` (deterministic ordering).
