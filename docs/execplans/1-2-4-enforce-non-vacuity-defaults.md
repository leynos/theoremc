# Enforce non-vacuity defaults for theorem validation

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, Step 1.2.4 in the roadmap is complete with explicit,
verifiable coverage for non-vacuity defaults: `Witness` is required by default,
and vacuity is accepted only when `Evidence.kani.allow_vacuous: true` and
`Evidence.kani.vacuity_because` is present and non-empty.

Observable success is that theorem loading behaviour is deterministic and
tested across happy and unhappy paths at two levels:

- unit tests for validation semantics,
- behavioural tests using `rstest-bdd` v0.5.0 for user-facing acceptance flows.

The roadmap item `docs/roadmap.md` Step 1.2.4 will be marked done only after
all acceptance criteria and quality gates pass.

## Constraints

- Keep behaviour aligned with signposts `TFS-6`, `ADR-4`, and `DES-8`.
- Preserve deterministic error wording shape through
  `SchemaError::ValidationFailed` messages.
- Do not change the public loader API signature
  `theoremc::schema::load_theorem_docs`.
- Keep file lengths under 400 lines; avoid growing `src/schema/validate.rs`
  beyond the limit.
- Use `rstest-bdd` v0.5.0 where behavioural tests are applicable for this step.
- Maintain existing valid fixtures as valid unless they intentionally encode an
  invalid vacuity declaration.
- Limit edits to `src/schema/`, `tests/`, and `docs/`.
- Pass commit gates: `make check-fmt`, `make lint`, and `make test`.
- Because docs change in this step, also run `make fmt`, `make markdownlint`,
  and `make nixie`.

## Tolerances (exception triggers)

- Scope: if implementation needs more than 8 files changed or more than 500 net
  lines, stop and escalate.
- Interfaces: if public API changes are required, stop and escalate.
- Dependencies: only `rstest-bdd` v0.5.0 and `rstest-bdd-macros` v0.5.0 may be
  added; any additional dependency requires escalation.
- Iterations: if the same failing gate persists after 5 fix attempts, stop and
  escalate with failure details.
- Ambiguity: if `TFS-6` and existing implementation disagree on vacuity
  semantics, stop and escalate with options.

## Risks

- Risk: the validator already enforces most Step 1.2.4 semantics, so work may
  drift into redundant refactoring. Severity: medium. Likelihood: high.
  Mitigation: begin with a gap audit against acceptance criteria; only change
  validation logic if behaviour is missing or incorrect.

- Risk: `src/schema/validate.rs` is near the 400-line cap.
  Severity: medium. Likelihood: high. Mitigation: add or expand unit tests in
  `src/schema/loader.rs`, dedicated integration tests, or both, instead of
  expanding `validate.rs` test module.

- Risk: introducing `rstest-bdd` may add macro wiring friction.
  Severity: medium. Likelihood: medium. Mitigation: follow crate examples from
  local registry source and keep the behaviour-driven development (BDD) surface
  focused on this single roadmap step.

- Risk: error-fragment assertions become brittle if message text drifts.
  Severity: low. Likelihood: medium. Mitigation: assert stable semantic
  fragments, not full-string equality.

## Progress

- [x] (2026-02-20 17:12Z) Draft this ExecPlan.
- [x] (2026-02-20 17:24Z) Milestone 0: audited current Step 1.2.4
  implementation and identified acceptance/test/documentation gaps.
- [x] (2026-02-20 17:27Z) Milestone 1: no validator logic changes required.
  Existing `validate_kani_vacuity` and `validate_kani_witnesses` semantics
  already matched `TFS-6` and `ADR-4`.
- [x] (2026-02-20 17:34Z) Milestone 2: added unit coverage for explicit
  `allow_vacuous: false` missing-witness failure in `src/schema/loader.rs`.
- [x] (2026-02-20 17:43Z) Milestone 3: added `rstest-bdd` v0.5.0 behavioural
  scenarios and fixtures for happy/unhappy vacuity flows.
- [x] (2026-02-20 17:51Z) Milestone 4: updated design and user documentation,
  indexed this ExecPlan, and marked Step 1.2.4 done in the roadmap.
- [x] (2026-02-20 17:58Z) Milestone 5: ran formatting, lint, and full test
  gates successfully.

## Surprises & discoveries

- The project memory Model Context Protocol (MCP) servers were not available in
  this environment, so no historical qdrant notes could be retrieved for this
  session.
- The current validator already contains dedicated vacuity checks in
  `validate_kani_vacuity` and `validate_kani_witnesses` in
  `src/schema/validate.rs`.
- Existing behavioural tests in `tests/schema_bdd.rs` are `rstest`-parameterized
  rather than `rstest-bdd` scenarios.
- `docs/rstest-bdd-users-guide.md` is not present in this repository; concrete
  `rstest-bdd` usage references were taken from the published crate source
  (`~/.cargo/registry/src/.../rstest-bdd-0.5.0/README.md` and tests).

## Decision log

- Decision: treat Step 1.2.4 as an acceptance-completion task first, not a
  blind rewrite. Rationale: current code already enforces core non-vacuity
  rules; missing work is primarily acceptance-proof coverage, `rstest-bdd`
  usage, and roadmap/doc traceability. Date/Author: 2026-02-20 / Codex.

- Decision: add `rstest-bdd` v0.5.0 and `rstest-bdd-macros` v0.5.0 as
  dev-dependencies for behavioural tests. Rationale: the task explicitly
  requires `rstest-bdd` where applicable. Date/Author: 2026-02-20 / Codex.

- Decision: prefer unit-test additions outside `src/schema/validate.rs` unless
  logic must change. Rationale: `validate.rs` is at 397 lines and near the hard
  file-size limit. Date/Author: 2026-02-20 / Codex.

## Outcomes & retrospective

Step 1.2.4 was completed without validator logic changes because existing
semantic checks in `src/schema/validate.rs` already satisfied the non-vacuity
policy contract.

Delivered changes:

- Added explicit-false missing-witness unit test in `src/schema/loader.rs`.
- Added `rstest-bdd` v0.5.0 + `rstest-bdd-macros` v0.5.0 dev dependencies.
- Added behavioural feature/scenario coverage in
  `tests/features/schema_vacuity.feature` and `tests/schema_vacuity_bdd.rs`.
- Added invalid fixtures for default and explicit-false missing witness plus
  missing vacuity reason.
- Updated `docs/theoremc-design.md` with Step 1.2.4 decisions.
- Updated `docs/users-guide.md` to clarify omitted-vs-false and rationale
  requirements.
- Marked roadmap Step 1.2.4 done and indexed this ExecPlan in
  `docs/contents.md`.

Quality gates and validation outcomes:

- `make fmt` passed.
- `make markdownlint` passed.
- `make nixie` passed.
- `make check-fmt` passed.
- `make lint` passed.
- `make test` passed, including new `tests/schema_vacuity_bdd.rs` scenarios.

Key lesson:

- When validator semantics already satisfy a roadmap item, acceptance closure is
  primarily a traceability task: add missing behavioural coverage, document
  rationale, and close roadmap/doc gaps without unnecessary refactoring.

## Context and orientation

This task is Roadmap Phase 1, Step 1.2.4 in `docs/roadmap.md:137`. The target
behaviour is defined by:

- `docs/theorem-file-specification.md:453` (`TFS-6` §6.2),
- `docs/adr-001-theorem-symbol-stability-and-non-vacuity-policy.md:78`
  (`ADR-4` decision 4),
- `docs/theoremc-design.md:926` (`DES-8` §8.4).

Current implementation state after completion:

- `src/schema/validate.rs` continues to enforce non-vacuity defaults via
  `validate_kani_vacuity` and `validate_kani_witnesses`.
- `src/schema/loader.rs` includes explicit-false missing-witness coverage in
  addition to existing vacuity unit tests.
- `tests/schema_vacuity_bdd.rs` and
  `tests/features/schema_vacuity.feature` provide `rstest-bdd` behavioural
  scenarios for happy and unhappy vacuity flows.
- `docs/theoremc-design.md` now includes Step 1.2.4 decisions.
- `docs/roadmap.md` marks Step 1.2.4 as done.

## Plan of work

### Milestone 0: baseline audit and acceptance matrix

Audit current vacuity behaviour against Step 1.2.4 acceptance. Confirm exact
pass/fail expectations for:

- default path (`allow_vacuous` omitted or false) with missing `Witness`,
- vacuous override path (`allow_vacuous: true`) with non-empty rationale,
- invalid vacuous declarations (missing rationale, blank rationale).

Create a concise acceptance matrix in the implementation notes so each case
maps to at least one unit test and one behavioural test.

### Milestone 1: validation logic adjustments (only if needed)

If audit reveals semantic gaps, update vacuity checks in
`src/schema/validate.rs` with minimal change scope. Keep deterministic ordering
of errors and avoid changing unrelated validation paths.

If no logic gap is found, record that decision and skip direct validator edits.

### Milestone 2: unit test coverage for vacuity policy

Add or extend unit tests to cover full acceptance matrix. Prefer
`src/schema/loader.rs` test module to avoid crossing the `validate.rs` size cap.

Add fixture-based or inline cases for:

- valid non-vacuous theorem with witness,
- valid vacuous theorem (`allow_vacuous: true` + non-empty `vacuity_because`),
- invalid default path: missing witness when `allow_vacuous` is omitted,
- invalid explicit false path: missing witness when `allow_vacuous: false`,
- invalid vacuous path: `allow_vacuous: true` without `vacuity_because`,
- invalid vacuous path: `allow_vacuous: true` with blank `vacuity_because`.

Fixture design rule for this milestone: each invalid fixture must isolate a
single vacuity-policy failure mode with minimal required fields so behavioural
intent remains stable as unrelated schema features evolve.

### Milestone 3: behavioural tests with `rstest-bdd` v0.5.0

Add behaviour-driven development (BDD) feature and scenario wiring:

- add dev dependencies in `Cargo.toml`:
  `rstest-bdd = "0.5.0"` and `rstest-bdd-macros = "0.5.0"`,
- add `tests/features/schema_vacuity.feature`,
- add `tests/schema_vacuity_bdd.rs` with `#[scenario(...)]` tests and
  `#[given]` step definitions asserting the same acceptance matrix.

Keep behavioural scenarios focused on externally visible loader behaviour and
error diagnostics, not internal helper function structure.

### Milestone 4: documentation and roadmap updates

Update documentation to reflect completed Step 1.2.4:

- `docs/theoremc-design.md`: add an “Implementation decisions (Step 1.2.4)”
  subsection under section 6 documenting policy and testing decisions.
- `docs/users-guide.md`: ensure non-vacuity default semantics and vacuity
  override contract are explicit for library consumers.
- `docs/roadmap.md`: mark Step 1.2.4 checkbox as done.
- `docs/contents.md`: add this ExecPlan entry under the execution plans list.

### Milestone 5: quality gates and final verification

Run all required format, lint, and tests with log capture. Verify no
regressions outside vacuity policy behaviour.

## Concrete steps

Run from the repository root. Keep command logs for review.

1. Baseline audit and targeted test loop.

    ```sh
    set -o pipefail
    make test | tee /tmp/step-1-2-4-baseline.log
    ```

2. Implement code/test/doc changes from milestones 1-4.

3. Format Markdown and Rust.

    ```sh
    set -o pipefail
    make fmt | tee /tmp/step-1-2-4-fmt.log
    ```

4. Validate docs and diagrams.

    ```sh
    set -o pipefail
    make markdownlint | tee /tmp/step-1-2-4-markdownlint.log

    set -o pipefail
    make nixie | tee /tmp/step-1-2-4-nixie.log
    ```

5. Run required commit gates.

    ```sh
    set -o pipefail
    make check-fmt | tee /tmp/step-1-2-4-check-fmt.log

    set -o pipefail
    make lint | tee /tmp/step-1-2-4-lint.log

    set -o pipefail
    make test | tee /tmp/step-1-2-4-test.log
    ```

6. Confirm roadmap checkbox updated and all logs show success.

## Validation and acceptance

Acceptance is met only if all conditions below are true:

- Unit tests prove:
  default non-vacuous mode rejects missing `Witness`, valid vacuous override is
  accepted, invalid vacuous declarations are rejected.
- Behavioural tests implemented with `rstest-bdd` v0.5.0 cover happy and unhappy
  vacuity flows with deterministic error fragments.
- `docs/theoremc-design.md` records implementation decisions for Step 1.2.4.
- `docs/users-guide.md` reflects any user-visible behaviour/API clarifications.
- `docs/roadmap.md` Step 1.2.4 is marked done.
- `make check-fmt`, `make lint`, and `make test` all pass.

## Theorem authoring guidance

When authoring theorem files that rely on non-vacuity defaults, ensure the
document communicates intent explicitly and aligns with the schema contract:

- Cite concrete rationale in `because` fields for `Prove`, `Assume`, and
  `Witness` entries so checks remain reviewable.
- For each `Prove`, `Assume`, and `Witness` entry, cite related clauses and
  external documents so provenance is reviewable; where your authoring workflow
  supports it, include a companion `references` field with clause IDs, section
  anchors, or source links.
- Keep witness intent explicit: if a theorem is expected to be non-vacuous,
  provide at least one `Witness` entry; if vacuity is intentional, set
  `allow_vacuous: true` and provide a non-empty `vacuity_because`.
- Use `Evidence.kani.expect` values to match intended verification outcomes
  (`SUCCESS`, `FAILURE`, `UNREACHABLE`, `UNDETERMINED`) and avoid ambiguous
  expectations in fixtures or examples.

## Contributor checklist

Before marking this roadmap step complete in future updates, contributors
should verify all artefacts and snapshots affected by schema/validation edits:

- Parser fixtures:
  add or update focused fixtures under `tests/fixtures/` for each new happy and
  unhappy path.
- Codegen snapshots:
  if a change affects generated output in downstream tooling, refresh and
  review any relevant codegen snapshots.
- Report snapshots:
  if diagnostics or reporting output changes, refresh and review report
  snapshots to ensure intended user-facing deltas.
- Behavioural and unit coverage:
  ensure unit tests and behavioural scenarios cover acceptance paths and error
  fragments.
- Gates:
  run `make check-fmt`, `make lint`, and `make test` with log capture.

## Compatibility policy

Validation and diagnostics for this area should follow a stability-first policy:

- Diagnostic code compatibility:
  if diagnostic codes are introduced for schema validation, treat published
  codes as stable and document any additions or deprecations.
- Diagnostic argument-schema compatibility:
  preserve argument names and semantic meaning for existing diagnostics;
  changes require explicit migration notes.
- Error message evolution:
  prefer additive clarifications over semantic rewrites, and keep deterministic
  fragments used by behavioural tests stable unless acceptance criteria change.

## Idempotence and recovery

- All added fixtures and tests are additive and safe to rerun.
- If a gate fails, fix forward and rerun the failed gate, then rerun full gates
  before completion.
- If `rstest-bdd` scenario wiring fails unexpectedly, keep existing tests
  untouched, isolate failure to the new BDD file, and retry with minimal
  scenario patterns proven in upstream crate tests.

## Artefacts and notes

Expected artefacts from implementation:

- New/updated vacuity fixtures under `tests/fixtures/`.
- New BDD feature file under `tests/features/`.
- New behavioural test module using `rstest-bdd` macros.
- Updated roadmap/design/user documentation and this ExecPlan status.
- Gate logs in `/tmp/step-1-2-4-*.log`.

## Interfaces and dependencies

Validation interfaces involved:

- `src/schema/validate.rs`:
  `validate_evidence`, `validate_kani_vacuity`, `validate_kani_witnesses`.
- Public API: `theoremc::schema::load_theorem_docs` (must remain stable).

Dependency additions for behavioural testing:

- `rstest-bdd = "0.5.0"` (dev-dependency)
- `rstest-bdd-macros = "0.5.0"` (dev-dependency)

BDD entry-point interfaces to add/use:

- `#[scenario(path = "...")]` from `rstest-bdd-macros`,
- `#[given]` step definitions from `rstest-bdd-macros`.
