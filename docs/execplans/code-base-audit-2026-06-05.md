# Address code base audit findings from 2026-06-05

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: APPROVED

## Purpose / big picture

The audit found that theoremc still permits invalid action names to travel
through domain objects as strings, validates some invariants too late, maps
validation diagnostics by parsing human-readable messages, and mixes loading,
semantic query, and rendering responsibilities in the proc-macro crate. After
this plan is implemented, a contributor can observe that invalid canonical
action names fail before they enter `TheoremDoc`, public mangling APIs cannot
silently produce invalid identifiers, validation diagnostics are located using
typed field paths instead of string matching, and macro expansion is split into
query and render modules with narrower contracts.

The work also updates documentation so schema defaults match implementation,
documents reusable testing and validation patterns in
`docs/developers-guide.md`, adds the missing Rustdoc examples for schema
newtypes, removes duplicate newtype deserialization boilerplate, and
centralizes duplicated integration fixture helpers.

The user has explicitly authorized implementation through the
`sdlc-implementation` workflow. Work proceeds one validated milestone at a
time, with this plan recording acceptance evidence and any remaining review
work.

## Constraints

Do not change theorem file syntax except where invalid action names already
violate the documented canonical grammar. Existing valid `.theorem` fixtures
must continue to load, macro expansion must remain deterministic, and generated
module and harness identifiers must remain stable for the same valid inputs.

Preserve ADR-003's boundary rule that the schema layer does not import from the
mangle layer. Shared action-name grammar must live in a neutral module that
both schema and mangle code can depend on without creating a schema-to-mangle
dependency.

Keep public domain types free of direct `serde` derives once the raw adapter
has enough raw types to own deserialization. The raw schema layer may use serde
and must convert into public domain types through explicit constructors or
conversion helpers.

Use the repository Makefile targets for gates. After each implementation
milestone, run `make check-fmt`, `make lint`, and `make test` sequentially,
capturing output with `tee` into `/tmp`. Also run documentation gates when a
milestone touches Markdown.

Before requesting `coderabbit review --agent` for a milestone, all applicable
deterministic gates for that milestone must pass. If CodeRabbit reports a rate
limit, run `vsleep $(shuf -i 15-30 -n 1)m` and retry.

Commit after each completed milestone. Do not commit a milestone while any
required gate or CodeRabbit concern remains unresolved.

Use `rstest` for unit tests and `rstest-bdd` for behavioural tests where the
change has externally observable behaviour. Use `googletest` assertions and
`pretty_assertions` where they improve failure output. Use `insta` snapshots
when a multivariant output format needs stable review. Use `proptest` when an
invariant spans many generated inputs. Do not introduce `kani` or `verus`
proofs unless the milestone introduces a new invariant or lemma that needs
bounded or exhaustive proof beyond the existing tests.

## Tolerances (exception triggers)

Escalate before proceeding if changing `TheoremDoc.actions` or
`ActionCall.action` requires more than one compatibility shim for downstream
callers, or if keeping old string-based APIs would preserve the invalid-state
problem the audit identified.

Escalate before proceeding if a typed validation error design cannot preserve
the current structured diagnostic code, source, line, column, and user-facing
message for existing parser and validation fixtures.

Escalate before proceeding if moving serde derives out of public schema types
requires more than six new raw mirror types or makes the raw-to-domain
conversion materially less readable than the current code.

Escalate before proceeding if the macro split needs more than three new modules
or substantially changes the token output shape beyond intentional test
snapshot updates.

Escalate before proceeding if `googletest`, `pretty_assertions`, or other test
dependency additions conflict with the repository lint policy, Rust version, or
dependency guidance.

## Risks

Changing action names from `String` to a newtype is a public API change. The
project is still pre-1.0, but this can still disrupt tests and examples. The
mitigation is to update all internal call sites and user documentation in the
same milestone, and to provide clear accessor methods such as `as_str()`.

Removing serde derives from domain types can temporarily duplicate shapes
between raw and domain modules. The mitigation is to move one coherent group at
a time and keep conversion helpers small, documented, and covered by existing
fixture tests.

Typed validation diagnostics may accidentally change user-facing messages or
locations. The mitigation is to keep the current diagnostic snapshots as a
compatibility net and add targeted unit tests for field-path-to-location
mapping.

Splitting `theoremc-macros/src/lib.rs` can disturb proc-macro output even when
behaviour is intended to stay the same. The mitigation is to preserve the
existing snapshot and trybuild coverage, then add query/render unit tests at
the new module boundaries.

CodeRabbit may be unavailable or rate-limited. The mitigation is to follow the
required `vsleep` retry rule and record any repeated tool failure in the
Decision Log before asking for direction.

## Related follow-up issues

The following audit findings are related but outside the direct scope of this
implementation plan:

- [#48](https://github.com/leynos/theoremc/issues/48): define or remove the
  placeholder `theoremc` binary contract. Still open.
- [#49](https://github.com/leynos/theoremc/issues/49): replace build-script
  `#[path]` sharing with a stable build support boundary. Completed on
  2026-10-18 while this plan was paused.
- [#50](https://github.com/leynos/theoremc/issues/50): split oversized schema
  modules to preserve the 400-line file limit. Still open; this plan's module
  splits reduced some pressure but did not close the issue.
- [#51](https://github.com/leynos/theoremc/issues/51): include provenance in
  macro action-signature conflict diagnostics. Completed on 2026-10-16.

## Repository orientation

The root crate in `src/` re-exports core APIs and build integration helpers.
Core theorem semantics live in `crates/theoremc-core/src/`. The schema modules
under `crates/theoremc-core/src/schema/` own raw YAML loading, public domain
types, semantic validation, structured diagnostics, and theorem value decoding.
Name mangling lives in `crates/theoremc-core/src/mangle.rs` and sibling
modules. The procedural macro implementation lives in
`crates/theoremc-macros/src/lib.rs`, with tests in sibling test modules and
trybuild fixtures under `crates/theoremc-macros/tests/expand/`. Behavioural
tests live under `tests/`, with reusable helpers in `tests/common/` and Gherkin
feature files under `tests/features/`.

`crates/theoremc-core/src/canonical_action_name.rs` now owns the single
canonical action-name grammar and newtype used by both schema and mangling
code, replacing the two separate validators this audit found. The remaining
milestones continue from that shared boundary.

## Conformance basis

This plan implements the schema-layer contract in
`docs/adr-003-architectural-boundary-enforcement.md`, the schema and diagnostic
requirements in `docs/theorem-file-specification.md`, the action naming rules in
`docs/name-mangling-rules.md`, and the macro pipeline in
`docs/theoremc-design.md` §3.2 and §7. The repository's `AGENTS.md` supplies
the Rust API, test, file-size, formatting, lint, and documentation constraints.
No separate terms of reference or technical design document governs this audit.

The target trace is: canonical grammar and domain boundary → Milestones 2–3 →
raw-conversion and mangle tests; typed validation paths → Milestone 4 → source
location tests and unchanged diagnostic snapshots; raw/domain separation →
Milestone 5 → schema fixtures and doctests; schema-default contract → Milestone
6 → omitted/explicit schema tests and documentation; macro separation →
Milestone 7 → expansion snapshots and trybuild tests.

## Verification plan

Each milestone uses the repository's corresponding `make` gates sequentially.
Rust changes require `make check-fmt`, `make lint`, and `make test`; Markdown
changes also require `make fmt`, `make markdownlint`, and `make nixie`. Tests
must exercise the observable boundary named by that milestone, not only helper
functions. The core acceptance invariants are stable valid-name mangling,
rejection of invalid names before domain construction, source/line/column/code
preservation for diagnostics, unchanged schema semantics, and deterministic
macro expansion. No new Kani, Verus, or comparable proof is required because
the milestones do not introduce a proof lemma or bounded state invariant.

## Milestone 0: plan, branch, and review setup

Create this ExecPlan at `docs/execplans/code-base-audit-2026-06-05.md`, rename
the branch to `code-base-audit-2026-06-05`, and create GitHub issues for audit
findings that are related but not part of this plan's direct implementation
scope.

Validation for this milestone is documentation-focused plus the standard
repository gates:

```sh
set -o pipefail
make fmt 2>&1 | tee /tmp/fmt-theoremc-code-base-audit-2026-06-05.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-code-base-audit-2026-06-05.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-code-base-audit-2026-06-05.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-code-base-audit-2026-06-05.out
make lint 2>&1 | tee /tmp/lint-theoremc-code-base-audit-2026-06-05.out
make test 2>&1 | tee /tmp/test-theoremc-code-base-audit-2026-06-05.out
coderabbit review --agent
```

Observable success is that the plan exists, the branch tracks
`origin/code-base-audit-2026-06-05`, a draft pull request exists for the plan,
and CodeRabbit has no unresolved concerns for the plan-only change.

## Milestone 1: test infrastructure and documented reusable patterns

Add `googletest` and `pretty_assertions` to the appropriate workspace
development dependencies. Update `docs/developers-guide.md` so contributors
know when to use `rstest`, `rstest-bdd`, `googletest`, `pretty_assertions`,
`insta`, and `proptest`, and how shared integration fixture helpers should be
owned.

Centralize duplicated integration fixture helpers in `tests/common/mod.rs`. The
shared helper API should cover loading a fixture, asserting successful loading,
asserting an error message fragment, and constructing a `SourceId` for a
fixture. Update the current BDD and integration tests that duplicate these
patterns to use the shared helpers.

Unit and behavioural validation must include at least one `rstest` case using
the new helper API and at least one existing `rstest-bdd` scenario that proves
fixture loading still succeeds and fails as expected. Use `googletest`
assertions and `pretty_assertions` in touched tests where they make expected
and actual values easier to read.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 2: canonical action-name core and checked mangling APIs

Introduce a neutral canonical action-name module in `theoremc-core` that owns
the grammar, validation reason enum, and `CanonicalActionName` domain newtype.
Both schema and mangle code must use this shared module rather than separate
validators.

Change identifier-sensitive mangling paths so they cannot accept arbitrary
strings. The expected API shape is:

```rust
pub fn try_action_slug(name: &str) -> Result<String, InvalidCanonicalActionName>;
pub fn action_slug(name: &CanonicalActionName) -> String;
pub fn try_mangle_action_name(name: &str) -> Result<MangledAction, InvalidCanonicalActionName>;
pub fn mangle_action_name(name: &CanonicalActionName) -> MangledAction;
```

If implementation discovers a better shape, update this plan's Decision Log
before proceeding. The key invariant is that public APIs used for code
generation must either require a validated action-name newtype or return a
typed error.

Add `rstest` unit tests for happy and unhappy validation paths and add
`proptest` coverage that valid generated canonical names always round-trip
through the newtype and produce identifier-safe mangled names. Preserve
existing golden mangle tests by constructing `CanonicalActionName` values in
test setup.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 3: carry canonical action names through schema domain objects

Change `TheoremDoc.actions` from `IndexMap<String, ActionSignature>` to a map
keyed by `CanonicalActionName`. Change `ActionCall.action` from `String` to
`CanonicalActionName`. Raw YAML structs may keep string or spanned string
inputs, but raw-to-domain conversion must validate and construct the newtype
before values enter public domain objects.

Update collision detection, referenced-action traversal, macro query code, and
tests to use `CanonicalActionName::as_str()` where textual output or map lookup
requires a string view. Remove late canonical-action validation that becomes
unreachable once action names are validated during raw conversion.

Add unit tests for raw conversion rejecting invalid `Actions` keys and invalid
`ActionCall.action` values. Add behavioural coverage with `.theorem` fixtures
that proves invalid action names fail with structured diagnostics and valid
canonical names still load. Existing action-name BDD scenarios should remain
green after being adjusted to the new domain type.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 4: typed validation diagnostics without string parsing

Replace the current `validate.rs` pattern of building
`SchemaError::ValidationFailed` strings directly with a typed validation error
model. The model must carry enough information for the loader to render the
same user-facing reason string and to ask the raw document for the correct
source location without parsing that string.

A suitable shape is a `ValidationError` containing a `ValidationPath` and a
`ValidationKind`. `ValidationPath` should identify locations such as `About`,
`Prove { index, field }`, `Assume { index, field }`, `Witness { index, field }`,
`KaniUnwind`, `KaniAllowVacuous`, and `KaniVacuityBecause`. `ValidationKind`
should describe the failure, for example blank field, empty collection, invalid
Rust expression, invalid Rust type, missing action signature, zero unwind, or
missing vacuity reason.

The implementation that landed on `main` realizes this contract as
`ValidationFailure` with `ValidationReasonKind` and `IndexedValidationField`;
see the 2026-11-01 Decision Log entry for the rebase-time substitution.

Update `RawTheoremDoc::location_for_validation_reason` into a typed
location-mapping function. Existing diagnostic snapshots under
`tests/snapshots/diagnostics/` should either remain unchanged or be updated
only for deliberate wording improvements recorded in the Decision Log.

Add `rstest` unit coverage for typed location mapping across at least About,
Prove, Assume, Witness, and Kani fields. Keep or add snapshot coverage when the
rendered diagnostic format has several variants. Behavioural diagnostics tests
must continue to prove source, line, column, and diagnostic code.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 5: remove serde derives from public schema domain types

Move serde-specific deserialization concerns out of public domain types in
`crates/theoremc-core/src/schema/types.rs`. Introduce raw mirror types where
needed in `raw.rs` or focused raw submodules, then convert those raw types into
domain types explicitly.

At minimum, remove direct serde derives from `Assumption`, `Assertion`,
`WitnessCheck`, `ActionSignature`, `Evidence`, `KaniEvidence`, and
`KaniExpectation`, unless the implementation documents a narrower exception in
the Decision Log and explains why it does not violate ADR-003. Keep public
domain constructors or conversion helpers readable and covered by fixture tests.

Remove duplicate `Deserialize` boilerplate for `TheoremName` and `ForallVar` by
introducing a small shared helper or macro for string-backed identifier
newtypes. Add Rustdoc examples for `SourceId::new`, `TheoremName::new`, and
`ForallVar::new`, including valid construction and invalid input where the
constructor is fallible.

Validation must include doctests, existing schema deserialization fixtures, and
targeted unit tests for the shared deserialization helper. If this milestone
changes user-visible schema defaults, update `docs/users-guide.md` and
`docs/theorem-file-specification.md` in the same commit.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 6: reconcile schema default documentation

Resolve the documented conflict around the top-level `Schema` field. The
current implementation preserves an omitted schema as `None`; documentation
reportedly says the default is `1`. Decide whether the implemented contract or
the documentation is wrong.

The preferred direction is to preserve the implemented `Option<u32>` contract
unless a product requirement says omitted schema must be materialized as version
`1`. If preserving implementation, update `docs/theorem-file-specification.md`
and `docs/users-guide.md` to state that the schema version is optional and
omitted values remain unspecified in the Rust model. If changing
implementation, update tests and public API docs to show the new defaulting
behaviour.

Add or update tests that explicitly cover both omitted and explicit schema
values. Use `pretty_assertions` for full-document comparisons where the expected
`TheoremDoc` is large.

After implementation, run documentation gates, standard gates, and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 7: split macro query and render responsibilities

Refactor `crates/theoremc-macros/src/lib.rs` so loading and semantic query
logic are separated from token rendering. A reasonable target shape is:

- `lib.rs` keeps the proc-macro entry point and thin orchestration.
- `query.rs` loads theorem files, checks harness inputs, gathers referenced
  actions, and returns typed intermediate data.
- `render.rs` converts typed intermediate data into `TokenStream2`.
- `error.rs` owns macro expansion errors if the enum grows enough to justify a
  separate module.

The split must preserve token output for existing valid fixtures. Keep the
existing `insta` expansion snapshot as the primary stability check and add unit
tests for query and render helpers where the split creates meaningful new seams.

After implementation, run the standard gates and then
`coderabbit review --agent`. Commit only after all deterministic gates pass and
CodeRabbit concerns are cleared.

## Milestone 8: final integration review and documentation sweep

Run a final repository-wide review against the original audit concerns. Confirm
that the direct concerns in this plan are implemented, related issues #48
through #51 remain separate follow-ups, and reusable patterns introduced by the
work are documented in `docs/developers-guide.md`.

Run all final gates:

```sh
set -o pipefail
make fmt 2>&1 | tee /tmp/fmt-theoremc-code-base-audit-2026-06-05-final.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-code-base-audit-2026-06-05-final.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-code-base-audit-2026-06-05-final.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-code-base-audit-2026-06-05-final.out
make lint 2>&1 | tee /tmp/lint-theoremc-code-base-audit-2026-06-05-final.out
make test 2>&1 | tee /tmp/test-theoremc-code-base-audit-2026-06-05-final.out
coderabbit review --agent
```

Observable success is that the branch has no unresolved CodeRabbit concerns,
the full repository gates pass, and the pull request description lists every
intentional public API or documentation change.

## Progress

- [x] 2026-11-01: User approved this ExecPlan for continued implementation and
  requested a rebase onto `origin/main` before Milestone 4 acceptance. The plan
  status is now `APPROVED`.
- [x] 2026-06-05: Renamed the branch through GitHub's branch rename endpoint
  and updated the local branch to track `origin/code-base-audit-2026-06-05`.
- [x] 2026-06-05: Created related follow-up issues #48, #49, #50, and #51 for
  audit findings outside this implementation plan's direct scope.
- [x] 2026-06-05: Drafted this ExecPlan.
- [x] 2026-06-05: Added this ExecPlan to `docs/contents.md`.
- [x] 2026-06-05: Milestone 0 validation gates passed: `make fmt`,
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`, and
  `make test`.
- [x] 2026-06-05: Milestone 0 CodeRabbit review exited successfully with no
  findings in the command output.
- [x] 2026-06-05: Pushed the Milestone 0 plan-only commit to
  `origin/code-base-audit-2026-06-05`.
- [x] 2026-06-05: Opened draft pull request
  [#52](https://github.com/leynos/theoremc/pull/52) for ExecPlan review.
- [x] 2026-06-05: User explicitly approved implementation and requested
  milestone-by-milestone progress with frequent commits.
- [x] 2026-06-05: Milestone 1 implementation drafted: added shared fixture
  helpers in `tests/common`, added `googletest` and `pretty_assertions`,
  updated duplicated BDD and schema fixture tests, and documented reusable test
  patterns in `docs/developers-guide.md`.
- [x] 2026-06-05: Milestone 1 validation gates passed: `make fmt`,
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`, and
  `make test`. The test suite now runs 568 nextest tests plus doctests.
- [x] Milestone 1 CodeRabbit review is clear after fixing its helper-test
  findings and rerunning `coderabbit review --agent`.
- [x] 2026-06-05: Milestone 1 implementation committed as `2cf0028`
  (`Centralize fixture test helpers`).
- [x] 2026-06-05: Milestone 2 implementation drafted: added
  `canonical_action_name.rs`, removed duplicate mangle validation, changed
  identifier-sensitive mangle APIs to checked/newtype entry points, and updated
  schema, collision, macro, unit, behavioural, and property tests.
- [x] 2026-06-05: Milestone 2 validation gates passed: `make fmt`,
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`, and
  `make test`. The test suite now runs 582 nextest tests plus doctests.
- [x] 2026-06-05: Milestone 2 CodeRabbit review is clear. The first review
  found a real Rustdoc example issue, which was fixed and revalidated. The
  rerun reported `src/app/outbound_response.rs`, which does not exist in this
  repository or worktree, so it was verified as not applicable.
- [x] 2026-06-05: Milestone 2 implementation committed as `dfeedc3`
  (`Share canonical action-name validation`).
- [x] 2026-06-05: Milestone 2 progress ledger committed as `ecae7eb`
  (`Record Milestone 2 completion`).
- [x] 2026-06-05: Milestone 3 implementation drafted: changed
  `TheoremDoc.actions` to `IndexMap<CanonicalActionName, ActionSignature>` and
  `ActionCall.action` to `CanonicalActionName`; moved action-name validation
  into raw-to-domain conversion; updated collision detection, referenced-action
  traversal, and macro action probes to consume canonical newtypes; and removed
  the now-duplicated schema action-name validator.
- [x] 2026-06-05: Milestone 3 tests drafted: added raw conversion unit tests
  for valid canonical action names, invalid `Actions` keys, and invalid
  `ActionCall.action` values; updated action-name BDD expectations to the new
  boundary diagnostic; and adjusted unit tests that previously constructed
  invalid domain action calls.
- [x] 2026-06-05: Milestone 3 focused validation passed with
  `cargo test -p theoremc-core` after formatting. The core suite now runs 269
  unit tests plus doctests.
- [x] 2026-06-05: Milestone 3 validation gates passed: `make fmt`,
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`, and
  `make test`. The test suite now runs 558 nextest tests plus doctests.
- [x] 2026-06-05: Milestone 3 CodeRabbit review completed on rerun with zero
  findings. The first invocation stalled during sandbox preparation and was
  terminated after more than five minutes; the retry reached analysis and
  completed successfully.
- [x] 2026-06-05: Milestone 3 implementation committed as `f19a24d`
  (`Carry canonical action names through schema domain`).
- [x] 2026-06-05: Milestone 3 progress ledger committed as `8db1de2`
  (`Record Milestone 3 completion`) and pushed to
  `origin/code-base-audit-2026-06-05`.
- [x] 2026-06-05: Milestone 4 implementation drafted: `schema::validate`
  now returns typed `ValidationError` values containing `ValidationPath` and
  `ValidationKind`, the loader renders the public `SchemaError` at the
  boundary, and `RawTheoremDoc` maps typed paths to source spans without
  parsing reason strings.
- [x] 2026-06-05: Milestone 4 tests drafted: added `rstest` path-to-location
  coverage for `About`, `Prove`, `Assume`, `Witness`, and Kani fields, and
  confirmed the focused `theoremc-core` suite passes with 276 unit tests plus
  doctests.
- [x] 2026-06-05: Milestone 4 deterministic gates passed: `make fmt`,
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`, and
  `make test`. The full suite runs 568 nextest tests plus doctests.
- [x] 2026-10-23: Read-only review verification confirmed that the two critical
  `source`-field findings are incorrect: `SchemaError::ValidationFailed` has
  only `theorem`, `reason`, and `diagnostic`. No compatibility field was added.
- [x] 2026-10-24: Applied the valid M4 review repairs: simplified typed source
  location dispatch, removed unused validation-helper state, made BDD helper
  errors return through `Result`, narrowed shared-test dead-code expectations,
  shared the canonical-name generators, extracted parser diagnostics, and
  reduced the loader and validator modules below 400 lines. The first Clippy
  run exposed forbidden `allow` attributes and assertions in result-returning
  BDD steps; both were corrected. The rerun passed `make check-fmt`,
  `make markdownlint`, `make nixie`, `make lint`, and `make typecheck`. The
  `make test` run was refused at compiler launch with
  `build-limits: the supervisor refused: launch-failed` while compiling
  `serde_json`; no test diagnostic was emitted. A subsequent supervisor status
  query timed out. A later test run started after healthy admission and
  compiled for 4,526 seconds before failing on an unused `proptest::Strategy`
  import in `canonical_action_name.rs`. The import was removed; rerun the
  candidate gates before M4 acceptance.
- [x] 2026-10-23: Moved `raw.rs` unit tests to `raw_tests.rs` during M4 review
  repair so the edited raw adapter also remains below the repository's 400-line
  limit. The remaining M5 work still includes raw serde mirrors and conversions.
- [x] 2026-11-01: Rebased the branch from `8f48abb` onto `origin/main` at
  `2bb0251` (PR #52 target). `main` had already landed equivalent typed schema
  validation diagnostics, so the Milestone 4 conflicts were resolved in favour
  of `main`'s `ValidationFailure` / `ValidationReasonKind` /
  `IndexedValidationField` design and its split `schema::loader_message` and
  `schema::loader_decode_location` modules. The branch's near-duplicate
  `ValidationError` / `ValidationPath` implementation and its superseded
  `loader/diagnostics.rs`, `raw_tests.rs`, and `validation_error.rs` modules
  were discarded. Branch-only repairs were retained: shared canonical-name
  property strategies, canonical construction inside `find_mangled_collisions`,
  `Result`-returning BDD steps, and the design and developers' guide updates.
- [x] 2026-11-01: Candidate-bound gates passed on the rebased candidate
  (`63e2c9d` plus the retained repairs) with all evidence captured under
  `/tmp/*-theoremc-code-base-audit-2026-06-05.out`: `make check-fmt` (53 files
  unchanged), `make markdownlint` (0 errors), `make nixie` (all diagrams valid),
  `make lint` (Whitaker then Clippy, exit 0), `make typecheck` (exit 0),
  `make test-workflow-contracts` (exit 0), and `make test`, which reported
  `817 tests run: 817 passed (16 slow), 0 skipped` after 4971.811s plus passing
  doctests. Compile admission stayed healthy throughout; the long runtime
  reflects a shared compile queue held deep by other agents' builds, not a
  fault. Two earlier `make test` attempts were invalidated by external SIGTERM,
  not assertion failures, and were discarded rather than counted.
- [x] 2026-11-01: Split `schema::raw` during the M4 repair so the module
  satisfies `module-max-lines`: raw-to-domain conversion moved to
  `schema/raw_convert.rs` and the canonical action-name tests to
  `schema/raw_action_name_tests.rs`, leaving `raw.rs` at 362 lines. The new
  tests cover a valid document, an invalid `Actions` key, and an invalid
  `ActionCall.action` value, asserting the typed reason text at each boundary.
- [x] 2026-11-01: Local `coderabbit review --agent` on candidate `ed4e135`
  reported seven raw findings (five distinct; two exact duplicate pairs) and no
  non-existent paths. All five were verified against source and repaired in
  `33d5722`: a gratuitous map destructuring in the collision property test, a
  self-contradictory sentence in this plan's repository orientation, a
  duplicated `## Project documentation` section in `AGENTS.md`, the
  `initialization` spelling in three documents, and
  `RawActionCallDecodeError::param()` returning an empty breadcrumb so an
  invalid action name fell back to the theorem header. The action-name fix
  carries the new `invalid_action_names_locate_the_action_key` regression test,
  which reported line 2 before the fix and line 7 after it.
- [x] 2026-11-01: Post-repair gates passed on the repaired candidate:
      `make fmt`, `make check-fmt`, `make markdownlint`, `make nixie`,
      `make lint`,
      `make typecheck`, and `make test` (`818 tests run: 818 passed, 0 skipped`
      plus passing doctests).
- [x] 2026-11-01: Milestone 5 implementation drafted. `schema/types.rs` no
  longer derives or names any serde trait: `Assumption`, `Assertion`,
  `WitnessCheck`, `ActionSignature`, `Evidence`, `KaniEvidence`, and
  `KaniExpectation` are plain data, and the module doc states the ADR-003
  reason. The YAML-visible half moved to the new `schema/raw_evidence.rs`
  (`RawEvidence`, `RawKaniEvidence`, `RawKaniExpectation`, and
  `RawActionSignature`) with explicit `to_evidence`, `to_kani_evidence`,
  `to_expectation`, and `to_action_signature` conversions, which also keeps
  `raw.rs` clear of evidence concerns. Both identifier newtypes now share
  `impl_identifier_deserialize!` instead of duplicated `Deserialize` impls, and
  `newtypes_tests.rs` pins the accepted and rejected forms.
- [x] 2026-11-01: Milestone 5 Rustdoc examples added for `TheoremName::new` and
  `ForallVar::new` (valid construction plus a rejected input) and for
  `SourceId::new` (infallible construction and `as_str`). Focused validation on
  the working candidate:
  `cargo check -p theoremc-core --all-targets --all-features` exited 0 with no
  diagnostics, the 209 filtered `schema` unit tests passed, and all 27 doctests
  passed with one ignored.
- [x] 2026-11-01: Documented the raw-mirror convention in
  `docs/developers-guide.md` so future YAML-visible fields extend the raw
  adapter instead of re-deriving serde on public domain types, with the
  identifier newtypes named as the deliberate exception.
- [x] 2026-11-01: Repaired the Milestone 4 review's diagnostic-location defect.
  `RawLetBinding` and `RawStep` used `#[serde(untagged)]`, which routes the
  mapping through `serde`'s `ContentDeserializer` and discards `Spanned`
  coordinates, so an invalid action name located at the theorem header instead
  of the value. Both types now dispatch their single `call`, `must`, or `maybe`
  key with a handwritten `Deserialize`/`MapAccess` visitor, and the loader
  consults `RawDocDecodeError::typed_location` before any textual fallback.
  `loader_diagnostic_tests::invalid_action_names_locate_the_action_value` pins
  the exact line and column for TitleCase, lowercase-alias, nested `maybe.do`,
  and flow-mapping forms (5:15, 6:15, 8:21, and 4:20 respectively).
- [x] 2026-11-01: Repaired the Milestone 4 review's broken public API example.
  `docs/users-guide.md` now imports `CanonicalActionName` and
  `mangle_action_name` from the public `theoremc::mangle` facade rather than an
  internal module path, and adds a `try_mangle_action_name` example plus the
  public building blocks (`action_slug`, `try_action_slug`, `hash12`).
- [x] 2026-11-01: Milestone 6 implementation complete. Omitted `Schema` stays
  `None` in `TheoremDoc` and is not materialized as version `1`; section 3.1 of
  `docs/theorem-file-specification.md` and the user's guide field notes now say
  that omission selects the version 1 compatibility profile when interpreted, so
  `None` and `Some(1)` agree semantically while remaining distinguishable.
  `schema_deser::schema_field_preserves_omitted_and_explicit_values` keeps both
  accepted forms, and
  `omitted_schema_only_differs_from_explicit_version_one_in_the_stored_value`
  compares the whole documents with `pretty_assertions` after clearing the
  explicit value.
- [x] 2026-11-01: Milestone 7 implementation complete. `lib.rs` is 155 lines and
  keeps only the `theorem_file!` entry point, `query.rs` returns a typed
  `ExpansionPlan` from loading and semantic checks, `render.rs` renders tokens
  with no I/O, and `error.rs` owns `MacroExpansionError` and its
  `compile_error!` rendering. Token output is unchanged: the `insta` expansion
  snapshot still matches. New seam tests cover `plan_expansion`,
  referenced-type and action-type parse failures, missing Kani evidence, empty
  probe lists, path anchoring, and absent referenced types. Focused validation:
  `cargo test -p theoremc-macros --lib` ran 48 tests with 0 failures.
- [x] 2026-11-01: Post-change review removed redundant canonical-name
  revalidation from `collision`: the mangler now takes `&CanonicalActionName`
  and returns `String`, so the unreachable `SchemaError::InvalidActionName`
  branch and the `Result` plumbing disappear while validation stays in the
  decoder and test helpers.
- [x] 2026-11-01: Candidate `74fc0d5` passed all seven deterministic gates
  sequentially: `make check-fmt`, `make markdownlint`, `make nixie`,
  `make lint` (Whitaker, rustdoc, and Clippy with `-D warnings`),
  `make typecheck`, `make test` (840 of 840 nextest tests plus doctests), and
  `make test-workflow-contracts`. Logs are under
  `/tmp/<gate>-theoremc-code-base-audit-2026-06-05.out`. The first attempt
  stopped at `make lint` because newly installed Whitaker rules
  (`string_continuation_style` and `clippy::shadow_reuse`) flagged pre-existing
  string continuations and one new shadowed binding; both were fixed before the
  green run.
- [x] 2026-11-01: Local `coderabbit review --agent` on `74fc0d5` completed in
  388 seconds with six findings and no rate limit. All were valid and are fixed:
  `schema::raw_evidence` and `schema::raw::raw_convert` were added to the
  design table, `CanonicalActionNameInvalidReason` became `#[non_exhaustive]`,
  five oxendict `-ize` corrections were applied, and the unbalanced parenthesis
  in `AGENTS.md` was closed.
- [x] 2026-11-01: Milestone 8 review sweep. The three smaller conformance gaps
  from the Milestone 4 assessment are closed by `fixture_source_id`, `729a143`,
  and `assert_malformed_action_diagnostic`. Issues #49 and #51 were completed
  while this plan was paused; #48 and #50 remain open follow-ups. The user's
  guide now states that `ActionCall.action` and `TheoremDoc::actions` carry
  `CanonicalActionName` values.
- [x] Milestone 4 implementation is complete, validated, reviewed, and
  committed. The rebased candidate carries `main`'s typed diagnostics plus the
  retained branch repairs, all deterministic gates pass, and the local review
  findings are cleared.
- [x] Milestone 5 implementation is complete, validated, reviewed, and
  committed as `e413a0f` (with the rustfmt fixup in `9a5602c`).
- [x] Milestone 6 implementation is complete, validated, reviewed, and
  committed as `d1cd4b6`.
- [x] Milestone 7 implementation is complete, validated, reviewed, and
  committed as `a12e342`.
- [x] 2026-11-08: First gate pass on the rebased candidate (`2ce2660`).
  `make check-fmt`, `make markdownlint`, and `make nixie` passed, and
  `make lint` rejected four sites that the newer Clippy and Whitaker rules now
  flag: `needless_raw_string_hashes` in `raw_location_tests.rs`, `shadow_reuse`
  in `collision.rs` and `raw_action_name_tests.rs`, and `panic_in_result_fn`,
  `assert_is_empty`, and `err_expect` in `query_tests.rs`. All were repaired in
  `55bb25d` without changing what any assertion or error branch checks.
- [x] 2026-11-08: Second and third gate passes on the repaired candidate.
  `make check-fmt`, `make markdownlint`, `make nixie`, `make lint` (rustdoc,
  Clippy, Whitaker), `make typecheck`, and `make test-workflow-contracts`
  passed, with logs under `/tmp/<gate>-theoremc-code-base-audit-2026-06-05.out`.
  `make test` did not finish inside nextest's configured 30-minute
  `global-timeout` on any of three attempts, although no assertion failed:
  attempt 1 ran 979 of 986 tests with 978 passing, attempt 2 ran 985 with 984
  passing, and attempt 3 ran all 986 with 985 passing before the last test was
  cancelled at 549.7 s. That test, `theoremc-macros::expand::compile_errors`,
  also exceeded its own 600 s allowance when run alone under nextest, then
  passed in 1093.58 s under
  `cargo test -p theoremc-macros --test expand --all-features`. Every test in
  the suite passed when run, but the gate itself did not complete on this host
  and is recorded as incomplete rather than a pass. The machine's shared
  compile pool (capacity 4, all held, with dozens of queued requests from other
  agents) makes the trybuild fixture builds roughly twice as slow as the
  profile's measured allowance. `cargo test --workspace --all-features --doc`
  passed with 36 doctests and 6 ignored.
- [x] 2026-11-08: Local
      `coderabbit review --agent --committed --base-commit 61b738d` on candidate
      `55bb25d` completed with four documentation findings
      and no code findings: missing examples on `try_action_slug` and
      `try_mangle_action_name`, stale `schema/raw.rs` paths and an outdated
      `decode.rs` description in the developer's guide, a module table that did
      not list the new `query.rs`, `render.rs`, and `error.rs` split, and two
      `docs/contents.md` bullets over 80 columns. All four were verified
      against source and fixed in `abcf704`; `make check-fmt`,
      `make markdownlint`, `make nixie`, `lint-rustdoc`, and the
      `theoremc-core` doctests were rerun green afterwards.
- [x] Milestone 8 final review is complete.
- [x] 2026-11-08: Rebased the branch from `41da59d` onto `origin/main` at
  `61b738d` (PR #52 target), replaying all 26 commits onto the new base. The
  replay used
  `--merge --no-fork-point --no-update-refs --no-autostash
  --reapply-cherry-picks --keep-empty --empty=stop`
  with `zdiff3` and a command-scoped `/dev/null` attributes file, because
  Weave was selected only by ambient global attributes and the repository does
  not opt in through tracked attributes. Every earlier commit identifier in
  this ledger was rewritten by the replay and has been updated to the current
  series; the rebased head is `4f65f8f`.
- [x] 2026-11-08: Conflict resolutions during the replay. `AGENTS.md` kept
  `main`'s generated spelling block and dropped the duplicate
  `## Project documentation` section, which `## Documentation maintenance`
  already covers. `docs/contents.md` kept the branch's restructured index
  (ADR-009 to ADR-012, the OrthoConfig reference material, and the issue-200
  ExecPlan) while adopting `main`'s newer `cargo theorem` user's-guide wording.
  The design and developer guides merged `main`'s split-module prose with the
  branch's added `Domain` and raw-adapter rows. `mangle_tests.rs` was adapted so
  `main`'s `parse_agrees_with_new_and_try_from` compiles against the branch's
  newtype helper, and `action_mangle_bdd.rs` kept the branch's
  `Result`-returning steps. The branch's deletion of `mangle_validate.rs` was
  preserved because `main`'s only change there was cosmetic trailing-comma
  removal.
- [x] 2026-11-08: Re-homed the branch's Milestone 5 work onto `main`'s module
  split. `main` had moved `schema/raw.rs` to `schema/raw/mod.rs` plus
  `schema/raw/decode.rs`, so the decode error enum and its typed-location
  accessors stayed in `decode.rs`, the raw-to-domain conversion helpers moved to
  `schema/raw/raw_convert.rs`, and the canonical action-name tests moved to
  `schema/raw_action_name_tests.rs`, declared from `raw/mod.rs` with
  `#[path = "../raw_action_name_tests.rs"]`. A duplicate pre-split
  `schema/raw_convert.rs` auto-applied by Git was verified identical and
  removed.
- [x] 2026-11-08: Audited the replay with
      `git range-diff 2bb0251..41da59d 61b738d..4f65f8f` and recorded the full
      output in
      `/tmp/rangediff-code-base-audit.out`. Twelve commits differ textually;
      every difference is explained by the conflict decisions above, the
      raw-module path remapping, or the action-mangle test adaptation.
      `git diff --check 61b738d HEAD` is clean, and the target-versus-candidate
      delta contains no deletions of `main`'s ADR-009 to ADR-012 or OrthoConfig
      material.
- [x] 2026-11-09: Hosted CodeScene review of `cdffc9f` failed two gates. "New
      code is healthy" reported `String Heavy Function Arguments` in
      `canonical_action_name.rs` and `query.rs`; "Enforce advisory code health
      rules" reported `Code Duplication` in `raw_action.rs` (10.00 to 9.39). A
      local `cs delta` also flagged `step.rs` for the same string-argument
      rule. All four were repaired by refactoring in `88a1c9c`, never by
      suppression: typed `Candidate` and `Segment` validation structs, a typed
      `&CanonicalActionName` parameter for `parse_action_type`, a shared
      `single_key_deserialize!` template in the new
      `schema/raw_single_key.rs` module, and a `StepPath` breadcrumb type with
      canonical-name test builders in `schema/step.rs`. Local `cs delta` now
      reports `9.68 -> 10.00` for every one of the four files and lists no
      remaining issue.
- [x] 2026-11-09: Gate run on `88a1c9c`. `make check-fmt`, `make markdownlint`
      and `make nixie` passed, and `make lint` (rustdoc, Clippy, Whitaker)
      passed in 10 minutes 25 seconds. `make test` did not pass: it ended at
      nextest's 30-minute `global-timeout` and exited with
      `error: test run failed`. 974 of 986 tests ran, 973 passed, and
      `theorem_file_macro_bdd::a_moved_actions_type_fails_ordinary_compilation`
      received `SIGTERM` while still running; no assertion failed. The 12
      tests the run did not reach were identified by diffing
      `cargo nextest list --workspace --all-targets --all-features` against the
      gate log and then run under `cargo test`, which has no global cap. All
      13 `theorem_file_macro_bdd` tests passed (12 in the first rerun job, the
      remaining `a_valid_theorem_file_exposes_a_kani_proof_harness` in
      190.01 s in the second), as did
      `theoremc-macros::expand::compile_errors` in 998.15 s. The gate itself
      is recorded as incomplete, not as a pass. Logs are under
      `/tmp/missing-tests*-theoremc-code-base-audit-2026-06-05.out`. The
      candidate afterwards changed only text: `721451c` corrected spelling in
      two doc comments and in this plan, and `0a2cbc4` corrected the plan's
      test-gate wording. `make check-fmt`, `make markdownlint`, `make nixie`,
      and `make spelling` were re-run on each later head. `make lint` was
      re-run on `905dea4` and passed in 21 minutes 55 seconds, and hosted
      `CI / build-test` run 42558639415 passed every step on that head,
      including `Test`, `Lint`, and `Typecheck`.
- [x] 2026-11-09: Disposition for the remaining local CodeScene figure.
      `mangle_tests.rs` shows an unchanged 9.68 in a `cs delta` that includes
      it; it is a pre-existing test-file string-argument ratio, the hosted
      review does not report it, and `88a1c9c` does not touch it.

## Surprises & Discoveries

- 2026-06-05: GitHub's branch rename API successfully renamed the remote
  branch, but the older draft pull request #47 still reports the old head ref
  through `gh pr view`. This plan branch will therefore get a fresh draft pull
  request after Milestone 0.
- 2026-06-05: The first `coderabbit review --agent` invocation stayed at
  sandbox preparation for more than five minutes. A bounded retry progressed
  through analysis and review, exited 0, and emitted no finding payload.
- 2026-11-01: `main` had already landed the substance of Milestone 4: typed
  `ValidationFailure` values with `ValidationReasonKind`, 0-based indexed
  section fields, and a loader that renders the public diagnostic at its
  boundary. Replaying the branch's own `ValidationError` / `ValidationPath`
  design would have restored a parallel, older implementation, so the rebase
  adopted `main`'s version and kept only changes `main` lacked.
- 2026-06-05: Integration tests include `tests/common/mod.rs` independently in
  each test crate, so shared helper functions are unused in some crates even
  though they are used by the suite as a whole. The allowance is scoped to each
  helper function, with the integration-crate ownership reason recorded beside
  it.
- 2026-06-05: CodeRabbit caught that the helper regression tests should use
  Result-returning `googletest` assertions instead of panicking or hiding
  failures inside broad helper tests. The new `tests/common_helpers.rs` cases
  now keep each helper behaviour independently attributable.
- 2026-06-05: Moving collision detection onto checked mangle APIs exposed
  existing property tests that generated Rust keyword segments such as `a.fn`
  and `if.a`. Those were invalid canonical names, so the properties now
  explicitly assume valid generated names before checking collision invariants.
- 2026-06-05: Once `ActionCall.action` became a `CanonicalActionName`, the
  remaining step-validation tests that expected blank or malformed action
  values were no longer representable as domain values. Those cases now belong
  at the raw conversion and loader boundary instead of `schema::step`.
- 2026-06-05: `serde_saphyr` can report line or column zero for some empty or
  blank scalar spans. Schema diagnostics promise one-indexed coordinates, so
  `schema::diagnostic` now normalizes zero coordinates to `1` at the diagnostic
  boundary.
- 2026-06-05: Typed validation paths make the previous reason-string parser in
  `RawTheoremDoc` unnecessary. Some semantic failures, such as missing action
  signatures or empty aggregate sections, still intentionally point at the
  theorem name because the raw document does not carry a narrower source span
  for an absent value.
- 2026-06-05: Three Milestone 4 `coderabbit review --agent` invocations
  remained in sandbox preparation for more than five minutes and had to be
  stopped. This appears to be the same CodeRabbit service stall seen earlier on
  this branch, not a deterministic project failure; all local gates passed
  before review was requested. The next retry will use a longer backoff before
  asking the service again.
- 2026-06-05: The post-backoff Milestone 4 CodeRabbit retry also remained in
  sandbox preparation for more than five minutes and had to be stopped. At this
  point the review service later recovered and produced a completed local
  review; the findings are recorded in the 2026-10-23 review disposition.
- 2026-10-23: The local M4 review returned 12 rows. Nine underlying concerns
  were valid: redundant validation-helper state, source-location dispatch
  complexity, a panicking BDD helper, broad test-helper suppression, duplicated
  property strategies, stale progress text, missing `pipefail`, oversized
  loader diagnostics, and a historical return-type mismatch. Two critical rows
  incorrectly claimed `SchemaError::ValidationFailed` has a `source` field;
  source inspection confirms the variant has only `theorem`, `reason`, and
  `diagnostic`, so no API change is warranted. Duplicate loader-size rows share
  one repair.
- 2026-11-01: `serde_saphyr::Spanned` loses its coordinates whenever `serde`
  buffers the value first. `#[serde(untagged)]`, `#[serde(flatten)]`, and
  `#[serde(tag = "...")]` all deserialize through `ContentDeserializer`, so a
  span that the raw type declares still arrives as `Location::UNKNOWN`. A
  single-key enum therefore has to dispatch `MapAccess` by hand: that keeps the
  span attached and lets diagnostics point at the value instead of the map key.
- 2026-11-01: The typed-location regression tests first reported 999/998 as
  expected values, which only proved that assertions without observed values
  encode guesses. The final cases pin the coordinates the implementation
  actually produces, discovered by asserting impossible bounds first and then
  replacing the probe with the real numbers.
- 2026-11-01: `make lint` is not stable across Whitaker installations. The
  installed suite gained `string_continuation_style` and `clippy::shadow_reuse`
  between runs, so code that passed the Milestone 4 gates failed here without
  being changed. Treat a lint failure in untouched code as a toolchain change
  first: inspect the rule, and fix the code rather than pinning an older lint
  set.

## Decision Log

- 2026-06-05: Use a neutral canonical action-name module instead of making the
  schema layer depend on mangle. This preserves ADR-003's layering intent while
  eliminating duplicated grammar code.
- 2026-06-05: The initial plan was drafted before implementation. The user
  explicitly authorized implementation later, so the former approval gate is
  discharged and the plan proceeds through its milestones.
- 2026-06-05: Track placeholder binary behaviour, build-script path sharing,
  oversized schema modules, and macro conflict provenance as separate GitHub
  issues because they are related audit findings but not part of the direct
  concern list to implement here.
- 2026-06-05: Keep `tests/common` as the owner of integration fixture helpers
  for this milestone rather than introducing a new integration test support
  crate. This matches the audit request and avoids changing the test crate
  topology before the schema and macro refactors.
- 2026-06-05: Use `googletest::Result` and `verify_that!` for new integration
  helper regression tests. This satisfies the requested assertion style while
  avoiding `panic_in_result_fn` and `expect_used` lint violations.
- 2026-06-05: Keep schema-facing error wording stable by mapping typed
  `CanonicalActionNameInvalidReason` values back into existing
  `SchemaError::InvalidActionName` message text for Milestone 2. Milestone 4
  will replace validation diagnostics with typed paths and kinds.
- 2026-06-05: Let action-signature map lookup accept string views by
  implementing `Borrow<str>` for `CanonicalActionName`. This preserves
  ergonomic read-only lookups such as `doc.actions.get("account.deposit")`
  without allowing strings to be stored in public domain maps.
- 2026-06-05: Delete `validate_action_call` instead of keeping it as a
  structural no-op. Clippy correctly flagged the no-op `Result` as misleading;
  `validate_step_list` still owns recursive `maybe` shape validation, while raw
  conversion owns action-name validation.
- 2026-06-05: Keep public validation wording stable while replacing diagnostic
  location lookup with typed paths. `ValidationError::reason()` renders the
  existing user-facing strings, `ValidationPath` is the machine contract for
  location lookup, and `RawTheoremDoc::location_for_validation_path()` owns the
  source-span mapping.
- 2026-10-23: Keep parser-message anchoring private to
  `schema::loader::diagnostics`; it exists only to recover a source location
  when `serde_saphyr` reports an unknown key at 1:1. Typed semantic validation
  continues to use `ValidationPath`, and no cross-module diagnostics helper is
  introduced.
- 2026-10-23: Reuse canonical action-name property generators only from the
  `theoremc-core` `#[cfg(test)]` support module. This keeps grammar samples in
  sync while preventing test strategy helpers from entering the crate API.
- 2026-10-23: Extract the raw adapter's existing unit tests in M4 rather than
  waiting for M5. This keeps the M4-edited `raw.rs` under the enforced
  file-size limit; M5 remains responsible for the planned serde boundary
  changes.
- 2026-11-01: Accept the action-name diagnostic wording change produced by the
  shared canonical-name boundary. Blank and single-segment names are now
  rejected by the canonical grammar and reported as
  `invalid canonical action
  name '<name>': must contain at least two dot-separated segments`,
  replacing the `action must be non-empty after trimming` message that `main`'s
  `schema::step` still emits. One grammar owns the rule, the message names the
  offending value, and `docs/users-guide.md` and the BDD cases were updated to
  match.
- 2026-11-01: Supersede the M4 `ValidationError` / `ValidationPath` design with
  `main`'s landed `ValidationFailure` / `ValidationReasonKind` model. The two
  designs have the same intent; `main`'s is 0-based, distinguishes repeated
  section value and `because` fields with `IndexedValidationField`, and is
  already exercised by the shipped loader. User-facing diagnostic wording is
  unchanged.
- 2026-11-08: `main`'s `.config/nextest.toml` bounds the whole run at 30
  minutes and each compiler-heavy test at 600 seconds, sized from a measured
  cold run of 818 tests. On this host the same suites take roughly twice that
  while other agents hold the shared compile pool, so `make test` can end at
  the global timeout with every executed test passing. The timeout is a
  wall-clock bound, not an assertion, and the remaining test passes alone under
  `cargo test`, which has no such cap. Re-measure the profile before treating a
  local `global-timeout` cancellation as a test failure.
- 2026-11-09: Record the local `make test` `global-timeout` as an incomplete
  gate and request an explicit exception, rather than describing it as a pass.
  The target exits non-zero (`error: test run failed`) once the profile's
  `global-timeout` fires, even though no assertion fails, so calling it a pass
  overstates the evidence. The supporting per-test reruns show what the
  unfinished run would have covered; they do not turn the gate into a pass.
- 2026-11-01: Keep `TheoremDoc::schema` as `Option<u32>` and treat omission as
  an interpretation-time selection of the version 1 profile. Materializing a
  default of `1` would erase the distinction the loader already preserves, and
  no product requirement asks for it; the specification and user's guide were
  aligned with the implementation instead.

## Outcomes & Retrospective

Milestones 1–7 are committed and their gates and local reviews passed. The
branch was rebased onto `main` at `2bb0251`, where `main`'s landed typed
validation diagnostics supersede the branch's original Milestone 4
implementation; the rebased candidate carries `main`'s `ValidationFailure` /
`ValidationReasonKind` design plus the branch repairs `main` lacked, and it
passed candidate-bound gates (818 nextest tests plus doctests) and local review
with all findings cleared. A second rebase onto `61b738d` replayed the whole
26-commit series onto the newer target; the candidate head is `4f65f8f`.

Milestone 8 is complete on that candidate. The rebase surfaced four lint
findings from the newer toolchain, which were repaired without behaviour
change, and the local CodeRabbit review found only documentation gaps, all
fixed. Every deterministic gate except `make test` passes: formatting, Markdown
lint, Mermaid validation, rustdoc, Clippy, Whitaker, type checking, doctests,
and the workflow contracts. The nextest run passes every test it completes, but
on this contended host it does not complete before the profile's global
timeout; the tests it does not reach pass under `cargo test`, which has no such
cap. The gate is therefore recorded as incomplete rather than a pass.

The audit introduced no Kani, Verus, CrossHair, or LemmaScript proofs. Its
proof-relevant change is the diagnostic-location repair, which is covered by
`loader_diagnostic_tests::invalid_action_names_locate_the_action_value` rather
than by a verifier, and by the property test that canonical names round-trip
through the mangler.

Hosted delivery added one more round of work. The CodeScene review of the
pushed candidate failed two gates on four modules; all four findings were
repaired by refactoring rather than suppression in `88a1c9c`, and local
`cs delta` now reports every changed file at 10.00. The repaired candidate
passes every deterministic gate except `make test`, which does not pass on this
host: nextest's 30-minute `global-timeout`, sized for a less contended host,
stops the run and the target exits non-zero with `error: test run failed`. 974
of 986 tests ran, 973 passed, and one was cancelled by `SIGTERM`; no assertion
failed. The 12 tests the run did not reach, together with the cancelled test,
all pass under `cargo test`, which has no global cap. That evidence shows what
the unfinished run would have covered, but it does not make the gate a pass, so
`make test` is recorded as incomplete and an explicit exception is requested
for it. The hosted candidate supplies the complementary evidence:
`CI / build-test` run 42558639415 passed every step on `905dea4`, including
`Test`, `Lint`, `Typecheck`, `Format`, `Markdown lint`, and `Spelling`, and the
CodeScene and Gecko Security reviews pass on the same head. The remaining work
is delivery: the two hosted assessments, readiness, and merge of the pull
request.

The 2026-11-09 CodeRabbit review raised eight pre-merge checks, all answered
with changes in `9c9dff9`. Its follow-up assessment kept two documentation rows
open and asked for the CodeScene failure to be cleared.
`tests/schema_deser_reject.rs` was rewritten around a `DispatchCase` enum and a
`Position` enum in `34ec8b3`, removing the string-carrying helpers and
collapsing the duplicated accept tests; local `cs delta origin/main` reports
the file at 10.00. `441262c` narrowed the `schema/step.rs` header to the
constraints it actually enforces and scoped the user-facing deserialization
boundary to the document and step types, listing `TheoremValue` beside
`TheoremName` and `ForallVar` as the deliberate `Deserialize` exceptions. Every
deterministic gate except the local `make test` run passes on that candidate:
the local run ended at nextest's 30-minute timeout, as recorded above, and the
explicit exception for that gate is requested separately. Hosted
`CI / build-test` passed on attempt 6 after earlier attempts met GitHub
release-download 504 responses.

CodeRabbit's next assessment found one remaining test gap: the valid `must`
controls asserted only that the decoded value was not a `call`, so they also
passed when the binding or step was absent, and the `Do` case also passed for
`Step::Maybe`. `d93024d` resolves both positions to one `Decoded` variant and
asserts it positively, so an absent `Let` binding or `Do` step fails the test.
Clippy passes on the affected target, the target's 33 tests pass, local
`cs delta origin/main` reports the file at 10.00, and hosted `CI / build-test`
passes on `d93024d`.
