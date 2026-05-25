# Step 3.3.1: emit typed action probes

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
 `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: IN PROGRESS

## Purpose / big picture

Implement the first Roadmap checkbox under Phase 3, Step 3.3 by making every
action referenced from a `.theorem` file participate in ordinary Rust type
checking before any runtime theorem reporting exists.

After this change, `theorem_file!("path/to/file.theorem")` still expands into
the deterministic per-file module and Kani harness stubs delivered by Step 3.2,
but it also emits compile-time binding probes for each referenced action. An
action probe is generated Rust code whose only job is to force the Rust
compiler to check that the expected action signature is still compatible with
the resolved function in `crate::theorem_actions`. The intended shape is:

```rust
let _: fn(ExpectedArg1, ExpectedArg2) -> ExpectedReturn =
    crate::theorem_actions::mangled_action_identifier;
```

The observable result is that renaming an action export, removing it from
`crate::theorem_actions`, or changing its signature in a way that no longer
matches the theorem compiler's expected action contract fails compilation in
the theorem owner crate. This is a compile-time connectedness feature only; it
does not run actions, discover actions at runtime, or implement Phase 4 Kani
execution semantics.

Observable success:

1. A fixture crate containing a valid theorem and matching
   `crate::theorem_actions` exports builds successfully in an ordinary non-Kani
   `cargo build`.
2. The generated expansion contains one deterministic typed action probe per
   distinct referenced canonical action, covering both `Let` bindings and nested
    `Do` steps.
3. A fixture crate whose theorem references a missing action export fails
   compilation with a Rust diagnostic pointing at the generated probe's
   `crate::theorem_actions::...` path.
4. A fixture crate whose action export has the right mangled name but the
   wrong function signature fails compilation with an `expected fn pointer`
   style type error from Rust.
5. Duplicate references to the same canonical action across one file do not
   emit duplicate probes, so diagnostics stay focused.
6. Existing schema validation, duplicate action collision checks, module
   naming, harness naming, and `#[cfg(kani)]` harness gating remain unchanged.
7. Unit tests using `rstest` cover action collection, deduplication, and the
   exact generated token shape.
8. Behavioural tests using `rstest-bdd` cover the theorem-author workflow for
   happy-path builds, missing actions, and signature drift.
9. Compile-fail tests cover generated-code diagnostics for missing action
   exports and incompatible signatures.
10. `docs/adr-004-action-signature-specification.md`,
    `docs/theoremc-design.md`, `docs/theorem-file-specification.md`,
    `docs/users-guide.md`, and `docs/developers-guide.md` describe the
    implemented probe contract.
11. `docs/roadmap.md` marks the Step 3.3.1 checkbox done only after
    implementation, validation, CodeRabbit review, and commit gates pass.
12. `make check-fmt`, `make lint`, and `make test` pass. Because documentation
    changes are in scope, `make fmt`, `make markdownlint`, and `make nixie`
    also pass before the implementation is complete.

## Constraints

- This plan must not be implemented until the user explicitly approves it.
- Scope is limited to Roadmap Step `3.3.1`, "Emit typed action probes".
  Do not implement referenced-type probes, nested struct-field introspection,
  Phase 4 action execution, `kani::any`, `kani::assume`, `assert!`,
  `kani::cover!`, `must`, `maybe`, evidence result policy, reporting, or
  runtime reflection.
- The implementation must satisfy the typed signature-compatibility acceptance
  criterion. A name-only probe such as `let _ = crate::theorem_actions::...;`
  is not sufficient.
- Preserve the action resolution target and mangling rules from
  `docs/name-mangling-rules.md`: canonical actions resolve under
  `crate::theorem_actions` using `mangle_action_name`.
- Preserve the Step 3.2 generated module layout: each theorem file expands
  into the same stable private module, keeps the source `include_str!`, and
  keeps Kani harnesses under `#[cfg(kani)]`.
- Action probes may be compiled in ordinary non-Kani builds because their
  purpose is to detect drift without requiring Kani.
- Do not introduce runtime action registries, `inventory`-based compile-time
  resolution, or reflection. DES-5 permits metadata for future reporting, but
  Step 3.3.1 must remain a compile-time binding check.
- Use the theorem-side `Actions` signature declarations specified by
  `docs/adr-004-action-signature-specification.md`. Do not broaden the
  `.theorem` syntax beyond that declaration shape while implementing this step.
- Keep new public API narrow. Prefer a small `theoremc-core` helper that
  returns distinct referenced actions over exposing collision internals.
- Use `cap_std` and `camino` for filesystem work if implementation needs to
  read crate-local files.
- Behavioural tests that model user workflows must use `rstest-bdd` v0.5.0,
  matching the existing BDD suites.
- Property tests are not required for simple deterministic traversal or token
  rendering. Add `proptest` only if the implementation introduces a new
  invariant over arbitrary action sets or path/name transformations.
- Kani and Verus proofs are not required for this change. The feature is a
  Rust compile-time type-checking contract, not a proof obligation or unsafe
  code invariant.
- Run code quality gates sequentially and capture long output with `tee` into
  `/tmp` logs. Do not run format checks, lints, or tests in parallel.
- Run `coderabbit review --agent` after each major implementation milestone,
  clear all concerns before moving to the next milestone, and record the result
  in this plan.
- Commit after each logical change that passes its gates. Use file-based commit
  messages with `git commit -F`, never `git commit -m`.
- Documentation and comments must use en-GB Oxford spelling and grammar.
- Keep Rust source files under 400 lines. If macro code or tests grow too
  large, split them into focused sibling modules.

## Tolerances

- Approval: if this plan is not explicitly approved, do not make implementation
  changes.
- Signature source: the maintainable source of expected action signatures is
  now ADR-004 theorem-side `Actions` declarations. If implementation cannot use
  that source without inventing a second manifest, source scanner, or
  attribute-macro side channel, stop and reassess.
- Scope: if implementation requires changing `.theorem` syntax, build-suite
  generation, Step 3.2 Kani harness layout, or Phase 4 harness bodies, stop and
  ask for direction.
- Public API: if the narrow `theoremc-core` action-reference helper cannot be
  added without exposing incidental collision internals, stop and present API
  options.
- Source scanning: if signature discovery requires building a general Rust
  module resolver, following arbitrary `pub use` graphs, or parsing files
  outside the theorem owner crate, stop. That is larger than Step 3.3.1.
- Dependencies: if a new crates.io dependency appears necessary, make one
  prototype and stop for approval before adding it.
- Diagnostics: if compile-fail diagnostics become unstable across local and CI
  environments after two attempts to narrow the asserted stderr, switch to
  structural assertions in BDD fixture builds and document the trade-off.
- Code size: if implementation grows beyond roughly 8 changed code files or
  500 net Rust lines before documentation updates, stop and reassess whether
  the task should be split.
- Validation: if any of `make check-fmt`, `make lint`, or `make test` still
  fails after five focused fix attempts, stop with captured logs and summarize
  the remaining failure.
- CodeRabbit: if `coderabbit review --agent` is unavailable in the local
  environment, record that fact and continue only after the ordinary repository
  gates pass. If it reports actionable concerns, address them before continuing
  or document why the concern is out of scope and seek direction.

## Risks

- Risk: the current code schema stores action names and argument values, but
  does not yet implement the ADR-004 `Actions` signature field. Severity: high.
  Likelihood: high. Mitigation: make schema support for `ActionSignature` the
  first implementation step and keep typed probes dependent on it.

- Risk: Rust procedural macros cannot ask the Rust type checker for a
  function's signature during expansion. Severity: high. Likelihood: high.
  Mitigation: rely only on explicit, parseable ADR-004 action signature
  declarations and do not attempt runtime reflection or source scanning.

- Risk: direct probes in non-Kani builds will make existing fixture crates fail
  unless they define `crate::theorem_actions`. Severity: medium. Likelihood:
  high. Mitigation: update fixture crate builders and pass tests to include the
  minimal matching action module when theorem files reference actions.

- Risk: exposing existing action traversal from
  `crates/theoremc-core/src/collision.rs` may accidentally make collision
  grouping details part of the internal API. Severity: medium. Likelihood:
  medium. Mitigation: expose a purpose-built helper such as
  `referenced_actions(&[TheoremDoc]) -> Vec<ReferencedAction>` and keep
  collision grouping private.

- Risk: duplicate action references can produce repeated probes and noisy
  diagnostics. Severity: low. Likelihood: medium. Mitigation: deduplicate by
  canonical action name while preserving first-seen deterministic ordering.

- Risk: trybuild stderr snapshots may be brittle because compiler diagnostics
  for type mismatch can vary by Rust version. Severity: medium. Likelihood:
  medium. Mitigation: use trybuild where exact user-facing errors matter, and
  use BDD fixture build assertions for stable fragments such as mangled action
  names and "expected fn pointer" shape.

- Risk: an implementation that tries to support all Rust function signatures
  in the first step can become an unbounded source parser. Severity: high.
  Likelihood: medium. Mitigation: keep this step bounded to the action
  signature forms already documented in DES-5: simple identifier parameters and
  return types `()`, `T`, `Result<T, E>`, or `Option<T>`.

- Risk: this task is adjacent to referenced-type probes and nested map
  lowering limitations, so it may be tempting to implement Step 3.3.2 at the
  same time. Severity: medium. Likelihood: medium. Mitigation: keep type
  phantom usages and nested map expansion explicitly out of scope for this
  ExecPlan.

## Signposts and required references

- Roadmap task: `docs/roadmap.md` Phase 3, Step 3.3, first checkbox.
- `DES-5`: `docs/theoremc-design.md` §5, Rust actions, action resolution,
  signature rules, and argument shaping.
- `DES-7`: `docs/theoremc-design.md` §7, build integration and binding probes.
- `NMR-1`: `docs/name-mangling-rules.md`, action mangling and
  `crate::theorem_actions` resolution.
- `TFS-4`: `docs/theorem-file-specification.md` §4, step and action schemas.
- `TFS-5`: `docs/theorem-file-specification.md` §5, value forms and explicit
  reference semantics.
- `ADR-004`: `docs/adr-004-action-signature-specification.md`, theorem-side
  action signature declarations and rejected inference approaches.
- `docs/rust-testing-with-rstest-fixtures.md` for unit-test style.
- `docs/reliable-testing-in-rust-via-dependency-injection.md` for fixture
  isolation and external process boundaries.
- `docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
  macro implementation small and extractable.
- `docs/rstest-bdd-users-guide.md` was referenced in the task prompt but is
  absent in this checkout. Use existing `tests/*_bdd.rs` files and `rstest-bdd`
  v0.5.0 dependency configuration as the local style reference.
- Skills used to prepare this plan: `leta`, `rust-router`,
  `rust-types-and-apis`, `arch-crate-design`, `execplans`, `firecrawl-mcp`,
  `commit-message`, `pr-creation`, and `en-gb-oxendict-style`.
- External prior art: the Rust Reference documents function pointer types and
  coercion from function items to `fn` pointers at
  <https://doc.rust-lang.org/reference/types/function-pointer.html>. This is
  the language mechanism behind the intended probe assignment.
- External prior art: `trybuild` documents compile-fail tests for procedural
  macros and generated code diagnostics at <https://docs.rs/trybuild>.

## Implementation plan

### Milestone 0: confirm the signature source and write red tests

Begin by creating failing tests that describe the behaviour before production
code changes. Use `rstest` unit tests in `crates/theoremc-macros/src/tests.rs`
or a focused sibling test module to assert that expansion for a theorem with a
referenced action contains a typed probe. Add or adjust test support in
`crates/theoremc-macros/src/tests_support.rs` so fixtures can include `Let`/
`Do` action references and expected action signatures.

Add a focused compile-fail fixture under `crates/theoremc-macros/tests/expand`
for a missing `crate::theorem_actions` export and one for a signature mismatch.
Use the existing `trybuild` harness in
`crates/theoremc-macros/tests/expand.rs`, staging any `.theorem` fixtures it
needs.

The signature-source decision is now made by ADR-004. Implement the theorem-side
 `Actions` schema first, and make the initial red tests use declared signatures
such as:

```yaml
Actions:
  account.deposit:
    params:
      account: "&mut crate::account::Account"
      amount: "u64"
    returns: "Result<(), crate::account::DepositError>"
```

Do not proceed to Milestone 1 until the red tests fail for the expected reason:
the schema understands signatures, but expansion does not yet emit typed action
probes. Run:

```sh
cargo test -p theoremc-macros --test expand 2>&1 | tee /tmp/test-theoremc-3-3-1-expand-red.out
cargo test -p theoremc-macros 2>&1 | tee /tmp/test-theoremc-3-3-1-macro-red.out
```

Expected result before implementation: the new tests fail because the expansion
does not yet emit typed action probes.

Run `coderabbit review --agent` after the red-test commit. Address any concern
that affects the planned contract before continuing.

### Milestone 1: expose distinct referenced actions from `theoremc-core`

Add a narrow helper in `crates/theoremc-core` that traverses
`TheoremDoc::let_bindings` and `TheoremDoc::do_steps`, including nested `maybe`
steps, and returns distinct referenced actions in deterministic first-seen
order. Reuse the existing traversal logic in
`crates/theoremc-core/src/collision.rs` rather than duplicating it in the
proc-macro crate.

The helper should return a small domain type or borrowed view that contains at
least the canonical action name and enough context for diagnostics or tests. It
should not expose collision grouping internals. Keep `check_action_collisions`
behaviour unchanged.

Add `rstest` unit coverage for:

- `Let` action references,
- `Do` action references,
- nested `maybe` action references,
- repeated canonical names in one theorem,
- repeated canonical names across multiple theorem documents, and
- deterministic first-seen ordering.

Run:

```sh
cargo test -p theoremc-core 2>&1 | tee /tmp/test-theoremc-3-3-1-core.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the core helper.

### Milestone 2: emit typed probes in `theorem_file!`

Update `crates/theoremc-macros/src/lib.rs` so `render_expansion` asks
`theoremc-core` for distinct referenced actions, maps each canonical action to
the existing `mangle_action_name` resolution target, and emits a deterministic
probe in the generated per-file module.

Keep probes outside the `#[cfg(kani)]` backend module so ordinary builds detect
action drift. Keep all direct `kani` references inside the existing
`#[cfg(kani)]` module. A file with no referenced actions should emit no action
probe block.

The generated probe names and layout should be stable and readable. Prefer a
private nested module or private function inside the per-file theorem module if
that keeps `let` probes syntactically simple and prevents unused-code warnings.
For example, one acceptable shape is:

```rust
#[allow(dead_code, reason = "compile-time theorem action probes are never called")]
fn __theoremc_action_probes() {
    let _: fn(ArgTy) -> ReturnTy =
        crate::theorem_actions::example__action__h0123456789ab;
}
```

If the final implementation needs a slightly different shape to satisfy Rust
syntax or lint policy, record the decision here and keep the same type-checking
semantics.

Update macro unit tests and snapshots to prove:

- no probes are emitted when no action is referenced,
- one probe is emitted for a single referenced action,
- duplicate references emit one probe,
- `Let` and `Do` references are both included, and
- probes use `crate::theorem_actions::<mangled_identifier>`.

Run:

```sh
cargo test -p theoremc-macros 2>&1 | tee /tmp/test-theoremc-3-3-1-macros.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the macro expansion change.

### Milestone 3: add behavioural compile checks

Update `tests/theorem_file_macro_bdd.rs` and
`tests/theorem_file_macro_bdd/fixture_crate.rs` so fixture crates can define a
minimal `crate::theorem_actions` module. Add BDD scenarios for:

- a theorem owner crate with matching action exports builds,
- a theorem owner crate missing the mangled action export fails, and
- a theorem owner crate with the mangled export but a mismatched signature
  fails.

Keep existing non-Kani and `cargo kani list` scenarios intact. These scenarios
prove the user workflow: theorem authors see drift at build time in their own
crate, without running runtime reflection or reporting.

Run:

```sh
cargo test --test theorem_file_macro_bdd 2>&1 | tee /tmp/test-theoremc-3-3-1-bdd.out
cargo test -p theoremc-macros --test expand 2>&1 | tee /tmp/test-theoremc-3-3-1-trybuild.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the behavioural tests.

### Milestone 4: update documentation and roadmap

Update `docs/theoremc-design.md` §7.3 with the concrete generated probe shape,
the signature source chosen in Milestone 0, and the distinction between action
probes in Step 3.3.1 and referenced-type probes in Step 3.3.2.

Update `docs/users-guide.md` to explain the user-visible behaviour: missing
action exports and signature drift fail ordinary Rust compilation in the
theorem owner crate, while schema validation failures still come from
theoremc's loader diagnostics.

Update `docs/developers-guide.md` §3.4.1 to list the new `theoremc-core`
internal helper used by the proc-macro crate and to describe the probe emission
testing convention.

Update `docs/roadmap.md` by marking only the Step 3.3.1 checkbox done. Leave
the referenced-type probe checkbox unchecked.

Run:

```sh
make fmt 2>&1 | tee /tmp/fmt-theoremc-3-3-1.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-3-3-1.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-3-3-1.out
```

Run `coderabbit review --agent`, clear concerns, update this plan, and commit
the documentation and roadmap updates.

### Milestone 5: full quality gate and final review

Run the repository gates sequentially:

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-3-3-1.out
make lint 2>&1 | tee /tmp/lint-theoremc-3-3-1.out
make test 2>&1 | tee /tmp/test-theoremc-3-3-1.out
```

Run a final `coderabbit review --agent`. Clear every actionable concern or
record a user-approved reason for leaving it unresolved. Update
`Outcomes & Retrospective`, set this ExecPlan status to `COMPLETE`, commit the
final plan update, push the branch, and update the pull request.

## Validation strategy

The validation strategy is intentionally layered.

Use `rstest` unit tests for pure traversal and token rendering because those
tests are fast, deterministic, and directly exercise the internal contract.

Use `trybuild` compile-fail tests for procedural macro diagnostics where the
generated Rust must fail compilation in a predictable way. The existing
`crates/theoremc-macros/tests/expand.rs` harness already stages `.theorem`
fixtures and compares compiler output, so extend that pattern.

Use `rstest-bdd` behavioural tests for end-to-end theorem-owner workflows. The
existing `tests/theorem_file_macro_bdd.rs` suite already builds temporary
fixture crates, serializes Cargo invocations, and optionally checks Kani
harness discovery when `cargo-kani` is installed.

Use full repository gates at the end because this change crosses
`theoremc-core`, the proc-macro crate, generated Rust shape, fixture crates,
and documentation.

## Progress

- [x] 2026-05-20: Loaded the requested `leta` and `rust-router` skills.
- [x] 2026-05-20: Loaded `execplans`, `firecrawl-mcp`, `commit-message`,
  `pr-creation`, `rust-types-and-apis`, `arch-crate-design`, and
  `en-gb-oxendict-style` for the planning, review, Rust API boundary, and PR
  work.
- [x] 2026-05-20: Created a leta workspace for this worktree with
  `leta workspace add`.
- [x] 2026-05-20: Renamed the branch to
  `3-3-1-emit-typed-action-probes`.
- [x] 2026-05-20: Reviewed `AGENTS.md`, `docs/roadmap.md`,
  `docs/theoremc-design.md`, `docs/name-mangling-rules.md`,
  `docs/users-guide.md`, `docs/developers-guide.md`, existing Step 3.2
  ExecPlans, macro expansion code, macro tests, trybuild fixtures, and BDD
  fixture-crate helpers.
- [x] 2026-05-20: Created context pack `pk_s5cozlvx`
  (`typed-action-probe-planning`) for shared planning evidence.
- [x] 2026-05-20: Used a Wyvern agent team for read-only planning
  reconnaissance over implementation touchpoints, documentation implications,
  behavioural acceptance, and non-goals.
- [x] 2026-05-20: Used Firecrawl to verify external prior art for Rust
  function pointer coercion and trybuild compile-fail testing.
- [x] 2026-05-20: Drafted this pre-implementation ExecPlan.
- [x] 2026-05-20: Ran targeted Markdown formatting and linting for this
  ExecPlan, `git diff --check`, and `make nixie`; all passed.
- [x] 2026-05-20: Ran `coderabbit review --agent`, addressed the reported
  prose and wrapping findings, and reran CodeRabbit with zero findings.
- [x] 2026-05-24: Received explicit user approval to proceed with
  implementation of this ExecPlan.
- [x] 2026-05-24: Began Milestone 0 and rechecked the schema, design
  documents, macro expansion path, and action traversal code for a source of
  expected action parameter and return types.
- [x] 2026-05-24: Prototyped Rust placeholder function-pointer probes with
  `let _: fn(_) -> _ = ...;`; rustc accepts the syntax, but it merely infers
  the current function item type and therefore cannot enforce a stable expected
  signature.
- [x] 2026-05-24: Implementation reached the Milestone 0 signature-source
  tolerance. A maintainable source for `ExpectedArg...` and `ExpectedReturn`
  was not present in the current theorem schema, macro input, or documented
  action metadata path.
- [x] 2026-05-24: Ran targeted Markdown linting, `git diff --check`, and
  `coderabbit review --agent` for the blocked-state ExecPlan update. CodeRabbit
  reported zero findings.
- [x] 2026-05-25: Drafted and finalized
  `docs/adr-004-action-signature-specification.md`, using Firecrawl prior-art
  research and local Rust experiments to decide on theorem-side `Actions`
  declarations instead of inference from Rust action implementations.
- [x] 2026-05-25: Updated `docs/theoremc-design.md`,
  `docs/theorem-file-specification.md`, and `docs/contents.md` for ADR-004.
- [x] 2026-05-25: Cleared the Milestone 0 signature-source blocker in this
  plan. Implementation can resume with schema support for ADR-004 action
  signatures.
- [x] 2026-05-25: Ran scoped Markdown linting for the changed documents,
  `git diff --check`, `make fmt`, `make markdownlint`, `make nixie`, and
  `coderabbit review --agent` for the ADR-004 design milestone. Scoped linting,
  diff checking, and `make nixie` passed. CodeRabbit reported ADR structure,
  prose, and wrapping findings across review passes; all were fixed, and the
  final rerun reported zero findings.
- [x] 2026-05-25 21:05 CEST: Implemented the first core slice for ADR-004 by
  adding `ActionSignature` schema support, validating declared action names,
  parameter identifiers, and `syn::Type` strings, requiring an `Actions` entry
  for every referenced action, and exposing deterministic `referenced_actions`
  traversal from `theoremc-core`.
- [x] 2026-05-25 21:05 CEST: Added `rstest` coverage for ordered signature
  parsing, default `returns: ()`, missing referenced action signatures, invalid
  Rust type strings, and first-seen referenced-action de-duplication.
  `cargo test -p theoremc-core` passed; log:
  `/tmp/test-theoremc-3-3-1-core-schema.out`.
- [x] 2026-05-25 21:13 CEST: Migrated existing action-bearing fixtures to
  include explicit `Actions` declarations, updated the parser diagnostic
  snapshot for the new accepted top-level key, and hardened the Kani BDD test
  against a locally installed but unusable Kani compiler.
- [x] 2026-05-25 21:13 CEST: Re-ran deterministic gates for the first core
  slice. `make check-fmt` and `make lint` passed. The default `make test`
  surfaced a local Kani shared-library failure, then
  the serialised full-suite command passed 551 nextest tests and all workspace
  doctests; log:
  `/tmp/test-theoremc-3-3-1-core-schema-gate-serial.out`.
- [x] 2026-05-25 21:22 CEST: Committed the ADR-004 schema and validation
  milestone as `0f9cb63` and ran `coderabbit review --agent`; CodeRabbit
  reported zero findings.
- [x] 2026-05-25 21:27 CEST: Implemented typed action probe emission in
  `theoremc-macros`, using `referenced_actions` plus `mangle_action_name` to
  generate non-Kani `let _: fn(...) -> ... = crate::theorem_actions::...;`
  checks for each distinct referenced action.
- [x] 2026-05-25 21:27 CEST: Added macro unit tests for emitted probe shape
  and conflicting shared action signatures, plus trybuild coverage proving a
  valid export compiles, a missing export fails, and return-type drift fails.
  `cargo test -p theoremc-macros` passed; log:
  `/tmp/test-theoremc-3-3-1-macro-probes-trybuild.out`.
- [x] 2026-05-25 21:29 CEST: Re-ran implementation gates for the macro probe
  milestone. `make check-fmt`, targeted Markdown linting for changed docs,
  `make lint`, and `make test` passed. The full test target ran 554 nextest
  tests plus all workspace doctests; log:
  `/tmp/test-theoremc-3-3-1-macro-probes-gate.out`.
- [x] 2026-05-25 21:29 CEST: Marked the Step 3.3.1 roadmap entry done after
  the deterministic gates passed.

## Surprises & Discoveries

- `leta workspace add` succeeded, but the initial `leta grep` query failed
  because `rust-analyzer` was not available to the leta daemon. Installing the
  rustup `rust-analyzer` component and restarting the daemon fixed semantic
  navigation.
- The prompt names `theorem-file-specification.md` at repository root, but the
  file in this checkout is `docs/theorem-file-specification.md`.
- The prompt names `docs/rstest-bdd-users-guide.md`, but that file is absent in
  this checkout. Existing `tests/*_bdd.rs` suites are the available local style
  reference.
- Both local inspection and Wyvern reconnaissance found the same central
  design gap: action names are already collected and mangled, but expected Rust
  action signatures are not yet an obvious product of the current schema or
  macro expansion path.
- `make fmt` and repository-wide `make markdownlint` are currently blocked by
  pre-existing Markdown lint findings outside this plan. This plan file passes
  targeted Markdown linting, and the unrelated formatter churn was discarded
  before commit.
- The current `ActionCall` model contains `action`, `args`, and optional `as`
  only. It does not contain parameter types, return type, or a reference to a
  separately declared signature object.
- `Forall` can provide types for symbolic inputs, but it does not type
  literals, `Let` outputs, `Do` bindings, return values, or actions whose
  arguments are derived from earlier actions. It is therefore insufficient as
  the general signature source for this feature.
- Rust accepts `let _: fn(_) -> _ = crate::theorem_actions::...;` in a local
  item body, but the placeholders are inferred from the right-hand side. This
  proves name reachability and function-item coercion, but not the stable
  theorem-expected signature. It would let a changed action signature compile,
  violating the roadmap acceptance criterion.
- Firecrawl prior-art research found no Rust mechanism that lets a proc macro
  infer arbitrary resolved function signatures during expansion. The relevant
  ecosystem patterns instead support either token-based procedural macros,
  link-time distributed registries (`inventory`, `linkme`), or executable step
  definitions such as Cucumber. None supplies a theorem-owned expected
  signature to `theorem_file!`.
- `make fmt` and `make markdownlint` still report pre-existing Markdown issues
  outside the ADR-004 change. The current blocking repo-wide
  `make markdownlint` finding is `docs/developers-guide.md:268:271` (`MD060`
  table column style). `make fmt` also reports existing `MD013` line-length
  findings across older documents after applying formatters. Unrelated
  formatter churn was discarded.
- ADR-004 schema support makes existing fixtures with `Let` or `Do` action
  references fail unless they include matching `Actions` entries. This is
  intentional for Step 3.3.1, but repository fixtures must be migrated as the
  implementation reaches macro and BDD tests.
- In this environment, `cargo kani --version` succeeds, but `cargo kani list`
  can still fail because Kani's compiler cannot load
  `libLLVM.so.21.1-rust-1.93.0-nightly`. The BDD scenario now treats this
  installed-but-unusable compiler state like an absent Kani installation while
  still failing ordinary harness-listing errors.
- `make fmt` still fails after running `cargo fmt` because repository-wide
  Markdown linting reports pre-existing `MD013` findings. The task-related
  Rust formatting from `cargo fmt` was retained; unrelated Markdown formatter
  churn was discarded.
- The default parallel `make test` path can also make local `cargo kani list`
  fail with `Broken pipe` panics in dependency build scripts. That is another
  installed-but-unusable Kani state, not a theoremc proof-harness failure, so
  the BDD skip guard now recognises it.

## Decision Log

- 2026-05-20: Keep this ExecPlan in DRAFT and do not mark the roadmap item
  done. Rationale: the user explicitly requires approval before implementation.
- 2026-05-20: Treat typed signature compatibility as non-negotiable. Rationale:
  the roadmap acceptance says signature drift must cause compile failure, so
  name-only existence checks do not satisfy Step 3.3.1.
- 2026-05-20: Make signature-source discovery the first implementation
  milestone with an explicit stop condition. Rationale: Rust proc macros cannot
  query type-checked function signatures during expansion, and the current
  schema does not obviously carry expected function pointer types.
- 2026-05-20: Plan to expose distinct referenced actions through
  `theoremc-core` rather than re-traversing theorem documents in
  `theoremc-macros`. Rationale: action traversal already exists for collision
  checking, and keeping it in core preserves the crate boundary.
- 2026-05-20: Keep referenced-type probes out of scope. Rationale: they are the
  second Step 3.3 checkbox and should remain a separate atomic change.
- 2026-05-24: Move the ExecPlan from DRAFT to IN PROGRESS. Rationale: the user
  explicitly requested implementation of the planned functionality and
  reiterated the requirement to keep this plan current.
- 2026-05-24: Stop at Milestone 0 instead of emitting name-only or inferred
  probes. Rationale: the acceptance criterion requires signature drift to fail
  compilation. The available data does not provide expected function pointer
  types, and placeholder `fn(_) -> _` probes infer the current action type
  rather than checking it against a theorem-declared contract.
- 2026-05-24: Treat broad Rust source scanning of `crate::theorem_actions` as
  out of scope for this implementation pass. Rationale: deriving the expected
  signature from the current exported function cannot detect drift, and a
  general source resolver is explicitly beyond this ExecPlan's tolerances.
- 2026-05-25: Resolve the signature-source blocker with ADR-004 theorem-side
  `Actions` declarations. Rationale: the declaration is available to
  `theorem_file!` during expansion, is independent of the current Rust action
  implementation, provides parameter order and types for future argument
  shaping, and keeps typed probes honest.
- 2026-05-25: Validate `Actions` Rust type strings during schema loading rather
  than during proc-macro rendering. Rationale: invalid theorem-side contracts
  should remain schema diagnostics, while generated Rust diagnostics should be
  reserved for missing exports and signature drift in the theorem owner crate.

## Outcomes & Retrospective

Implementation was blocked before production code changes. Milestone 0
confirmed that the previous design had no maintainable source for expected
action signatures that can feed probes of the required form:

```rust
let _: fn(ExpectedArg1, ExpectedArg2) -> ExpectedReturn =
    crate::theorem_actions::mangled_action_identifier;
```

ADR-004 now supplies that source through theorem-side `Actions` declarations.
The implementation remains incomplete, but the design blocker is cleared. The
next implementation step is to add `ActionSignature` schema support and red
tests that prove typed probe emission is still absent before Milestone 1
continues.
