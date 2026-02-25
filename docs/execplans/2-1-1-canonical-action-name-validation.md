# Step 2.1.1: canonical action-name validation

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Implement Roadmap Phase 2, Step 2.1, first acceptance item: canonical
`ActionCall.action` name validation for dot-separated segments with per-segment
identifier and Rust-keyword rejection rules.

After this change, theorem documents that reference malformed action names (for
example missing dots, empty segments, invalid identifier characters, or keyword
segments like `fn`) fail deterministically during schema validation. Valid
canonical names continue to load successfully.

This fulfils signposts `NMR-1`, `TFS-4`, and `ADR-1` for the validation slice
only. Mangling, collision detection, and resolution to `crate::theorem_actions`
remain separate follow-up steps in roadmap Step 2.1.

Observable success:

- `load_theorem_docs` rejects malformed canonical action names in both `Let`
  bindings and `Do` steps, including nested `maybe.do` steps.
- Validation failures are deterministic and actionable.
- Unit tests and behavioural tests (`rstest-bdd` v0.5.0) cover happy,
  unhappy, and edge cases.
- `make check-fmt`, `make lint`, and `make test` pass.
- `docs/theoremc-design.md`, `docs/users-guide.md`, and the relevant roadmap
  checkbox are updated.

## Constraints

- Scope is limited to roadmap item 2.1.1 (canonical action-name validation).
- Do not implement action mangling, hash generation, collision detection, or
  binding resolution in this change.
- Preserve existing `TheoremDoc` schema and loader public API:
  `theoremc::schema::load_theorem_docs` and
  `theoremc::schema::load_theorem_docs_with_source`.
- Keep deterministic validation ordering and error messaging.
- Keep source files under 400 lines; extract helpers into dedicated modules
  where needed.
- Use `rstest-bdd` v0.5.0 for behavioural coverage where scenario-style tests
  add value.
- Keep dependency surface unchanged for this sub-step (no new crates expected).
- Documentation updates are required in:
  `docs/theoremc-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.
- Required quality gates: `make check-fmt`, `make lint`, `make test`.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 12 files or more than
  500 net lines, stop and escalate with a narrowed split.
- Interface: if a public API signature change is required, stop and escalate.
- Dependencies: if a new dependency is needed for this sub-step, stop and
  escalate.
- Diagnostics: if action-name failures cannot remain deterministic, stop and
  escalate with options.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop and escalate with logs.

## Risks

- Risk: current action validation lives in `src/schema/step.rs`; adding more
  lexical logic there may create a bumpy-road function and file growth.
  Severity: medium. Likelihood: medium. Mitigation: extract canonical
  action-name checks into a dedicated helper module and keep `step.rs` as
  orchestration.

- Risk: keyword and identifier logic could diverge from existing theorem-name
  validation if duplicated. Severity: medium. Likelihood: medium. Mitigation:
  reuse shared lexical predicates from `identifier` validation, or extract
  common helpers.

- Risk: newly rejected edge cases (for example single-segment action names)
  may break undocumented assumptions. Severity: low. Likelihood: medium.
  Mitigation: codify expected behaviour in docs and fixtures, then validate all
  existing valid fixtures still pass.

## Progress

- [x] (2026-02-24 19:29Z) Draft ExecPlan for Step 2.1.1.
- [x] (2026-02-24 20:24Z) Milestone 0: baseline audit and test gap mapping.
- [x] (2026-02-24 20:24Z) Milestone 1: implement canonical action-name
  validator module.
- [x] (2026-02-24 20:24Z) Milestone 2: integrate validation into `Let` and
  `Do` action-call paths.
- [x] (2026-02-24 20:24Z) Milestone 3: add unit + behavioural
  (`rstest-bdd` v0.5.0) coverage.
- [x] (2026-02-24 20:24Z) Milestone 4: update user/design docs and mark
  roadmap checkbox done.
- [x] (2026-02-24 20:24Z) Milestone 5: run full quality gates and capture
  logs.

## Surprises & discoveries

- Observation: `docs/rstest-bdd-users-guide.md` is not present in this
  repository. Evidence: `sed` read fails with "No such file or directory".
  Impact: rely on existing `rstest-bdd` usage in `tests/schema_vacuity_bdd.rs`
  and `tests/schema_diagnostics_bdd.rs` as local implementation reference.

- Observation: project-memory Model Context Protocol (MCP)/qdrant resources
  are not available in this environment. Evidence: `list_mcp_resources` and
  `list_mcp_resource_templates` returned empty lists. Impact: planning
  proceeded from repository docs and current code only.

- Observation: `qdrant-find` calls returned
  `"tools/call failed: Unexpected response type"` even when queried with exact
  error strings. Evidence: all recall calls (startup and follow-up on lint
  failure) returned the same tool error. Impact: no project-memory recall was
  available during implementation.

- Observation: Clippy denied `uninlined_format_args` in
  `src/schema/action_name.rs`. Evidence: `make lint` failed on
  `"variables can be used directly in the format! string"` for a named-arg
  `format!` call. Impact: simplified the `format!` call to inline args; lint
  then passed.

## Decision log

- Decision: scope this change strictly to canonical action-name validation
  (`NMR-1` grammar + keyword rules) and defer mangling/collision work to
  roadmap items 2.1.2 and 2.1.3. Rationale: keeps the change atomic,
  reviewable, and aligned with roadmap sequencing. Date/Author: 2026-02-24 /
  Codex.

- Decision: add dedicated action-name validation helpers rather than embedding
  all grammar checks into `validate_action_call`. Rationale: preserves
  readability, avoids high cognitive complexity in `step.rs`, and creates
  reusable logic for upcoming Step 2.1.2 work. Date/Author: 2026-02-24 / Codex.

- Decision: enforce the canonical grammar's minimum of two segments (at least
  one dot) now, not as a warning. Rationale: `NMR-1` and `TFS-4` treat
  canonical dotted names as the accepted grammar shape for deterministic
  resolution. Date/Author: 2026-02-24 / Codex.

## Outcomes & retrospective

Step 2.1.1 is complete.

Implemented outcomes:

- Added canonical action-name validator module:
  `src/schema/action_name.rs`.
- Reused shared identifier lexical predicates via new crate-visible helpers in
  `src/schema/identifier.rs`.
- Integrated canonical validation into `ActionCall` validation path in
  `src/schema/step.rs`, which applies to `Let`, `Do`, and nested `maybe.do`.
- Added new fixtures for malformed and keyword-segment action names:
  - `tests/fixtures/invalid_action_missing_dot.theorem`
  - `tests/fixtures/invalid_action_empty_segment.theorem`
  - `tests/fixtures/invalid_action_keyword_segment.theorem`
  - `tests/fixtures/invalid_let_action_keyword_segment.theorem`
- Added behavioural coverage with `rstest-bdd` v0.5.0:
  - `tests/features/schema_action_name.feature`
  - `tests/schema_action_name_bdd.rs`
- Extended `tests/schema_bdd.rs` invalid-step/let cases to include canonical
  grammar and keyword-segment failures.
- Updated user/design/roadmap docs:
  - `docs/users-guide.md`
  - `docs/theoremc-design.md`
  - `docs/roadmap.md` (Step 2.1.1 checkbox marked done)

Quality-gate result:

- Documentation gates: `make fmt`, `make markdownlint`, `make nixie` passed.
- Rust gates: `make check-fmt`, `make lint`, and `make test` passed.
- Test suite now includes 118 unit tests in `src/lib.rs`, plus the new
  `schema_action_name_bdd` behavioural scenarios.

Retrospective:

- Isolating canonical grammar in its own module avoided bloating `step.rs` and
  keeps Step 2.1.2 (mangling + resolution) implementation-ready.
- Reusing shared identifier predicates kept lexical rules consistent across
  theorem identifiers and action segments, reducing drift risk.

## Context and orientation

Current validation pipeline:

- `src/schema/loader.rs` loads YAML docs and applies semantic validation.
- `src/schema/validate.rs` orchestrates post-deserialization checks.
- `src/schema/step.rs` currently enforces only non-empty `ActionCall.action`
  and `maybe` shape rules.

Relevant schema/types:

- `src/schema/types.rs` defines `ActionCall`, `LetBinding`, `Step`, and
  `MaybeBlock`.
- `ActionCall.action` is documented as dot-separated but currently not
  grammar-validated beyond non-empty checks.

Relevant tests/docs:

- `tests/schema_bdd.rs` covers schema happy/unhappy paths via `rstest`.
- `tests/schema_vacuity_bdd.rs` and `tests/schema_diagnostics_bdd.rs` show the
  established `rstest-bdd` v0.5.0 pattern.
- `docs/name-mangling-rules.md` defines canonical action-name grammar and
  keyword rejection.
- `docs/theorem-file-specification.md` section 7.1 references action-name
  grammar.
- `docs/theoremc-design.md` section 5.2 defines action naming intent.

## Plan of work

### Milestone 0: baseline audit and test gap mapping

Confirm all current valid fixtures use action names that satisfy
`Segment ("." Segment)+` with per-segment identifier compliance. Record the
current failure shape for blank action names to preserve deterministic ordering
while extending validation.

Go/no-go check:

- Existing valid fixtures still pass before any code edits.
- Exact target failure classes for new tests are enumerated.

### Milestone 1: implement canonical action-name validator module

Create `src/schema/action_name.rs` with a focused validation API used by schema
validation.

Planned interface:

- `pub(crate) fn validate_canonical_action_name(name: &str) -> Result<(), SchemaError>`

Validation rules to enforce:

- Name must match grammar `Segment ("." Segment)+` (at least one dot).
- No empty segments.
- Every segment must match ASCII identifier pattern
  `^[A-Za-z_][A-Za-z0-9_]*$`.
- No segment may be a Rust reserved keyword.

Implementation notes:

- Reuse existing identifier/keyword logic from `src/schema/identifier.rs`
  through shared helpers to avoid drift.
- Return deterministic, context-ready reason strings that `step.rs` can prefix
  with path context.
- Add module-level docs and unit tests in this file.

Go/no-go check:

- Unit tests cover happy, unhappy, and edge cases for grammar + keywords.
- Module remains under 400 lines.

### Milestone 2: integrate into step/let validation paths

Wire canonical validation into existing action-call checks so every
`ActionCall` path uses the same rule set.

Planned edits:

- `src/schema/mod.rs`: declare new internal module.
- `src/schema/step.rs`: update `validate_action_call` to call
  `validate_canonical_action_name` after non-empty check.
- `src/schema/validate.rs`: no behaviour change expected beyond new action-name
  failure reasons through existing `validate_let_bindings` and
  `validate_do_steps` calls.

Go/no-go check:

- Existing `maybe`/shape validation behaviour remains unchanged.
- Malformed action names fail in both `Let` and `Do` contexts.

### Milestone 3: tests (unit + behavioural)

Add coverage for happy/unhappy/edge cases.

Unit tests:

- In `src/schema/action_name.rs`:
  - valid examples: `account.deposit`, `hnsw.graph_with_capacity`, `_a._b1`.
  - invalid grammar: missing dot, leading/trailing dot, double dot,
    non-identifier segments, whitespace contamination.
  - invalid keyword segments: cases such as `account.fn`, `self.deposit`,
    `graph.type`.

Behavioural tests with `rstest-bdd` v0.5.0:

- Add `tests/features/schema_action_name.feature` with scenarios for:
  - valid canonical action names load successfully,
  - malformed action names are rejected,
  - keyword segments are rejected.
- Add `tests/schema_action_name_bdd.rs` using `rstest_bdd_macros` and fixture
  helpers in `tests/common/mod.rs`.
- Add minimal fixtures under `tests/fixtures/` for happy and unhappy paths.

Go/no-go check:

- New scenarios fail before implementation and pass after.
- Existing BDD suites continue to pass.

### Milestone 4: documentation and roadmap update

Update consumer and design documentation for the new behaviour.

Planned docs edits:

- `docs/users-guide.md`: expand `ActionCall.action` rules with canonical grammar
  and per-segment keyword/identifier constraints.
- `docs/theoremc-design.md`: add implementation decision notes for Step 2.1.1,
  including grammar enforcement and rationale.
- `docs/roadmap.md`: mark the Step 2.1.1 checkbox (canonical action-name
  validation) as done when implementation and tests are complete.

Go/no-go check:

- Documentation reflects actual implemented behaviour and examples.
- Roadmap update occurs only after all quality gates pass.

### Milestone 5: full quality gates

Run required quality gates with log capture via `tee` and `set -o pipefail`.

Go/no-go check:

- `make check-fmt` passes.
- `make lint` passes.
- `make test` passes.

## Concrete steps

Run from repository root: `/home/user/project`.

1. Baseline and focused verification:

       set -o pipefail
       make test 2>&1 | tee /tmp/2-1-1-baseline-test.log

   Expected signal: existing suite passes before changes.

2. After code + test edits, run formatting gate:

       set -o pipefail
       make check-fmt 2>&1 | tee /tmp/2-1-1-check-fmt.log

   Expected signal: formatter check exits 0.

3. Run lint gate:

       set -o pipefail
       make lint 2>&1 | tee /tmp/2-1-1-lint.log

   Expected signal: rustdoc + clippy exit 0 with no denied warnings.

4. Run full tests:

       set -o pipefail
       make test 2>&1 | tee /tmp/2-1-1-test.log

   Expected signal: all tests pass, including new action-name unit and
   behavioural tests.

5. Review logs for deterministic proof of success:

       rg -n "error:|FAILED|failures:" /tmp/2-1-1-*.log

   Expected signal: no failure markers found.

## Validation and acceptance

Acceptance behaviours:

- A theorem fixture with `action: account.deposit` in `Let` or `Do` loads.
- A theorem fixture with malformed action name (for example `action: account`,
  `action: account..deposit`, `action: account.deposit-now`) fails validation.
- A theorem fixture with keyword segment (for example `action: account.fn`)
  fails validation.
- Existing valid fixtures remain valid.

Quality criteria:

- Tests: all existing and new unit/integration/BDD tests pass.
- Lint/typecheck/docs: `make lint` passes with denied warnings.
- Format: `make check-fmt` passes.
- Final verification: `make test` passes after docs updates and roadmap tick.

## Idempotence and recovery

- All steps are idempotent; rerunning commands is safe.
- If a gate fails, inspect `/tmp/2-1-1-*.log`, apply minimal corrective edits,
  and rerun only the failing gate before rerunning the full gate sequence.
- If scope exceeds tolerances, stop and record escalation options in
  `Decision Log` before continuing.

## Artefacts and notes

Expected new/updated artefacts during implementation:

- `src/schema/action_name.rs` (new).
- `src/schema/step.rs` (updated integration).
- `tests/features/schema_action_name.feature` (new).
- `tests/schema_action_name_bdd.rs` (new).
- `tests/fixtures/invalid_action_*.theorem` and `valid_action_*.theorem`
  (new fixtures).
- `docs/theoremc-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
  (updated docs).

## Interfaces and dependencies

Planned internal interfaces:

- `crate::schema::action_name::validate_canonical_action_name(&str)` returns a
  deterministic `Result<(), SchemaError>` used by
  `crate::schema::step::validate_action_call`.

Dependency stance for this sub-step:

- No new dependencies.
- Reuse existing `identifier`/keyword logic to keep lexical rules consistent
  across theorem-name and action-segment validation.

Revision note (2026-02-24): Updated status from `DRAFT` to `COMPLETE`, marked
all milestones done with timestamps, recorded implementation discoveries
(qdrant tool failures and clippy formatting constraint), and replaced
placeholder outcomes with concrete delivered artefacts, documentation updates,
and passing gate results.
