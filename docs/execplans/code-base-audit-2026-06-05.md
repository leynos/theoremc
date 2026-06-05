# Address code base audit findings from 2026-06-05

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: IN PROGRESS

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

The initial deliverable is this plan. Implementation must not begin until the
plan is explicitly approved.

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

## Tolerances

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
  placeholder `theoremc` binary contract.
- [#49](https://github.com/leynos/theoremc/issues/49): replace build-script
  `#[path]` sharing with a stable build support boundary.
- [#50](https://github.com/leynos/theoremc/issues/50): split oversized schema
  modules to preserve the 400-line file limit.
- [#51](https://github.com/leynos/theoremc/issues/51): include provenance in
  macro action-signature conflict diagnostics.

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

The current code has two separate canonical action-name validators:
`crates/theoremc-core/src/schema/action_name.rs` returns `SchemaError`, while
`crates/theoremc-core/src/mangle_validate.rs` returns
`InvalidCanonicalActionName`. The implementation must replace this duplication
with one neutral grammar validator and typed wrappers for each caller.

## Milestone 0: plan, branch, and review setup

Create this ExecPlan at `docs/execplans/code-base-audit-2026-06-05.md`, rename
the branch to `code-base-audit-2026-06-05`, and create GitHub issues for audit
findings that are related but not part of this plan's direct implementation
scope.

Validation for this milestone is documentation-focused plus the standard
repository gates:

```sh
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
- [x] 2026-06-05: Milestone 1 implementation committed as `a4ec375`
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
- [x] 2026-06-05: Milestone 2 implementation committed as `eb651e3`
  (`Share canonical action-name validation`).
- [x] 2026-06-05: Milestone 2 progress ledger committed as `70bf0c9`
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
- [ ] 2026-06-05: Milestone 3 implementation is committed.
- [ ] Milestone 4 implementation is complete, validated, reviewed, and
  committed.
- [ ] Milestone 5 implementation is complete, validated, reviewed, and
  committed.
- [ ] Milestone 6 implementation is complete, validated, reviewed, and
  committed.
- [ ] Milestone 7 implementation is complete, validated, reviewed, and
  committed.
- [ ] Milestone 8 final review is complete.

## Surprises & Discoveries

- 2026-06-05: GitHub's branch rename API successfully renamed the remote
  branch, but the older draft pull request #47 still reports the old head ref
  through `gh pr view`. This plan branch will therefore get a fresh draft pull
  request after Milestone 0.
- 2026-06-05: The first `coderabbit review --agent` invocation stayed at
  sandbox preparation for more than five minutes. A bounded retry progressed
  through analysis and review, exited 0, and emitted no finding payload.
- 2026-06-05: Integration tests include `tests/common/mod.rs` independently in
  each test crate, so shared helper functions are unused in some crates even
  though they are used by the suite as a whole. The common module needs a scoped
  `dead_code` allowance with a reason unless the helper module is moved into a
  separate support crate.
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

## Decision Log

- 2026-06-05: Use a neutral canonical action-name module instead of making the
  schema layer depend on mangle. This preserves ADR-003's layering intent while
  eliminating duplicated grammar code.
- 2026-06-05: Treat this initial plan as a draft and wait for explicit user
  approval before implementation. The ExecPlan skill requires this approval
  gate for non-trivial changes.
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

## Outcomes & Retrospective

No implementation outcomes yet. This section must be updated after each
milestone and completed during Milestone 8.
