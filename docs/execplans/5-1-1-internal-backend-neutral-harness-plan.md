# Task 5.1.1: Introduce the internal backend-neutral `HarnessPlan`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting design review and user approval)

Branch: `5-1-1-internal-backend-neutral-harness-plan`

Roadmap item: `5.1.1` in `docs/roadmap.md`.

## Purpose / big picture

Theoremc turns a readable `.theorem` YAML file into a Kani proof harness during
an ordinary Cargo build. Today the pipeline stops short of meaning: the
`theorem_file!` procedural macro in `crates/theoremc-macros` emits a Kani
harness function with the right name, attributes, and compile-time probes, but
its body is empty. Nothing in the repository yet says, in a form a renderer can
consume, "acquire a symbolic `u8` named `x`; assume `x <= 200`; call
`demo.normalize` with a shared borrow of `x` and bind the result to `once`;
assert `once == twice`; cover `once != x`".

Architecture decision record (ADR) 0006 fixes where that meaning must live: in
an internal, backend-neutral harness plan owned by `crates/theoremc-core`,
between the validated schema types and any backend renderer. The plan must
carry resolved action invocations, source provenance, stable obligation
identities, and explicitly opaque Rust predicates, and it must not carry Kani
syntax, token streams, or generated Rust local names as semantic identity.

This task introduces that plan as a data model with enforced invariants. It
does not lower theorem documents into the plan (task 5.1.2), render the plan to
Kani (task 5.1.3), or run Kani (task 5.1.4). After this task a developer can
write, in Rust, the longhand idempotence theorem's plan:

```rust,no_run
let plan = HarnessPlan::try_new(theorem, evidence, vec![
    Operation::Symbolic(/* forall[0]: x: u8 */),
    Operation::Assume(/* assume[0]: opaque Rust `x <= 200` */),
    Operation::Invoke(/* let[0]: once = demo.normalize(value: &x) */),
    Operation::Invoke(/* let[1]: twice = demo.normalize(value: &once) */),
    Operation::Assert(/* prove[0]: opaque Rust `once == twice` */),
    Operation::Cover(/* witness[0]: opaque Rust `once != x` */),
])?;
```

and observe three things. The plan is accepted and lists the obligation
identities `forall[0]`, `assume[0]`, `let[0]`, `let[1]`, `prove[0]`, and
`witness[0]`. The same operations with the two invocations swapped are rejected
with "binding `once` is used at `let[1]` before it is defined", because the
plan refuses to exist in an ill-formed state. And a snapshot of the plan
contains no `kani::` syntax, no token streams, and no mangled `theorem__…__h…`
or `crate::theorem_actions::…` names.

No user-visible behaviour changes. `cargo build`, `theorem_file!` expansion,
diagnostics, and the `cargo theorem` command line are untouched. The
outstanding design decision this closes is the concrete shape of ADR-0006's
sketch: identity grammar, provenance content, predicate representation,
argument representation, and the well-formedness contract that renderers may
rely on.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- Place the plan in `crates/theoremc-core/src/harness_plan/` per ADR-0006
  decision 2 and roadmap step 5.1. Do not place it in
  `crates/theoremc-build-support` (ADR-0008) or in `crates/theoremc-macros`.
- Do not modify `crates/theoremc-macros`, `crates/theoremc-build-support`,
  `crates/cargo-theorem`, root `build.rs`, or the root `src/` facade. In
  particular, do not re-export `harness_plan` from the `theoremc` facade: the
  plan is an internal interface with no public stability commitment (ADR-0006
  non-goals; ADR-0007 "What may break after theoremc 0.2").
- Do not change the public schema domain model (`schema::types`,
  `schema::arg_value`, `schema::newtypes`) or the loader's return types.
  ADR-0006 decision 3 keeps the version 1 public model unchanged until RFC 0001
  lands.
- No type reachable from `HarnessPlan` may contain `proc_macro2` values,
  `syn` syntax trees, `quote` output, Kani attribute or macro names, mangled
  action, module, or harness identifiers, or any other generated Rust local
  name (ADR-0006 decision 1; ADR-0007 invariant 7; roadmap success criterion).
  `syn` may be used inside constructors to validate text, and the parsed tree
  must be discarded.
- Every opaque Rust predicate must be represented by an explicit
  `Predicate::OpaqueRust` variant (ADR-0006 decision 3; ADR-0007 invariant 6).
- Do not add a backend trait, capability checker, Law IR, `Branch` or
  `Compare` operations, `must` semantics, or lowering from `TheoremDoc`. Those
  belong to tasks 5.1.2 to 5.4.x (ADR-0006 "Minimum plan repertoire"; ADR-0011
  decisions 2 and 3).
- No new production dependency. Authorized new development dependencies of
  `theoremc-core` only: `insta` (1.47.2, matching `theoremc-macros`),
  `rstest-bdd` and `rstest-bdd-macros` (0.6.0, matching the root package).
  `googletest`, `pretty_assertions`, `proptest`, and `rstest` are already
  present.
- Verus tooling must come from the pinned `rust-prover-tools`
  (`tools/rust-prover-tools/REF`) through `prover-tools verus install` and
  `prover-tools verus run`. Do not add shell wrappers, `curl`, `unzip`, or
  `sha256sum` recipes. Keep `make verus` out of `make test`, `make lint`, and
  `make all`.
- Verus proofs must not use `assume`, `admit`, `#[verifier::external_body]`
  on proof obligations, or any other trusted shortcut.
- Repository rules from `AGENTS.md`: no Rust file over 400 lines; every module
  starts with `//!`; every `assert!` family call carries a message; no
  `.expect()` or `.unwrap()` outside tests; `expect_used` stays strict; lints
  are not silenced except by item-scoped `#[expect(lint, reason = "...")]` as a
  last resort; en-GB-oxendict spelling; caret dependency requirements;
  `cap_std::fs_utf8` and `camino` for filesystem access in tests.
- Completed ExecPlans in `docs/execplans/` are historical and must not be
  edited.

## Tolerances (exception triggers)

The comparison base is commit `4c059669c0503ff0cb95764283449e27128e7781`
(`4c05966`, the base of this branch). Every size limit below excludes this
ExecPlan file, including when it is updated in the same commit. Each category
is enforced independently: spare capacity in one category never offsets another.

Classification uses whole-file rules:

- Code: `crates/**/*.rs`, `tests/**/*.rs`, `crates/**/tests/**/*.rs`,
  `verus/**/*.rs`, `*.feature` files, and `Makefile`.
- Configuration: `**/Cargo.toml`, `.github/workflows/*.yml`,
  `tools/verus/VERSION`, and `tools/verus/SHA256SUMS`.
- Documentation: `docs/**/*.md` except `docs/execplans/`, and
  `tools/verus/README.md`.
- Generated exclusions from line counts (still counted as files):
  `Cargo.lock` and `insta` `*.snap` files.

Count net lines (additions minus deletions) per category with:

```sh
git diff --numstat 4c05966 -- <category paths> ':(exclude)docs/execplans/**' \
  ':(exclude)Cargo.lock' ':(exclude)**/*.snap' \
  | awk '{ net += $1 - $2 } END { print net }'
```

Count changed files with
`git diff --name-only 4c05966 -- ':(exclude)docs/execplans/**' | wc -l`.

- Files: more than 45 changed non-ExecPlan files.
- Code: more than 3,000 net lines. Within that, production (non-test) source
  under `crates/theoremc-core/src/harness_plan/` more than 1,400 net lines.
- Configuration: more than 150 net lines.
- Documentation: more than 700 net lines.
- Interface: any change to a public item outside
  `theoremc_core::harness_plan`, or any re-export of it from another crate.
- Dependencies: any dependency beyond those authorized in `Constraints`.
- Architecture: any need to change the schema domain types, the loader, the
  macro crate, or the build-support crate; any need for line or column spans in
  the plan (see decision D3).
- Verus tooling: if `prover-tools verus install` or `verus run` cannot install
  or run the pinned release on this host after two attempts, stop and report;
  do not hand-roll an installer.
- Proof engineering: if the Verus milestone exceeds 8 hours of active work or
  requires an assumption or trusted body to close, stop and report.
- Iterations: if a gate still fails after 5 fix attempts, stop and escalate.
- Time: if any other milestone exceeds 6 hours of active work, stop and
  escalate.
- Ambiguity: if ADR-0006, ADR-0007, ADR-0011, or the roadmap admit two
  materially different readings of a requirement, stop and present options.

Budget rationale. The survey in `Context and orientation` estimates about 1,150
production lines (12 small modules), 1,250 test lines (unit, property,
snapshot, boundary, and behavioural), 300 Verus lines, and 100 contract-test
and Makefile lines: roughly 2,800 code lines. The 3,000 limit leaves a small
contingency. Comparable completed plans set 500 to 700 net lines across 8 to 12
files (`2-2-2`, `3-2-2`, `3-3-1`, `3-3-2`, `1-2-4`) and none revised their
budgets; their surprises were the 400-line file limit and tooling, not
underestimated scope. This task is about four times their surface because it
introduces a new module family and a new proof toolchain, so the limits scale
accordingly rather than borrowing theirs. Configuration covers two manifest
edits, one workflow job, and two pin files. Documentation covers one new ADR
(about 200 lines), one design-document section (about 200 lines), and
developers' guide, layout, contents, and roadmap edits.

## Risks

- Risk: Verus installation or toolchain probing fails on this host or in CI.
  Severity: medium. Likelihood: medium. Mitigation: use only
  `prover-tools verus install` and `verus run` from the pinned
  `rust-prover-tools`; choose the newest Verus release that publishes an
  `x86-linux` archive; record the digest computed from the downloaded archive in
  `tools/verus/SHA256SUMS`; on failure apply the Verus tooling tolerance.
- Risk: proof engineering for the scope kernel takes longer than the obligation
  warrants. Severity: medium. Likelihood: medium. Mitigation: keep the model to
  `Seq<Event>` over `nat` keys; state lemmas before proving; apply the proof
  engineering tolerance; the executable evidence (property tests against an
  independent oracle) stands on its own if the proof is escalated.
- Risk: the 400-line file limit forces late splits. Severity: low. Likelihood:
  medium. Mitigation: the module layout below already places each concern in
  its own file with sibling `*_tests.rs` files.
- Risk: Whitaker or Clippy pedantic lints (`missing_const_for_fn`,
  `must_use_candidate`, `needless_pass_by_value`, `shadow_*`) force API churn.
  Severity: low. Likelihood: high. Mitigation: run `make lint` at every
  milestone; use the `addressing-whitaker-findings` skill.
- Risk: `insta` debug snapshots churn when unrelated fields change. Severity:
  low. Likelihood: medium. Mitigation: snapshot one representative fixture
  only; pair it with semantic assertions; update snapshots only for real
  contract changes.
- Risk: CodeScene "String Heavy Arguments" scoring on newtype constructors.
  Severity: low. Likelihood: medium. Mitigation: constructors accept `&str`
  only at the leaf newtypes; aggregate constructors take newtypes.
- Risk: local `make test` fails in the two Kani scenarios for environmental
  reasons unrelated to this work (a known shadow-loading problem with
  `librustc_driver`). Severity: low. Likelihood: medium. Mitigation: record the
  baseline result in milestone `EP-M0`; treat an identical pre-existing failure
  as baseline, not regression, and say so in the PR.
- Risk: `make check-fmt` does not inspect committed Markdown because
  `mdtablefix --git` only sees uncommitted files. Severity: low. Likelihood:
  high. Mitigation: run `mdtablefix --check` on each changed Markdown file
  explicitly before committing documentation.
- Risk: positional obligation identities change when authors reorder items.
  Severity: low. Likelihood: certain. Mitigation: this is ADR-0006's accepted
  scheme; stable external aliases are roadmap step 6.1. Document it.

## Progress

- [x] (2026-11-16) Branch renamed, Lody session titled, skills loaded.
- [x] (2026-11-16) Reconnaissance by a Wyvern team: core model, codegen
  pipeline and tests, historical ExecPlan budgets, Kani and Verus wiring.
- [x] (2026-11-16) External tooling gaps resolved with Firecrawl:
  `rust-prover-tools` Verus support and Verus release naming.
- [x] (2026-11-16) Initial draft written.
- [ ] Design review by a community-of-experts team and revision.
- [ ] Plan approved by the user.
- [ ] `EP-M0` Baseline gates recorded.
- [ ] `EP-M1` Identity, provenance, and leaf values.
- [ ] `EP-M2` Resolved invocations, operations, and evidence.
- [ ] `EP-M3` Well-formed `HarnessPlan` with executable verification.
- [ ] `EP-M4` Verus proof of the scope kernel and proof tooling.
- [ ] `EP-M5` Documentation, ADR, roadmap, and delivery closure.

## Surprises & discoveries

- Observation: the schema domain model keeps no source spans or document index.
  Evidence: `TheoremDoc` (`crates/theoremc-core/src/schema/types.rs`) has no
  location fields; `load_theorem_docs_with_source` returns `Vec<TheoremDoc>`.
  Impact: plan provenance is structural (decision D3).
- Observation: `Let` keys and `as` names are plain `String`s, and no validator
  rejects a name bound twice across `Forall`, `Let`, and `as`. Evidence:
  `TheoremDoc.let_bindings: IndexMap<String, LetBinding>`,
  `ActionCall.as_binding: Option<String>`; no uniqueness rule in
  `schema/validate*.rs`. Impact: the plan's well-formedness check owns binding
  uniqueness (invariant `INV-unique-binding`); task 5.1.2 decides how lowering
  reports it.
- Observation: Rust expressions are validated with `syn` and then stored as
  untrimmed strings. Evidence: `schema/expr.rs` `validate_rust_expr` and
  `Assertion.assert_expr: String`. Impact: `RustExpression` stores trimmed,
  validated text (decision D2).
- Observation: `theoremc-macros` already has an `ExpansionPlan` and
  `plan_expansion`. Impact: name the new type `HarnessPlan` exactly and refer
  to "harness plan" in prose to avoid confusion.

## Decision log

- D1. Decision: add `pub mod harness_plan` to `theoremc-core`, documented as an
  internal interface, not feature-gated and not re-exported by the `theoremc`
  facade. Rationale: task 5.1.3's renderer lives in `theoremc-macros` and must
  reach the plan across a crate boundary, so `pub(crate)` is impossible; the
  developers' guide already treats `theoremc-core` exports consumed by the
  macro crate as a "stable internal API" (§3.6.1). A Cargo feature would add a
  build-configuration axis for no consumer. Date/Author: 2026-11-16, planning
  agent.
- D2. Decision: represent each opaque Rust predicate as
  `Predicate::OpaqueRust(RustExpression)`, where `RustExpression` stores the
  trimmed source text after `schema::expr::validate_rust_expr` accepts it.
  Rationale: the roadmap success criterion forbids token streams in plan
  fixtures; a `syn::Expr` would leak a syntax-tree type, lacks `Eq` under the
  current `syn` features, and would make the plan depend on a Rust parser
  representation that a future Verus backend cannot use. The text is what the
  Kani renderer re-parses. Rejected: storing `syn::Expr` (ADR-0006's sketch
  named `ParsedRustExpression`; the ADR says exact names may evolve).
  Date/Author: 2026-11-16, planning agent.
- D3. Decision: provenance is structural. A plan records a
  `TheoremIdentity` (the theorem file path exactly as supplied by the caller,
  the zero-based YAML document index, and the `TheoremName`), and every
  operation records an `ObligationId` naming its source position. Line and
  column spans are not recorded. Rationale: the schema domain model drops spans
  after raw conversion; adding them changes public schema types, which ADR-0006
  decision 3 defers. Structural provenance identifies the source element
  exactly, which is what ADR-0011's acceptance criterion requires ("the failure
  preserves the source theorem and proof-obligation identity"). Revisit when
  the run ledger (roadmap step 6.2) needs line-level navigation. Date/Author:
  2026-11-16, planning agent.
- D4. Decision: an `ObligationId` is a section (`forall`, `assume`, `let`,
  `do`, `prove`, `witness`) and a zero-based position, displayed as
  `section[index]`. The display grammar reserves child segments,
  `section[index](.label([index])?)*`, for `let[1].must` (task 5.2.2),
  `do[2].maybe.do[0]` (task 5.2.3), and law children (task 5.4.x), but this
  task implements top-level identities only. Rationale: ADR-0006 decision 6
  fixes this exact textual form; reserving the grammar now keeps later
  identities injective without changing existing ones. Date/Author: 2026-11-16,
  planning agent.
- D5. Decision: lift non-vacuity into a backend-neutral `VacuityPolicy`
  (`RequireWitness` or `AllowVacuous { because }`) on `EvidencePlan`, while
  Kani's unwind bound and expected outcome stay in a backend-indexed
  `KaniEvidencePlan`. Rationale: ADR-0007 invariant 11 makes witness intent and
  vacuity policy theorem semantics and only the mechanism backend-specific.
  Date/Author: 2026-11-16, planning agent.
- D6. Decision: plans exist only through `HarnessPlan::try_new`, which checks
  every plan-level invariant; leaf values and invocations exist only through
  validating constructors; fields are private and accessors return shared
  references. No builder. Rationale: renderers may then rely on well-formedness
  without re-checking ("parse, don't validate"); a builder would add API for no
  current consumer. Date/Author: 2026-11-16, planning agent.
- D7. Decision: binding identity is the author's source identifier
  (`BindingName`), and a plan rejects any name defined twice. Rationale: source
  names are not generated local names, they read well in fixtures and
  diagnostics, and forbidding shadowing removes a class of renderer scoping
  bugs. A renderer that needs fresh Rust locals derives them without changing
  identity. Date/Author: 2026-11-16, planning agent.
- D8. Decision: `ResolvedAction` records the canonical action name and the
  declared signature (ordered parameters with declared types and passing modes,
  and the declared return type). It does not record the mangled Rust path.
  Rationale: ADR-0007 invariant 2 separates semantic primary identity from
  backend binding; the Kani renderer derives `crate::theorem_actions::…__h…`
  through `mangle::mangle_action_name`. Date/Author: 2026-11-16, planning agent.
- D9. Decision: include `InvocationMode` with only `Call`, and `Predicate` with
  only `OpaqueRust`, and omit `Branch` and `Compare` operations. Rationale:
  ADR-0006's minimum repertoire for the vertical slice; the single-variant
  enums name the axes that tasks 5.2.2 and 5.4.x extend, so adding `Must` or a
  structured predicate is additive. Date/Author: 2026-11-16, planning agent.
- D10. Decision: this task provides no conversion from schema types
  (`TheoremDoc`, `ActionSignature`, `ArgValue`) into plan types. Rationale:
  that is the lowering of task 5.1.2; adding it here would pre-empt that task's
  error model. Date/Author: 2026-11-16, planning agent.
- D11. Decision: formally verify only the scope-and-identity kernel with Verus,
  and record the adoption of Verus for repository-internal kernels in a new
  ADR-0014. Rationale: see `Formal verification decision`. Date/Author:
  2026-11-16, planning agent.
- D12. Decision: plan fixtures (the longhand idempotence plan and its
  variants) live in `harness_plan::test_fixtures`, compiled for unit tests and
  behind the existing `test-support` feature, matching `schema::test_fixtures`.
  Rationale: unit tests, the snapshot test, and the behavioural suite share
  them, and task 5.1.3's renderer snapshots will reuse them. Date/Author:
  2026-11-16, planning agent.

## Outcomes & retrospective

Not started.

## Context and orientation

Base revision: `4c059669c0503ff0cb95764283449e27128e7781`. Line numbers below
refer to that revision; relocate symbols with `rg -n '<symbol>' crates`.

### The workspace

`Cargo.toml` at the repository root declares the `theoremc` facade package and
the workspace members `crates/theoremc-core`, `crates/theoremc-macros`,
`crates/theoremc-build-support`, `crates/cargo-theorem`, and
`crates/test-helpers`. The toolchain is pinned in `rust-toolchain.toml`
(`nightly-2026-11-16`); the minimum supported Rust version is 1.89. Workspace
lints in the root `Cargo.toml` deny, among others, `expect_used`, `unwrap_used`,
`indexing_slicing`, `missing_docs`, `missing_docs_in_private_items`,
`missing_assert_message`, `missing_const_for_fn`, `must_use_candidate`, and
`shadow_*`.

### The current pipeline

The root `build.rs` calls `theoremc_build_support::prepare_build_script()`,
which discovers `theorems/**/*.theorem` and writes `OUT_DIR/theorem_suite.rs`
containing one `theorem_file!("<path>");` line per file. The root `src/lib.rs`
includes that suite. The `theorem_file!` macro
(`crates/theoremc-macros/src/lib.rs`, `expand_theorem_file_at`) loads documents
through `theoremc_core::load_theorem_file_from_manifest_dir`, builds a
macro-local `ExpansionPlan` in `crates/theoremc-macros/src/query.rs`
(`plan_expansion`), and renders tokens in
`crates/theoremc-macros/src/render.rs`. The output is a per-file module with
action probes, referenced-type probes, and a `#[cfg(kani)]` module of empty
`#[kani::proof]` harnesses. Nothing consumes `Let`, `Do`, `Assume`, `Prove`, or
`Witness` yet. The root crate's test-only `src/arg_lowering.rs` prototype turns
`ArgValue` into `TokenStream`s; task 5.1.3 promotes it.

### The schema domain model (input to future lowering)

`crates/theoremc-core/src/schema/types.rs` defines `TheoremDoc` with
`theorem: TheoremName`, `forall: IndexMap<ForallVar, String>`,
`actions: IndexMap<CanonicalActionName, ActionSignature>`,
`assume: Vec<Assumption>`, `witness: Vec<WitnessCheck>`,
`let_bindings: IndexMap<String, LetBinding>`, `do_steps: Vec<Step>`,
`prove: Vec<Assertion>`, and `evidence: Evidence`. Expressions
(`Assumption.expr`, `Assertion.assert_expr`, `WitnessCheck.cover`) and Rust
types (`ActionSignature.params: IndexMap<String, String>`,
`ActionSignature.returns: String`) are strings validated with `syn` and then
discarded. `KaniEvidence` has `unwind: u32`, `expect: KaniExpectation`,
`allow_vacuous: bool`, and `vacuity_because: Option<String>`. Argument values
are `schema::ArgValue` (`Literal(LiteralValue)`, `Reference(String)`,
`RawSequence(Vec<TheoremValue>)`, `RawMap(IndexMap<String, TheoremValue>)`);
`LiteralValue` holds `Bool`, `Integer(i64)`, `Float(f64)`, or `String` and is
`PartialEq` but not `Eq`.

Reusable validators already exist and must be reused rather than duplicated:

- `schema::identifier::validate_identifier(&str)` (public) for identifiers;
- `schema::expr::validate_rust_expr(&str)` (`pub(crate)`) for expressions;
- `schema::rust_type::parse` and `parse_with_free_named_lifetime`
  (`pub(crate)`) for Rust types and the free-named-lifetime rule;
- `CanonicalActionName` (`crates/theoremc-core/src/canonical_action_name.rs`)
  for action names;
- `schema::str_newtype::StrNewtype` and the `&str`-backed newtype convention in
  `docs/developers-guide.md` §3.5.1 for string newtypes.

### Naming and identity today

`crates/theoremc-core/src/mangle.rs`, `mangle_path.rs`, and `mangle_harness.rs`
hold pure functions: `mangle_action_name` yields
`crate::theorem_actions::{slug}__h{hash12}`, `mangle_module_path` yields
`__theoremc__file__…__{hash12}`, and `mangle_theorem_harness` yields
`theorem__{slug}__h{hash12(P#T)}`. These are generated Rust names. They must
not appear in the plan. `mangle::theorem_key(path, theorem)` builds `{P}#{T}`;
canonical external theorem IDs with path normalization are roadmap task 4.2.2
and are not part of this task.

### Test infrastructure

Unit tests live in sibling `*_tests.rs` files declared with
`#[cfg(test)] #[path = "…"] mod tests;`. `rstest` 0.28, `googletest` 0.14,
`pretty_assertions` 1.4, and `proptest` 1.11 are `theoremc-core` development
dependencies. `insta` 1.47.2 is used only by `theoremc-macros`; `rstest-bdd`
0.6.0 only by the root package, whose feature files live in `tests/features/`
and are bound with `#[scenario(path = "…", name = "…")]`. Makefile contract
tests live in `tests/lint_makefile_contract.rs` and siblings. `make kani` runs
two Kani scenarios; Kani is pinned in `tools/kani/VERSION` (0.67.0) and
installed by `rust-prover-tools` (`tools/rust-prover-tools/REF`).
`.github/workflows/ proofs.yml` runs `make kani`. Verus is not wired anywhere.

### Gates

- `make check-fmt`: `cargo fmt --all -- --check` and `mdtablefix --check`.
- `make lint`: rustdoc, Clippy, and Whitaker, in that order.
- `make test`: `cargo nextest run --workspace --all-targets --all-features`
  then doctests.
- `make markdownlint`, `make nixie`, `make spelling` for documentation.
- New in `EP-M4`: `make install-verus` and `make verus`.

### Budget survey

The new module family is about a dozen production files of 60 to 150 lines,
each with Rustdoc examples, judged against the nearest analogues
(`schema/newtypes.rs` at about 200 lines for two newtypes, and
`crates/theoremc-macros/src/query.rs`). Completed comparable plans:
`2-2-2-deterministic-harness-naming.md` (12 files, 700 lines, not revised),
`3-2-2-gate-generated-kani-harnesses.md` (10 files, 600 lines, not revised),
`3-3-1-emit-typed-action-probes.md` and `3-3-2-…` (8 files, 500 lines, not
revised; 3-3-1 stopped once for a design gap that became ADR-0004), and
`1-2-4-enforce-non-vacuity-defaults.md` (8 files, 500 lines, not revised).
Their recorded surprises were file-size splits, Kani tooling, and Markdown
formatting gates, which are carried into `Risks` above.

### Signposts

Documents: `docs/roadmap.md` (task 5.1.1 and step 5.1);
`docs/adrs/adr-0006-backend-neutral-harness-planning-boundary.md`;
`docs/adrs/adr-0007-verus-backend-preservation-invariants.md`;
`docs/adrs/adr-0008-workspace-build-support-boundary.md`;
`docs/adrs/adr-0011-vertical-slice-first-roadmap-sequencing.md`;
`docs/adrs/adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md`
(decision 4); `docs/theoremc-design.md` (DES) §§3, 4.3 to 4.6, 5.4, 5.5, 8.1 to
8.4; `docs/theorem-file-specification.md` (TFS) §§2.3, 2.4, 3.6 to 3.11, 4, 5,
6, 7.4; `docs/rfcs/rfc-0001-semantic-law-templates-over-composable-primaries.md`
("Lowering architecture", "Stable sub-obligation identities");
`docs/developers-guide.md` §§2, 3.3, 3.5.1, 3.6.1, 3.9;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rstest-bdd-users-guide.md`;
`docs/rust-doctest-dry-guide.md`;
`docs/reliable-testing-in-rust-via-dependency-injection.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md`;
`docs/documentation-style-guide.md` (ADR template);
`docs/ortho-config-users-guide.md` (not affected: no configuration surface is
added).

Skills: `rust-router` then `rust-types-and-apis` (newtypes, smart constructors),
`rust-errors` (error enums), `rust-unit-testing` (rstest, googletest, insta),
`proptest`, `verus` (with its `project-on-ramp` reference), `rust-verification`,
`hexagonal-architecture` (pure kernel and adapter split),
`addressing-whitaker-findings`, `execplans`, `commit-message`, `pr-creation`,
and `en-gb-oxendict-style`.

## Conformance basis

Upstream artefacts at base revision `4c05966`:

- `docs/roadmap.md` task 5.1.1 and step 5.1.
- ADR-0006 (accepted 2026-08-19, addendum 2026-11-15), decisions 1 to 6, 9,
  10, and "Minimum plan repertoire".
- ADR-0007 (accepted 2026-08-19), invariants 2, 6, 7, 9, 11, and 12.
- ADR-0008 (accepted 2026-10-11), the build-support boundary.
- ADR-0011 (accepted 2026-08-19), decisions 1, 2, and 7.
- ADR-0001 decision 4 (non-vacuity witness policy).
- DES §§3, 8.1 to 8.4; TFS §§2.3, 3.6 to 3.11, 4.1, 7.4.
- `AGENTS.md` and `docs/developers-guide.md` conventions.

No Terms of Reference document exists for this repository; the roadmap and ADRs
are the governing artefacts.

Requirements:

- `REQ-location`: the plan lives in `theoremc-core` as an internal module
  (ADR-0006 d2; roadmap 5.1; ADR-0008).
- `REQ-invocation`: action use is represented by one resolved-invocation type
  carrying the action identity, declared signature, arguments in declared
  parameter order with expected types and passing modes, invocation mode, and
  optional result binding (ADR-0006 d4, d5).
- `REQ-provenance`: the plan records the source theorem and the source
  position of every operation (ADR-0006 d1; ADR-0011 acceptance).
- `REQ-identity`: every operation has a deterministic structural identity in
  ADR-0006's textual form, independent of generated names (ADR-0006 d6;
  ADR-0007 inv 9).
- `REQ-opaque`: opaque Rust assumptions, assertions, and covers are explicit
  `OpaqueRust` predicate nodes (ADR-0006 d3; ADR-0007 inv 6).
- `REQ-backend-free`: the plan contains no token streams, Kani syntax, or
  generated local names as identity (roadmap success; ADR-0007 inv 7).
- `REQ-repertoire`: the plan supports typed symbolic binding, opaque
  assumption, resolved infallible invocation with result binding, opaque
  assertion, opaque cover, stable identity, and Kani unwind evidence, and
  nothing more (ADR-0006 "Minimum plan repertoire").
- `REQ-vacuity`: the plan preserves witness intent and the vacuity policy as
  semantics (ADR-0007 inv 11; ADR-0001 d4).
- `REQ-no-adapters`: the plan introduces no implicit clones, defaults, or
  coercions (ADR-0006 d5; ADR-0007 inv 12).

Gaps from the baseline:

- `GAP-no-plan`: no harness-plan type exists.
- `GAP-no-identity`: no structural obligation identity exists.
- `GAP-no-provenance`: theorem path and document index are not retained beyond
  the loader.
- `GAP-binding-uniqueness`: nothing rejects a binding name defined twice.
- `GAP-no-verus`: the repository has no Verus tooling or proofs.

Trace links:

```plaintext
ADR-0006 d2 -> REQ-location -> EP-M1 -> harness_plan module exists; boundary test
ADR-0006 d4,d5 -> REQ-invocation -> EP-M2 -> invocation_tests::arguments_follow_declared_order
ADR-0006 d1 -> REQ-provenance -> EP-M1 -> identity_tests::theorem_identity_round_trips
ADR-0006 d6 -> REQ-identity -> EP-M1 -> identity_tests::display_table; identity property test
ADR-0006 d3 -> REQ-opaque -> EP-M2 -> predicate_tests; plan snapshot
roadmap 5.1.1 -> REQ-backend-free -> EP-M3 -> plan snapshot; marker scan; boundary test
ADR-0006 repertoire -> REQ-repertoire -> EP-M2 -> operation_tests
ADR-0007 inv 11 -> REQ-vacuity -> EP-M3 -> well_formed_tests::witness_policy_table
ADR-0007 inv 12 -> REQ-no-adapters -> EP-M2 -> invocation_tests::value_is_preserved_verbatim
GAP-binding-uniqueness -> INV-unique-binding -> EP-M3, EP-M4 -> property test; Verus lemma
GAP-no-verus -> ADR-0014 -> EP-M4 -> make verus
```

At each milestone boundary, check: the assigned requirements hold; nothing in
`harness_plan` depends on `proc_macro2`, `quote`, or Kani; no public item
outside `harness_plan` changed; no new dependency appeared beyond the
authorized list; and the trace links above still name real tests.

## Verification plan

### Abstract model

A harness plan is a triple `(T, E, ops)`. `T` is the theorem identity
`(path, document_index, theorem_name)`. `E` is evidence: a vacuity policy
`V ∈ {RequireWitness, AllowVacuous(r)}` with `r` a non-empty rationale, and an
optional Kani evidence `(unwind, expect)`. `ops` is a finite sequence
`o_0 … o_{n-1}` of operations.

Each operation `o_i` has:

- an identity `id(o_i)`: a section and a position;
- a defined binding `def(o_i)`: the symbolic variable of a `Symbolic`
  operation, the result binding of an `Invoke` operation if any, otherwise none;
- a set of used bindings `uses(o_i)`: every binding referenced, at any depth,
  by the plan values of an `Invoke` operation's arguments, otherwise empty.
  Opaque predicates contribute no uses: the plan does not interpret Rust text
  (see residual gap `GAP-R1`).

Well-formedness `WF(T, E, ops)` is the conjunction of:

- scope: for every `i` and every `b ∈ uses(o_i)`, there exists `j < i` with
  `def(o_j) = b` (strictly earlier, so `let x = f(&x)` is ill-formed);
- unique definitions: for all `i < j`, if `def(o_i)` and `def(o_j)` are both
  present then `def(o_i) ≠ def(o_j)`;
- unique identities: for all `i < j`, `id(o_i) ≠ id(o_j)`;
- witness: if `V = RequireWitness` then some `o_i` is a `Cover`.

Leaf validity (enforced by constructors, before a plan exists): binding names
are Rust identifiers and not keywords; expressions parse as a single
`syn::Expr` and are not statement-like; declared types parse as `syn::Type`
without a free named lifetime; rationales are non-empty after trimming; an
action's parameter names are distinct identifiers; an invocation supplies
exactly one argument per declared parameter and none other, and stores them in
declared order.

Result contract: `HarnessPlan::try_new(T, E, ops)` returns `Ok(plan)` exactly
when `WF(T, E, ops)` holds, and the plan exposes `T`, `E`, and `ops` unchanged.
Otherwise it returns the first violation in operation order, where within one
operation an undefined use (in argument order, depth-first) is reported before
a duplicate definition, which is reported before a duplicate identity; a
missing witness is reported only when the sequence scan finds no violation.

### Invariants and lemmas

- `INV-scope` (from `REQ-invocation`, `REQ-backend-free`): every binding
  used by an invocation is defined by a strictly earlier operation in an
  accepted plan.
- `INV-unique-binding` (from `GAP-binding-uniqueness`, decision D7): no
  binding is defined twice in an accepted plan.
- `INV-unique-identity` (from `REQ-identity`): no two operations in an accepted
  plan share an identity.
- `INV-witness` (from `REQ-vacuity`): an accepted plan with
  `RequireWitness` contains at least one `Cover`; `AllowVacuous` always carries
  a non-empty rationale (type-level).
- `INV-arguments` (from `REQ-invocation`): a resolved invocation's arguments
  correspond one-to-one with the action's declared parameters and appear in
  declared order, whatever order the caller supplied.
- `INV-passing-mode` (from `REQ-invocation`): a parameter declared `&mut T`
  has passing mode `MutableBorrow`, `&T` (including `&'static T`) has
  `SharedBorrow`, and every other type has `Move`; nested references such as
  `Box<&mut T>` are `Move`.
- `INV-identity-display` (from `REQ-identity`): identity display is
  deterministic and injective: distinct identities display distinctly, and the
  display grammar parses back to the same identity.
- `INV-leaf-validity`: invalid names, expressions, types, empty rationales,
  duplicate parameters, missing arguments, and unknown arguments are rejected
  with a typed error and never produce a value.
- `INV-opaque` (from `REQ-opaque`): every predicate is `OpaqueRust`. This is
  type-level: `Predicate` has no other variant.
- `INV-backend-free` (from `REQ-backend-free`): the debug rendering of every
  plan fixture contains none of `kani::`, `#[kani`, `TokenStream`, `theorem__`,
  `__theoremc__`, `crate::theorem_actions`, or `__h` followed by hexadecimal;
  and the `harness_plan` source tree imports none of `proc_macro2`, `quote`, or
  `kani`.
- `INV-no-adapters` (from `REQ-no-adapters`): plan values are stored exactly
  as supplied; construction never inserts, clones into, or defaults a value.
- `INV-immutable`: an accepted plan cannot be mutated (type-level: private
  fields, shared-reference accessors only).

Lemmas over the scope-and-identity kernel. The production kernel is a pure
function, `first_violation(events) -> Option<(usize, Violation)>`, over a
sequence of `ScopeEvent { defines, uses, identity }` values, threading the set
of defined bindings and the set of seen identities; `HarnessPlan::try_new` maps
operations to events (the adapter) and checks the witness clause afterwards.

- `L-step` (prefix extension): `wf(s ++ [e])` holds if and only if `wf(s)`
  holds and `e` is admissible against the definitions and identities of `s`
  (its uses are all defined in `s`, its definition is not defined in `s`, and
  its identity is not an identity of `s`).
- `L-sound`: if `first_violation(s)` is none, then `wf(s)`.
- `L-complete`: if `wf(s)`, then `first_violation(s)` is none.
- `L-first`: if `first_violation(s) = Some((i, v))`, then the prefix of `s`
  of length `i` is well-formed, the prefix of length `i + 1` is not, and `v`
  names the first clause that `s[i]` violates under the priority order above.
- `L-abstraction`: the kernel inspects bindings and identities only through
  equality, so its result is invariant under any injective renaming of bindings
  and identities. This justifies proving the kernel over `nat` keys.
- `L-adapter`: for each operation kind, the adapter's `defines` and `uses`
  equal `def(o)` and `uses(o)` of the abstract model, including references
  nested at any depth in lists and record fields.
- `L-witness`: `try_new` rejects with `MissingWitness` exactly when the scan
  is clean, the policy is `RequireWitness`, and no operation is a `Cover`.

These lemma statements are hypotheses about the proof structure. Proof
engineering (auxiliary set lemmas, triggers, induction helpers) is deferred to
`EP-M4`, where the verification conditions become visible; any statement that
turns out to be unnecessary or needs decomposition is revised here with a
`Decision log` entry.

### Axioms (trusted, not verified here)

- `AXIOM-syn`: `syn::parse_str::<syn::Expr>` and `syn::parse_str::<syn::Type>`
  accept exactly the Rust expression and type grammar relevant to validation.
  Source: `syn` 3 documentation. Boundary: `RustExpression::new`,
  `DeclaredType::new`.
- `AXIOM-std-collections`: `BTreeSet` or `HashSet` membership and insertion
  behave as sets with respect to `Eq` and `Ord`/`Hash`. Source: standard
  library documentation. Boundary: the production kernel.
- `AXIOM-indexmap`: `IndexMap` iteration follows insertion order. Source:
  `indexmap` documentation. Boundary: argument matching input.
- `AXIOM-verus`: Verus and its solver are sound, and `vstd` sequence and set
  axioms hold. Source: Verus documentation. Boundary: `verus/` proofs.
- `AXIOM-model-correspondence`: the Verus model's `check` function mirrors the
  production kernel's loop and violation priority. This is established by
  review and by the differential property test, not mechanically; it is the
  bridge named in the `verus` skill's on-ramp. Boundary:
  `verus/harness_plan_scope.rs` and `harness_plan/well_formed.rs`.

### Methods per obligation

- Obligation: `INV-scope`, `INV-unique-binding`, `INV-unique-identity`
  through `L-step`, `L-sound`, `L-complete`, and `L-first`. Method: Verus proof
  over an abstract `Seq<Event>` model, combined with a differential property
  test of the production kernel against an independent oracle, plus named
  `rstest` cases. Rationale: the property must hold for sequences of any
  length, and later tasks (nested `maybe` scopes, law children) extend exactly
  this kernel; a proof over a small pure kernel is tractable and durable. The
  property test connects the proof to the production code. Domain: Verus, all
  finite sequences of events over `nat` keys; proptest, sequences of 0 to 12
  events over a pool of 4 binding names and 6 identities, which forces
  collisions, forward references, and self-references. Artefact:
  `verus/harness_plan_scope.rs`;
  `crates/theoremc-core/src/harness_plan/well_formed_property_tests.rs`;
  `crates/theoremc-core/src/harness_plan/well_formed_tests.rs`. Evidence:
  `make verus` exits 0 with every lemma verified and no `assume`, `admit`, or
  `external_body` in the file (`rg -n 'assume|admit|external_body' verus/`
  prints nothing). The property test compares `first_violation` with an oracle
  that evaluates the quantified `WF` definition directly (for each position,
  search the prefix) and computes the expected first violation by scanning
  prefixes, sharing no code with the kernel.
  `cargo nextest run -p theoremc-core harness_plan` passes. Non-vacuity: Verus
  includes two witness lemmas, one proving a concrete three-event sequence
  well-formed and the checker accepting it, and one proving a concrete
  forward-reference sequence rejected at index 1, so the antecedents of
  `L-sound` and `L-first` are inhabited. The property test records how many
  generated cases fall into each class (accepted, undefined use,
  self-reference, duplicate definition, duplicate identity) through a companion
  `rstest` that runs the same strategy with a fixed seed for 512 cases and
  asserts that every class occurs at least 10 times. Negative controls:
  temporarily change the production kernel to insert the definition before
  checking uses; the named test `self_reference_is_rejected` and the property
  test must fail. Temporarily make the same change in the Verus `check`
  function; `make verus` must fail on `L-complete` or `L-sound`. Revert both
  and record the outputs in `Artefacts and notes`.
- Obligation: `L-abstraction`.
  Method: argument recorded in the design document and ADR-0014, plus the
  property test, which uses string names and therefore exercises a concrete
  injective renaming. Rationale: the kernel uses only `==` and set membership;
  a mechanized proof of renaming invariance adds proof machinery for no
  additional assurance over the differential test. Residual gap: recorded as
  `AXIOM-model-correspondence`.
- Obligation: `L-adapter`.
  Method: `rstest` table over every operation kind and every value shape
  (literal, top-level binding, binding inside a list, binding inside a record
  field, binding inside a list inside a record). Artefact:
  `well_formed_tests.rs`. Non-vacuity: the negative control "skip nested
  values" (adapter collects only top-level bindings) must make the
  nested-reference cases fail.
- Obligation: `INV-witness` and `L-witness`.
  Method: `rstest` table of the four combinations of policy and cover presence,
  plus a case where a scan violation and a missing witness coexist (the scan
  violation wins). Artefact: `well_formed_tests.rs`; behavioural scenarios.
  Non-vacuity: the table includes the rejecting combination.
- Obligation: `INV-arguments`.
  Method: named `rstest` cases (declared order preserved, unknown argument,
  missing argument, duplicate parameter in the action) and a property test: for
  any permutation of a complete argument map over 0 to 6 distinct parameters,
  the resolved order equals the declared order and the values are carried
  unchanged. Artefact:
  `crates/theoremc-core/src/harness_plan/invocation_tests.rs`. Non-vacuity: the
  generator includes empty and single-parameter actions and non-identity
  permutations; the negative control "keep caller order" must fail the property.
- Obligation: `INV-passing-mode`.
  Method: `rstest` table: `&mut T`, `&T`, `&'static str`, `T`, `Box<&mut T>`,
  `Option<&T>`, `[u8; 4]`, `&[u8]`, `&mut [u8]`, `()`. Artefact:
  `crates/theoremc-core/src/harness_plan/declared_type_tests.rs`.
- Obligation: `INV-identity-display`.
  Method: `rstest` display table for every section; property test that a
  test-local parser for the display grammar (written in the test module,
  independent of the `Display` implementation) recovers the identity, which
  implies injectivity on the generated domain, and that distinct generated
  identities display distinctly. Artefact:
  `crates/theoremc-core/src/harness_plan/identity_tests.rs`. Rationale: the
  grammar is regular and the alphabets are disjoint (section names are
  lower-case letters, positions are decimal digits, delimiters are `[`, `]`, and
  `.`); a property test over all sections and positions up to `u32::MAX` gives
  adequate assurance, and a Verus proof would require decimal-string lemmas out
  of proportion to the risk.
- Obligation: `INV-leaf-validity`.
  Method: `rstest` accept and reject tables per constructor, asserting typed
  error variants with `googletest` matchers. Artefact: sibling `*_tests.rs`
  files.
- Obligation: `INV-backend-free`.
  Method: an `insta` debug snapshot of the longhand idempotence plan; a
  marker-scan assertion over the same debug text; a boundary test that reads
  every file under `crates/theoremc-core/src/harness_plan/` with
  `cap_std::fs_utf8` and rejects `use` of `proc_macro2`, `quote`, or `kani`,
  and the identifiers `TokenStream`, `kani::`, and `#[kani`. Artefact:
  `crates/theoremc-core/src/harness_plan/plan_snapshot_tests.rs`;
  `crates/theoremc-core/tests/harness_plan_boundary.rs`. Non-vacuity: a unit
  test feeds the marker scan a contaminated string containing each forbidden
  marker and requires each to be reported; a unit test feeds the boundary
  scanner a synthetic source line `use quote::quote;` and requires rejection.
- Obligation: `INV-no-adapters`, `INV-immutable`, `INV-opaque`.
  Method: type-level (private fields, no `&mut` accessors, single `Predicate`
  variant) plus one `rstest` case comparing supplied and stored values with
  `assert_eq!`.

### Formal verification decision

Verus is warranted for one kernel only: the scope-and-identity check (`L-step`,
`L-sound`, `L-complete`, `L-first`). It is the contract on which every renderer
relies to emit only defined locals and distinct obligation labels; it must hold
for plans of any length; it is a small, stable, pure function; and tasks 5.2.3
(nested `maybe` scopes) and 5.4.x (law sub-obligations) will extend it, so an
unbounded proof now pays for itself. The proof is over a model, so the plan
pairs it with a differential property test against an independent oracle, and
records the model correspondence as an explicit axiom.

Verus is not warranted for identity display (regular grammar; property test
suffices), argument matching (finite permutation property; property test
suffices), passing-mode classification (finite table), leaf validation
(delegates to `syn`, an axiom), or the backend-free property (structural and
textual). Kani adds nothing here: there is no `unsafe` code, no arithmetic
beyond indexing, and the bounded state space is already covered by the property
tests. Kani remains the backend under test from task 5.1.4.

Residual gaps:

- `GAP-R1`: opaque predicates may name bindings that are undefined or defined
  later; the plan cannot see inside Rust text. rustc reports this when the Kani
  renderer's output compiles (task 5.1.3 onward).
- `GAP-R2`: moves after by-value passing and `&mut` aliasing are not checked by
  the plan; rustc reports them as theorem authoring errors, as DES §5.5.3
  requires.
- `GAP-R3`: the Verus model's correspondence with the production kernel is
  established by review and differential testing, not mechanically.

## Plan of work

The module layout, under `crates/theoremc-core/src/harness_plan/`:

- `mod.rs`: module documentation (purpose, internal status, reuse policy),
  re-exports, `HarnessPlan`, `HarnessPlan::try_new`, accessors.
- `identity.rs`: `TheoremIdentity`, `PlanSection`, `ObligationId`, and
  `Display`.
- `binding.rs`: `BindingName`.
- `value.rs`: `PlanValue` (`Literal(LiteralValue)`, `Binding(BindingName)`,
  `List(Vec<PlanValue>)`, `Record(Vec<RecordField>)`) and `RecordField`.
- `declared_type.rs`: `DeclaredType` and `PassingMode`.
- `predicate.rs`: `RustExpression`, `Rationale`, `Predicate`,
  `PredicateObligation`.
- `invocation.rs`: `ActionParameter`, `ResolvedAction`, `InvocationMode`,
  `ResolvedArgument`, `ResolvedInvocation`.
- `operation.rs`: `SymbolicBinding`, `Operation`.
- `evidence.rs`: `EvidencePlan`, `VacuityPolicy`, `KaniEvidencePlan`.
- `well_formed.rs`: `ScopeEvent`, `Violation`, `first_violation`, and the
  operation-to-event adapter.
- `error.rs`: `PlanValueError` (leaf and invocation construction) and
  `PlanError` (plan well-formedness), both `thiserror` enums with small,
  boxed-where-large payloads.
- `test_fixtures.rs`: `#[cfg(any(test, feature = "test-support"))]`
  `#[doc(hidden)]` fixtures.
- Sibling tests: `identity_tests.rs`, `binding_tests.rs`,
  `declared_type_tests.rs`, `predicate_tests.rs`, `invocation_tests.rs`,
  `well_formed_tests.rs`, `well_formed_property_tests.rs`,
  `plan_snapshot_tests.rs`, and `snapshots/`.

### Milestone `EP-M0`: baseline

Run the three gates on the unmodified branch through the `scrutineer` agent and
record pass counts and any pre-existing failure (notably the two Kani
scenarios) in `Surprises & discoveries`. No commit unless this plan changes.
Acceptance: logs
`/tmp/{check-fmt,lint,test}-theoremc-5-1-1-internal-backend-neutral-harness-plan.out`
exist and their results are recorded.

### Milestone `EP-M1`: identity, provenance, and leaf values

Assigned: `REQ-location`, `REQ-provenance`, `REQ-identity`; `GAP-no-plan`
(module skeleton), `GAP-no-identity`, `GAP-no-provenance`.

Red: create `harness_plan/mod.rs` with only module documentation and the module
declarations, then write the failing tests first in `identity_tests.rs`,
`binding_tests.rs`, `declared_type_tests.rs`, and `predicate_tests.rs` (display
table, theorem identity accessors, identifier validation, passing-mode table,
expression accept and reject table, rationale trimming). Run
`cargo nextest run -p theoremc-core harness_plan` and observe compile failure
for the missing items. Green: implement `identity.rs`, `binding.rs`,
`declared_type.rs`, `predicate.rs`, and the leaf variants of `error.rs`, reusing
`validate_identifier`, `validate_rust_expr`, and `rust_type::parse` /
`parse_with_free_named_lifetime`. Refactor. Add the identity property test.

End state: leaf types exist and are tested; `harness_plan` is declared
`pub mod` in `crates/theoremc-core/src/lib.rs` with module documentation
stating its internal status. Acceptance: `make check-fmt`, `make lint`, and
`make test` pass; new tests named in the trace links pass. Commit: "Add
harness-plan identity and leaf values". Recovery: revert the commit. Remaining
gaps: invocations, operations, plan.

### Milestone `EP-M2`: resolved invocations, operations, and evidence

Assigned: `REQ-invocation`, `REQ-opaque`, `REQ-repertoire`, `REQ-no-adapters`.

Red: write `invocation_tests.rs` (declared order, unknown and missing
arguments, duplicate parameters, values carried verbatim, result binding,
argument permutation property) and operation and evidence cases. Green:
implement `value.rs`, `invocation.rs`, `operation.rs`, `evidence.rs`, and the
invocation error variants. `ResolvedInvocation::try_new` takes the site
identity, a `ResolvedAction`, the caller's arguments as an
`IndexMap<BindingName, PlanValue>` keyed by parameter name, the
`InvocationMode`, and an optional result `BindingName`.

End state: every operation of ADR-0006's minimum repertoire is representable.
Acceptance: three gates pass. Commit: "Add resolved invocations and
harness-plan operations". Recovery: revert.

### Milestone `EP-M3`: well-formed `HarnessPlan` with executable verification

Assigned: `INV-scope`, `INV-unique-binding`, `INV-unique-identity`,
`INV-witness`, `REQ-backend-free`, `REQ-vacuity`; `GAP-binding-uniqueness`.

Red: write `well_formed_tests.rs` (named cases, including
`self_reference_is_rejected`, forward reference, nested references, duplicate
binding across symbolic and invocation results, duplicate identity, witness
table, violation priority), the oracle and differential property test in
`well_formed_property_tests.rs`, the class-coverage `rstest`, and the
behavioural feature with failing step definitions. Green: implement
`well_formed.rs` (pure kernel plus adapter), `HarnessPlan::try_new`, the plan
error variants, and `test_fixtures.rs`. Then add `plan_snapshot_tests.rs` with
the `insta` debug snapshot and marker scan, and
`crates/theoremc-core/tests/harness_plan_boundary.rs`. Add `insta`,
`rstest-bdd`, and `rstest-bdd-macros` to `theoremc-core`'s development
dependencies, and a `[[test]]` entry with
`required-features = ["test-support"]` for the behavioural suite. Run the
negative controls for the kernel, adapter, argument order, and marker scan;
record their failing output in `Artefacts and notes`; revert them.

End state: an ill-formed plan cannot be constructed; the longhand idempotence
fixture is accepted and snapshotted. Acceptance: three gates pass;
`cargo insta test -p theoremc-core --check` reports no pending snapshots.
Commit: "Enforce harness-plan well-formedness". Recovery: revert.

### Milestone `EP-M4`: Verus proof of the scope kernel and proof tooling

Assigned: `L-step`, `L-sound`, `L-complete`, `L-first`; `GAP-no-verus`.

Steps: add `tools/verus/VERSION` naming the newest Verus release with an
`x86-linux` archive (at planning time, `0.2026.11.02.8bf68ee`, whose asset is
`verus-0.2026.11.02.8bf68ee-x86-linux.zip`; confirm at implementation),
`tools/verus/SHA256SUMS` with the digest of that archive computed locally, and
`tools/verus/README.md`. Add `install-verus` and `verus` targets to the
`Makefile` that delegate to `$(PROVER_TOOLS) verus install --repo-root .` and
`$(PROVER_TOOLS) verus run --repo-root . --proof-file verus/harness_plan_scope.rs`.
Extend the Makefile contract tests to assert that both recipes delegate through
`$(PROVER_TOOLS)` and contain no `curl`, `unzip`, `sha256sum`, or
`rustup toolchain install`. Add a `verus` job to `.github/workflows/proofs.yml`
using the shared `install-prover-tools` action with `tools: verus`, and add
`verus/**` and `tools/verus/**` to its path filters. Write
`verus/harness_plan_scope.rs`: spec types mirroring `ScopeEvent` and
`Violation`, a spec `wf`, a spec `check` mirroring `first_violation`, the
lemmas, and the two witness lemmas; each item's `///` comment names the
production item it models. Run the Verus negative control and record it.

Acceptance: `make install-verus` then `make verus` exit 0; the trusted-surface
scan prints nothing; three gates pass. Commit: "Prove harness-plan scope kernel
with Verus". Recovery: revert; the executable evidence from `EP-M3` stands.

### Milestone `EP-M5`: documentation, ADR, and closure

Assigned: documentation requirements from `AGENTS.md`.

- Add ADR-0014, "Verus proofs for repository-internal pure kernels",
  following the ADR template in `docs/documentation-style-guide.md`: when Verus
  is used, the spec-mirror bridge, the differential-test companion, the
  trusted-surface rule, tooling through `rust-prover-tools`, and keeping
  `make verus` out of `make test`.
- Add `docs/theoremc-design.md` §8.6 "Harness plan (Task 5.1.1)": the plan
  shape, the identity grammar (decision D4), structural provenance (D3), opaque
  predicates (D2), vacuity lift (D5), binding identity (D7), the
  well-formedness contract and its proof obligations, and residual gaps. Update
  §3 and the §3.1 diagram labels so the backend-agnostic plan reads as
  implemented and lowering as planned.
- Replace "Accepted harness-planning boundary (pending implementation)" in
  `docs/developers-guide.md` §2 with the implemented internal interface, its
  re-use policy (permitted call sites: `theoremc-core` lowering and the
  `theoremc-macros` renderer; no facade re-export; extend by adding operation
  variants and identity children, not parallel types), the proof inventory and
  `make verus`, and add the plan to the §3.6.1 table.
- Update `docs/repository-layout.md` and `docs/contents.md` (ADR-0014 and
  this ExecPlan).
- `docs/users-guide.md`: no change, because nothing user-visible changes; say
  so in the PR.
- Mark roadmap task 5.1.1 done (`- [x]`) only after every acceptance item
  passes.

Acceptance: `make fmt`, `make check-fmt`, `make markdownlint`, `make nixie`,
`make spelling`, `make lint`, `make test`, and `make verus` pass;
`mdtablefix --check` passes on each changed Markdown file. Commit: "Document
the harness plan and Verus kernel proofs". Then set this plan's status to
`COMPLETE` after reconciling every discovery with the conformance basis.

## Concrete steps

Run all commands from the repository root. Route full gate runs through the
`scrutineer` agent, which tees output to
`/tmp/$ACTION-theoremc-5-1-1-internal-backend-neutral-harness-plan.out`.

```sh
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-5-1-1-internal-backend-neutral-harness-plan.out
make lint 2>&1 | tee /tmp/lint-theoremc-5-1-1-internal-backend-neutral-harness-plan.out
make test 2>&1 | tee /tmp/test-theoremc-5-1-1-internal-backend-neutral-harness-plan.out
```

Focused loop during red-green-refactor:

```sh
cargo nextest run -p theoremc-core --all-features harness_plan
cargo nextest run -p theoremc-core --all-features --test harness_plan_bdd
cargo insta test -p theoremc-core --all-features --check
```

Proof milestone:

```sh
make install-verus
make verus 2>&1 | tee /tmp/verus-theoremc-5-1-1-internal-backend-neutral-harness-plan.out
rg -n 'assume|admit|external_body' verus/
```

Expected focused-test output after `EP-M3` (counts are illustrative):

```plaintext
Summary [  1.2s] 60 tests run: 60 passed, 0 skipped
```

Expected `make verus` tail after `EP-M4`:

```plaintext
verification results:: N verified, 0 errors
```

## Validation and acceptance

Behavioural specification, at
`crates/theoremc-core/tests/features/harness_plan.feature`, bound by
`crates/theoremc-core/tests/harness_plan_bdd.rs` (traces: `REQ-identity`,
`REQ-invocation`, `REQ-vacuity`, `REQ-backend-free`; milestone `EP-M3`):

```gherkin
Feature: Backend-neutral harness plan
  Theoremc normalizes validated theorem meaning into an internal harness plan
  before any backend renders code. A plan records resolved invocations, source
  provenance, stable obligation identities and opaque Rust predicates, and it
  cannot be constructed in an ill-formed state.

  Scenario: A longhand idempotence plan is accepted
    Given the longhand idempotence plan operations
    When the harness plan is constructed
    Then construction succeeds
    And the operation identities are "forall[0], assume[0], let[0], let[1], prove[0], witness[0]"
    And the plan description contains no backend syntax or generated names

  Scenario: A binding used before its definition is rejected
    Given the longhand idempotence plan operations
    And the two invocations are swapped
    When the harness plan is constructed
    Then construction fails because binding "once" is used at "let[1]" before it is defined

  Scenario: A binding defined twice is rejected
    Given the longhand idempotence plan operations
    And the second invocation binds its result to "x"
    When the harness plan is constructed
    Then construction fails because binding "x" is defined again at "let[1]"

  Scenario: A plan without a witness is rejected by default
    Given the longhand idempotence plan operations without its witness
    When the harness plan is constructed
    Then construction fails because no witness covers the plan

  Scenario: A justified vacuous plan is accepted
    Given the longhand idempotence plan operations without its witness
    And vacuity is allowed because "the claim is a pure algebraic identity"
    When the harness plan is constructed
    Then construction succeeds

  Scenario: Invocation arguments follow the declared parameter order
    Given an action "demo.combine" declaring parameters "left" then "right"
    When an invocation supplies "right" before "left"
    Then the resolved arguments are ordered "left, right"

  Scenario: An argument for an undeclared parameter is rejected
    Given an action "demo.combine" declaring parameters "left" then "right"
    When an invocation supplies "left", "right" and "extra"
    Then resolution fails because "extra" is not a parameter of "demo.combine"
```

The longhand idempotence fixture is: `forall[0]` symbolic `x: u8`; `assume[0]`
opaque `x <= 200` because "keep the domain small"; `let[0]`
`once = demo.normalize(value: x)` with `value: &u8` (shared borrow) returning
`u8`; `let[1]` `twice = demo.normalize(value: once)`; `prove[0]` opaque
`once == twice` because "normalizing twice equals normalizing once";
`witness[0]` opaque `once != x` because "some input changes under
normalization"; evidence `RequireWitness` with Kani unwind 1 and expectation
`Success`. It matches the example ADR-0011 decision 6 prefers, so task 5.1.4
and the later RFC 0001 plan-equivalence fixture can reuse it.

Acceptance, as observable behaviour:

- `cargo nextest run -p theoremc-core --all-features harness_plan` passes and
  includes the named tests in the trace links; each fails when the
  corresponding negative control is applied.
- The behavioural suite passes all seven scenarios.
- The snapshot shows the six operations with the identities above, the binding
  names `x`, `once`, and `twice`, the canonical action name `demo.normalize`,
  the declared types, the `SharedBorrow` passing mode, and the opaque
  expression texts, and none of the forbidden markers.
- `make verus` verifies every lemma with an empty trusted surface.
- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, and `make spelling` pass.

## Idempotence and recovery

Every step is re-runnable. Each milestone ends in a commit that passes the
gates, so `git revert <commit>` restores the previous plateau. `EP-M4` can be
reverted alone without affecting `EP-M1` to `EP-M3`. Negative controls are
temporary edits: apply, observe the failure, then restore with
`git checkout -- <file>` and confirm `git status` is clean before continuing. If
`prover-tools verus install` fails halfway, delete the `.verus/` install
directory it created and retry once before applying the Verus tooling
tolerance. Do not use `git stash` (the stash stack is shared between worktrees).

## Artefacts and notes

Record here, during implementation: baseline gate results; negative-control
outputs for the kernel, adapter, argument order, marker scan, and Verus model;
the chosen Verus release and digest; and the final `make verus` summary.

## Interfaces and dependencies

All items live in `theoremc_core::harness_plan` and are `pub` for the macro
crate's later use, documented as internal. Indicative signatures, to be refined
only within the tolerances:

```rust,no_run
pub struct TheoremIdentity { /* path: Utf8PathBuf, document_index: usize, name: TheoremName */ }
impl TheoremIdentity {
    pub fn new(path: Utf8PathBuf, document_index: usize, name: TheoremName) -> Self;
    pub fn path(&self) -> &Utf8Path;
    pub const fn document_index(&self) -> usize;
    pub const fn name(&self) -> &TheoremName;
}

pub enum PlanSection { Forall, Assume, Let, Do, Prove, Witness }
pub struct ObligationId { /* section: PlanSection, position: usize */ }
impl ObligationId {
    pub const fn new(section: PlanSection, position: usize) -> Self;
    pub const fn section(&self) -> PlanSection;
    pub const fn position(&self) -> usize;
}
impl fmt::Display for ObligationId { /* "prove[0]" */ }

pub struct BindingName(String);          // Rust identifier, not a keyword
pub struct DeclaredType { /* text: String, passing: PassingMode */ }
pub enum PassingMode { Move, SharedBorrow, MutableBorrow }
pub struct RustExpression(String);       // trimmed, parses as one syn::Expr
pub struct Rationale(String);            // non-empty after trimming

pub enum PlanValue {
    Literal(LiteralValue),
    Binding(BindingName),
    List(Vec<PlanValue>),
    Record(Vec<RecordField>),
}
pub struct RecordField { /* name: BindingName, value: PlanValue */ }

pub struct ActionParameter { /* name: BindingName, declared: DeclaredType */ }
pub struct ResolvedAction { /* name: CanonicalActionName, parameters: Vec<ActionParameter>, returns: DeclaredType */ }
impl ResolvedAction {
    pub fn try_new(
        name: CanonicalActionName,
        parameters: Vec<ActionParameter>,
        returns: DeclaredType,
    ) -> Result<Self, PlanValueError>;
}
pub enum InvocationMode { Call }
pub struct ResolvedArgument { /* parameter: ActionParameter, value: PlanValue */ }
pub struct ResolvedInvocation { /* site, action, arguments, mode, result */ }
impl ResolvedInvocation {
    pub fn try_new(
        site: ObligationId,
        action: ResolvedAction,
        arguments: IndexMap<BindingName, PlanValue>,
        mode: InvocationMode,
        result: Option<BindingName>,
    ) -> Result<Self, PlanValueError>;
}

pub enum Predicate { OpaqueRust(RustExpression) }
pub struct PredicateObligation { /* id: ObligationId, predicate: Predicate, rationale: Rationale */ }
pub struct SymbolicBinding { /* site: ObligationId, binding: BindingName, declared: DeclaredType */ }
pub enum Operation {
    Symbolic(SymbolicBinding),
    Assume(PredicateObligation),
    Invoke(ResolvedInvocation),
    Assert(PredicateObligation),
    Cover(PredicateObligation),
}

pub enum VacuityPolicy { RequireWitness, AllowVacuous { because: Rationale } }
pub struct KaniEvidencePlan { /* unwind: u32, expect: KaniExpectation */ }
pub struct EvidencePlan { /* vacuity: VacuityPolicy, kani: Option<KaniEvidencePlan> */ }

pub struct HarnessPlan { /* theorem, evidence, operations */ }
impl HarnessPlan {
    pub fn try_new(
        theorem: TheoremIdentity,
        evidence: EvidencePlan,
        operations: Vec<Operation>,
    ) -> Result<Self, PlanError>;
    pub const fn theorem(&self) -> &TheoremIdentity;
    pub const fn evidence(&self) -> &EvidencePlan;
    pub fn operations(&self) -> &[Operation];
}
```

`PlanError` variants: `UndefinedBinding { binding, site }`,
`DuplicateBinding { binding, site }`, `DuplicateIdentity { id }`, and
`MissingWitness`. `PlanValueError` variants cover invalid binding names,
invalid expressions, invalid declared types, empty rationales, duplicate
parameters, unknown arguments, and missing arguments. Reused types:
`CanonicalActionName`, `TheoremName`, `LiteralValue`, `KaniExpectation`.

Dependencies: no new production dependency. New `theoremc-core` development
dependencies: `insta = "1.47.2"`, `rstest-bdd = "0.6.0"`,
`rstest-bdd-macros = "0.6.0"`. New tooling: Verus, pinned in
`tools/verus/VERSION` and `tools/verus/SHA256SUMS`, installed by
`rust-prover-tools` at the existing `tools/rust-prover-tools/REF`.

## Revision notes

- 2026-11-16: initial draft from Wyvern reconnaissance and Firecrawl checks of
  `rust-prover-tools` and Verus releases. Awaiting design review.
