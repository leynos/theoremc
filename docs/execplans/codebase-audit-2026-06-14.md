# Resolve the 2026-06-14 codebase audit

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: ACTIVE

## Purpose / big picture

Resolve every finding from the 2026-06-14 codebase audit in a deliberate,
reviewable sequence. The audit found diagnostic coupling, repeated validation
and test scaffolding, documentation drift, dependency policy drift, and several
smaller ergonomics problems. The desired outcome is a repository where schema
diagnostics are typed before they are rendered, macro expansion avoids
avoidable repeated scans, tests share clear support code, public documentation
matches the implemented API, and dependency declarations obey the local policy.

Observable success means that every audit finding is either fixed, documented
as intentionally deferred with approval, or superseded by a better design
decision recorded in the appropriate documentation. After each major milestone,
`make check-fmt`, `make lint`, and `make test` pass, any applicable Markdown
gates pass, `coderabbit review --agent` reports no unresolved actionable
concerns, and the work is committed as a small logical unit.

## Constraints

- This plan was approved for implementation by the user on 2026-06-16.
- Address the findings in the order listed under `Milestones`. Do not skip a
  finding because it looks optional; the audit findings are requirements unless
  the user approves a deferral.
- Preserve the public `.theorem` file format unless a milestone explicitly
  fixes documented drift between the format and the implementation.
- Preserve existing action mangling, module naming, harness naming, source
  diagnostics, and build-script workflows unless a milestone explicitly targets
  that behaviour.
- Maintain Rust source files below 400 lines. If a milestone would leave a
  touched source file above that limit, split the file in the same milestone.
- Keep public APIs narrow. New helper APIs should expose stable domain concepts,
  not incidental traversal or test scaffolding details.
- Use `cap_std`, `cap_std::fs_utf8`, and `camino` for filesystem work.
- Use Red-Green-Refactor for behavioural changes: add or update a focused test
  that fails for the intended reason, make the smallest production change to
  pass it, then refactor with the focused test and wider gates passing.
- Use `rstest` for unit-test fixtures and parameterized cases, `rstest-bdd` for
  behavioural workflow tests, `googletest` and `pretty_assertions` for clear
  assertions, and `insta` where multivariant rendered output needs snapshot
  stability.
- Use `proptest` or Kani when a canonical implementation governs an invariant
  over a range of inputs, states, orderings, or transitions. Use Verus only for
  substantive lemmas or contractual business logic that require exhaustive
  proof rather than example or property testing.
- Record design decisions in `docs/theoremc-design.md` unless they are
  substantive enough for an Architecture Decision Record (ADR). Reference any
  new ADR from the design document.
- Update `docs/users-guide.md` for user-visible behaviour or API changes.
- Update relevant internal design material and `docs/developers-guide.md` for
  internal interfaces, conventions, and practices introduced by the work.
- Update `docs/contents.md` when adding, renaming, or removing documentation.
- Run gates sequentially and capture long output with `tee` into `/tmp` logs.
  Do not run formatting, linting, or test gates in parallel.
- Run `coderabbit review --agent` only after deterministic gates for that
  milestone have passed. Resolve all applicable concerns before continuing.
- Commit after each logical milestone that passes gates and review. Use
  file-based commit messages with `git commit -F`.

## Tolerances

- Approval: if this DRAFT plan has not been explicitly approved, do not begin
  implementation.
- Scope: if a milestone needs more than roughly 500 net Rust lines or more than
  eight production files, stop and split the milestone in the plan before
  continuing.
- Public API: if fixing a finding requires a breaking public API change, stop
  and present the compatibility options before implementing.
- Documentation behaviour drift: if code and documentation disagree and both
  interpretations are plausible, stop and record the choice in `Decision Log`
  before changing either side.
- Dependencies: if a milestone needs a new third-party dependency rather than
  tightening an existing version requirement, prototype the smallest option and
  stop for approval before committing it.
- Diagnostics: if typed diagnostic reasons cannot preserve the current
  structured diagnostic locations after two focused attempts, stop and document
  the failing cases.
- Tests: if a deterministic gate still fails after five focused fix attempts,
  stop with the relevant `/tmp` log path and summarize the remaining failure.
- CodeRabbit: if `coderabbit review --agent` is unavailable, record the tool
  failure in `Surprises & Discoveries` and continue only after all ordinary
  repository gates pass. If CodeRabbit reports actionable concerns, address
  them before moving to the next milestone.
- Property and proof tooling: if a proposed property, Kani harness, or Verus
  proof becomes larger than the implementation it protects, stop and reassess
  whether the invariant is being specified at the right level.

## Risks

- Risk: typed validation reasons may require touching `SchemaError`,
  diagnostic rendering, raw deserialization, and validation tests at once.
  Mitigation: introduce a typed internal payload first, keep rendered messages
  unchanged, and use existing diagnostic snapshot tests as regression guards.
- Risk: splitting validation modules can produce churn without behavioural
  value. Mitigation: split only along existing validation responsibilities and
  make each split pass tests before changing behaviour.
- Risk: test helper consolidation can hide scenario intent behind generic
  abstractions. Mitigation: keep shared helpers low level and let BDD files
  retain domain-specific `Given`, `When`, and `Then` wording.
- Risk: tightening dependency versions can refresh `Cargo.lock` and expose
  unrelated upstream changes. Mitigation: change manifest requirements without
  unnecessary updates, inspect any lockfile movement, and run the full gates.
- Risk: documentation drift fixes may reveal unresolved product decisions, such
  as whether omitted `Schema` means `1` or means unspecified. Mitigation: make
  one explicit decision, record it in the design document or an ADR, and update
  user-facing docs and tests together.
- Risk: replacing timestamp sleeps in behavioural tests can become
  platform-specific. Mitigation: prefer observable build outputs or supported
  file mtime APIs, and keep existing sleep behaviour until the replacement is
  proven reliable on Linux in CI-like conditions.

## Traceability

- Milestone 1 addresses roadmap Step 1.3 and Step 7.1 by replacing
  string-parsed validator diagnostics with structured reasons. It satisfies
  `DES-6`, `DES-6.5`, ADR 002 decision 2, and `TFS-1`.
- Milestone 2 addresses roadmap Step 1.2 and Step 6.2 by separating validator
  responsibilities along the schema-layer boundary. It satisfies `DES-3.2`,
  `DES-6`, ADR 003 decision 1, `TFS-1`, `TFS-4`, and `TFS-6`.
- Milestone 3 addresses roadmap Step 2.3 by keeping explicit argument value
  semantics maintainable as nested decode contexts grow. It satisfies `DES-5`,
  `TFS-5`, and ADR 001 decision 3.
- Milestone 4 addresses roadmap Step 3.3 and ADR 004 by preserving typed
  action probe behaviour while removing repeated signature scans. It satisfies
  `DES-7`, `DES-5`, `TFS-4`, and the theorem-side action-signature decision.
- Milestone 5 addresses roadmap Steps 1.3, 3.1, and 3.2 by making parser,
  build-discovery, suite-generation, and macro behavioural tests share reliable
  setup. It satisfies `DES-6`, `DES-7`, `TFS-1`, and `TFS-4`.
- Milestone 6 addresses roadmap Step 3.1 by making build-fixture dependency
  extraction deterministic and shared. It satisfies `DES-7` and the developer
  guide's test-support expectations.
- Milestone 7 addresses roadmap Steps 3.1 and 3.2 by centralizing path
  normalization and theorem-file path rejection policy. It satisfies `DES-7`,
  `NMR-1`, and `TFS-1`.
- Milestone 8 addresses roadmap Step 3.1 by removing wall-clock dependence from
  build-discovery behavioural tests. It satisfies `DES-7`.
- Milestone 9 addresses roadmap Steps 4.1 and 4.2 readiness by keeping argument
  lowering lint-clean before Kani step emission grows. It satisfies `DES-8`,
  `TFS-4`, and `TFS-5`.
- Milestone 10 addresses the documentation sources referenced by the roadmap's
  signpost section: `docs/contents.md`, `docs/theoremc-design.md`,
  `docs/theorem-file-specification.md`, `docs/users-guide.md`, and
  `docs/developers-guide.md`.
- Milestone 11 addresses dependency policy drift under roadmap Step 6.2 and
  ADR 003 decision 5. It makes version intent explicit for future
  dependency-policy checks without pretending Cargo's existing bare
  requirements are not already caret-compatible.
- Milestone 12 verifies that all audit work still satisfies the roadmap
  signposts above and the repository-wide quality gates.

## Milestones

### Milestone 1: establish the diagnostic reason model

Replace string-coupled validation location mapping with typed validation
reasons. Start by adding focused unit tests that demonstrate the current
problem: changing a rendered validation reason should not be able to change the
selected source location. The tests should cover blank `About`, indexed `Prove`,
`Assume`, and `Witness` failures, `because` field failures, Kani `unwind`, and
Kani vacuity reasons.

Introduce a typed internal reason, for example `ValidationReasonKind`, that
captures the field, index, and backend-specific reason separately from the
rendered message. Carry that reason through `SchemaError::ValidationFailed` or
an internal diagnostic payload. Keep public error text stable unless the
decision log records an approved wording change.

Update `crates/theoremc-core/src/schema/raw.rs`,
`crates/theoremc-core/src/schema/validate.rs`,
`crates/theoremc-core/src/schema/validation_reason.rs`, and associated tests.
Remove `crates/theoremc-core/src/schema/validate_reason_markers.rs` once the
typed path is complete. Use snapshot tests where rendered diagnostic variants
need stable formatting.

Validation for this milestone:

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-codebase-audit-2026-06-14.out
make lint 2>&1 | tee /tmp/lint-theoremc-codebase-audit-2026-06-14.out
make test 2>&1 | tee /tmp/test-theoremc-codebase-audit-2026-06-14.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-codebase-audit-2026-06-14.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-codebase-audit-2026-06-14.out
coderabbit review --agent 2>&1 | tee /tmp/coderabbit-theoremc-codebase-audit-2026-06-14.out
```

### Milestone 2: split and simplify schema validation

Split `crates/theoremc-core/src/schema/validate.rs` into coherent
responsibilities. Milestone 1 moved the existing unit tests into
`crates/theoremc-core/src/schema/validate_tests.rs`, which brought the file
below the 400-line limit, but this milestone still needs the production
responsibility split. The target split is: field presence and non-blank text,
expression parsing checks, action signature checks, step shape checks, and
evidence/Kani policy. Keep `validate_theorem_doc` as the orchestration
entrypoint.

Replace repeated non-empty and expression loops with small descriptor-driven
helpers. The helper API should make the field label, section name, index, and
source value explicit without returning rendered message strings as control
data.

Add or update `rstest` cases for the shared validators. Behavioural schema
tests should continue to cover the same fixture corpus through `rstest-bdd`.
Property tests are not required unless the refactor introduces a new generic
validator whose ordering or indexing invariant is easier to specify over
generated inputs.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 3: centralize argument decode error remapping

Remove repeated `ArgDecodeError` variant remapping from raw action conversion.
Add a method such as `ArgDecodeError::with_param_prefix` or a typed context
wrapper that applies nested parameter context in one place. Cover every current
variant with unit tests, including unhappy paths for invalid references and
literal wrapper type errors.

Update `crates/theoremc-core/src/schema/arg_value.rs`,
`crates/theoremc-core/src/schema/raw_action.rs`, and their tests. Ensure error
messages remain stable unless a message change is explicitly recorded.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 4: precompute macro action signatures

Replace the repeated scan in
`crates/theoremc-macros/src/lib.rs::action_signature_for` with a precomputed
canonical action signature index built once per expansion. Preserve conflict
detection for semantically different signatures and preserve deterministic
first-seen ordering for generated probes.

Add `rstest` unit cases for the index: one action in one document, one action
repeated across documents with equivalent signatures, conflicting signatures,
and missing signatures. Keep existing `trybuild`, `insta`, and BDD coverage for
generated action probes. Add a property test only if the index accepts
arbitrary action/signature collections directly and must preserve deduplicated
ordering across many generated inputs.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 5: consolidate fixture crate and schema BDD support

Extract repeated integration-test fixture crate scaffolding from
`tests/build_discovery_bdd.rs`, `tests/build_suite_bdd.rs`, and
`tests/theorem_file_macro_bdd/fixture_crate.rs` into shared test support under
`tests/common` or a clearly named sibling module. Keep high-level BDD step
functions readable and scenario-specific.

Also consolidate schema BDD loading and error assertion helpers from
`tests/schema_action_name_bdd.rs`, `tests/schema_diagnostics_bdd.rs`,
`tests/schema_vacuity_bdd.rs`, and `tests/schema_bdd.rs`. Shared helpers should
return typed data or clear `Result` values rather than panic during discovery.

Use `rstest-bdd` scenarios as the behavioural safety net. Add unit tests for
shared support only where the helper contains non-trivial parsing or path logic.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 6: remove duplicated manual TOML section parsing

Replace the duplicated `toml_section` line scanner in the build BDD suites with
one shared helper or a small TOML parser. Prefer a parser if the required
dependency already exists or can be justified. If adding `toml` would be a new
dependency, stop under the dependency tolerance and ask for approval before
committing.

Tests must cover section extraction with comments, neighbouring sections, and
missing sections. If the helper remains test-only, keep it in test support and
avoid exposing it from production crates.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 7: unify path normalization and path policy

Introduce one helper for path separator normalization and string escaping used
by macro expansion and fixture builders. Then isolate theorem file path
validation in `crates/theoremc-core/src/theorem_file.rs` behind a helper that
names the rejected path classes: relative accepted paths, root-anchored paths,
drive-prefixed paths, and parent traversal.

Add unit tests for each path class and behavioural tests for externally
observable theorem-file loading errors. Use property tests if the normalization
helper claims an invariant over arbitrary path-like strings, such as preserving
non-separator characters while normalizing separators.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 8: remove timing-dependent BDD sleep

Replace `pause_for_timestamp_tick` in `tests/build_discovery_bdd.rs` with a
deterministic trigger for build-script reruns. Prefer observable output or an
explicit file timestamp update over wall-clock sleeping. Keep the old approach
only if the replacement is less reliable, and record that decision.

The behavioural test must still prove that editing an existing `.theorem` file
reruns the build script and that ignored files do not participate in discovery.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 9: retire broad dead-code suppressions in argument lowering

Review each `#[allow(dead_code)]` in `src/arg_lowering.rs`. Remove suppressions
for active implementation paths, move genuinely phased work behind the feature
or test boundary that owns it, and keep any remaining lint suppression tightly
scoped with a reason.

Add tests only when a moved helper becomes newly reachable through a public or
internal interface. The primary acceptance criterion is that lint signal
remains strict without hiding active code.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 10: fix documentation and API drift

Resolve the missing `docs/repository-layout.md` finding by adding the file and
indexing it in `docs/contents.md`. Update the documentation style guide only if
the canonical filename changes, which is not expected.

Resolve `Schema` default drift by choosing one behaviour: either materialize
the default schema value `1` during loading or update the specification and
user guide to say the field remains unspecified when omitted. Because this is
user-visible file-format behaviour, record the decision in
`docs/theoremc-design.md` or an ADR if the choice has compatibility impact. Add
tests for omitted and explicit schema values.

Fix the stale `ActionCall.as_` example in `docs/theorem-file-specification.md`
so it matches `as_binding`. Scan the docs for related stale field names.

Reconcile `docs/theoremc-design.md` with the current workspace members. Mark
unimplemented components such as `theoremd` and `theoremc-dylint` as planned or
historical, or remove them from current-state diagrams and prose. Add a short
documentation map to `README.md` that points readers to `docs/contents.md`.

Run:

```sh
make fmt 2>&1 | tee /tmp/fmt-theoremc-codebase-audit-2026-06-14.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-codebase-audit-2026-06-14.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-codebase-audit-2026-06-14.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-codebase-audit-2026-06-14.out
make lint 2>&1 | tee /tmp/lint-theoremc-codebase-audit-2026-06-14.out
make test 2>&1 | tee /tmp/test-theoremc-codebase-audit-2026-06-14.out
coderabbit review --agent 2>&1 | tee /tmp/coderabbit-theoremc-codebase-audit-2026-06-14.out
```

Then commit.

### Milestone 11: tighten dependency version requirements

Replace broad but already caret-compatible requirements such as
`proptest = "1"`, `tempfile = "3"`, `insta = "1"`, `prettyplease = "0.2"`, and
`trybuild = "1"` with explicit caret-compatible versions such as `"1.11.0"` or
`"3.27.0"`. This does not change Cargo's requirement syntax from caret
semantics; it makes the intended compatible version floor precise and aligned
with the repository policy. Inspect every workspace `Cargo.toml` and record any
justified exception in the dependency management documentation or developer
guide.

Run `cargo update --workspace` only if Cargo requires lockfile adjustment.
Inspect any `Cargo.lock` change and ensure it is a consequence of the manifest
policy fix, not an unrelated update.

Run the same deterministic gates and CodeRabbit review command as Milestone 1,
then commit.

### Milestone 12: final reconciliation

Review all touched files against the audit list and this plan. Confirm that no
Rust source file exceeds 400 lines, public APIs have Rustdoc examples where
required, documentation links resolve, and `docs/users-guide.md`,
`docs/developers-guide.md`, and `docs/theoremc-design.md` describe the final
behaviour and internal practices.

Run the full gate sequence one final time:

```sh
make fmt 2>&1 | tee /tmp/fmt-theoremc-codebase-audit-2026-06-14.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-codebase-audit-2026-06-14.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-codebase-audit-2026-06-14.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-codebase-audit-2026-06-14.out
make lint 2>&1 | tee /tmp/lint-theoremc-codebase-audit-2026-06-14.out
make test 2>&1 | tee /tmp/test-theoremc-codebase-audit-2026-06-14.out
coderabbit review --agent 2>&1 | tee /tmp/coderabbit-theoremc-codebase-audit-2026-06-14.out
```

Update `Outcomes & Retrospective`, then commit the final plan update.

## Progress

- [x] 2026-06-14: Drafted this ExecPlan from the 2026-06-14 audit findings.
- [x] 2026-06-16: User approved implementation and requested work proceed
  through the planned milestones in order.
- [x] 2026-06-16: Milestone 1 established the diagnostic reason model. Added
  `ValidationReasonKind` and `ValidationFailure`, switched the loader to use
  typed validation locations, deleted marker-string parsing, and added focused
  typed-location unit tests. `make check-fmt`, `make lint`, `make test`,
  `make markdownlint`, and `make nixie` passed with logs under `/tmp`.
  CodeRabbit was attempted after deterministic gates but stalled at
  `preparing_sandbox`; see `Surprises & Discoveries`.
- [x] 2026-06-16: Milestone 2 split schema validation into action, evidence,
  expression, field, and step modules while keeping `validate_theorem_doc` as
  the orchestration entrypoint. The focused core validation test, `make fmt`,
  `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
  `make nixie` passed with logs under `/tmp`. CodeRabbit was attempted after
  deterministic gates but timed out at `preparing_sandbox`; see
  `Surprises & Discoveries`.
- [ ] Milestone 3: centralize argument decode error remapping.
- [ ] Milestone 4: precompute macro action signatures.
- [ ] Milestone 5: consolidate fixture crate and schema BDD support.
- [ ] Milestone 6: remove duplicated manual TOML section parsing.
- [ ] Milestone 7: unify path normalization and path policy.
- [ ] Milestone 8: remove timing-dependent BDD sleep.
- [ ] Milestone 9: retire broad dead-code suppressions in argument lowering.
- [ ] Milestone 10: fix documentation and API drift.
- [ ] Milestone 11: tighten dependency version requirements.
- [ ] Milestone 12: final reconciliation.

## Surprises & Discoveries

- The audit branch was renamed from `feat/refactorauditwithleta` to
  `codebase-audit-2026-06-14` before this plan was drafted.
- 2026-06-16: `crates/theoremc-core/src/schema/validate.rs` was already above
  the 400-line file limit before Milestone 1 code edits. Because Milestone 1
  had to touch the file, the existing validation unit tests were moved to
  `crates/theoremc-core/src/schema/validate_tests.rs` during this milestone
  rather than waiting for the broader production split in Milestone 2.
- 2026-06-16: `crates/theoremc-core/src/schema/loader.rs` also crossed the
  400-line limit after Milestone 1 edits. A small
  `crates/theoremc-core/src/schema/loader_message.rs` module now owns
  `FieldName` and `ErrorMessage`, bringing the touched loader file down to 375
  lines.
- 2026-06-16: CodeRabbit did not report a rate limit. One normal invocation
  stalled at `preparing_sandbox` and was terminated after no progress. A
  bounded 300-second retry also produced only setup output through
  `preparing_sandbox`, with no findings or completion summary in
  `/tmp/coderabbit-theoremc-codebase-audit-2026-06-14.out`.
- 2026-06-16: Milestone 2 did not require new behavioural fixtures because the
  split preserved validation order and rendered error messages. Existing
  `rstest` cases in `validate_tests.rs`, BDD schema suites, and full nextest
  coverage remain the behavioural safety net.
- 2026-06-16: CodeRabbit again did not report a rate limit for Milestone 2. A
  bounded 300-second invocation after deterministic gates remained at
  `preparing_sandbox` until `timeout` exited with code 124. The log at
  `/tmp/coderabbit-theoremc-codebase-audit-2026-06-14.out` contains setup
  output only and no actionable findings.

## Decision Log

- 2026-06-14: Keep this document as a pre-implementation DRAFT. The ExecPlan
  skill requires explicit user approval before implementation begins, and the
  requested pull request should therefore carry the plan rather than the fixes.
- 2026-06-14: Treat the audit findings as ordered implementation milestones.
  This keeps each fix reviewable, gives CodeRabbit and deterministic gates a
  clear boundary, and provides rollback points through frequent commits.
- 2026-06-16: Move the plan from DRAFT to ACTIVE after explicit user approval
  to proceed. The approval satisfies the ExecPlan implementation gate, so work
  now proceeds milestone-by-milestone within the stated tolerances.
- 2026-06-16: Preserve the public `SchemaError::ValidationFailed` variant
  shape for Milestone 1. The typed reason travels in a private
  `ValidationFailure` until the loader attaches diagnostics and converts back to
  `SchemaError`, avoiding a public API break while removing string-coupled
  source-location selection.
- 2026-06-16: No property, Kani, or Verus check is required for Milestone 1.
  The new reason model is finite dispatch over explicit validation classes, so
  focused `rstest` unit cases cover each supported variant without introducing
  a generated input invariant.
- 2026-06-16: Treat the CodeRabbit sandbox stall as tool unavailability for
  Milestone 1 under the plan tolerance. The milestone is allowed to proceed
  because deterministic repository gates passed after the final code change and
  CodeRabbit produced no actionable findings to clear.
- 2026-06-16: Keep Milestone 2 as a pure module split rather than introducing
  new validator abstractions. The descriptor helper introduced in Milestone 1
  already removed the immediate string-control coupling, so this milestone
  should only isolate responsibilities and leave deeper validator API changes
  to later findings if needed.
- 2026-06-16: No property, Kani, or Verus check is required for Milestone 2.
  The change is a responsibility split that preserves validation ordering and
  message semantics instead of introducing a new invariant over generated input
  ranges or contractual business logic.

## Outcomes & Retrospective

This section is intentionally empty while implementation is in progress.
Populate it as milestones are implemented and validated.
