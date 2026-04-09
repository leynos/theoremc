# Step 2.2.2: deterministic harness naming and theorem-key checks

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Implement the remaining naming-stability work in Roadmap Phase 2, Step 2.2:
stable theorem harness naming plus build-time duplicate theorem-key rejection.

After this change, a caller will be able to derive a deterministic Kani harness
symbol for any theorem document using the normative format
`theorem__{theorem_slug(T)}__h{hash12(P#T)}`, where `P` is the literal theorem
file path and `T` is the `Theorem` identifier. The human-readable slug will be
stable across builds, acronym-safe, and predictable for already-snake theorem
names. The hash suffix will make the final symbol injective for distinct
theorem keys. The loader will also reject duplicate theorem keys before code
generation, so the naming scheme is compile-time checked rather than merely
conventional.

Observable success:

- `theoremc::mangle` exposes theorem-harness helpers that turn
  `"theorems/bidirectional.theorem"` and `"BidirectionalLinksCommitPath3Nodes"`
  into a stable slug, theorem key, and harness identifier.
- Unit tests prove the snake-case conversion is deterministic for acronym runs,
  number boundaries, and theorem names that are already snake_case.
- Behavioural tests using `rstest-bdd` v0.5.0 cover both happy and unhappy
  paths, including duplicate theorem-key rejection.
- `load_theorem_docs_with_source` fails with actionable diagnostics when two
  theorem documents share the same theorem key.
- `docs/theoremc-design.md` records the implementation decisions,
  `docs/users-guide.md` explains the new public naming helpers and duplicate
  theorem-key failure mode, and `docs/roadmap.md` marks the relevant entry or
  entries done once the implementation is complete.
- `make check-fmt`, `make lint`, and `make test` pass.

This plan covers the normative requirements in `NMR-1`, `ADR-2`, `DES-7`, and
`TFS-1`. It intentionally excludes report-level alias resolution, which remains
out of scope until the later reporting work.

## Constraints

- The implementation must build on the completed Step 2.2.1 module-mangling
  work in `src/mangle.rs` and must not replace that algorithm.
- Keep the architectural boundary from ADR 003 intact:
  `schema` remains responsible for loading and validation, `mangle` remains
  responsible for deterministic symbol derivation, and any cross-cutting
  duplicate-theorem-key checking lives outside those two concerns if the
  boundary would otherwise be crossed.
- Do not introduce report-level theorem ID alias resolution or path
  normalization beyond the exact theorem-key definition `"{P}#{T}"`.
- Harness naming must follow the normative format exactly:
  `theorem__{theorem_slug(T)}__h{hash12(P#T)}`.
- `theorem_slug(T)` must preserve theorem names already matching
  `^[a-z_][a-z0-9_]*$` without modification.
- Non-snake theorem names must be converted using the deterministic rule from
  `docs/name-mangling-rules.md` and `docs/theorem-file-specification.md`:
  1. insert `_` between a lower-case letter or digit and an upper-case letter,
  1. split acronym runs before the last capital when followed by lower-case
     text,
  1. lowercase the final result.
- Duplicate theorem-key checking must use the literal file path string supplied
  by the caller plus the theorem identifier exactly as loaded, joined by `#`.
- Error reporting for duplicate theorem keys must be deterministic and include
  enough source context to identify the colliding theorem documents.
- Keep code files under 400 lines. If new tests or helpers would push
  `src/mangle.rs`, `src/schema/loader.rs`, or a collision-related module over
  the limit, extract focused sibling files using the existing `#[path = ...]`
  pattern already used by `src/mangle_tests.rs`.
- Behavioural coverage must use `rstest-bdd` v0.5.0 where it adds value.
- The final implementation must update:
  `docs/theoremc-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.
- Use en-GB-oxendict spelling and grammar in all documentation and comments.

## Tolerances

- Scope: if the work grows beyond 12 changed files or roughly 700 net lines,
  stop and split the theorem-key collision portion into a follow-up plan before
  proceeding.
- API churn: if satisfying the feature requires breaking or renaming existing
  public `schema` or `mangle` APIs rather than additive changes, stop and
  escalate.
- Error-model change: if duplicate theorem-key rejection cannot fit cleanly in
  the existing `SchemaError` model without broader caller breakage, stop and
  document the options before proceeding.
- Test instability: if `make check-fmt`, `make lint`, or `make test` fails more
  than five consecutive fix attempts, stop and escalate with the captured logs.
- Specification ambiguity: if the deterministic CamelCase-to-snake_case rules
  admit materially different outputs for a theorem identifier, document the
  candidate interpretations in `Decision Log` and seek direction before locking
  in a public contract.

## Risks

- Risk: acronym-heavy theorem names such as `HNSWInvariant` and
  `HTTP2StreamID` are easy to snake-case incorrectly. Mitigation: encode golden
  unit tests for acronym runs and mixed acronym-plus-number boundaries before
  implementation, and derive behaviour strictly from the normative text rather
  than a generic case-conversion crate.

- Risk: duplicate theorem-key checking needs the theorem source path, but the
  current `TheoremDoc` shape may not retain that path after loading.
  Mitigation: inspect the current loader pipeline first and choose the
  narrowest place to run the check, likely where `SourceId` and raw theorem
  names are both still available, instead of threading new state broadly
  through the schema model without need.

- Risk: the short harness slug may collide even when theorem keys differ.
  Mitigation: treat slug collisions as expected and harmless because uniqueness
  comes from `hash12(P#T)`; only duplicate theorem keys are a hard error.

- Risk: error diagnostics for duplicate theorem keys may be less actionable than
  other validation errors if implemented only as a flat string. Mitigation:
  prefer a dedicated `SchemaError` variant and reuse the existing
  source-located diagnostic pattern from loader validation where feasible.

- Risk: adding too much logic to `src/mangle.rs` or a single test module can
  violate the repository line-count policy. Mitigation: follow the existing
  split-file pattern (`mangle_tests.rs`, `collision_tests.rs`) early rather
  than as a cleanup afterthought.

## Progress

- [x] 2026-03-06: reviewed roadmap, name-mangling rules, ADR 001, design
  document sections for Steps 2.1.x and 2.2.1, current `mangle` and collision
  implementations, and current user guide coverage.
- [x] 2026-03-06: drafted this ExecPlan for deterministic harness naming and
  theorem-key checks.
- [x] 2026-03-06: Milestone 0 complete. Duplicate theorem-key validation is
  implemented in `src/schema/loader.rs`, after per-document conversion and
  validation and before cross-document action-collision checks.
- [x] 2026-03-06: Milestone 1 complete. Unit tests now cover theorem slugging,
  theorem-key construction, and harness identifier generation in
  `src/mangle_tests.rs`.
- [x] 2026-03-06: Milestone 2 complete. Behavioural coverage now lives in
  `tests/harness_naming_bdd.rs` with scenarios for deterministic harness
  naming, snake-case preservation, and duplicate theorem-key rejection.
- [x] 2026-03-06: Milestone 3 complete. Public theorem-harness helpers are
  implemented in `theoremc::mangle`, with code extracted into
  `src/mangle_harness.rs` and `src/mangle_path.rs` to keep files under the
  line-count limit.
- [x] 2026-03-06: Milestone 4 complete. Duplicate theorem keys now return
  `SchemaError::DuplicateTheoremKey` with deterministic source-aware
  diagnostics.
- [x] 2026-03-06: Milestone 5 complete. Design, user-guide, and roadmap docs
  now describe the public API and failure mode.
- [x] 2026-03-06: Milestone 6 complete. Validation gates passed:
  `make fmt`, `make markdownlint`, `make nixie`, `make check-fmt`, `make lint`,
  and `make test`.

## Surprises & Discoveries

- 2026-03-06: Step 2.2.1 already established `src/mangle.rs` as the shared home
  for all naming logic and extracted tests to `src/mangle_tests.rs`, so this
  step should extend that module rather than creating a parallel harness-naming
  module.
- 2026-03-06: the current loader already runs cross-cutting action collision
  detection after document validation, which gives a precedent for theorem-key
  checking near `load_theorem_docs_with_source`.
- 2026-03-06: the requested `docs/rstest-bdd-users-guide.md` reference is not
  present in this checkout. The plan therefore relies on existing repository
  usage of `rstest-bdd` v0.5.0 and the current BDD tests as the local style
  reference.
- 2026-03-06: `src/mangle.rs` was already at 398 lines before this step. The
  implementation therefore had to extract `src/mangle_path.rs` and
  `src/mangle_harness.rs` rather than extending the root file directly.

## Decision Log

- 2026-03-06: this plan covers both harness naming and duplicate theorem-key
  rejection even though the file name is `2-2-2-...`. Rationale: the user asked
  for the full Step 2.2 implementation slice, and ADR 001 explicitly couples
  harness naming with build-time duplicate theorem-key rejection.
- 2026-03-06: plan the work as additive public helpers in `theoremc::mangle`
  plus loader-level validation, rather than a monolithic new subsystem.
  Rationale: the current code already separates naming derivation from loading
  and uses the loader as the place where cross-document invariants are enforced.
- 2026-03-06: use a dedicated `SchemaError::DuplicateTheoremKey` variant
  rather than overloading `ValidationFailed`. Rationale: duplicate theorem-key
  rejection is a cross-document invariant with different payload needs (theorem
  key plus duplicate-site diagnostic) than single-document semantic validation.

## Outcomes & Retrospective

- The final public API stayed additive. New public helpers are
  `theorem_key`, `theorem_slug`, `mangle_theorem_harness`, and `MangledHarness`.
- Duplicate theorem-key checking is hosted in
  `src/schema/loader.rs`, where the loader still has access to both `SourceId`
  and spanned raw theorem identifiers.
- New automated coverage consists of 11 unit tests for theorem-key and harness
  naming plus 3 new behavioural scenarios in `tests/harness_naming_bdd.rs`.
- Documentation updates were made in `docs/theoremc-design.md`,
  `docs/users-guide.md`, and `docs/roadmap.md`.
- Validation results:
  `make fmt`, `make markdownlint`, `make nixie`, `make check-fmt`, `make lint`,
  and `make test` all passed on 2026-03-06.

## Context and orientation

The current theoremc crate already has the first half of Step 2.2 complete:
`src/mangle.rs` can derive stable per-file module names via `path_stem`,
`path_mangle`, `hash12`, and `mangle_module_path`, and
`tests/module_naming_bdd.rs` exercises that behaviour with `rstest-bdd`.

The relevant existing code is:

- `src/mangle.rs`
  Defines the public naming helpers and their domain types. This is the natural
  home for new theorem naming helpers such as `theorem_key`, `theorem_slug`,
  and `mangle_theorem_harness`.
- `src/mangle_tests.rs`
  Holds unit tests for both action mangling and module naming. Extend this file
  or split it further if theorem naming pushes it near the 400-line limit.
- `src/schema/loader.rs`
  Loads theorem documents and already invokes
  `crate::collision::check_action_collisions(&docs)` after per-document
  validation. This is the first place to inspect for duplicate theorem-key
  enforcement.
- `src/schema/error.rs`
  Defines the current error surface. Duplicate theorem-key rejection will
  likely require a new typed variant rather than overloading `ValidationFailed`.
- `docs/name-mangling-rules.md`
  Normative source for harness naming.
- `docs/theorem-file-specification.md`
  Restates the deterministic theorem-snake rules and duplicate theorem-key
  requirement in §7.4.
- `docs/theoremc-design.md`
  Contains the implementation-decision log sections for Steps 2.1.x and 2.2.1;
  add a new Step 2.2.2 section once the design is settled.
- `docs/users-guide.md`
  Already documents action mangling and per-file module naming. It should gain
  a user-facing section for theorem harness naming and duplicate theorem-key
  errors.
- `docs/roadmap.md`
  Currently shows the Step 2.2 harness-naming and duplicate-theorem-key items
  as incomplete.

Key terms used in this plan:

- `P`: the literal `.theorem` file path string, relative to the crate root.
- `T`: the theorem identifier loaded from the `Theorem` field.
- `theorem_key(P, T)`: the exact string `"{P}#{T}"`.
- `theorem_slug(T)`: the deterministic snake-case form used in the readable part
  of the harness name.
- Harness identifier: `theorem__{theorem_slug(T)}__h{hash12(P#T)}`.
- Full harness path:
  `__theoremc__file__...::kani::theorem__...__h{hash12(P#T)}`.

## Plan of work

### Stage A: preflight and failing tests first

Inspect how theorem file path information reaches the loader today. Confirm
whether duplicate theorem-key checking can run:

1. in the loader using the existing `SourceId` plus theorem names,
1. in a new cross-cutting module analogous to `src/collision.rs`, or
1. in a future build-generation layer only if neither of the first two can
   preserve source diagnostics.

Before implementation, add failing tests that lock the intended public contract:

- unit tests in `src/mangle_tests.rs` for theorem slugging,
  theorem-key formatting, and harness identifier generation;
- behavioural tests using `rstest-bdd` for deterministic harness naming and
  duplicate theorem-key rejection.

Go/no-go:

- the insertion point for duplicate theorem-key checking is selected and
  documented in `Decision Log`;
- at least one new unit test and one new behavioural test fail for the right
  reason before production code changes begin.

### Stage B: theorem slugging and harness-name helpers

Extend `src/mangle.rs` with additive theorem naming support. The likely helper
set is:

1. `pub fn theorem_key(path: impl AsRef<str>, theorem: impl AsRef<str>) -> String`
1. `pub fn theorem_slug(theorem: impl AsRef<str>) -> String`
1. `pub struct MangledHarness { slug, theorem_key, hash, identifier }`
1. `pub fn mangle_theorem_harness(path: impl AsRef<str>, theorem: impl AsRef<str>) -> MangledHarness`

The exact names may change if a better additive API emerges, but the final API
must let callers inspect the readable slug, the hash, the theorem key, and the
final harness identifier without reparsing strings.

Implementation notes:

- Fast-path already-snake theorem names unchanged.
- For CamelCase inputs, iterate character-by-character so underscore insertion
  is explicit and deterministic.
- Cover these cases in golden tests:
  - acronym runs: `HNSWInvariant` -> `hnsw_invariant`
  - number boundaries: `BidirectionalLinksPath3Nodes` ->
    `bidirectional_links_path_3_nodes`
  - mixed acronym plus lower-case transition: `HTTPServerReady` ->
    `http_server_ready`
  - already-snake inputs: `hnsw_smoke` -> `hnsw_smoke`
  - single-word capitalized input: `Deposit` -> `deposit`

Do not add a case-conversion dependency unless the specification proves too
subtle to implement safely by hand; that would exceed the dependency tolerance
and require escalation.

### Stage C: duplicate theorem-key rejection

Implement the compile-time check for duplicate theorem keys. Prefer the
narrowest design that preserves the existing boundary:

- if loader-level access to `(path, theorem name, source location)` is
  sufficient, perform the check there after per-document validation and before
  returning `Ok(docs)`;
- if the logic would muddy the loader, extract a small top-level module such as
  `src/theorem_key_collision.rs` that receives the minimal data needed and
  returns a typed `SchemaError`.

Implementation requirements:

- group theorem documents by exact theorem key using `BTreeMap` / `BTreeSet` so
  iteration order and error messages stay deterministic;
- report all collisions in one failure, not just the first;
- include both the theorem key and the participating source/theorem locations in
  the error payload or formatted message;
- keep the check orthogonal to harness slug uniqueness, because slug collisions
  are expected to be resolved by the hash suffix.

If path information is not currently retained long enough for this check,
introduce the smallest possible representation needed for duplicate-theorem-key
validation rather than threading full filesystem objects through the schema.

### Stage D: behavioural coverage and unhappy paths

Add a dedicated BDD runner and feature file for harness naming, or extend the
existing module-naming BDD tests only if that keeps the scope readable. A
separate harness-naming suite is preferable because it exercises different
behaviour.

Required scenarios:

1. happy path: representative theorem IDs produce the expected harness names;
1. happy path: already-snake theorem IDs are preserved exactly in the slug;
1. unhappy path: duplicate theorem keys are rejected before code generation;
1. edge case: theorem IDs with acronym runs and numeric boundaries produce the
   documented slug.

BDD implementation guidance:

- follow the style already used in `tests/action_mangle_bdd.rs` and
  `tests/module_naming_bdd.rs`;
- use shared helpers or a local fixture struct if repeated YAML assembly would
  otherwise create duplication;
- keep unhappy-path assertions focused on observable diagnostics rather than
  internal helper names.

### Stage E: documentation and roadmap sync

Update the design and consumer documentation after the implementation is
settled:

- add a new implementation-decision subsection to `docs/theoremc-design.md`
  covering where theorem slugging lives, how acronym splitting works, why hash
  uniqueness is attached to `P#T`, and where duplicate theorem-key checking is
  enforced;
- extend `docs/users-guide.md` with a theorem harness naming section containing
  a short code example and a note that duplicate theorem keys are rejected by
  the loader/build pipeline;
- mark the Step 2.2 harness-naming item done in `docs/roadmap.md`, and if this
  implementation also completes duplicate theorem-key rejection, mark that item
  done too. Do not mark either roadmap checkbox done until the implementation
  and all gates pass.

### Stage F: validation and evidence capture

Run the required gates with `tee` and `pipefail` so failures are visible
despite truncated output:

```bash
set -o pipefail; make fmt | tee /tmp/2-2-2-make-fmt.log
set -o pipefail; make markdownlint | tee /tmp/2-2-2-make-markdownlint.log
set -o pipefail; make nixie | tee /tmp/2-2-2-make-nixie.log
set -o pipefail; make check-fmt | tee /tmp/2-2-2-make-check-fmt.log
set -o pipefail; make lint | tee /tmp/2-2-2-make-lint.log
set -o pipefail; make test | tee /tmp/2-2-2-make-test.log
```

Review the tail of each log before concluding the work. Record any deviations
or waived issues in `Outcomes & Retrospective`; the target outcome is zero
deviations.

## Concrete file plan

Expected files to edit during implementation:

- `src/mangle.rs`
- `src/mangle_tests.rs`
- `tests/harness_naming_bdd.rs` or an equivalent focused BDD test file
- `tests/features/harness_naming.feature`
- `src/schema/loader.rs` and possibly `src/schema/error.rs`
- optionally a small new top-level collision/check module if needed for
  duplicate theorem-key logic
- `docs/theoremc-design.md`
- `docs/users-guide.md`
- `docs/roadmap.md`

Files to use as references without changing unless needed:

- `tests/action_mangle_bdd.rs`
- `tests/module_naming_bdd.rs`
- `docs/name-mangling-rules.md`
- `docs/theorem-file-specification.md`
- `docs/adr-001-theorem-symbol-stability-and-non-vacuity-policy.md`

## Acceptance checklist

The implementation is complete only when all of the following are true:

1. A public caller can derive the deterministic harness identifier for a theorem
   using additive helpers in `theoremc::mangle`.
1. Unit tests cover acronym runs, numeric boundaries, and already-snake theorem
   identifiers.
1. Behavioural tests cover both successful harness naming and duplicate
   theorem-key rejection.
1. Duplicate theorem keys fail before code generation with actionable,
   deterministic errors.
1. `docs/theoremc-design.md` and `docs/users-guide.md` reflect the shipped
   behaviour.
1. `docs/roadmap.md` is updated only for the work actually completed.
1. `make check-fmt`, `make lint`, and `make test` succeed, with documentation
   validation also passing because this task changes Markdown files.
