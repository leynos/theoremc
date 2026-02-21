# Theoremc development roadmap

This roadmap translates the settled design into an implementation sequence that
can be executed as atomic, testable increments. It is grounded in the normative
specification and decision record documents.

## Sources and requirement signposts

Use these signposts to trace each roadmap task to the defining requirement.

- `DES-2`:
  [docs/theoremc-design.md §2](theoremc-design.md#2-non-negotiable-constraints)
  (non-negotiable constraints).
- `DES-3`:
  [docs/theoremc-design.md §3](theoremc-design.md#3-high-level-architecture)
  (high-level architecture and pipeline shape).
- `DES-4`:
  [docs/theoremc-design.md §4](theoremc-design.md#4-the-theorem-file-format)
  (theorem format and step semantics).
- `DES-5`:
  [docs/theoremc-design.md §5](theoremc-design.md#5-rust-actions-step-definitions-for-proofs)
   (action model and argument shaping).
- `DES-6`:
  [docs/theoremc-design.md §6](theoremc-design.md#6-parsing-and-validation)
  (parsing, validation, and diagnostics).
- `DES-6.5`:
  [docs/theoremc-design.md §6.5](theoremc-design.md#65-localized-diagnostics-contract-adr-002)
   (localized diagnostics contract and localization boundaries).
- `DES-7`:
  [docs/theoremc-design.md §7](theoremc-design.md#7-build-integration-always-connected)
   (build integration and compile-time connectedness).
- `DES-8`: [docs/theoremc-design.md §8](theoremc-design.md#8-kani-backend-mvp)
  (Kani backend semantics, witnesses, and vacuity policy).
- `DES-9`: [docs/theoremc-design.md §9](theoremc-design.md#9-reporting-theoremd)
  (reporting scope and formats).
- `DES-10`:
  [docs/theoremc-design.md §10](theoremc-design.md#10-enforcement-guardrails-not-the-primary-binding-mechanism)
   (optional enforcement via lints).
- `DES-4.7`:
  [docs/theoremc-design.md §4.7](theoremc-design.md#47-theorem-schema-internationalization-scope)
   (theorem schema keyword internationalization scope).
- `TFS-1`:
  [docs/theorem-file-specification.md §§1-3](theorem-file-specification.md#1-yaml-a-human-readable-data-serialization-format-schema-reference-v1)
   (document model and conformance rules).
- `TFS-4`:
  [docs/theorem-file-specification.md §4](theorem-file-specification.md#4-step-and-action-schemas)
   (step and action schema).
- `TFS-5`:
  [docs/theorem-file-specification.md §5](theorem-file-specification.md#5-value-forms-and-how-they-compile)
   (value forms and explicit reference semantics).
- `TFS-6`:
  [docs/theorem-file-specification.md §6](theorem-file-specification.md#6-evidence-schema)
   (evidence schema).
- `NMR-1`:
  [docs/name-mangling-rules.md §§Action and harness mangling](name-mangling-rules.md#action-name-mangling)
   (action and harness mangling rules).
- `NMR-2`:
  [docs/name-mangling-rules.md §Stable external theorem identifiers](name-mangling-rules.md#stable-external-theorem-identifiers)
   (stable external theorem identifiers and alias migration rules).
- `ADR-1`:
  [ADR 0001 decision 1](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
   (injective action mangling).
- `ADR-2`:
  [ADR 0001 decision 2](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
   (injective harness naming).
- `ADR-3`:
  [ADR 0001 decision 3](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
   (explicit `{ ref: ... }` semantics).
- `ADR-4`:
  [ADR 0001 decision 4](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
   (non-vacuity witness policy).
- `ADR-5`:
  [ADR 0001 decision 5](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md)
   (stable external IDs and migration aliases).
- `ADR2-1`:
  [ADR 002 decision 1](adr-002-library-first-internationalization-and-localization-with-fluent.md)
   (library-first localizer injection; no theoremc locale negotiation).
- `ADR2-2`:
  [ADR 002 decision 2](adr-002-library-first-internationalization-and-localization-with-fluent.md)
   (stable diagnostic code + args + English fallback as source of truth).
- `ADR2-3`:
  [ADR 002 decision 3](adr-002-library-first-internationalization-and-localization-with-fluent.md)
   (Fluent default backend with consumer-over-default layering).
- `ADR2-4`:
  [ADR 002 decision 4](adr-002-library-first-internationalization-and-localization-with-fluent.md)
   (deterministic English for compile-time and machine-facing artefacts).
- `ADR2-5`:
  [ADR 002 decision 5](adr-002-library-first-internationalization-and-localization-with-fluent.md)
   (parser keyword internationalization deferred to future ADR).

## Phase 1: schema and validation foundation

Outcome: theorem documents are parsed, validated, and diagnosed
deterministically with source-located errors.

### Step 1.1: implement strict theorem document deserialization

Dependencies: none.

In scope: schema structs, key alias support, unknown-key rejection, and
multi-document file loading.

Out of scope: code generation and backend emission.

- [x] Implement `TheoremDoc` and subordinate schema types with
  `serde(deny_unknown_fields)` and TitleCase plus lower-case aliases exactly as
  specified. Acceptance: unit tests prove unknown keys and wrong scalar types
  fail deserialization with actionable errors. Signposts: `TFS-1`, `DES-6`.
- [x] Implement `.theorem` multi-document loading (`---` separation) into an
  ordered in-memory collection. Acceptance: parser tests cover one-document and
  many-document files with stable document ordering. Signposts: `TFS-1`,
  `DES-6`.
- [x] Enforce theorem identifier lexical rules (`^[A-Za-z_][A-Za-z0-9_]*$`) and
  Rust keyword rejection. Acceptance: validation tests reject reserved keywords
  and invalid ASCII identifiers with line/column diagnostics. Signposts:
  `TFS-1`, `DES-6`.

### Step 1.2: implement semantic validation rules

Dependencies: step 1.1.

In scope: structural and semantic checks for `Let`, `Do`, `Prove`, `Witness`,
`Evidence`, and expression syntax validation.

Out of scope: Rust typechecking of expressions.

- [x] Validate required fields and non-empty constraints for `Theorem`,
  `About`, `Prove`, and Kani evidence requirements. Acceptance: negative tests
  cover each missing/empty field and confirm deterministic error messages.
  Signposts: `TFS-1`, `TFS-6`, `DES-6`.
- [x] Parse `Assume.expr`, `Prove.assert`, and `Witness.cover` as `syn::Expr`
  and reject statement blocks. Acceptance: tests demonstrate single-expression
  acceptance and block-style rejection. Signposts: `TFS-1`, `DES-6`.
- [x] Enforce `Step` and `LetBinding` shape rules (`Let` allows only `call` or
  `must`, `maybe` requires `because` and nested `do`). Acceptance: validation
  tests cover each invalid variant combination. Signposts: `TFS-4`, `DES-4`.
- [x] Enforce non-vacuity defaults (`Witness` required unless
  `allow_vacuous: true` with non-empty `vacuity_because`). Acceptance: tests
  cover valid and invalid vacuity declarations and default-failure behaviour.
  Signposts: `TFS-6`, `ADR-4`, `DES-8`.

### Step 1.3: implement diagnostics and parser test corpus

Dependencies: steps 1.1 and 1.2.

In scope: error reporting quality, location fidelity, and regression fixtures.

Out of scope: backend-specific error rendering.

- [x] Wrap parser and validator failures in structured diagnostics that include
  source file, line, and column. Acceptance: snapshot tests assert stable,
  source-located diagnostic output for representative failures. Signposts:
  `DES-6`, `TFS-1`.
- [x] Build a fixture suite of valid and invalid `.theorem` files that covers
  aliases, nested `maybe`, `must` semantics preconditions, and witness policy.
  Acceptance: fixtures run in continuous integration (CI) and gate parser and
  validator regressions. Signposts: `TFS-1`, `TFS-4`, `TFS-6`, `ADR-4`.

## Phase 2: action resolution and deterministic naming

Outcome: action references and generated symbols are injective, stable, and
compile-time checked.

### Step 2.1: implement action name mangling and resolution

Dependencies: phase 1.

In scope: action name grammar checks, mangled identifier generation, collision
detection, and binding to `crate::theorem_actions`.

Out of scope: runtime action registries.

- [ ] Implement canonical action-name validation for dot-separated segments with
  per-segment identifier and keyword rules. Acceptance: tests reject malformed
  names and reserved-keyword segments. Signposts: `NMR-1`, `TFS-4`, `ADR-1`.
- [ ] Implement action mangling (`segment_escape`, `action_slug`,
  `hash12(blake3)`) and canonical path resolution into
  `crate::theorem_actions`. Acceptance: golden tests cover representative names
  and underscore edge cases. Signposts: `NMR-1`, `ADR-1`, `DES-5`.
- [ ] Fail compilation on duplicate canonical action names and duplicate mangled
  identifiers, reporting all colliding sources. Acceptance: integration tests
  prove both collision classes are detected before backend execution.
  Signposts: `NMR-1`, `ADR-1`, `DES-5`.

### Step 2.2: implement harness and module naming stability

Dependencies: step 2.1.

In scope: per-file module mangling, harness naming, and theorem-key collision
checks.

Out of scope: report-level ID alias resolution.

- [ ] Implement per-file module naming using `path_mangle(path_stem(P))` and
  `hash12(P)`. Acceptance: snapshot tests confirm deterministic names for mixed
  separators and punctuation-heavy paths. Signposts: `NMR-1`, `ADR-2`, `DES-7`.
- [ ] Implement harness naming
  `theorem__{theorem_slug(T)}__h{hash12(P#T)}` with deterministic CamelCase to
  snake_case conversion. Acceptance: tests cover acronym runs, numeric
  boundaries, and already-snake identifiers. Signposts: `NMR-1`, `ADR-2`.
- [ ] Enforce duplicate theorem-key rejection (`P#T`) at build time. Acceptance:
  integration tests prove collisions fail with actionable theorem source
  diagnostics. Signposts: `NMR-1`, `ADR-2`, `TFS-1`.

### Step 2.3: implement explicit argument value semantics

Dependencies: phase 1 and step 2.1.

In scope: value lowering for literals, references, lists, and map-driven struct
literal synthesis.

Out of scope: implicit reference inference.

- [ ] Implement argument decoding so plain YAML strings are always literals and
  variable references require `{ ref: name }`. Acceptance: tests prove adding a
  new binding cannot alter existing literal argument semantics. Signposts:
  `TFS-5`, `ADR-3`, `DES-5`.
- [ ] Implement optional `{ literal: "text" }` wrapper and reject ambiguous
  wrapper maps containing unsupported sentinel keys. Acceptance: parser tests
  cover valid wrapper use and deterministic rejection cases. Signposts:
  `TFS-5`, `ADR-3`.
- [ ] Implement struct-literal synthesis from YAML maps based on action
  parameter types, plus recursive list lowering to `vec![...]`. Acceptance:
  compile-fail tests show type mismatches are surfaced by Rust compilation.
  Signposts: `TFS-5`, `DES-5`.

## Phase 3: compile-time integration and harness generation

Outcome: theorem files are always connected to Rust compilation and generated
proof harnesses.

### Step 3.1: implement build discovery and suite generation

Dependencies: phases 1 and 2.

In scope: `build.rs` discovery of theorem files, change tracking, and generated
suite include wiring.

Out of scope: theorem execution.

- [ ] Implement `build.rs` scanning of `theorems/**/*.theorem` and emission of
  `cargo::rerun-if-changed` lines for directory and file paths. Acceptance:
  build integration tests confirm theorem edits trigger rebuilds reliably.
  Signposts: `DES-7`.
- [ ] Generate `OUT_DIR/theorem_suite.rs` with one `theorem_file!(...)`
  invocation per discovered path, then include it from crate code. Acceptance:
  empty, single-file, and multi-file suites compile deterministically.
  Signposts: `DES-7`.

### Step 3.2: implement `theorem_file!` proc-macro expansion

Dependencies: step 3.1.

In scope: module scaffolding, backend submodule layout, `include_str!` wiring,
and generated harness stubs.

Out of scope: final backend semantics.

- [ ] Implement macro expansion that emits the stable per-file module, includes
  theorem source content via `include_str!`, and builds one harness entry per
  theorem document. Acceptance: macro-expansion snapshots remain stable across
  repeated builds. Signposts: `DES-7`, `NMR-1`, `ADR-2`.
- [ ] Gate generated Kani harnesses with `#[cfg(kani)]` and emit required
  `#[kani::proof]` and `#[kani::unwind(n)]` attributes from evidence.
  Acceptance: non-Kani `cargo build` succeeds and Kani-targeted builds discover
  harnesses. Signposts: `DES-7`, `DES-8`, `TFS-6`.

### Step 3.3: implement compile-time binding probes

Dependencies: step 3.2.

In scope: generated probe bindings for actions and types to detect drift.

Out of scope: runtime reflection.

- [ ] Emit typed action probes (`let _: fn(...) -> ... = ...;`) for every
  referenced action to force signature compatibility at compile time.
  Acceptance: signature drift causes compile failure in the theorem owner
  crate. Signposts: `DES-7`, `DES-5`, `NMR-1`.
- [ ] Emit referenced-type probes for generated struct literal synthesis and
  step bindings to surface missing-type and moved-type breakages early.
  Acceptance: compile-fail tests validate predictable drift diagnostics.
  Signposts: `DES-7`, `DES-5`.

## Phase 4: Kani backend semantics and safety policy

Outcome: theorem steps compile into correct Kani proof harnesses with explicit
non-vacuity guarantees.

### Step 4.1: implement theorem step emission for Kani

Dependencies: phases 1 to 3.

In scope: `Forall`, `Assume`, `Let`, `Do`, `Prove`, and `Witness` emission.

Out of scope: non-Kani backends.

- [ ] Emit `Forall` symbolic bindings as typed `kani::any::<T>()` declarations
  preserving declared order. Acceptance: generated code snapshots match theorem
  declaration order and types. Signposts: `DES-8`, `TFS-1`.
- [ ] Emit `Assume` clauses as `kani::assume(...)`, and `Prove` clauses as
  `assert!(..., because)` using the supplied human rationale text. Acceptance:
  harness tests show rationale strings appear in failure output. Signposts:
  `DES-8`, `TFS-1`.
- [ ] Emit `Witness` clauses as `kani::cover!(...)` checks. Acceptance: witness
  presence is reflected in run output and reported reachability data.
  Signposts: `DES-8`, `TFS-1`, `ADR-4`.

### Step 4.2: implement `call`, `must`, and `maybe` operational semantics

Dependencies: step 4.1 and phase 2.

In scope: call binding behaviour, failure obligations for `must`, and symbolic
branching for `maybe`.

Out of scope: probabilistic branching semantics.

- [ ] Implement `call` semantics with `as` binding rules, including rejection
  of unbound fallible results in `Do`. Acceptance: semantic tests cover `()`,
  value-returning, `Result`, and `Option` action signatures. Signposts:
  `TFS-4`, `DES-4`, `DES-8`.
- [ ] Implement `must` semantics for `Result` and `Option` (`assert` then
  unwrap), and pass-through semantics for infallible actions. Acceptance:
  harness tests prove failed `must` steps produce counterexamples. Signposts:
  `TFS-4`, `DES-4`, `DES-8`.
- [ ] Implement `maybe` semantics using symbolic boolean branching and nested
  step emission. Acceptance: branch-coverage tests confirm both branches are
  explored by Kani under bounded settings. Signposts: `TFS-4`, `DES-4`, `DES-8`.

### Step 4.3: implement evidence-driven result policy

Dependencies: steps 4.1 and 4.2.

In scope: expected result handling, vacuity policy enforcement, and mismatch
reporting.

Out of scope: extended policy for future backends.

- [ ] Enforce `Evidence.kani.expect` handling and fail runs when actual status
  differs from expected status. Acceptance: integration tests cover SUCCESS,
  FAILURE, UNREACHABLE, and UNDETERMINED cases. Signposts: `TFS-6`, `DES-8`.
- [ ] Enforce default failure for UNREACHABLE and UNDETERMINED unless explicitly
  expected and justified via evidence configuration. Acceptance: policy tests
  cover default and override paths. Signposts: `ADR-4`, `DES-8`.
- [ ] Enforce vacuity override contract requiring both
  `allow_vacuous: true` and non-empty `vacuity_because`. Acceptance: validation
  and runtime tests confirm missing rationale is rejected. Signposts: `ADR-4`,
  `TFS-6`, `DES-8`.

## Phase 5: reporting and stable theorem identity

Outcome: theorem runs produce actionable artefacts with stable IDs across
renames and moves.

### Step 5.1: implement theorem run result model and report outputs

Dependencies: phase 4.

In scope: `theoremd` run model and output formats (human report plus CI
artefacts).

Out of scope: dashboard hosting.

- [ ] Implement a canonical theorem run record that includes theorem ID,
  metadata, assumptions, step outcomes, assertion outcomes, witness outcomes,
  evidence config, and final status. Acceptance: serialized fixtures round-trip
  without field loss. Signposts: `DES-9`, `TFS-1`, `TFS-6`.
- [ ] Implement Markdown/HTML report rendering from the canonical run record.
  Acceptance: golden snapshots cover pass, fail, unreachable, and undetermined
  examples. Signposts: `DES-9`.
- [ ] Implement JUnit XML and Cucumber JSON emitters for CI integration.
  Acceptance: schema validation tests pass for both formats. Signposts: `DES-9`.

### Step 5.2: implement stable external theorem IDs and alias migration

Dependencies: phase 4.

In scope: canonical ID generation, alias graph loading, cycle detection, and
resolution semantics.

Out of scope: automatic alias file editing.

- [ ] Implement canonical external theorem ID generation as
  `{normalized_path(P)}#{T}` with path normalization rules. Acceptance: tests
  cover path separator normalization and leading `./` removal. Signposts:
  `NMR-2`, `ADR-5`, `DES-9`.
- [ ] Implement alias file loading from `theorems/theorem-id-aliases.yaml` and
  deterministic resolution of deprecated IDs to canonical IDs. Acceptance:
  tests cover direct aliases and multi-hop alias chains. Signposts: `NMR-2`,
  `ADR-5`, `DES-9`.
- [ ] Detect and reject alias cycles and ambiguous resolutions at load time.
  Acceptance: cycle and ambiguity fixtures fail with actionable diagnostics.
  Signposts: `NMR-2`, `ADR-5`.

### Step 5.3: implement counterexample playback integration

Dependencies: step 5.1.

In scope: Kani failure replay orchestration and report attachment of playback
artefacts.

Out of scope: automated source rewriting workflows.

- [ ] Integrate Kani concrete playback execution for failed harnesses and
  capture generated replay artefacts. Acceptance: failing theorem integration
  test produces a linked playback artefact in the run output. Signposts:
  `DES-8`, `DES-9`.
- [ ] Surface playback metadata and retrieval paths in human and CI reports.
  Acceptance: report snapshots include replay references only when available.
  Signposts: `DES-9`.

## Phase 6: enforcement, examples, and developer ergonomics

Outcome: theoremc remains hard to bypass, easy to adopt, and easier to maintain.

### Step 6.1: implement optional enforcement guardrails

Dependencies: phases 4 and 5.

In scope: opt-in lint crate and marker-based checks.

Out of scope: mandatory lint enforcement for all adopters.

- [ ] Implement theorem-generated marker attributes and metadata needed by
  enforcement lints. Acceptance: generated harnesses carry stable markers for
  lint identification. Signposts: `DES-10`, `DES-7`.
- [ ] Implement Dylint rules that flag raw `kani::assume` and unmarked
  `#[kani::proof]` usage outside theoremc-generated modules. Acceptance: lint
  test crate demonstrates expected warnings and zero false positives in
  generated modules. Signposts: `DES-10`.

### Step 6.2: provide examples and authoring guidance

Dependencies: phases 1 to 5.

In scope: runnable examples, theorem authoring guidance, and maintenance
checklists.

Out of scope: production deployment templates.

- [ ] Create end-to-end example crates (`account`, `hnsw`) that demonstrate
  action exports, theorem files, and generated harness behaviour. Acceptance:
  examples compile and execute through the documented theorem workflow.
  Signposts: `DES-3`, `DES-5`, `DES-8`.
- [ ] Write and publish user-facing guidance for theorem authoring rules,
  especially explicit references, witness policy, and expected status usage.
  Acceptance: docs contain copy-paste-ready examples that match implementation
  semantics. Signposts: `TFS-5`, `ADR-3`, `ADR-4`, `DES-4`.
- [ ] Add a contributor checklist that requires parser fixtures, codegen
  snapshots, and report snapshots for behavioural changes. Acceptance: pull
  request template and contributor docs reference the checklist explicitly.
  Signposts: `DES-6`, `DES-7`, `DES-9`.

## Phase 7: library-first localization and Fluent diagnostics

Outcome: theoremc diagnostics remain deterministic and machine-readable while
supporting localized human rendering through injected localizers.

### Step 7.1: implement canonical diagnostic model and compatibility policy

Dependencies: phase 1.

In scope: stable code-and-arguments diagnostics model and deterministic English
fallback text.

Out of scope: locale negotiation and user interface (UI)-specific message
composition.

- [ ] Define a canonical diagnostic payload type containing code, structured
  arguments, source location, and required English fallback text. Acceptance:
  parser and validator diagnostics are emitted through this model with
  snapshot-backed stability tests. Signposts: `DES-6`, `DES-6.5`, `ADR2-2`.
- [ ] Define compatibility policy for diagnostic codes and argument schemas.
  Acceptance: contributor documentation and tests guard against accidental
  breaking changes in existing codes and argument keys. Signposts: `DES-6.5`,
  `ADR2-2`.

### Step 7.2: implement localizer contract and Fluent default backend

Dependencies: step 7.1.

In scope: library-safe localizer abstraction, embedded `en-US` resources,
consumer layering, and deterministic fallback semantics.

Out of scope: process-wide locale globals.

- [ ] Introduce a library-level `Localizer` trait (or equivalent) that renders
  diagnostic messages from code and structured arguments. Acceptance: theoremc
  APIs accept localization as injected context and never read locale
  environment variables. Signposts: `DES-6.5`, `ADR2-1`.
- [ ] Embed theoremc `en-US` Fluent resources and expose them for consumer
  loader composition. Acceptance: integration tests show host applications can
  load theoremc Fluent assets into an existing Fluent language loader.
  Signposts: `DES-6.5`, `ADR2-3`.
- [ ] Implement optional Fluent-backed localizer layering consumer catalogues
  over theoremc defaults with deterministic fallback for missing keys and
  formatting failures. Acceptance: tests cover consumer hit, default fallback,
  and formatting-error fallback paths. Signposts: `DES-6.5`, `ADR2-3`.

### Step 7.3: enforce rendering boundaries and report semantics

Dependencies: steps 7.1 and 7.2, plus phase 5.

In scope: deterministic compile-time diagnostics and dual machine/human report
fields.

Out of scope: parser keyword localization.

- [ ] Keep proc-macro and code-generation diagnostics deterministic English.
  Acceptance: compile-fail and snapshot tests confirm output stability across
  host locale changes. Signposts: `DES-6.5`, `ADR2-4`.
- [ ] Extend `theoremd` outputs to always include stable diagnostic code and
  arguments, include required English fallback text, and attach localized text
  only when a localizer is configured. Acceptance: report snapshots for
  Markdown, HTML, JUnit XML, and Cucumber JSON confirm invariant machine fields
  and optional localized projection fields. Signposts: `DES-9`, `DES-6.5`,
  `ADR2-2`, `ADR2-4`.
- [ ] Add locale-determinism regression tests proving machine-facing artefacts
  are identical across locales while localized human-facing strings vary only
  in localized fields. Signposts: `DES-9`, `DES-6.5`, `ADR2-4`.

### Step 7.4: document and guard deferred parser keyword internationalization

Dependencies: step 7.3.

In scope: explicit deferral policy and regression coverage for canonical schema
keywords.

Out of scope: implementation of localized theorem schema keys.

- [ ] Document and enforce that `.theorem` top-level keys remain canonical in
  this release line. Acceptance: parser tests reject unsupported localized key
  synonyms and docs explain the deferral policy with migration implications.
  Signposts: `DES-4`, `DES-4.7`, `ADR2-5`.

## Sequencing summary

- Execute phases in order.
- Within each phase, complete steps in order unless dependencies indicate they
  are independent.
- Do not start reporting and alias migration before the Kani execution model is
  stable.
- Treat vacuity policy implementation as a release gate, not an optional
  enhancement.
- Start localization integration only after core diagnostics are structured and
  stable.
