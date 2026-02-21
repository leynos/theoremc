# Implement structured diagnostics for parser and validation failures

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Complete roadmap Phase 1, Step 1.3 by making theorem parsing and validation
failures deterministic, source-located, and regression-tested.

After this change, a library consumer should be able to load theorem documents
and receive structured diagnostics that include:

- source file identifier,
- line,
- column,
- stable diagnostic code/category,
- deterministic human-readable fallback message.

The implementation must satisfy roadmap signposts `DES-6` and `TFS-1`, and must
not implement backend-specific rendering logic.

Observable success:

- representative parser and validator failures produce stable diagnostics with
  source file, line, and column,
- snapshot tests lock diagnostic output shape and content,
- fixture corpus covers valid and invalid theorem documents for aliases,
  nested `maybe`, `must` semantics preconditions, and witness policy,
- unit and behavioural coverage (using `rstest-bdd` v0.5.0 where applicable)
  cover happy, unhappy, and edge paths,
- `docs/theoremc-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
  reflect the completed behaviour.

## Constraints

- Keep implementation aligned to:
  `docs/theoremc-design.md` (`DES-6`), `docs/theorem-file-specification.md`
  (`TFS-1`), and roadmap Step 1.3 acceptance.
- Out of scope: backend-specific diagnostic rendering.
- Preserve deterministic ordering of diagnostics and deterministic output text.
- Keep `src/` files under 400 lines by extracting helpers/modules where needed.
- Avoid `unsafe` and avoid lint suppressions unless strictly necessary and
  tightly scoped with rationale.
- Keep the existing public loader behaviour backwards compatible where
  practical; if an API shape change is required, document it in
  `docs/users-guide.md` and call it out in `Decision Log`.
- Use `rstest-bdd` v0.5.0 for behavioural scenarios where they provide
  externally meaningful acceptance coverage.
- Run required quality gates with logs captured via `tee` and
  `set -o pipefail`.

## Tolerances (exception triggers)

- Scope: if implementation needs more than 10 files changed or more than 700
  net new lines, stop and escalate with a narrower split plan.
- Dependencies: do not add new runtime dependencies. A new dev-dependency for
  snapshots is allowed only if file-based snapshots are demonstrably
  insufficient; otherwise prefer file snapshots without adding crates.
- API: if existing `SchemaError` matching semantics must break, stop and
  document migration options before proceeding.
- Location fidelity: if deterministic line/column cannot be produced for a
  validator failure class, stop and document the specific class plus fallback.
- Iterations: if any one gate (`check-fmt`, `lint`, `test`) fails more than 5
  consecutive fix attempts, stop and escalate with logs.

## Risks

- Risk: current loader API `load_theorem_docs(&str)` has no source file input,
  but Step 1.3 requires source file in diagnostics. Mitigation: add a
  source-aware entry point and preserve the current function as a convenience
  wrapper with a deterministic default source identifier.

- Risk: existing validation emits reason strings without location metadata.
  Mitigation: introduce location-carrying validation context and migrate
  validators incrementally so each emitted diagnostic has a concrete location.

- Risk: `serde(untagged)` plus nested schema can complicate per-field span
  extraction. Mitigation: use `serde-saphyr` location APIs and `Spanned<T>`
  where feasible, and maintain deterministic fallback mapping when direct spans
  are unavailable.

- Risk: snapshots can become brittle if they include non-deterministic paths.
  Mitigation: normalize source paths in snapshots to stable fixture-relative
  paths.

## Progress

- [x] (2026-02-21) Draft ExecPlan for Step 1.3.
- [x] (2026-02-21) Milestone 0: baseline audit and diagnostic contract
  definition.
- [x] (2026-02-21) Milestone 1: introduced structured diagnostic model and
  error plumbing.
- [x] (2026-02-21) Milestone 2: implemented parser failure wrapping with source
  locations.
- [x] (2026-02-21) Milestone 3: implemented validator failure wrapping with
  source locations.
- [x] (2026-02-21) Milestone 4: built fixture corpus and snapshot harness.
- [x] (2026-02-21) Milestone 5: added unit + behavioural (`rstest-bdd` v0.5.0)
  coverage.
- [x] (2026-02-21) Milestone 6: updated design and user documentation.
- [x] (2026-02-21) Milestone 7: quality gates passed and roadmap Step 1.3
  marked done.

## Surprises & discoveries

- `docs/rstest-bdd-users-guide.md` is not present in this repository. Existing
  guidance is in `docs/rust-testing-with-rstest-fixtures.md`, and current code
  already includes `rstest-bdd = "0.5.0"` and `rstest-bdd-macros = "0.5.0"`.
- Project-memory MCP/qdrant resources are not available in this environment,
  so no historical notes were retrievable for this planning session.
- `src/schema/loader.rs` exceeded the 400-line module limit when diagnostics
  tests were added inline. The tests were moved to `src/schema/loader_tests.rs`
  and wired via `#[path = "loader_tests.rs"]` to keep `loader.rs` concise.
- `serde-saphyr` parse error `Display` output includes multiline snippets.
  Structured diagnostics now normalize parser messages to the first line so
  snapshot baselines remain deterministic.

## Decision log

- Decision: implement Step 1.3 as a diagnostics-foundation change, not just
  message polishing. Rationale: roadmap acceptance explicitly requires
  structured, source-located, stable diagnostics and snapshot assertions.
  Date/Author: 2026-02-21 / Codex.

- Decision: prefer a source-aware loader API while preserving the existing
  convenience API. Rationale: diagnostics must include source file identifiers
  without forcing immediate consumer breakage. Date/Author: 2026-02-21 / Codex.

- Decision: use deterministic file-based snapshots first, then evaluate adding
  a snapshot crate only if needed. Rationale: minimizes dependency churn and
  keeps snapshot review explicit. Date/Author: 2026-02-21 / Codex.

## Outcomes & retrospective

Step 1.3 is complete.

Implemented outcomes:

- Added structured diagnostics (`SchemaDiagnostic`) with stable codes and
  source/line/column fields.
- Added source-aware loading API:
  `load_theorem_docs_with_source(source, input)`, while preserving
  `load_theorem_docs(input)` compatibility.
- Wrapped parser and validator failures with structured diagnostics.
- Added span-aware raw schema mapping for key validation fields to improve
  location fidelity.
- Added snapshot tests for representative parser and validator failures:
  `tests/schema_diagnostics_snapshot.rs`.
- Added fixture corpus regression tests:
  `tests/schema_fixture_corpus.rs`.
- Added behavioural diagnostics coverage with `rstest-bdd` v0.5.0:
  `tests/schema_diagnostics_bdd.rs`.
- Expanded fixture suite with alias, nested `maybe`, and `must`-shape focused
  fixtures.
- Updated `docs/theoremc-design.md`, `docs/users-guide.md`,
  `docs/roadmap.md`, and `docs/contents.md`.

## Context and orientation

Relevant current implementation files:

- `src/schema/error.rs` carries structured diagnostics alongside semantic error
  variants.
- `src/schema/loader.rs` and `src/schema/raw.rs` implement source-aware loading
  and diagnostic location mapping.
- `src/schema/validate.rs` remains the semantic rule engine with deterministic
  reason strings.
- `tests/schema_diagnostics_snapshot.rs` locks source-located diagnostic output
  shape.
- `tests/schema_fixture_corpus.rs` and `tests/schema_diagnostics_bdd.rs`
  validate fixture and behavioural coverage for Step 1.3.

Roadmap target for this plan:

- `docs/roadmap.md` Step 1.3
  - structured diagnostics with source file/line/column,
  - parser/validator regression fixture suite for aliases, nested `maybe`,
    `must` semantics preconditions, and witness policy.

## Plan of work

### Milestone 0: baseline audit and diagnostic contract definition

Define the diagnostic contract before code edits:

- identify parser failure classes from current fixtures (unknown keys, wrong
  scalar types, missing required fields, malformed YAML),
- identify validator failure classes from current fixtures (blank fields,
  expression rejection, `maybe` shape issues, vacuity policy),
- define canonical rendered shape used for snapshots (for example:
  `CODE | source:line:column | message`).

Record this contract in `Decision Log` and use it consistently in tests.

### Milestone 1: structured diagnostic model and error plumbing

Introduce a structured diagnostic type in `src/schema/error.rs` (or a new
`src/schema/diagnostic.rs` module), with stable fields for:

- diagnostic code/category,
- source file identifier,
- line,
- column,
- deterministic English fallback message.

Update `SchemaError` plumbing so all load failures can expose the structured
payload in addition to `Display` output.

### Milestone 2: parser failure wrapping with source locations

Wrap `serde-saphyr` deserialization failures into structured diagnostics using
available location APIs (`location()`/`locations()` equivalents), mapping into
line and column.

Add source-path awareness to loading:

- introduce a source-aware entry point (for example,
  `load_theorem_docs_with_source(source, input)`),
- keep `load_theorem_docs(&str)` as a wrapper using a deterministic default
  source identifier.

Ensure parser errors always include source, line, and column when provided by
upstream parser APIs, with deterministic fallback only where unavoidable.

### Milestone 3: validator failure wrapping with source locations

Refactor validation paths so failures carry precise source locations:

- add location context to validation helpers,
- map semantic checks to specific field/token locations,
- preserve deterministic first-failure ordering.

For nested structures (`Do`/`maybe`/`Let`), ensure location mapping remains
stable and the emitted path context remains actionable.

### Milestone 4: fixture corpus and snapshot harness

Expand fixture set under `tests/fixtures/` to cover Step 1.3 acceptance:

- aliases (TitleCase/lowercase key aliases),
- nested `maybe` success and failure cases,
- `must` precondition-oriented fixtures,
- witness policy happy/unhappy cases.

Add snapshot tests for representative parser and validator failures. Snapshot
artefacts should be deterministic and reviewable, and should assert source
file, line, and column stability.

### Milestone 5: unit and behavioural tests

Add or update tests in:

- unit/integration tests for diagnostic struct mapping and location fidelity,
- behavioural tests using `rstest-bdd` v0.5.0 where scenario phrasing improves
  acceptance clarity.

Coverage matrix must include:

- happy: valid theorem corpus parses cleanly,
- unhappy: parser and validator errors produce structured diagnostics,
- edge: nested `maybe` and multi-document ordering preserve deterministic
  diagnostics.

### Milestone 6: documentation and roadmap updates

Update docs to record behaviour and decisions:

- `docs/theoremc-design.md`: add Step 1.3 implementation decisions under
  parsing/validation diagnostics.
- `docs/users-guide.md`: document any API or behavioural changes that library
  consumers must know (new loader entry point, structured diagnostics, and
  error-shape expectations).
- `docs/roadmap.md`: mark Step 1.3 items as done when implementation and gates
  pass.

### Milestone 7: quality gates and completion checks

Run formatting and quality gates with log capture, verify snapshots and tests,
then close roadmap Step 1.3.

## Concrete steps

Run all commands from repository root.

1. Baseline tests before code edits.

    ```sh
    set -o pipefail
    make test | tee /tmp/step-1-3-baseline-test.log
    ```

2. Implement milestones 1-6.

3. Format sources and documentation.

    ```sh
    set -o pipefail
    make fmt | tee /tmp/step-1-3-fmt.log
    ```

4. Validate Markdown and Mermaid diagrams (docs are modified in this step).

    ```sh
    set -o pipefail
    make markdownlint | tee /tmp/step-1-3-markdownlint.log

    set -o pipefail
    make nixie | tee /tmp/step-1-3-nixie.log
    ```

5. Run required commit gates.

    ```sh
    set -o pipefail
    make check-fmt | tee /tmp/step-1-3-check-fmt.log

    set -o pipefail
    make lint | tee /tmp/step-1-3-lint.log

    set -o pipefail
    make test | tee /tmp/step-1-3-test.log
    ```

6. Verify all acceptance criteria and mark roadmap Step 1.3 done.

## Validation and acceptance

Step 1.3 is complete only when all are true:

- parser and validator failures are wrapped into structured diagnostics with
  source file, line, and column,
- snapshot tests assert stable diagnostic output for representative failures,
- fixture corpus covers aliases, nested `maybe`, `must` preconditions, and
  witness policy,
- unit and behavioural tests (`rstest-bdd` v0.5.0 where applicable) cover happy,
  unhappy, and edge cases,
- `docs/theoremc-design.md` records Step 1.3 design decisions,
- `docs/users-guide.md` documents any consumer-visible behaviour/API changes,
- `docs/roadmap.md` Step 1.3 entries are marked done,
- `make check-fmt`, `make lint`, and `make test` pass.

## Idempotence and recovery

- All fixture and test additions are additive and safe to rerun.
- If a gate fails, fix forward and rerun the failed gate, then rerun full
  gates.
- If location extraction for a failure class is non-deterministic, stop,
  document the blocker in `Decision Log`, and choose a deterministic fallback
  before continuing.

## Artefacts

Expected artefacts from implementation of this plan:

- Structured diagnostics implementation in `src/schema/`.
- New or updated diagnostic-focused tests and snapshots under `tests/`.
- Expanded fixture corpus under `tests/fixtures/`.
- Updated `docs/theoremc-design.md`, `docs/users-guide.md`, and
  `docs/roadmap.md`.
- Gate logs in `/tmp/step-1-3-*.log`.
