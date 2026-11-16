# Task 5.1.1: Introduce the internal backend-neutral `HarnessPlan`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (design review applied; awaiting user approval)

Branch: `5-1-1-internal-backend-neutral-harness-plan`

Roadmap item: `5.1.1` in `docs/roadmap.md`.

## Purpose / big picture

Theoremc turns a readable `.theorem` YAML file into a Kani proof harness during
an ordinary Cargo build. Today the pipeline stops short of meaning: the
`theorem_file!` procedural macro in `crates/theoremc-macros` emits a Kani
harness function with the right name, attributes, and compile-time probes, but
its body is empty. Nothing in the repository yet says, in a form a renderer can
consume, "acquire a symbolic `u8` named `x`; assume `x <= 200`; call
`demo.normalize` with `x` for its `&u8` parameter and bind the result to
`once`; assert `once == twice`; cover `once != x`".

Architecture decision record (ADR) 0006 fixes where that meaning must live: in
an internal, backend-neutral harness plan owned by `crates/theoremc-core`,
between the validated schema types and any backend renderer. The plan must
carry resolved action invocations, source provenance, stable obligation
identities, and explicitly opaque Rust predicates, and it must not carry Kani
syntax, token streams, or generated Rust local names as semantic identity.

This task introduces that plan as a data model whose invariants are enforced at
construction. It does not lower theorem documents into the plan (task 5.1.2),
render the plan to Kani (task 5.1.3), or run Kani (task 5.1.4). After this task
a developer can build, in Rust, the plan of the longhand idempotence theorem
that ADR-0011 decision 6 prefers as the first vertical-slice example:

```rust,no_run
let plan = HarnessPlan::try_new(theorem, provenance, evidence, vec![
    Operation::Symbolic(/* forall[0]: x: u8 */),
    Operation::Assume(/* assume[0]: opaque Rust `x <= 200` */),
    Operation::Invoke(/* let[0]: once = demo.normalize(value: x) */),
    Operation::Invoke(/* let[1]: twice = demo.normalize(value: once) */),
    Operation::Assert(/* prove[0]: opaque Rust `once == twice` */),
    Operation::Cover(/* witness[0]: opaque Rust `once != x` */),
])?;
```

and observe three things. The plan is accepted and lists the operation
identities `forall[0]`, `assume[0]`, `let[0]`, `let[1]`, `prove[0]`, and
`witness[0]`. The same operations with the two invocations swapped are rejected
with an error naming binding `once`, site `let[1]`, and argument `value`,
because a plan cannot exist in an ill-formed state. And a snapshot of the plan
contains no `kani::` syntax, no token streams, and no mangled `theorem__…__h…`,
`__theoremc__…`, or `crate::theorem_actions::…` names.

No user-visible behaviour changes. `cargo build`, `theorem_file!` expansion,
diagnostics, and the `cargo theorem` command line are untouched, so
`docs/users-guide.md` needs no edit. The outstanding design decision this
closes is the concrete shape of ADR-0006's sketch: identity grammar, provenance
content, binding identity, predicate and argument representation, and the
well-formedness contract that renderers may rely on.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- Place the plan in `crates/theoremc-core/src/harness_plan/` per ADR-0006
  decision 2 and roadmap step 5.1. Do not place it in
  `crates/theoremc-build-support` (ADR-0008) or `crates/theoremc-macros`.
- Do not modify `crates/theoremc-macros`, `crates/theoremc-build-support`,
  `crates/cargo-theorem`, root `build.rs`, or the root `src/` facade. Do not
  re-export `harness_plan` from the `theoremc` facade: the plan is an internal
  interface with no public stability commitment (ADR-0006 non-goals; ADR-0007
  "What may break after theoremc 0.2").
- Do not change the public schema domain model (`schema::types`,
  `schema::arg_value`, `schema::newtypes`) or the loader's return types
  (ADR-0006 decision 3). Moving crate-private validation modules is allowed
  (milestone `EP-M1`, decision D13) provided every public path is unchanged.
- The plan model must not depend on structural schema types. It may import
  only this allow-list of shared leaf items: `CanonicalActionName` (from
  `crate::canonical_action_name`), `schema::TheoremName`,
  `schema::KaniExpectation`, `schema::validate_identifier`, and the
  crate-private `crate::rust_syntax` validators. It must never import
  `TheoremDoc`, `Step`, `LetBinding`, `ActionCall`, `Assertion`, `Assumption`,
  `WitnessCheck`, `ArgValue`, or `LiteralValue`. A boundary test enforces this
  list.
- No type reachable from `HarnessPlan` may contain `proc_macro2` values,
  `syn` syntax trees, `quote` output, Kani attribute or macro names, mangled
  action, module, or harness identifiers, or any other generated Rust local
  name (ADR-0006 decision 1; ADR-0007 invariant 7; roadmap success criterion).
  `syn` may be used inside constructors to validate and canonicalize text;
  parsed trees must be discarded.
- Every opaque Rust predicate is an explicit `Predicate::OpaqueRust` value
  (ADR-0006 decision 3; ADR-0007 invariant 6).
- Do not add a backend trait, capability checker, Law IR, `Branch` or
  `Compare` operations, `must` semantics, lowering from `TheoremDoc`, renderer
  queries, or a plan-equivalence relation. Those belong to tasks 5.1.2 to 5.4.x
  (ADR-0006 "Minimum plan repertoire"; ADR-0011 decisions 2 and 3).
- Do not add Verus tooling, proofs, or CI in this task (decision D11).
- No new production dependency. The only authorized new development
  dependency is `insta` 1.47.2 for `theoremc-core`, matching `theoremc-macros`.
  `googletest`, `pretty_assertions`, `proptest`, and `rstest` are already
  present.
- Repository rules from `AGENTS.md`: no Rust file over 400 lines; every module
  starts with `//!`; every `assert!` family call carries a message; no
  `.expect()` or `.unwrap()` outside tests; `expect_used` stays strict; lints
  are not silenced except by item-scoped `#[expect(lint, reason = "...")]` as a
  last resort; error enums are small and semantic; en-GB-oxendict spelling;
  caret dependency requirements; `cap_std::fs_utf8` and `camino` for filesystem
  access in tests.
- Completed ExecPlans in `docs/execplans/` are historical and must not be
  edited.

## Tolerances (exception triggers)

The comparison base is commit `4c059669c0503ff0cb95764283449e27128e7781`
(`4c05966`, the base of this branch). Every size limit below excludes this
ExecPlan file, including when it is updated in the same commit. Each category
is enforced independently: spare capacity in one category never offsets another.

Classification uses whole-file rules:

- Code: `crates/**/*.rs`, `tests/**/*.rs`, and `crates/**/tests/**/*.rs`.
- Configuration: `**/Cargo.toml`.
- Documentation: `docs/**/*.md` except `docs/execplans/`.
- Generated exclusions from line counts (still counted as files):
  `Cargo.lock` and `insta` `*.snap` files.
- File moves count once in the file count and contribute their net line
  change (normally zero).

Count net lines per category with:

```sh
git diff --numstat 4c05966 -- <category paths> ':(exclude)docs/execplans/**' \
  ':(exclude)Cargo.lock' ':(exclude)**/*.snap' \
  | awk '{ net += $1 - $2 } END { print net }'
```

Count changed files with:

```sh
git diff --name-only --find-renames 4c05966 -- ':(exclude)docs/execplans/**' | wc -l
```

- Files: more than 36 changed non-ExecPlan files.
- Code: more than 2,600 net lines. Within that, production (non-test) source
  under `crates/theoremc-core/src/harness_plan/` more than 1,500 net lines.
- Configuration: more than 20 net lines.
- Documentation: more than 450 net lines.
- Interface: any change to a public item outside
  `theoremc_core::harness_plan`, or any re-export of it from another crate.
- Dependencies: any dependency beyond those authorized in `Constraints`.
- Architecture: any need to change the schema domain types, the loader, the
  macro crate, or the build-support crate; any need for line or column spans in
  the plan (decision D3); any need to import a schema item outside the
  allow-list.
- Iterations: if a gate still fails after 5 fix attempts, stop and escalate.
- Time: if any milestone exceeds 6 hours of active work, stop and escalate.
- Ambiguity: if ADR-0006, ADR-0007, ADR-0011, or the roadmap admit two
  materially different readings of a requirement, stop and present options.

Budget rationale. The survey in `Context and orientation` estimates about 1,300
production lines across 14 small modules (each with Rustdoc examples), about
1,050 test lines (unit tables, the exhaustive and generated differential tests,
the snapshot, and the boundary test), and the `rust_syntax` move (net near
zero): roughly 2,350 code lines, so 2,600 leaves about 10% contingency. The
estimated file count is 30 (14 production files, 8 test files, 1 snapshot, 2
moved files and their 2 moved test files, the core manifest, `Cargo.lock`, and
5 documentation files, counting `lib.rs` and `schema/mod.rs` edits), so 36
tolerates two 400-line splits. Comparable completed plans set 500 to 700 net
lines across 8 to 12 files (`2-2-2`, `3-2-2`, `3-3-1`, `3-3-2`, `1-2-4`) and
none revised their budgets; their surprises were the 400-line file limit and
tooling, not underestimated scope. This task's surface is roughly three times
theirs because it introduces a new module family of a dozen types with their
own tests. Configuration covers one development-dependency line. Documentation
covers one design-document section (about 200 lines), the developers' guide
(about 120 lines), and layout, contents, and roadmap edits.

## Risks

- Risk: the 400-line file limit forces late splits. Severity: low. Likelihood:
  medium. Mitigation: the layout places each concern in its own file with a
  sibling `*_tests.rs` file; the file tolerance allows two splits.
- Risk: Whitaker or Clippy pedantic lints (`missing_const_for_fn`,
  `must_use_candidate`, `needless_pass_by_value`, `shadow_*`,
  `missing_docs_in_private_items`) force API churn. Severity: low. Likelihood:
  high. Mitigation: run `make lint` at every milestone; use the
  `addressing-whitaker-findings` skill.
- Risk: moving `schema/expr.rs` and `schema/rust_type.rs` into
  `crate::rust_syntax` disturbs schema tests or Whitaker exclusions that name
  paths. Severity: low. Likelihood: low. Mitigation: do the move as its own
  refactor commit with all gates green before any feature code; search
  `dylint.toml`, `clippy.toml`, and `docs/` for the old paths.
- Risk: `insta` debug snapshots churn when unrelated fields change. Severity:
  low. Likelihood: medium. Mitigation: snapshot one representative fixture
  only; pair it with semantic assertions; update snapshots only for real
  contract changes.
- Risk: CodeScene "String Heavy Arguments" scoring on newtype constructors.
  Severity: low. Likelihood: medium. Mitigation: only leaf newtypes accept
  `&str`; aggregate constructors take newtypes.
- Risk: local `make test` fails in the two Kani scenarios for environmental
  reasons unrelated to this work (a known shadow-loading problem with
  `librustc_driver`). Severity: low. Likelihood: medium. Mitigation: record the
  baseline in milestone `EP-M0`; treat an identical pre-existing failure as
  baseline, not regression, and say so in the PR.
- Risk: `make check-fmt` does not inspect committed Markdown, because
  `mdtablefix --git` only sees uncommitted files. Severity: low. Likelihood:
  high. Mitigation: run `mdtablefix --check` on each changed Markdown file
  explicitly before committing documentation.
- Risk: positional identities change when authors reorder items. Severity:
  low. Likelihood: certain. Mitigation: this is ADR-0006 decision 6's accepted
  scheme; stable external aliases are roadmap step 6.1. Document it.
- Risk: the flat scope kernel is adopted by task 5.2.3 without restating its
  contract for nested scopes. Severity: medium. Likelihood: medium. Mitigation:
  the design document records the flat kernel's contract and the unproved lemma
  statements, says explicitly that block scoping changes them, and the proposed
  roadmap task (decision D11) owns the scoped proof.

## Progress

- [x] (2026-11-16) Branch renamed, Lody session titled, skills loaded.
- [x] (2026-11-16) Reconnaissance by a Wyvern team: core model, codegen
  pipeline and tests, historical ExecPlan budgets, Kani and Verus wiring.
- [x] (2026-11-16) External tooling gaps resolved with Firecrawl:
  `rust-prover-tools` Verus support and Verus release naming.
- [x] (2026-11-16) Initial draft written and committed.
- [x] (2026-11-16) Design review by a four-member community-of-experts panel
  (structure, contracts, alternatives and viability, failure modes and cost);
  verdict "revise"; revisions applied (decisions D11 to D20).
- [ ] Plan approved by the user.
- [ ] `EP-M0` Baseline gates recorded.
- [ ] `EP-M1` Neutral Rust-syntax module, identity, provenance, and leaf
  values.
- [ ] `EP-M2` Resolved invocations, operations, and evidence.
- [ ] `EP-M3` Well-formed `HarnessPlan` with executable verification.
- [ ] `EP-M4` Documentation, roadmap, and delivery closure.

## Surprises & discoveries

- Observation: the schema domain model keeps no source spans or document index.
  Evidence: `TheoremDoc` (`crates/theoremc-core/src/schema/types.rs`) has no
  location fields; `load_theorem_docs_with_source` returns `Vec<TheoremDoc>`.
  Impact: plan provenance is structural (decision D3).
- Observation: `Let` keys and `as` names are plain `String`s, and no validator
  rejects a name bound twice across `Forall`, `Let`, and `as`. Evidence:
  `TheoremDoc.let_bindings: IndexMap<String, LetBinding>`,
  `ActionCall.as_binding: Option<String>`; no uniqueness rule in
  `schema/validate*.rs`. Impact: the plan owns binding uniqueness
  (`INV-unique-binding`); task 5.1.2 decides how lowering reports it.
- Observation: Rust expressions and types are validated with `syn` and then
  stored as untrimmed strings; the schema compares types by canonical token text
  (`rust_type::canonical_token_stream`, used by
  `ActionSignature::is_semantically_equivalent` and `collision.rs`). Impact:
  plan equality uses canonical forms (decision D2).
- Observation: `schema::expr` is a private module and `schema::rust_type` is
  `pub(crate)` inside `schema`, so the plan cannot reuse the expression
  validator without depending on schema's validator layer. Impact: decision D13
  moves both into a neutral crate-private `rust_syntax` module.
- Observation: `theoremc-macros` already has a module-private `ExpansionPlan`
  and `plan_expansion`. Impact: none beyond prose; the name `HarnessPlan` does
  not collide.
- Observation: `theorem_file!` normalizes path separators with
  `path_format::normalize_path_separators` before calling
  `mangle::theorem_key`. Impact: `TheoremPath` applies the same normalization
  (decision D16).
- Observation: schema diagnostics count steps from one ("Do step 1",
  `schema/step.rs`), while ADR-0006 identities count from zero (`prove[0]`).
  Impact: documented mapping for task 5.1.2 (decision D18).

## Decision log

- D1. Decision: add `#[doc(hidden)] pub mod harness_plan` to `theoremc-core`,
  documented as an unstable internal interface exempt from semantic versioning,
  not feature-gated and not re-exported by the `theoremc` facade. Rationale:
  task 5.1.3's renderer lives in `theoremc-macros` and must reach the plan
  across a crate boundary, so `pub(crate)` is impossible; hiding it from
  rendered documentation and stating the exemption avoids implying the "stable
  internal API" status of the developers' guide §3.6.1 table, which ADR-0006's
  non-goals forbid. Date/Author: 2026-11-16, planning agent, revised after
  review (Pandalump).
- D2. Decision: `RustExpression` and `RustType` store the author's trimmed
  source text (for messages and rendering) and a canonical token text (for
  equality and hashing), computed by `syn` at construction and validated by the
  shared `rust_syntax` rules. `Predicate::OpaqueRust(RustExpression)` is the
  only predicate form. Rationale: the roadmap success criterion forbids token
  streams in plan fixtures; a `syn` tree lacks `Eq` under the current `syn`
  features and would bind the plan to a parser representation; canonical text
  makes `a==b` and `a == b` compare equal, as task 5.4's plan-equivalence
  fixtures will need, and matches how the schema already compares types.
  Rejected: storing `syn::Expr` (ADR-0006's `ParsedRustExpression` sketch; the
  ADR says exact names may evolve). Date/Author: 2026-11-16, planning agent,
  revised after review (Doggylump).
- D3. Decision: provenance is structural and separate from identity. A plan
  records a `TheoremIdentity` (normalized theorem path and `TheoremName`, the
  same inputs as `mangle::theorem_key`), a `PlanProvenance` (the zero-based
  YAML document index), and, per operation, an `ObligationId`. Line and column
  spans are not recorded. Rationale: the schema domain model drops spans after
  raw conversion, and adding them changes public schema types, which ADR-0006
  decision 3 defers. Structural provenance identifies the source element
  exactly, as ADR-0011's acceptance criterion requires. Keeping the document
  index out of identity means reordering documents in a file does not change
  theorem identity (ADR-0007 invariant 9). Revisit spans when the run ledger
  (roadmap step 6.2) needs line-level navigation. Date/Author: 2026-11-16,
  planning agent, revised after review (Pandalump).
- D4. Decision: an `ObligationId` is opaque, not `Copy`, and not `const`
  constructible. Its root is a section (`forall`, `assume`, `let`, `do`,
  `prove`, `witness`) and a zero-based position, displayed as
  `section[position]`. It derives `Clone`, `Eq`, `Ord`, and `Hash`. The display
  grammar reserves child segments, `section[position](.label([index])?)*` with
  labels matching `[a-z][a-z0-9_]*`, for `let[1].must` (task 5.2.2),
  `do[2].maybe.do[0]` (task 5.2.3), and
  `prove[2].law.idempotent.first_application` (task 5.4.x); this task stores
  and implements roots only, behind private fields. Rationale: ADR-0006
  decision 6 fixes the textual form; refusing `Copy` and `const` now lets
  children be added inside the type without changing call sites, with no
  speculative child representation. Date/Author: 2026-11-16, planning agent,
  revised after review (Telefono, Pandalump).
- D5. Decision: lift non-vacuity into a backend-neutral `VacuityPolicy`
  (`RequireWitness` or `AllowVacuous { because: Rationale }`) on
  `EvidencePlan`; Kani's unwind bound (`NonZeroU32`) and expected outcome stay
  in a backend-indexed `KaniEvidencePlan`. Under `RequireWitness`, only a
  `Cover` at a source `witness[i]` site satisfies the rule. Rationale: ADR-0007
  invariant 11 makes witness intent and vacuity policy theorem semantics and
  only the mechanism backend-specific; RFC 0001 "Non-vacuity policy" says a
  generated cover (task 5.4.x) does not satisfy the document witness
  requirement. Task 5.1.2 owns mapping `allow_vacuous` and `vacuity_because`
  into the policy; documents without `Evidence.kani` are already rejected
  before planning (`MissingKaniEvidence`). Date/Author: 2026-11-16, planning
  agent, revised after review (Pandalump, Telefono).
- D6. Decision: plans exist only through `HarnessPlan::try_new`, which checks
  every plan-level invariant; leaf values, sites, and invocations exist only
  through validating constructors; fields are private and accessors return
  shared references. No builder in this task. Rationale: renderers may rely on
  well-formedness without re-checking. A scope-handing builder (where bindings
  are opaque handles and identities are assigned by the builder) would make
  some violations unrepresentable, but its interface should be designed with
  its only consumer, task 5.1.2's lowering; record it as an option for that
  task's spike. Date/Author: 2026-11-16, planning agent, revised after review
  (Wafflecat).
- D7. Decision: binding identity is `BindingId`, an enum whose only variant in
  this task is `Source(BindingName)`; task 5.4.x adds `Derived(ObligationId)`
  for law intermediates that have no author name. A plan rejects any
  `BindingId` defined twice, plan-wide. `BindingName` rejects `_` and names
  beginning with the reserved renderer prefix `__theoremc`. Source binding
  names are emitted verbatim as Rust locals by every Rust-based renderer,
  because opaque predicates refer to them by their source names. Rationale:
  ADR-0006 decision 4 sketches `binding: Option<BindingId>`, not a name;
  inventing names for law intermediates would put generated names into
  identity, which ADR-0007 invariant 7 forbids. Plan-wide uniqueness is the
  simplest rule for flat plans; task 5.2.3 decides whether sibling `maybe`
  scopes may reuse a name. Rejected: index-based binding tables (less readable
  fixtures and snapshots, and source names must still reach the renderer).
  Date/Author: 2026-11-16, planning agent, revised after review (Pandalump,
  Wafflecat, Doggylump).
- D8. Decision: `ResolvedAction` records the canonical action name and the
  declared signature (ordered `ActionParameter`s and the return `RustType`),
  not the mangled Rust path. Each `ResolvedInvocation` embeds its
  `ResolvedAction`, and `HarnessPlan::try_new` rejects two invocations of the
  same action name with different signatures (`INV-consistent-action`).
  Rationale: ADR-0007 invariant 2 separates semantic primary identity from
  backend binding; the Kani renderer derives `crate::theorem_actions::…__h…`
  through `mangle::mangle_action_name`. Embedding keeps ADR-0006's sketch and
  gives renderers the signature at the call; the consistency clause prevents
  conflicting probes. Rejected: a plan-level action table referenced by name
  (one more lookup for every consumer for the same guarantee). Date/Author:
  2026-11-16, planning agent, revised after review (Telefono, Pandalump).
- D9. Decision: include `InvocationMode` with only `Call`, `Predicate` with
  only `OpaqueRust`, and `BindingId` with only `Source`, and omit `Branch` and
  `Compare` operations. Rationale: ADR-0006's minimum repertoire; the
  single-variant enums name the axes that tasks 5.2.2 and 5.4.x extend.
  Date/Author: 2026-11-16, planning agent.
- D10. Decision: this task provides no conversion from schema types into plan
  types, and recommends that task 5.1.2 place lowering in a sibling module (for
  example `crate::harness_plan_lowering`), so that `harness_plan` never depends
  on `TheoremDoc`. Rationale: lowering is task 5.1.2; keeping the model free of
  structural schema imports is what the allow-list boundary test enforces.
  Date/Author: 2026-11-16, planning agent, revised after review (Pandalump).
- D11. Decision: do not adopt Verus in this task. State the flat kernel's
  abstract model and lemma statements (`L-step`, `L-sound`, `L-complete`) in
  the design document as unproved obligations, discharge them for this task by
  a small-scope exhaustive differential test and a generated differential test,
  and propose a new roadmap task after 5.2.3, "Prove the scoped binding and
  identity kernel with Verus", whose model is the tree-shaped kernel with a
  scope stack. Rationale: three of four reviewers independently found that task
  5.2.3's block-scoped `maybe` changes the kernel's meaning rather than
  extending it: a binding defined inside a branch must not be visible after it,
  so `L-step` over a growing set of definitions is false for scoped plans and
  the proof would be rewritten. The Verus on-ramp requires a kernel that is
  "small and stable enough to prove" and keeps Verus off the pull-request gate;
  ADR-0011 decision 7 keeps noncritical work off the vertical-slice path, and
  this task blocks 5.1.2 to 5.1.4. The scoped kernel is where bounded tests
  weaken and an unbounded proof earns its upkeep. Superseded draft: a Verus
  milestone with ADR-0014, pins, Makefile targets, a CI job, and contract
  tests. This is a user-visible scope choice: the user may direct its
  reinstatement. Date/Author: 2026-11-16, planning agent, revised after review
  (Wafflecat, Dinolump, Doggylump, Pandalump).
- D12. Decision: plan fixtures (the longhand idempotence plan and its
  variants) live in `harness_plan::test_fixtures`, compiled for unit tests and
  behind the existing `test-support` feature, matching `schema::test_fixtures`.
  Rationale: unit tests and the snapshot share them, and task 5.1.3's renderer
  snapshots will reuse them. Date/Author: 2026-11-16, planning agent.
- D13. Decision: move `schema/expr.rs` and `schema/rust_type.rs` (with their
  test files) into a crate-private neutral module `crate::rust_syntax`, used by
  both `schema` and `harness_plan`, as a separate refactor commit. Rationale:
  the developers' guide §2 says shared concepts that two layers need belong in
  neutral top-level modules (as with `canonical_action_name.rs`); the plan must
  not depend on schema's validator layer, and `schema::expr` is private.
  Rejected: widening `schema::expr` to `pub(crate)` (creates a
  plan-to-schema-validator edge). Date/Author: 2026-11-16, planning agent,
  after review (Pandalump).
- D14. Decision: distinct newtypes for distinct identifier spaces:
  `BindingName` (bindings), `ParameterName` (action parameters), and
  `FieldName` (struct-literal fields), sharing one identifier validator.
  Rationale: one type for three spaces defeats the newtype guidance and invites
  an adapter that counts field or parameter names as binding uses. Date/Author:
  2026-11-16, planning agent, after review (Telefono, Pandalump, Doggylump).
- D15. Decision: argument values are two-level. `ArgumentValue` is either
  `Value(PlanValue)` or `Record(Vec<RecordField>)`; `PlanValue` is
  `Literal(PlanLiteral)`, `Binding(BindingId)`, or `List(Vec<PlanValue>)`, so
  records appear only at the root of an argument. A `Record` argument is
  accepted only when the parameter's declared type, after removing at most one
  reference layer, is a path type without a qualified self and without generic
  arguments. `PlanLiteral` is `Bool`, `Integer(i64)`, `Float(FiniteF64)`, or
  `String`; `FiniteF64` rejects non-finite values and compares by bit pattern,
  so the plan derives `Eq`. Rationale: struct-literal synthesis needs a type
  path, which a nested map lacks; the existing prototype (`src/arg_lowering.rs`,
  `lower_nested_mapping`, `extract_type_path`) already rejects these shapes,
  and the plan should reject them at construction with structural context
  rather than at render time. Task 5.1.2 decodes nested `RawSequence` and
  `RawMap` values through `decode_arg_value`, so nested `{ ref: … }` and
  `{ literal: … }` wrappers become `Binding` and `Literal` values. Date/Author:
  2026-11-16, planning agent, after review (Telefono).
- D16. Decision: `TheoremPath` stores the theorem file path after
  `path_format::normalize_path_separators`, and rejects absolute paths and
  paths containing `..`. Rationale: the macro already normalizes separators
  before `mangle::theorem_key`; rejecting absolute and parent-relative paths
  keeps snapshots machine-independent. Leading `./` removal and the full
  canonical external ID remain task 4.2.2. Date/Author: 2026-11-16, planning
  agent, after review (Telefono, Doggylump).
- D17. Decision: operation sites are typed, so an operation's kind and section
  always agree (`INV-section-kind` holds by construction): `SymbolicBinding`
  takes a `forall` position, `PredicateObligation` has `assumption`,
  `assertion`, and `witness` constructors, and an invocation site is
  `InvocationSite::Let(position)` (which requires a result binding) or
  `InvocationSite::Do(position)` (result optional). Rationale: a renderer that
  labels a failure with the wrong section breaks ADR-0011's identity acceptance
  criterion. The `Do`-without-`as` rule for non-unit results is task 5.2.1's
  and is not enforced here. Section order is not enforced: version 2 laws may
  interleave operations, and rustc reports an opaque predicate that names a
  later binding. Date/Author: 2026-11-16, planning agent, after review
  (Telefono, Doggylump).
- D18. Decision: errors carry structural locations. `PlanError` variants are
  `UndefinedBinding { binding, site, argument: ArgumentPath }`,
  `DuplicateBinding { binding, site }`, `DuplicateIdentity { id }`,
  `InconsistentAction { action, site }`, and `MissingWitness`. Construction
  errors are `PlanValueError` for leaf values and
  `InvocationError { action, parameter: Option<ParameterName>, kind }`, where
  the caller (task 5.1.2) adds the site. All error types derive `Clone`, `Eq`,
  and `thiserror::Error`, carry reason text rather than `SchemaError` (which
  boxes a trait object), and are `#[non_exhaustive]`. Identities count from
  zero; task 5.1.2 converts to the schema's one-based wording. Rationale:
  lowering has no source text after loading, so a structural location is the
  only location it can report. Date/Author: 2026-11-16, planning agent, after
  review (Telefono).
- D19. Decision: the scope kernel is a pure function generic over binding and
  identity key types that uses only `Eq` and `Ord`, over events whose uses are
  an ordered sequence. Rationale: genericity makes renaming invariance
  (`L-abstraction`) hold by parametricity rather than by argument, lets the
  exhaustive test enumerate small integer keys, and gives a future Verus model
  the same shape. Ordered uses make "first undefined use" well defined.
  Date/Author: 2026-11-16, planning agent, after review (Doggylump).
- D20. Decision: no behaviour-driven development (BDD) suite in this task.
  Rationale: the plan model has no external actor; every candidate scenario
  restated a named `rstest` case, and adding `rstest-bdd` to `theoremc-core`
  would open a second BDD home for internal types. The first applicable BDD
  boundary is task 5.1.2 ("Given a theorem file … the plan has identities …"),
  and the end-to-end boundary is task 5.1.4's real Kani run. The proposed
  scenarios are retained in `Validation and acceptance` as named-test
  acceptance. Date/Author: 2026-11-16, planning agent, after review (Dinolump).

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
`#[kani::proof]` harnesses whose unwind comes from `doc.evidence.kani`. Nothing
consumes `Let`, `Do`, `Assume`, `Prove`, or `Witness` yet. The root crate's
test-only `src/arg_lowering.rs` prototype turns `ArgValue` into `TokenStream`s;
task 5.1.3 promotes it.

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
discarded. `KaniEvidence` has `unwind: u32` (zero is rejected by
`schema/validate_evidence.rs`), `expect: KaniExpectation`,
`allow_vacuous: bool`, and `vacuity_because: Option<String>`. Argument values
are `schema::ArgValue` (`Literal(LiteralValue)`, `Reference(String)`,
`RawSequence(Vec<TheoremValue>)`, `RawMap(IndexMap<String, TheoremValue>)`);
`LiteralValue` holds `Bool`, `Integer(i64)`, `Float(f64)`, or `String` and is
`PartialEq` but not `Eq`.

Validators to reuse rather than duplicate:

- `schema::validate_identifier(&str)` (public; implemented in the private
  `schema/identifier.rs`) for identifiers;
- `schema/expr.rs` `validate_rust_expr` (private module) and
  `schema/rust_type.rs` `parse`, `parse_with_free_named_lifetime`, and
  `canonical_token_stream` (`pub(crate)`), which milestone `EP-M1` moves to
  `crate::rust_syntax` (decision D13);
- `CanonicalActionName` (`crates/theoremc-core/src/canonical_action_name.rs`);
- `path_format::normalize_path_separators`;
- `schema::str_newtype::StrNewtype` and the `&str`-backed newtype convention in
  `docs/developers-guide.md` §3.5.1. Do not add inherent `new` or `as_str`
  methods that shadow the trait (they make it unreachable and trip
  `unused_imports`).

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
dependencies; `insta` 1.47.2 is used by `theoremc-macros`. `make kani` runs two
Kani scenarios; Kani is pinned in `tools/kani/VERSION` (0.67.0) and installed by
`rust-prover-tools` (`tools/rust-prover-tools/REF`). Verus is not wired
anywhere, and this task does not wire it.

### Gates

- `make check-fmt`: `cargo fmt --all -- --check` and `mdtablefix --check`.
- `make lint`: rustdoc, Clippy, and Whitaker, in that order.
- `make test`: `cargo nextest run --workspace --all-targets --all-features`
  then doctests.
- `make markdownlint`, `make nixie`, and `make spelling` for documentation.

### Budget survey

The new module family is about 14 production files of 50 to 150 lines, each
with Rustdoc examples, judged against the nearest analogues
(`schema/newtypes.rs`, about 200 lines for two newtypes, and
`crates/theoremc-macros/src/query.rs`). Completed comparable plans:
`2-2-2-deterministic-harness-naming.md` (12 files, 700 lines, not revised),
`3-2-2-gate-generated-kani-harnesses.md` (10 files, 600 lines, not revised),
`3-3-1-emit-typed-action-probes.md` and
`3-3-2-referenced-type-probes-for-generated-struct-literal-synthesis.md` (8
files, 500 lines, not revised; 3-3-1 stopped once for a design gap that became
ADR-0004), and `1-2-4-enforce-non-vacuity-defaults.md` (8 files, 500 lines, not
revised). Their recorded surprises were file-size splits, Kani tooling, and
Markdown formatting gates, which are carried into `Risks`. The first draft of
this plan set 3,000 code lines and 45 files including a Verus milestone; review
found that a smell, and the Verus deferral (decision D11) removed about 8 files
and 500 lines.

### Signposts

Documents: `docs/roadmap.md` (task 5.1.1 and step 5.1);
`docs/adrs/adr-0006-backend-neutral-harness-planning-boundary.md`;
`docs/adrs/adr-0007-verus-backend-preservation-invariants.md`;
`docs/adrs/adr-0008-workspace-build-support-boundary.md`;
`docs/adrs/adr-0011-vertical-slice-first-roadmap-sequencing.md`;
`docs/adrs/adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md`
(decision 4);
`docs/adrs/adr-0003-architectural-boundary-enforcement-for-schema-layers.md`;
`docs/theoremc-design.md` (DES) §§3, 4.3 to 4.6, 5.4, 5.5, 8.1 to 8.4;
`docs/theorem-file-specification.md` (TFS) §§2.3, 2.4, 3.6 to 3.11, 4, 5, 6,
7.4; `docs/rfcs/rfc-0001-semantic-law-templates-over-composable-primaries.md`
("Lowering architecture", "Non-vacuity policy", "Stable sub-obligation
identities"); `docs/developers-guide.md` §§2, 3.3, 3.5.1, 3.6.1, 3.9;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rust-doctest-dry-guide.md`;
`docs/reliable-testing-in-rust-via-dependency-injection.md` (no environment
access is introduced);
`docs/complexity-antipatterns-and-refactoring-strategies.md`;
`docs/rstest-bdd-users-guide.md` (why BDD is deferred, decision D20);
`docs/documentation-style-guide.md`; `docs/ortho-config-users-guide.md` (not
affected: no configuration surface is added).

Skills: `rust-router` then `rust-types-and-apis` (newtypes, smart constructors),
`rust-errors` (error enums), `rust-unit-testing` (rstest, googletest, insta),
`proptest`, `rust-verification` (method selection), `verus` (for the deferral
rationale and the proposed follow-up), `hexagonal-architecture` (pure kernel
and adapter split), `addressing-whitaker-findings`, `execplans`, `mapsplice`
(roadmap edit), `commit-message`, `pr-creation`, and `en-gb-oxendict-style`.

## Conformance basis

Upstream artefacts at base revision `4c05966`:

- `docs/roadmap.md` task 5.1.1 and step 5.1.
- ADR-0006 (accepted 2026-08-19, addendum 2026-11-15), decisions 1 to 6, 8,
  9, 10, and "Minimum plan repertoire".
- ADR-0007 (accepted 2026-08-19), invariants 2, 6, 7, 9, 11, and 12.
- ADR-0008 (accepted 2026-10-11), the build-support boundary.
- ADR-0011 (accepted 2026-08-19), decisions 1, 2, 6, and 7.
- ADR-0001 decision 4 (non-vacuity witness policy).
- ADR-0003 (schema layer contract) and developers' guide §2 (neutral modules).
- RFC 0001, "Non-vacuity policy" and "Stable sub-obligation identities".
- DES §§3, 5.5, 8.1 to 8.4; TFS §§2.3, 3.6 to 3.11, 4.1, 7.4.
- `AGENTS.md` and `docs/developers-guide.md` conventions.

No Terms of Reference document exists for this repository; the roadmap and ADRs
are the governing artefacts.

Requirements:

- `REQ-location`: the plan lives in `theoremc-core` as an internal module that
  depends on no structural schema type (ADR-0006 d2; roadmap 5.1; ADR-0008;
  ADR-0003).
- `REQ-invocation`: action use is represented by one resolved-invocation type
  carrying the action identity, declared signature, arguments in declared
  parameter order with declared types and passing modes, invocation mode, and
  optional result binding (ADR-0006 d4, d5).
- `REQ-provenance`: the plan records the source theorem, the document, and the
  source position of every operation (ADR-0006 d1; ADR-0011 acceptance).
- `REQ-identity`: every operation has a deterministic structural identity in
  ADR-0006's textual form whose section matches its kind, independent of
  generated names and of document order (ADR-0006 d6; ADR-0007 inv 9).
- `REQ-opaque`: opaque Rust assumptions, assertions, and covers are explicit
  `OpaqueRust` predicate nodes (ADR-0006 d3; ADR-0007 inv 6).
- `REQ-backend-free`: the plan contains no token streams, Kani syntax, or
  generated local names as identity (roadmap success; ADR-0007 inv 7).
- `REQ-repertoire`: the plan supports typed symbolic binding, opaque
  assumption, resolved infallible invocation with result binding, opaque
  assertion, opaque cover, stable identity, and Kani unwind evidence, and
  nothing more (ADR-0006 "Minimum plan repertoire").
- `REQ-vacuity`: the plan preserves source witness intent and the vacuity
  policy as semantics (ADR-0007 inv 11; ADR-0001 d4; RFC 0001).
- `REQ-no-adapters`: the plan introduces no implicit clones, defaults, or
  coercions (ADR-0006 d5; ADR-0007 inv 12).

Gaps from the baseline:

- `GAP-no-plan`: no harness-plan type exists.
- `GAP-no-identity`: no structural obligation identity exists.
- `GAP-no-provenance`: theorem path and document index are not retained beyond
  the loader.
- `GAP-binding-uniqueness`: nothing rejects a binding name defined twice.
- `GAP-shared-syntax`: Rust expression validation is private to `schema`.

Trace links:

```plaintext
ADR-0006 d2, ADR-0003 -> REQ-location -> EP-M1, EP-M3 -> boundary_tests::imports_only_allowed_schema_leaves
developers' guide §2 -> GAP-shared-syntax -> EP-M1 -> rust_syntax tests pass after the move
ADR-0006 d4,d5 -> REQ-invocation -> EP-M2 -> invocation_tests::arguments_follow_declared_order
ADR-0006 d1 -> REQ-provenance -> EP-M1 -> identity_tests::theorem_identity_excludes_document_index
ADR-0006 d6 -> REQ-identity -> EP-M1, EP-M2 -> identity_tests::display_table; site_tests::section_matches_kind
ADR-0006 d3 -> REQ-opaque -> EP-M2 -> predicate_tests; plan snapshot
roadmap 5.1.1 -> REQ-backend-free -> EP-M3 -> plan_snapshot_tests; boundary_tests
ADR-0006 repertoire -> REQ-repertoire -> EP-M2 -> operation_tests::minimum_repertoire_is_representable
ADR-0007 inv 11 -> REQ-vacuity -> EP-M3 -> well_formed_tests::witness_policy_table
ADR-0007 inv 12 -> REQ-no-adapters -> EP-M2 -> invocation_tests::values_are_stored_verbatim
GAP-binding-uniqueness -> INV-unique-binding -> EP-M3 -> kernel_exhaustive_tests; kernel_property_tests
```

At each milestone boundary, check: the assigned requirements hold; nothing in
`harness_plan` depends on `proc_macro2`, `quote`, Kani, or a schema item
outside the allow-list; no public item outside `harness_plan` changed; no new
dependency appeared beyond `insta`; and the trace links above still name real
tests.

## Verification plan

### Abstract model

A harness plan is a quadruple `(T, P, E, ops)`. `T` is the theorem identity
`(path, theorem_name)`. `P` is provenance, the document index. `E` is evidence:
a vacuity policy `V ∈ {RequireWitness, AllowVacuous(r)}` with `r` a non-empty
rationale, and an optional Kani evidence `(unwind > 0, expect)`. `ops` is a
finite sequence `o_0 … o_{n-1}` of operations.

Each operation `o_i` has:

- a kind in `{Symbolic, Assume, Invoke, Assert, Cover}`;
- a site `id(o_i)`, a section and a position, whose section is determined by
  the kind (`Symbolic ↦ forall`, `Assume ↦ assume`, `Invoke ↦ let | do`,
  `Assert ↦ prove`, `Cover ↦ witness`);
- a defined binding `def(o_i)`: the binding of a `Symbolic` operation, the
  result binding of an `Invoke` operation if any (always present at a `let`
  site), otherwise none;
- an ordered sequence of used bindings `uses(o_i)`: for an `Invoke`
  operation, the `Binding` values of its arguments in declared parameter order,
  each argument traversed depth-first (list elements in order, record fields in
  order); otherwise empty. Opaque predicates contribute no uses: the plan does
  not interpret Rust text (residual gap `GAP-R1`). Parameter and field names
  are never uses.

Well-formedness of the operation sequence, `wf(ops)`, is the conjunction of:

- scope: for every `i` and every `b` in `uses(o_i)`, some `j < i` has
  `def(o_j) = b` (strictly earlier, so `let x = f(&x)` is ill-formed);
- unique definitions: for all `i < j`, if `def(o_i)` and `def(o_j)` are both
  present then they differ;
- unique identities: for all `i < j`, `id(o_i) ≠ id(o_j)`.

Plan well-formedness `WF(T, P, E, ops)` is `wf(ops)` and:

- consistent actions: any two `Invoke` operations naming the same action carry
  equal resolved signatures;
- witness: if `V = RequireWitness`, some `o_i` is a `Cover` at a `witness`
  site.

Leaf validity (enforced by constructors, before a plan exists): binding,
parameter, and field names are Rust identifiers that are not keywords, and
binding names are neither `_` nor prefixed `__theoremc`; expressions parse as
one `syn::Expr` and are not statement-like; types parse as `syn::Type` without
a free named lifetime; rationales are non-empty after trimming; theorem paths
are relative and contain no `..`; floats are finite; an action's parameter
names are distinct; an invocation supplies exactly one argument per declared
parameter and no other, stored in declared order; a `Record` argument targets a
parameter whose type, after at most one reference layer, is a plain path
without generic arguments; a `let` invocation site has a result binding.

Result contract: `HarnessPlan::try_new(T, P, E, ops)` returns `Ok(plan)`
exactly when `WF(T, P, E, ops)` holds, and the plan exposes its inputs
unchanged. Otherwise it returns a deterministic error: the first scan violation
in operation order; within one operation an undefined use (first in `uses`
order) before a duplicate definition before a duplicate identity before an
inconsistent action; a missing witness only when the scan is clean. This
priority is documented, tested behaviour, not a proof obligation.

### Invariants and lemmas

- `INV-scope` (from `REQ-invocation`, `REQ-backend-free`): every binding used
  by an invocation in an accepted plan is defined by a strictly earlier
  operation.
- `INV-unique-binding` (from `GAP-binding-uniqueness`, decision D7): no
  binding is defined twice in an accepted plan.
- `INV-unique-identity` (from `REQ-identity`): no two operations in an accepted
  plan share an identity.
- `INV-section-kind` (from `REQ-identity`, decision D17): every operation's
  section matches its kind. Type-level, by typed site constructors.
- `INV-consistent-action` (from `REQ-invocation`, decision D8): invocations of
  one action in an accepted plan carry one signature.
- `INV-witness` (from `REQ-vacuity`, decision D5): an accepted plan with
  `RequireWitness` contains a `Cover` at a `witness` site; `AllowVacuous`
  always carries a non-empty rationale (type-level).
- `INV-arguments` (from `REQ-invocation`): a resolved invocation's arguments
  correspond one-to-one with the declared parameters and appear in declared
  order, whatever order the caller supplied, with values unchanged.
- `INV-passing-mode` (from `REQ-invocation`): a parameter declared `&mut T`
  has passing mode `MutableBorrow`, `&T` (including `&'static T`) has
  `SharedBorrow`, and every other type has `Move`; nested references such as
  `Box<&mut T>` are `Move`. Passing mode classifies the declared type; how a
  renderer passes each value shape is task 5.1.3's decision (DES §5.5.3).
- `INV-identity-display` (from `REQ-identity`): identity display is
  deterministic and injective over roots.
- `INV-leaf-validity`: every rule in the leaf-validity paragraph rejects
  invalid input with a typed error and never produces a value.
- `INV-canonical-equality` (decision D2): two `RustType` or `RustExpression`
  values are equal exactly when their canonical token texts are equal.
- `INV-opaque` (from `REQ-opaque`): every predicate is `OpaqueRust`
  (type-level).
- `INV-backend-free` (from `REQ-backend-free`): `HarnessPlan` is `Send + Sync`,
  which excludes `proc_macro2` values, and therefore `syn` trees, at any depth
  (premise `AXIOM-proc-macro2-auto-traits`); the debug rendering of every plan
  fixture contains none of `kani::`, `#[kani`, `TokenStream`, `theorem__`,
  `__theoremc__`, `crate::theorem_actions`, or `__h` followed by twelve
  hexadecimal digits; and the `harness_plan` sources import nothing from
  `proc_macro2`, `quote`, or `kani`, and from `schema` only the allow-list.
- `INV-no-adapters` (from `REQ-no-adapters`): plan values are stored exactly
  as supplied.
- `INV-immutable`: an accepted plan cannot be mutated (type-level).

Lemmas over the scope kernel. The production kernel (decision D19) is a pure
function `first_violation(events) -> Option<(usize, Violation)>` over a slice of
`ScopeEvent<B, I> { defines: Option<B>, uses: Vec<B>, identity: I }`, generic
over `B: Ord` and `I: Ord`, threading a set of defined bindings and a set of
seen identities. `HarnessPlan::try_new` maps operations to events (the adapter)
and checks the consistent-action and witness clauses afterwards. These
statements are unproved hypotheses about the eventual proof structure; in this
task they are discharged by exhaustive and generated testing, and the design
document records them for the proposed Verus task.

- `L-step` (prefix extension): `wf(s ++ [e])` holds if and only if `wf(s)`
  holds and `e` is admissible against `s`: every use of `e` is defined in `s`,
  the definition of `e` (if any) is not defined in `s`, and the identity of `e`
  is not an identity in `s`.
- `L-sound`: if `first_violation(s)` is none, then `wf(s)`.
- `L-complete`: if `wf(s)`, then `first_violation(s)` is none.
- `L-abstraction`: the kernel's result is invariant under any injective
  renaming of bindings and identities. Holds by parametricity: the kernel is
  generic and uses only `Eq` and `Ord`.
- `L-adapter`: for each operation kind and value shape, the adapter's
  `defines` and `uses` equal `def(o)` and `uses(o)`.
- `L-witness`: with a clean scan and consistent actions, `try_new` rejects with
  `MissingWitness` exactly when the policy is `RequireWitness` and no `Cover`
  sits at a `witness` site.

For nested `maybe` scopes (task 5.2.3), `L-step` is false as stated: a branch
binding must not be visible after the branch. The scoped kernel needs a stack
of scopes and restated lemmas. The design document says so explicitly.

### Axioms (trusted, not verified here)

- `AXIOM-syn`: `syn::parse_str::<syn::Expr>` and `syn::parse_str::<syn::Type>`
  accept exactly the Rust expression and type grammar relevant to validation,
  and the token-stream rendering used for canonical text is deterministic for a
  given `syn` version. Source: `syn` 3 documentation. Boundary:
  `RustExpression::new`, `RustType::new`.
- `AXIOM-std-collections`: `BTreeSet` membership and insertion behave as sets
  with respect to `Ord`. Source: standard library documentation. Boundary: the
  kernel.
- `AXIOM-indexmap`: `IndexMap` iteration follows insertion order. Source:
  `indexmap` documentation. Boundary: argument matching input.
- `AXIOM-proc-macro2-auto-traits`: `proc_macro2::TokenStream`, and every `syn`
  syntax-tree type that contains one, is neither `Send` nor `Sync`. Source: the
  "Auto Trait Implementations" section of `proc_macro2`'s documentation for the
  pinned version. Evidence: during `EP-M3`, check that section in the locally
  built rustdoc (`cargo doc -p proc-macro2 --no-deps`) and record the result in
  `Artefacts and notes`; if the premise is false, replace the static assertion
  with the boundary test alone and record the change. Boundary:
  `INV-backend-free`.

### Methods per obligation

- Obligation: `INV-scope`, `INV-unique-binding`, and `INV-unique-identity`
  through `L-step`, `L-sound`, and `L-complete`, plus the documented violation
  priority. Method: small-scope exhaustive differential test, then a generated
  differential test for longer sequences, both against an independent oracle,
  plus named `rstest` cases. Rationale: the flat kernel is a small fold whose
  behaviour depends on short-range collisions; the small-scope hypothesis (most
  kernel bugs show on sequences of three or fewer events over two names and
  three identities) makes exhaustive enumeration strong evidence, and the
  generated test covers longer interleavings. An unbounded proof is deferred to
  the scoped kernel (decision D11). Domain: exhaustive, every sequence of
  length 0 to 3 whose events draw `defines` from `{none, 0, 1}`, `uses` from
  every ordered sequence of length 0 to 2 over `{0, 1}`, and `identity` from
  `{0, 1, 2}` (63 distinct events, 254,080 sequences); generated, sequences of
  0 to 12 events over 4 binding keys and 6 identity keys, 1,024 cases. Artefact:
  `crates/theoremc-core/src/harness_plan/kernel_exhaustive_tests.rs`;
  `crates/theoremc-core/src/harness_plan/kernel_property_tests.rs`;
  `crates/theoremc-core/src/harness_plan/well_formed_tests.rs`. Oracle: a
  test-local function that evaluates the quantified `wf` definition literally
  (for each position and each use, search the strict prefix for a definition;
  compare all pairs for duplicates), and computes the expected first violation
  by finding the shortest ill-formed prefix and classifying its last event
  under the documented priority. It shares no code with the kernel. Evidence:
  `cargo nextest run -p theoremc-core --all-features harness_plan` passes; the
  exhaustive test finishes in under five seconds in the debug profile.
  Non-vacuity: the exhaustive test classifies each sequence by the oracle's
  result and asserts minimum counts for the classes accepted-with-a-use,
  accepted-with-three-events, undefined use, self-reference, forward reference,
  duplicate definition, duplicate identity, first violation at index 2, and
  conflicting violations in one event. Negative controls (each a temporary
  mutation of the kernel that must make the exhaustive test fail, with the
  counterexample recorded in `Artefacts and notes`): `M1` insert the definition
  before checking uses (self-reference accepted); `M2` check uses against all
  definitions in the sequence (forward reference accepted); `M3` skip the
  identity check for events without a definition; `M4` report a duplicate
  definition before an undefined use; `M5` report index `i + 1`.
- Obligation: `L-abstraction`.
  Method: type-level, by the kernel's generic signature with only `Eq` and
  `Ord` bounds; the exhaustive test instantiates it with integers and the plan
  with `BindingId` and `ObligationId`.
- Obligation: `L-adapter`.
  Method: `rstest` table over every operation kind and every value shape
  (literal, top-level binding, binding inside a list, binding inside a record
  field, binding inside a list inside a record field), including a case where a
  record field name and a parameter name equal an undefined binding name and
  must not be reported. Artefact: `well_formed_tests.rs`. Non-vacuity: negative
  control `M6` (collect only top-level bindings) must fail the nested cases.
- Obligation: `INV-consistent-action`, `INV-witness`, and `L-witness`.
  Method: `rstest` tables: two invocations of one action with equal and with
  different signatures; the four combinations of policy and witness presence; a
  scan violation coexisting with a missing witness (the scan violation wins).
  Artefact: `well_formed_tests.rs`. Non-vacuity: each table includes its
  rejecting rows.
- Obligation: `INV-arguments`.
  Method: named `rstest` cases (declared order preserved, unknown argument,
  missing argument, duplicate parameter, record argument for a generic or
  qualified-self parameter rejected, record argument for `&Node` accepted) and
  a property test: for any permutation of a complete argument map over 0 to 6
  distinct parameters, the resolved order equals the declared order and every
  value is unchanged. Artefact:
  `crates/theoremc-core/src/harness_plan/invocation_tests.rs`. Non-vacuity: the
  generator includes empty and single-parameter actions and non-identity
  permutations; negative control `M7` (keep the caller's order) must fail the
  property.
- Obligation: `INV-passing-mode`.
  Method: `rstest` table: `&mut T`, `&T`, `&'static str`, `T`, `Box<&mut T>`,
  `Option<&T>`, `[u8; 4]`, `&[u8]`, `&mut [u8]`, and `()`. Artefact:
  `crates/theoremc-core/src/harness_plan/rust_type_tests.rs`.
- Obligation: `INV-identity-display`.
  Method: `rstest` display table for every section, and a property test that
  distinct generated roots (all sections, positions up to `usize::MAX`) display
  distinctly and that displaying is deterministic. Artefact:
  `crates/theoremc-core/src/harness_plan/identity_tests.rs`. Rationale: the
  grammar is regular and its alphabets are disjoint (section names are
  lower-case letters, positions are decimal digits, delimiters are `[` and
  `]`); injectivity over generated roots is adequate.
- Obligation: `INV-leaf-validity` and `INV-canonical-equality`.
  Method: `rstest` accept and reject tables per constructor, asserting typed
  error variants with `googletest` matchers, including `_`, `__theoremc_x`,
  `r#type`, a non-ASCII name, keywords, `NaN`, infinity, an absolute path, a
  `..` path, statement-like expressions, and free named lifetimes; and equality
  cases `a==b` versus `a == b` and `Vec<u8>` versus `Vec <u8>`. The identifier
  cases must agree with `schema::validate_identifier`'s existing behaviour;
  record the observed outcome for `r#type` and non-ASCII names. Artefact:
  sibling `*_tests.rs` files.
- Obligation: `INV-backend-free`.
  Method: a static `Send + Sync` assertion on `HarnessPlan`; an `insta` debug
  snapshot of the longhand idempotence plan; a marker-scan assertion over the
  same debug text; a boundary unit test that parses every file under
  `crates/theoremc-core/src/harness_plan/` with `syn::parse_file` (read through
  `cap_std::fs_utf8`) and checks every `use` path against the forbidden roots
  and the schema allow-list. Artefact:
  `crates/theoremc-core/src/harness_plan/plan_snapshot_tests.rs`;
  `crates/theoremc-core/src/harness_plan/boundary_tests.rs`. Non-vacuity: a
  unit test feeds the marker scan a contaminated string with each forbidden
  marker and requires each to be reported; a unit test feeds the boundary
  checker synthetic sources containing `use quote::quote;` and
  `use crate::schema::TheoremDoc;` and requires both to be rejected.
- Obligation: `INV-no-adapters`, `INV-immutable`, `INV-opaque`, and
  `INV-section-kind`. Method: type-level (private fields, no `&mut` accessors,
  single `Predicate` variant, typed site constructors) plus one `rstest` case
  per operation kind asserting that the constructed site's section matches, and
  one comparing supplied and stored values.

### Formal verification decision

No formal proof is introduced in this task (decision D11). The only lemma
family suitable for deductive proof is the scope kernel's (`L-step`, `L-sound`,
`L-complete`). It is contractual (every renderer relies on it), but it is not
yet stable: task 5.2.3 replaces the growing set of definitions with a scope
stack, which falsifies `L-step` as stated. Proving the flat kernel now would
produce a proof to be rewritten within the same phase, plus toolchain pins,
Makefile targets, contract tests, and a CI job, on the critical path that
ADR-0011 decision 7 keeps clear. The exhaustive small-scope differential test
gives strong evidence for the flat kernel; the generated test extends it to
longer sequences; and the lemma statements are recorded, unproved, in the
design document.

Verus becomes warranted when the scoped kernel exists: it must hold for any
nesting depth and any length, bounded tests weaken there, and the kernel is
then stable. This plan therefore proposes, in milestone `EP-M4`, a new roadmap
task after 5.2.3: "Prove the scoped binding and identity kernel with Verus",
requiring 5.2.3, with success criteria: `make verus` verifies prefix extension,
soundness, and completeness of the scoped kernel over a `Seq`-based model with
an empty trusted surface; a shared golden corpus of event sequences is checked
by both the Rust tests and Verus; and the Verus job runs on schedule and on
demand only, per the Verus on-ramp.

Kani adds nothing here: there is no `unsafe` code and no arithmetic beyond
indexing, and the exhaustive test already enumerates the bounded state space.
Kani remains the backend under test from task 5.1.4.

Residual gaps:

- `GAP-R1`: opaque predicates may name bindings that are undefined or defined
  later; the plan cannot see inside Rust text. rustc reports this when the Kani
  renderer's output compiles (task 5.1.3 onward).
- `GAP-R2`: moves after by-value passing and `&mut` aliasing are not checked by
  the plan; rustc reports them as theorem authoring errors, as DES §5.5.3
  requires.
- `GAP-R3`: the flat kernel's lemmas are evidenced by exhaustive and generated
  testing, not proved; the proposed roadmap task owns the proof for the scoped
  kernel.

## Plan of work

The module layout under `crates/theoremc-core/src/`:

- `rust_syntax/mod.rs`, `rust_syntax/expr.rs`, `rust_syntax/rust_type.rs`
  (moved from `schema/expr.rs` and `schema/rust_type.rs`, with their test
  files), crate-private.
- `harness_plan/mod.rs`: module documentation (purpose, unstable internal
  status, allow-list, reuse policy), re-exports, `HarnessPlan`,
  `HarnessPlan::try_new`, and accessors.
- `harness_plan/identity.rs`: `TheoremPath`, `TheoremIdentity`,
  `PlanProvenance`, `PlanSection`, `ObligationId`, and `Display`.
- `harness_plan/names.rs`: `BindingName`, `ParameterName`, `FieldName`, and
  `BindingId`.
- `harness_plan/value.rs`: `FiniteF64`, `PlanLiteral`, `PlanValue`,
  `RecordField`, and `ArgumentValue`.
- `harness_plan/rust_type.rs`: `RustType` and `PassingMode`.
- `harness_plan/predicate.rs`: `RustExpression`, `Rationale`, `Predicate`, and
  `PredicateObligation` with its three site constructors.
- `harness_plan/invocation.rs`: `ActionParameter`, `ResolvedAction`,
  `InvocationMode`, `InvocationSite`, `ResolvedArgument`, and
  `ResolvedInvocation`.
- `harness_plan/operation.rs`: `SymbolicBinding` and `Operation`.
- `harness_plan/evidence.rs`: `EvidencePlan`, `VacuityPolicy`, and
  `KaniEvidencePlan`.
- `harness_plan/kernel.rs`: `ScopeEvent`, `Violation`, and `first_violation`.
- `harness_plan/well_formed.rs`: the operation-to-event adapter, the
  consistent-action and witness checks, and `ArgumentPath`.
- `harness_plan/error.rs`: `PlanValueError`, `InvocationError`, and
  `PlanError`.
- `harness_plan/test_fixtures.rs`: `#[cfg(any(test, feature = "test-support"))]`
  `#[doc(hidden)]` fixtures.
- Sibling tests: `identity_tests.rs`, `names_tests.rs`, `value_tests.rs`,
  `rust_type_tests.rs`, `predicate_tests.rs`, `invocation_tests.rs`,
  `well_formed_tests.rs`, `kernel_exhaustive_tests.rs`,
  `kernel_property_tests.rs`, `plan_snapshot_tests.rs`, `boundary_tests.rs`, and
  `snapshots/`. Merge small test files where that keeps every file under 400
  lines and the file count within tolerance.

### Milestone `EP-M0`: baseline

Run the three gates on the unmodified branch through the `scrutineer` agent and
record pass counts and any pre-existing failure (notably the two Kani
scenarios) in `Surprises & discoveries`. No commit unless this plan changes.
Acceptance: logs
`/tmp/{check-fmt,lint,test}-theoremc-5-1-1-internal-backend-neutral-harness-plan.out`
exist and their results are recorded.

### Milestone `EP-M1`: neutral Rust-syntax module, identity, provenance, and leaf values

Assigned: `REQ-location`, `REQ-provenance`, `REQ-identity`; `GAP-shared-syntax`,
`GAP-no-plan` (module skeleton), `GAP-no-identity`, `GAP-no-provenance`.

Refactor first (separate commit, behaviour unchanged): move `schema/expr.rs` and
`schema/rust_type.rs` and their test files into `crate::rust_syntax`, update
`schema` imports, and run the three gates. Commit: "Move Rust syntax validators
into a neutral module".

Then red: create `harness_plan/mod.rs` with module documentation and module
declarations, and write the failing tests in `identity_tests.rs`,
`names_tests.rs`, `value_tests.rs`, `rust_type_tests.rs`, and
`predicate_tests.rs`. Run
`cargo nextest run -p theoremc-core --all-features harness_plan` and observe
the compile failure for missing items. Green: implement the leaf types and
`PlanValueError`. Refactor. Add the identity property test.

End state: leaf types exist and are tested; `harness_plan` is declared in
`crates/theoremc-core/src/lib.rs` as `#[doc(hidden)] pub mod`. Acceptance:
`make check-fmt`, `make lint`, and `make test` pass; the named tests in the
trace links for this milestone pass. Commit: "Add harness-plan identity and
leaf values". Recovery: revert the commit; the refactor commit stands alone.
Remaining gaps: invocations, operations, plan.

### Milestone `EP-M2`: resolved invocations, operations, and evidence

Assigned: `REQ-invocation`, `REQ-opaque`, `REQ-repertoire`, `REQ-no-adapters`;
`INV-section-kind`.

Red: write `invocation_tests.rs` (declared order, unknown and missing
arguments, duplicate parameters, record-argument shape rules, values stored
verbatim, `let` sites requiring a result, argument permutation property) and
operation, site, and evidence cases. Green: implement `invocation.rs`,
`operation.rs`, `evidence.rs`, and `InvocationError`.
`ResolvedInvocation::try_new` takes the `InvocationSite`, the `ResolvedAction`,
the caller's arguments as an `IndexMap<ParameterName, ArgumentValue>`, the
`InvocationMode`, and an optional result `BindingId`.

End state: every operation of ADR-0006's minimum repertoire is representable
with a section-correct site. Acceptance: three gates pass. Commit: "Add
resolved invocations and harness-plan operations". Recovery: revert.

### Milestone `EP-M3`: well-formed `HarnessPlan` with executable verification

Assigned: `INV-scope`, `INV-unique-binding`, `INV-unique-identity`,
`INV-consistent-action`, `INV-witness`, `REQ-backend-free`, `REQ-vacuity`;
`GAP-binding-uniqueness`.

Red: write `well_formed_tests.rs` (named cases including
`self_reference_is_rejected`, forward reference, nested references, field and
parameter names that are not uses, duplicate binding across symbolic and
invocation results, duplicate identity, inconsistent action, witness table,
violation priority), the oracle and the exhaustive and generated differential
tests, and the class-count assertions. Green: implement `kernel.rs`,
`well_formed.rs`, `HarnessPlan::try_new`, `PlanError`, and `test_fixtures.rs`.
Then add `plan_snapshot_tests.rs` (static `Send + Sync` assertion, `insta`
debug snapshot, marker scan) and `boundary_tests.rs`. Add `insta` to
`theoremc-core`'s development dependencies. Run negative controls `M1` to `M7`
and the marker and boundary controls one at a time; record each failing output
in `Artefacts and notes`; restore each with `git checkout -- <file>` and confirm
`git status` shows only intended changes.

End state: an ill-formed plan cannot be constructed; the longhand idempotence
fixture is accepted and snapshotted. Acceptance: three gates pass;
`cargo insta test -p theoremc-core --all-features --check` reports no pending
snapshots. Commit: "Enforce harness-plan well-formedness". Recovery: revert.

### Milestone `EP-M4`: documentation, roadmap, and closure

- Add `docs/theoremc-design.md` §8.6 "Harness plan (Task 5.1.1)": the plan
  shape; identity grammar and its reserved children (D4); structural provenance
  and the identity–provenance split (D3); opaque predicates and canonical
  equality (D2); binding identity and verbatim local names (D7); argument value
  shapes (D15); the vacuity lift and source-witness rule (D5); the
  well-formedness contract, its abstract model, and the unproved lemma
  statements, with the explicit note that nested scopes falsify `L-step` as
  stated; and residual gaps. Update §3 and the §3.1 diagram labels so the
  backend-agnostic plan reads as implemented and lowering as planned.
- Replace "Accepted harness-planning boundary (pending implementation)" in
  `docs/developers-guide.md` §2 with the implemented internal interface: its
  unstable status, the schema allow-list, the `rust_syntax` neutral module, the
  reuse policy (permitted call sites: `theoremc-core` lowering in a sibling
  module and the `theoremc-macros` renderer; no facade re-export; extend by
  adding operation variants, `BindingId` variants, and identity children, not
  parallel types), and the verification inventory. Add a separate "unstable
  internal interfaces" table rather than extending the §3.6.1 "stable internal
  API" table. Update the §2 module-layer table.
- Update `docs/repository-layout.md` and `docs/contents.md`.
- Add the proposed roadmap task with `mapsplice insert` after task 5.2.3
  (preview, review the diff, then apply with `--in-place`), only if the user
  approved decision D11's follow-up: "Prove the scoped binding and identity
  kernel with Verus", `Requires 5.2.3`, citing ADR-0006, ADR-0007, and
  theoremc-design.md §8.6, with the success criteria in
  `Formal verification decision`.
- `docs/users-guide.md`: no change, because nothing user-visible changes; say
  so in the PR.
- Mark roadmap task 5.1.1 done (`- [x]`) only after every acceptance item
  passes.

Acceptance: `make fmt`, `make check-fmt`, `make markdownlint`, `make nixie`,
`make spelling`, `make lint`, and `make test` pass; `mdtablefix --check` passes
on each changed Markdown file. Commit: "Document the harness plan". Then set
this plan's status to `COMPLETE` after reconciling every discovery with the
conformance basis.

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
cargo nextest run -p theoremc-core --all-features rust_syntax
cargo insta test -p theoremc-core --all-features --check
```

Expected focused-test output after `EP-M3` (counts are illustrative):

```plaintext
Summary [  2.4s] 90 tests run: 90 passed, 0 skipped
```

## Validation and acceptance

Behavioural acceptance is expressed as named unit tests rather than a BDD suite
(decision D20). Each scenario below is a test in
`crates/theoremc-core/src/harness_plan/well_formed_tests.rs` or
`invocation_tests.rs`, built from the shared fixtures:

- `longhand_idempotence_plan_is_accepted`: construction succeeds; the
  operation identities are `forall[0]`, `assume[0]`, `let[0]`, `let[1]`,
  `prove[0]`, and `witness[0]`; the debug description contains no forbidden
  marker.
- `binding_used_before_definition_is_rejected`: with the two invocations
  swapped, construction fails with `UndefinedBinding` for `once` at `let[1]` in
  argument `value`.
- `binding_defined_twice_is_rejected`: with the second invocation's result
  renamed to `x`, construction fails with `DuplicateBinding` for `x` at
  `let[1]`.
- `plan_without_witness_is_rejected_by_default`: without its witness and with
  `RequireWitness`, construction fails with `MissingWitness`.
- `justified_vacuous_plan_is_accepted`: without its witness and with
  `AllowVacuous` because "the claim is a pure algebraic identity", construction
  succeeds.
- `arguments_follow_declared_order`: for `demo.combine` declaring `left` then
  `right`, an invocation supplying `right` before `left` resolves to
  `left, right`.
- `undeclared_argument_is_rejected`: supplying `left`, `right`, and `extra`
  fails with an `InvocationError` naming `extra` and `demo.combine`.

The longhand idempotence fixture is: theorem path
`theorems/idempotence.theorem`, document 0, theorem `NormalizeIsIdempotent`;
`forall[0]` symbolic `x: u8`; `assume[0]` opaque `x <= 200` because "keep the
domain small"; `let[0]` `once = demo.normalize(value: x)` where `value: &u8`
(`SharedBorrow`) and the return type is `u8`; `let[1]`
`twice = demo.normalize(value: once)`; `prove[0]` opaque `once == twice`
because "normalizing twice equals normalizing once"; `witness[0]` opaque
`once != x` because "some input changes under normalization"; evidence
`RequireWitness` with Kani unwind 1 and expectation `Success`. Task 5.1.4 and
the later RFC 0001 plan-equivalence fixture can reuse it.

Acceptance, as observable behaviour:

- `cargo nextest run -p theoremc-core --all-features harness_plan` passes and
  includes the tests named above and in the trace links; each negative control
  makes its target test fail for the intended reason.
- The snapshot shows the six operations with the identities above, the binding
  names `x`, `once`, and `twice`, the canonical action name `demo.normalize`,
  the declared types, the `SharedBorrow` passing mode, and the opaque
  expression texts, and none of the forbidden markers.
- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, and `make spelling` pass.

## Idempotence and recovery

Every step is re-runnable. Each milestone ends in a commit that passes the
gates, so `git revert <commit>` restores the previous plateau; the
`rust_syntax` move is its own commit and can be reverted independently.
Negative controls are temporary edits: apply, observe the failure, then restore
with `git checkout -- <file>` and confirm `git status` is clean before
continuing. Do not use `git stash` (the stash stack is shared between
worktrees). If `cargo insta` leaves `*.snap.new` files after a failed run,
review them with `cargo insta review` or delete them; never accept a snapshot
without confirming the change is a real contract change.

## Artefacts and notes

Record here, during implementation: baseline gate results; the outcome for
`r#type` and non-ASCII names under `validate_identifier`; the auto-trait
premise for `proc_macro2`; negative-control outputs for `M1` to `M7`, the
marker scan, and the boundary checker; exhaustive-test class counts and run
time.

## Interfaces and dependencies

All items live in `theoremc_core::harness_plan`, are `pub` for the macro
crate's later use, and are documented as unstable internal interfaces.
Indicative signatures, to be refined only within the tolerances:

```rust,no_run
pub struct TheoremPath(Utf8PathBuf);        // separators normalized; relative; no `..`
pub struct TheoremIdentity { /* path: TheoremPath, name: TheoremName */ }
pub struct PlanProvenance { /* document_index: usize */ }

pub enum PlanSection { Forall, Assume, Let, Do, Prove, Witness }
pub struct ObligationId { /* private; root section and position */ } // Clone, Eq, Ord, Hash; not Copy
impl ObligationId {
    pub fn root(section: PlanSection, position: usize) -> Self;
    pub fn section(&self) -> PlanSection;
    pub fn position(&self) -> usize;
}
impl fmt::Display for ObligationId { /* "prove[0]" */ }

pub struct BindingName(String);   // identifier; not `_`; not prefixed `__theoremc`
pub struct ParameterName(String); // identifier
pub struct FieldName(String);     // identifier
pub enum BindingId { Source(BindingName) }

pub struct RustType { /* text, canonical */ }       // Eq/Hash by canonical
pub enum PassingMode { Move, SharedBorrow, MutableBorrow }
pub struct RustExpression { /* text, canonical */ } // Eq/Hash by canonical
pub struct Rationale(String);                       // non-empty after trimming

pub struct FiniteF64(f64);                          // finite; Eq/Hash by bits
pub enum PlanLiteral { Bool(bool), Integer(i64), Float(FiniteF64), String(String) }
pub enum PlanValue { Literal(PlanLiteral), Binding(BindingId), List(Vec<PlanValue>) }
pub struct RecordField { /* name: FieldName, value: PlanValue */ }
pub enum ArgumentValue { Value(PlanValue), Record(Vec<RecordField>) }

pub struct ActionParameter { /* name: ParameterName, declared: RustType, passing: PassingMode */ }
pub struct ResolvedAction { /* name: CanonicalActionName, parameters, returns: RustType */ }
pub enum InvocationMode { Call }
pub enum InvocationSite { Let(usize), Do(usize) }
pub struct ResolvedArgument { /* parameter: ActionParameter, value: ArgumentValue */ }
pub struct ResolvedInvocation { /* site, action, arguments, mode, result: Option<BindingId> */ }
impl ResolvedInvocation {
    pub fn try_new(
        site: InvocationSite,
        action: ResolvedAction,
        arguments: IndexMap<ParameterName, ArgumentValue>,
        mode: InvocationMode,
        result: Option<BindingId>,
    ) -> Result<Self, InvocationError>;
}

pub enum Predicate { OpaqueRust(RustExpression) }
pub struct PredicateObligation { /* id: ObligationId, predicate, rationale */ }
impl PredicateObligation {
    pub fn assumption(position: usize, predicate: Predicate, rationale: Rationale) -> Self;
    pub fn assertion(position: usize, predicate: Predicate, rationale: Rationale) -> Self;
    pub fn witness(position: usize, predicate: Predicate, rationale: Rationale) -> Self;
}
pub struct SymbolicBinding { /* id: forall site, binding: BindingId, declared: RustType */ }
pub enum Operation {
    Symbolic(SymbolicBinding),
    Assume(PredicateObligation),
    Invoke(ResolvedInvocation),
    Assert(PredicateObligation),
    Cover(PredicateObligation),
}

pub enum VacuityPolicy { RequireWitness, AllowVacuous { because: Rationale } }
pub struct KaniEvidencePlan { /* unwind: NonZeroU32, expect: KaniExpectation */ }
pub struct EvidencePlan { /* vacuity: VacuityPolicy, kani: Option<KaniEvidencePlan> */ }

pub struct HarnessPlan { /* theorem, provenance, evidence, operations */ }
impl HarnessPlan {
    pub fn try_new(
        theorem: TheoremIdentity,
        provenance: PlanProvenance,
        evidence: EvidencePlan,
        operations: Vec<Operation>,
    ) -> Result<Self, PlanError>;
    pub const fn theorem(&self) -> &TheoremIdentity;
    pub const fn provenance(&self) -> &PlanProvenance;
    pub const fn evidence(&self) -> &EvidencePlan;
    pub fn operations(&self) -> &[Operation];
}
```

Whether `PredicateObligation`'s three constructors are better expressed as one
constructor taking a `PredicateSite` enum is an implementation choice within
these tolerances; either way the section must follow from the site type.
`ObligationId::root` is public so renderers and tests can name identities, but
operations obtain their identities only from typed sites.

Errors (decision D18): `PlanError`
(`UndefinedBinding { binding, site, argument }`,
`DuplicateBinding { binding, site }`, `DuplicateIdentity { id }`,
`InconsistentAction { action, site }`, `MissingWitness`);
`InvocationError { action, parameter, kind }` with kinds for unknown and
missing arguments, duplicate parameters, missing `let` result, and record
arguments for unsupported parameter types; `PlanValueError` for leaf values.
All derive `Clone`, `Eq`, and `thiserror::Error` and are `#[non_exhaustive]`.

Reused items (the allow-list): `CanonicalActionName`, `TheoremName`,
`KaniExpectation`, `schema::validate_identifier`, and `crate::rust_syntax`.

Dependencies: no new production dependency. New `theoremc-core` development
dependency: `insta = "1.47.2"`.

## Revision notes

- 2026-11-16: initial draft from Wyvern reconnaissance and Firecrawl checks of
  `rust-prover-tools` and Verus releases.
- 2026-11-16: revised after a four-member community-of-experts design review
  (verdict "revise"). Changes: Verus milestone, ADR-0014, and CI wiring removed
  and replaced by a proposed roadmap task after 5.2.3, because nested scopes
  falsify the flat kernel's prefix-extension lemma (D11); BDD suite replaced by
  named tests (D20); binding identity generalized to `BindingId` (D7); identity
  split from provenance (D3); typed sites (D17); distinct name newtypes (D14);
  root-only records and finite floats (D15); canonical equality for types and
  expressions (D2); opaque, non-`Copy` identities (D4); source-witness rule
  (D5); consistent actions (D8); locatable errors (D18); generic kernel with
  ordered uses and exhaustive small-scope testing with seven mutants (D19);
  neutral `rust_syntax` module (D13); `TheoremPath` normalization (D16);
  `#[doc(hidden)]` module (D1); budgets reduced to 2,600 code lines and 36
  files. Remaining work is unchanged in shape: four implementation milestones
  after approval.
