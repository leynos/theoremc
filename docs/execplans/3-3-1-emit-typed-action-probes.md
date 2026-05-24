# Step 3.3.1: emit typed action probes

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`,
`Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: BLOCKED

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
10. `docs/theoremc-design.md`, `docs/users-guide.md`, and
    `docs/developers-guide.md` describe the implemented probe contract.
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
- Do not broaden the `.theorem` file syntax unless the signature source
  prototype proves there is no maintainable path using existing documented
  action declarations and the user approves the schema change.
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
- Signature source: if a maintainable source of expected action signatures
  cannot be established within one focused prototype milestone, stop, document
  the failed approaches, and ask whether to split a preceding action-signature
  declaration task.
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

- Risk: the current theorem schema stores action names and argument values, but
  does not obviously store Rust action signatures. Severity: high. Likelihood:
  high. Mitigation: start implementation with a signature-source prototype, and
  do not weaken the acceptance criterion to a name-only probe.

- Risk: Rust procedural macros cannot ask the Rust type checker for a
  function's signature during expansion. Severity: high. Likelihood: high.
  Mitigation: rely on explicit, parseable action signature information or stop
  for a preceding design change rather than attempting runtime reflection.

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

Before implementing the feature, prototype the source of expected function
signatures. The prototype must answer where `ExpectedArg1`, `ExpectedArg2`, and
`ExpectedReturn` come from without runtime reflection. Acceptable outcomes are:

1. Reuse existing documented action declaration information if it is already
   parseable and sufficient.
2. Add a narrow explicit signature declaration path that is still within the
   documented action model and does not change `.theorem` syntax.
3. Stop and ask for approval to split a preceding signature-declaration design
   task.

Do not proceed to Milestone 1 until the red tests fail for the expected reason
and the signature-source decision is recorded in this plan's Decision Log. Run:

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
- [ ] 2026-05-24: Implementation is blocked at the Milestone 0 signature-source
  tolerance. A maintainable source for `ExpectedArg...` and `ExpectedReturn`
  is not present in the current theorem schema, macro input, or documented
  action metadata path.
- [x] 2026-05-24: Ran targeted Markdown linting, `git diff --check`, and
  `coderabbit review --agent` for the blocked-state ExecPlan update.
  CodeRabbit reported zero findings.

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

## Outcomes & Retrospective

Implementation is blocked before production code changes. Milestone 0
confirmed that the current repository has no maintainable source for expected
action signatures that can feed probes of the required form:

```rust
let _: fn(ExpectedArg1, ExpectedArg2) -> ExpectedReturn =
    crate::theorem_actions::mangled_action_identifier;
```

The next viable step is a preceding design/implementation task that introduces
an explicit action-signature declaration consumed by `theorem_file!`. Possible
directions include a theorem-side declaration section, an action manifest
generated by a build step, or an attribute-macro sidecar that produces a
compile-time-readable signature list. Each option changes the contract beyond
the current Step 3.3.1 implementation plan and requires approval before this
feature can proceed.
