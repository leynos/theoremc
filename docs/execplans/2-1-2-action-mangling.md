# Step 2.1.2: action name mangling and canonical path resolution

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Implement Roadmap Phase 2, Step 2.1, second acceptance item: deterministic,
injective action name mangling (`segment_escape`, `action_slug`,
`hash12(blake3)`) and canonical path resolution into
`crate::theorem_actions`.

After this change, library consumers can transform a validated canonical
action name (e.g., `account.deposit`) into a stable mangled Rust identifier
(`account__deposit__h05158894bfb4`) that resolves to a fully qualified path
in `crate::theorem_actions`. The mangling is injective: different canonical
names always produce different identifiers.

This fulfils signposts `NMR-1`, `ADR-1`, and `DES-5` for the mangling slice.
Collision detection remains a separate follow-up step (roadmap Step 2.1.3).

Observable success:

- `theoremc::mangle::mangle_action_name("account.deposit")` returns a
  `MangledAction` with slug `"account__deposit"`, hash `"05158894bfb4"`,
  identifier `"account__deposit__h05158894bfb4"`, and path
  `"crate::theorem_actions::account__deposit__h05158894bfb4"`.
- Golden unit tests verify exact blake3 hash values for representative names.
- Underscore edge cases prove the injectivity property: `a.b_c` and `a_b.c`
  produce distinct mangled identifiers.
- Behavioural tests (`rstest-bdd` v0.5.0) cover slug correctness, injectivity,
  and resolution path structure.
- `make check-fmt`, `make lint`, and `make test` pass.
- `docs/theoremc-design.md`, `docs/users-guide.md`, and the relevant roadmap
  checkbox are updated.

## Constraints

- Scope is limited to roadmap item 2.1.2 (action name mangling and canonical
  path resolution).
- Do not implement collision detection in this change (that is Step 2.1.3).
- Do not modify the existing `schema` module or its public API. The `mangle`
  module is a separate top-level module with no cross-dependency on `schema`.
- Mangling functions assume pre-validated input from
  `validate_canonical_action_name` (Step 2.1.1). They do not re-validate.
- Keep source files under 400 lines.
- Use `rstest-bdd` v0.5.0 for behavioural coverage.
- The `blake3` crate must be added as a regular dependency (not dev-only)
  because mangling will be used at build time in future code generation steps.
- Documentation updates are required in: `docs/theoremc-design.md`,
  `docs/users-guide.md`, and `docs/roadmap.md`.
- Required quality gates: `make check-fmt`, `make lint`, `make test`.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 10 files or more than
  500 net lines, stop and escalate with a narrowed split.
- Interface: if a public API signature change to the `schema` module is
  required, stop and escalate.
- Dependencies: if a dependency other than `blake3` is needed, stop and
  escalate.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop and escalate with logs.

## Risks

- Risk: blake3 dependency adds compile time. Severity: low. Likelihood: low.
  Mitigation: blake3 is well-optimised; measure before/after if significant.

- Risk: Clippy lint `string_slice = "deny"` triggered by hash hex slicing.
  Severity: medium. Likelihood: medium. Mitigation: use
  `.get(..12).unwrap_or_default()` instead of `&hex[..12]`.

- Risk: Clippy lint `too_many_arguments` triggered by rstest parameterised
  golden tests. Severity: medium. Likelihood: high. Mitigation: use a helper
  struct (`Golden`) instead of 5+ function parameters.

- Risk: module placement questioned (should it be `resolver/`?). Severity: low.
  Likelihood: low. Mitigation: document the decision; `mangle.rs` is easily
  moved later when the workspace split happens.

## Progress

- [x] (2026-02-25 00:00Z) Draft ExecPlan for Step 2.1.2.
- [x] (2026-02-25 00:05Z) Milestone 0: baseline verification (all existing
  tests pass).
- [x] (2026-02-25 00:10Z) Milestone 1: add `blake3 = "1.8.3"` to Cargo.toml
  and verify compilation.
- [x] (2026-02-25 00:15Z) Milestone 2: compute golden blake3 hash values for
  representative action names.
- [x] (2026-02-25 00:20Z) Milestone 3: implement `src/mangle.rs` with core
  functions, `MangledAction` type, and comprehensive unit tests.
- [x] (2026-02-25 00:22Z) Milestone 4: wire module into `src/lib.rs`.
- [x] (2026-02-25 00:25Z) Milestone 5: fix Clippy `too_many_arguments` lint by
  refactoring golden tests to use a `Golden` struct.
- [x] (2026-02-25 00:27Z) Milestone 6: fix rustdoc link warnings by removing
  intra-doc links to private modules.
- [x] (2026-02-25 00:30Z) Milestone 7: add BDD feature file and test runner.
- [x] (2026-02-25 00:35Z) Milestone 8: update design docs, users guide, and
  roadmap.
- [x] (2026-02-25 00:40Z) Milestone 9: run full quality gates and capture
  logs.

## Surprises & discoveries

- Observation: Clippy lint `too_many_arguments` is triggered by rstest
  parameterised test functions with 5+ `#[case]` parameters, because rstest
  expands each case set into a function with that many arguments. Evidence:
  `make lint` failed with `too-many-arguments` on the `golden_mangle` test
  function. Impact: refactored golden tests to use a `Golden` helper struct
  whose `assert()` method validates all fields, keeping each test function
  at zero parameters.

- Observation: rustdoc intra-doc links like
  `[`segment_escape`]` in module-level `//!` comments do not
  resolve when `cargo doc` runs. Evidence: `RUSTDOCFLAGS="-D warnings"`
  produced unresolved link warnings. Impact: replaced `[`fn_name`]` with
  plain `` `fn_name` `` in module-level documentation.

- Observation: rustdoc link
  `` [`validate_canonical_action_name`](crate::schema::action_name) ``
  fails because `action_name` is a private module. Evidence:
  `cargo doc --no-deps` warned about missing item.
  Impact: replaced with plain prose reference to avoid coupling public docs to
  private module paths.

## Decision log

- Decision: place mangling in a new top-level `src/mangle.rs` module, not
  inside `src/schema/`. Rationale: mangling is an action-resolution concern,
  not a schema concern (ADR-003 boundary rules). The `schema` module validates
  action name grammar; the `mangle` module transforms validated names into
  Rust identifiers. Keeping them separate preserves architectural boundaries.
  Date/Author: 2026-02-25 / DevBoxer.

- Decision: all mangling functions are infallible (return concrete types, not
  `Result`). Rationale: input is pre-validated by Step 2.1.1; `segment_escape`
  is a pure string transformation; `blake3::hash` cannot fail; assembly is
  string formatting. Fallible APIs would add unnecessary error handling for
  conditions that cannot occur. Date/Author: 2026-02-25 / DevBoxer.

- Decision: use `blake3 = "1.8.3"` as a regular dependency (not dev-only).
  Rationale: mangling will be used at build time in code generation (Step 3.2).
  The hash computation must be available in non-test builds. Date/Author:
  2026-02-25 / DevBoxer.

- Decision: expose `segment_escape`, `action_slug`, and `hash12` as public
  functions. Rationale: Step 2.2 (harness naming) reuses `hash12` and
  downstream consumers may need the building blocks. A public API is cheaper
  to narrow later than to broaden. Date/Author: 2026-02-25 / DevBoxer.

- Decision: use a `Golden` helper struct for golden tests instead of rstest
  parameterised cases with 5 parameters. Rationale: Clippy denies
  `too_many_arguments` and rstest expands each case set into a function whose
  argument count equals the number of `#[case]` parameters. A struct keeps the
  test data cohesive and the function signature at zero parameters. Date/Author:
  2026-02-25 / DevBoxer.

## Outcomes & retrospective

Step 2.1.2 is complete.

Implemented outcomes:

- Added `blake3 = "1.8.3"` to `Cargo.toml` `[dependencies]`.
- Created `src/mangle.rs` (new top-level module) with:
  - `MangledAction` struct with accessor methods (`slug`, `hash`,
    `identifier`, `path`).
  - `segment_escape(segment)` — replaces `_` with `_u`.
  - `action_slug(canonical_name)` — splits, escapes, joins with `__`.
  - `hash12(value)` — blake3 first 12 lowercase hex characters.
  - `mangle_action_name(canonical_name)` — composite returning
    `MangledAction`.
- Wired module into `src/lib.rs` with `pub mod mangle;`.
- Added 33 unit tests in `src/mangle.rs` covering:
  - `segment_escape`: 10 cases (no underscores, single, multiple, leading,
    consecutive, lone, single char, alphanumeric, already-escaped-looking,
    multiple mid).
  - `action_slug`: 6 cases (two segments, underscore, three segments,
    leading underscores, lone underscore segment, minimal).
  - `hash12`: 6 golden values, length, lowercase hex, determinism.
  - `mangle_action_name`: 4 golden tests (account.deposit, hnsw.attach_node,
    three segments, leading underscores).
  - Injectivity: 2 tests (distinct slugs, distinct identifiers for `a.b_c`
    vs `a_b.c`).
  - Resolution: 2 tests (path prefix, path suffix).
- Added BDD behavioural coverage with `rstest-bdd` v0.5.0:
  - `tests/features/action_mangle.feature` (3 scenarios).
  - `tests/action_mangle_bdd.rs` (3 scenario runners).
- Updated documentation:
  - `docs/theoremc-design.md` — added §6.7.4 implementation decisions.
  - `docs/users-guide.md` — added "Action name mangling" section.
  - `docs/roadmap.md` — marked Step 2.1.2 checkbox done.

Quality-gate result:

- `make check-fmt` passed.
- `make lint` passed (zero warnings).
- `make test` passed (all existing + new tests).

Retrospective:

- Placing mangling in its own top-level module kept the schema layer clean and
  provides a natural home for future resolution logic (Step 2.1.3 collision
  detection, Step 2.2 harness naming).
- The `Golden` struct pattern works well for multi-field golden tests under
  strict Clippy lints. Worth reusing in Step 2.2.
- Computing golden hash values before writing tests (via a temporary
  integration test) was efficient and avoided guess-and-check iteration.

## Context and orientation

The theoremc crate has two top-level modules after this change:

- `src/schema/` — schema types, deserialization, validation, and diagnostics
  for `.theorem` documents.
- `src/mangle.rs` — action name mangling for deterministic, injective
  resolution.

The connection between them is the canonical action name string. The `schema`
module validates `ActionCall.action` strings using
`validate_canonical_action_name` (Step 2.1.1). The `mangle` module transforms
validated strings into `MangledAction` values. The two modules have no
cross-dependency; callers (future code generation in Steps 3.x) wire them
together.

Key files:

- `src/mangle.rs` — core mangling implementation.
- `src/lib.rs` — crate root, declares `pub mod mangle;` and `pub mod schema;`.
- `Cargo.toml` — dependency manifest including `blake3 = "1.8.3"`.
- `src/schema/action_name.rs` — canonical action-name validation (Step 2.1.1).
- `src/schema/identifier.rs` — shared identifier lexical predicates.
- `tests/action_mangle_bdd.rs` — BDD test runner.
- `tests/features/action_mangle.feature` — BDD feature file.

## Plan of work

### Milestone 0: baseline verification

Run `make test` to confirm all existing tests pass before any changes.

Go/no-go check: existing suite passes.

### Milestone 1: add blake3 dependency

Add `blake3 = "1.8.3"` to `Cargo.toml` `[dependencies]`. Run `cargo build` to
verify the dependency integrates.

Go/no-go check: `cargo build` succeeds.

### Milestone 2: compute golden hash values

Use a temporary integration test to compute blake3 hashes for representative
canonical action names. Record the 12-character hex prefixes for use in golden
tests.

Go/no-go check: golden values recorded for at least 6 representative names.

### Milestone 3: implement core module

Create `src/mangle.rs` with:

- `MangledAction` struct with private fields and public accessor methods.
- `segment_escape(segment: &str) -> String` — replace `_` with `_u`.
- `action_slug(canonical_name: &str) -> String` — split, escape, join.
- `hash12(value: &str) -> String` — blake3 first 12 hex chars.
- `mangle_action_name(canonical_name: &str) -> MangledAction` — composite.
- Comprehensive `#[cfg(test)]` section with rstest parameterised cases and
  golden tests.

Wire into `src/lib.rs` with `pub mod mangle;`.

Go/no-go check: `cargo test --lib -- mangle` passes all unit tests.

### Milestone 4: add BDD tests

Create `tests/features/action_mangle.feature` with scenarios:

1. Simple action names produce correct mangled identifiers.
2. Underscore escaping preserves injectivity.
3. Mangled identifiers resolve to `crate::theorem_actions`.

Create `tests/action_mangle_bdd.rs` following the existing BDD patterns.

Go/no-go check: `cargo test --test action_mangle_bdd` passes all scenarios.

### Milestone 5: documentation and roadmap

Update:

- `docs/theoremc-design.md` — add §6.7.4 implementation decisions.
- `docs/users-guide.md` — add "Action name mangling" section with API
  documentation and examples.
- `docs/roadmap.md` — mark Step 2.1.2 checkbox done.

Go/no-go check: documentation reflects actual implemented behaviour.

### Milestone 6: quality gates

Run `make check-fmt`, `make lint`, `make test` with `set -o pipefail` and
`tee` for log capture.

Go/no-go check: all three gates pass with zero errors and zero warnings.

## Concrete steps

Run from repository root: `/home/user/project`.

1. Baseline verification:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-1-2-baseline-test.log
   ```

   Expected signal: existing suite passes.

2. After code and test edits, run formatting gate:

   ```shell
   set -o pipefail
   make check-fmt 2>&1 | tee /tmp/2-1-2-check-fmt.log
   ```

   Expected signal: formatter check exits 0.

3. Run lint gate:

   ```shell
   set -o pipefail
   make lint 2>&1 | tee /tmp/2-1-2-lint.log
   ```

   Expected signal: rustdoc + clippy exit 0 with no denied warnings.

4. Run full tests:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-1-2-test.log
   ```

   Expected signal: all tests pass, including 33 new mangle unit tests and
   3 new BDD scenarios.

5. Review logs:

   ```shell
   grep -E "error:|FAILED|failures:" /tmp/2-1-2-*.log
   ```

   Expected signal: no failure markers found.

## Validation and acceptance

Acceptance behaviours:

- `mangle_action_name("account.deposit")` returns a `MangledAction` with
  identifier `"account__deposit__h05158894bfb4"` and path
  `"crate::theorem_actions::account__deposit__h05158894bfb4"`.
- `mangle_action_name("hnsw.attach_node")` returns identifier
  `"hnsw__attach_unode__h8d74e77b55f2"`.
- `mangle_action_name("a.b_c")` and `mangle_action_name("a_b.c")` produce
  different identifiers (injectivity).
- All existing schema tests continue to pass (no regressions).

Quality criteria:

- Tests: all existing and new unit/BDD tests pass.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Final verification: `make test` passes after docs updates and roadmap tick.

## Idempotence and recovery

- All steps are idempotent; rerunning commands is safe.
- If a gate fails, inspect `/tmp/2-1-2-*.log`, apply minimal corrective edits,
  and rerun only the failing gate before rerunning the full gate sequence.

## Artefacts and notes

New artefacts:

- `src/mangle.rs` — core mangling module.
- `tests/action_mangle_bdd.rs` — BDD test runner.
- `tests/features/action_mangle.feature` — BDD feature file.

Updated artefacts:

- `Cargo.toml` — added `blake3 = "1.8.3"`.
- `src/lib.rs` — added `pub mod mangle;`.
- `docs/theoremc-design.md` — added §6.7.4.
- `docs/users-guide.md` — added "Action name mangling" section.
- `docs/roadmap.md` — marked Step 2.1.2 done.

## Interfaces and dependencies

### Public API (`theoremc::mangle`)

In `src/mangle.rs`, define:

```rust
/// The result of mangling a canonical action name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledAction { /* private fields */ }

impl MangledAction {
    pub fn slug(&self) -> &str;
    pub fn hash(&self) -> &str;
    pub fn identifier(&self) -> &str;
    pub fn path(&self) -> &str;
}

pub fn segment_escape(segment: &str) -> String;
pub fn action_slug(canonical_name: &str) -> String;
pub fn hash12(value: &str) -> String;
pub fn mangle_action_name(canonical_name: &str) -> MangledAction;
```

### Dependencies

- `blake3 = "1.8.3"` — blake3 cryptographic hash (CC0/Apache-2.0 licence).
- No new dev-dependencies required.
- No changes to existing dependencies.
