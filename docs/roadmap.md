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
- `DES-3.2`:
  [docs/theoremc-design.md §3.2](theoremc-design.md#32-schema-layering-boundary-contract)
  (schema layering boundary contract).
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
- `DES-10.1`:
  [docs/theoremc-design.md §10.1](theoremc-design.md#101-architectural-boundary-rules)
  (architectural layer-boundary rules).
- `DES-4.7`:
  [docs/theoremc-design.md §4.7](theoremc-design.md#47-theorem-schema-internationalization-scope)
  (theorem schema keyword internationalization scope).
- `CLI-DES`:
  [`cargo theorem` CLI design](cargo-theorem-cli-design.md) (Cargo-native
  command surface, project integration, backend management, execution,
  reporting, and agent-native contracts).
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
- `ADR3-1`:
  [ADR 003 decision 1](adr-003-architectural-boundary-enforcement.md) (explicit
  schema layer contract and dependency direction).
- `ADR3-2`:
  [ADR 003 decision 2](adr-003-architectural-boundary-enforcement.md) (module
  visibility as first-layer enforcement).
- `ADR3-3`:
  [ADR 003 decision 3](adr-003-architectural-boundary-enforcement.md) (baseline
  CI architecture checks).
- `ADR3-4`:
  [ADR 003 decision 4](adr-003-architectural-boundary-enforcement.md) (custom
  Dylint checks for forbidden edges).
- `ADR3-5`:
  [ADR 003 decision 5](adr-003-architectural-boundary-enforcement.md) (optional
  `cargo-deny` dependency policy checks).

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

- [x] Implement canonical action-name validation for dot-separated segments with
  per-segment identifier and keyword rules. Acceptance: tests reject malformed
  names and reserved-keyword segments. Signposts: `NMR-1`, `TFS-4`, `ADR-1`.
- [x] Implement action mangling (`segment_escape`, `action_slug`,
  `hash12(blake3)`) and canonical path resolution into
  `crate::theorem_actions`. Acceptance: golden tests cover representative names
  and underscore edge cases. Signposts: `NMR-1`, `ADR-1`, `DES-5`.
- [x] Fail compilation on duplicate canonical action names and duplicate mangled
  identifiers, reporting all colliding sources. Acceptance: integration tests
  prove both collision classes are detected before backend execution. Signposts:
  `NMR-1`, `ADR-1`, `DES-5`.

### Step 2.2: implement harness and module naming stability

Dependencies: step 2.1.

In scope: per-file module mangling, harness naming, and theorem-key collision
checks.

Out of scope: report-level ID alias resolution.

- [x] Implement per-file module naming using `path_mangle(path_stem(P))` and
  `hash12(P)`. Acceptance: snapshot tests confirm deterministic names for mixed
  separators and punctuation-heavy paths. Signposts: `NMR-1`, `ADR-2`, `DES-7`.
- [x] Implement harness naming
  `theorem__{theorem_slug(T)}__h{hash12(P#T)}` with deterministic CamelCase to
  snake_case conversion. Acceptance: tests cover acronym runs, numeric
  boundaries, and already-snake identifiers. Signposts: `NMR-1`, `ADR-2`.
- [x] Enforce duplicate theorem-key rejection (`P#T`) at build time. Acceptance:
  integration tests prove collisions fail with actionable theorem source
  diagnostics. Signposts: `NMR-1`, `ADR-2`, `TFS-1`.

### Step 2.3: implement explicit argument value semantics

Dependencies: phase 1 and step 2.1.

In scope: value lowering for literals, references, lists, and map-driven struct
literal synthesis.

Out of scope: implicit reference inference.

- [x] Implement argument decoding so plain YAML strings are always literals and
  variable references require `{ ref: name }`. Acceptance: tests prove adding a
  new binding cannot alter existing literal argument semantics. Signposts:
  `TFS-5`, `ADR-3`, `DES-5`.
- [x] Implement optional `{ literal: "text" }` wrapper and reject sentinel
  wrappers with invalid value types (e.g. `{ literal: 42 }`). Single-key maps
  whose key is not a recognized sentinel pass through as struct-literal
  candidates per TFS-5 §5.3. Acceptance: parser tests cover valid wrapper use
  and deterministic rejection cases. Signposts: `TFS-5`, `ADR-3`.
- [x] Implement struct-literal synthesis from YAML maps based on action
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

- [x] Implement `build.rs` scanning of `theorems/**/*.theorem` and emission of
  `cargo::rerun-if-changed` lines for directory and file paths. Acceptance:
  build integration tests confirm theorem edits trigger rebuilds reliably.
  Signposts: `DES-7`.
- [x] Generate `OUT_DIR/theorem_suite.rs` with one `theorem_file!(...)`
  invocation per discovered path, then include it from crate code. Acceptance:
  empty, single-file, and multi-file suites compile deterministically.
  Signposts: `DES-7`.

### Step 3.2: implement `theorem_file!` proc-macro expansion

Dependencies: step 3.1.

In scope: module scaffolding, backend submodule layout, `include_str!` wiring,
and generated harness stubs.

Out of scope: final backend semantics.

- [x] Implement macro expansion that emits the stable per-file module, includes
  theorem source content via `include_str!`, and builds one harness entry per
  theorem document. Acceptance: macro-expansion snapshots remain stable across
  repeated builds. Signposts: `DES-7`, `NMR-1`, `ADR-2`.
- [x] Gate generated Kani harnesses with `#[cfg(kani)]` and emit required
  `#[kani::proof]` and `#[kani::unwind(n)]` attributes from evidence.
  Acceptance: non-Kani `cargo build` succeeds and Kani-targeted builds discover
  harnesses. Signposts: `DES-7`, `DES-8`, `TFS-6`.

### Step 3.3: implement compile-time binding probes

Dependencies: step 3.2.

In scope: generated probe bindings for actions and types to detect drift.

Out of scope: runtime reflection.

- [x] Emit typed action probes (`let _: fn(...) -> ... = ...;`) for every
  referenced action to force signature compatibility at compile time.
  Acceptance: signature drift causes compile failure in the theorem owner
  crate. Signposts: `DES-7`, `DES-5`, `NMR-1`.
- [x] Emit referenced-type probes for generated struct literal synthesis and
  step bindings to surface missing-type and moved-type breakages early.
  Acceptance: compile-fail tests validate predictable drift diagnostics.
  Signposts: `DES-7`, `DES-5`.

## Phase 3A: `cargo theorem` foundation and project integration

Outcome: theoremc has one Cargo-native, agent-readable entry point for project
inspection, scaffolding, build integration, and proof-backend management.

### Step 3A.1: establish the Cargo subcommand and OrthoConfig spine

Dependencies: phase 1 and step 3.1.

In scope: package structure, command metadata, configuration merging, minimum
Rust versions, direct and Cargo-mediated invocation, and base output contracts.

Out of scope: theorem execution and project mutation.

- [ ] Add a dedicated `crates/cargo-theorem` package and `cargo-theorem`
  executable while keeping application-only dependencies outside the root
  theoremc facade, and remove the placeholder root `src/main.rs`. Acceptance:
  both `cargo theorem --help` and `cargo-theorem --help` execute the same
  command tree in end-to-end tests, and no unrelated `theoremc` binary remains.
  Signposts: `CLI-DES`.
- [ ] Adopt `ortho_config` for CLI, environment, file, selected-profile, and
  selected-subcommand merging, using `SelectedSubcommandMerge` and
  `OrthoConfigSubcommandDocs`. Acceptance: precedence tests prove
  defaults < files < profile < environment < flags, and generated recursive
  documentation matches the Clap command tree. Signposts: `CLI-DES`.
- [ ] Establish the mixed-MSRV policy required by `ortho_config` 0.9.0: retain
  Rust 1.88 for library packages and declare Rust 1.89 for `cargo-theorem`.
  Acceptance: CI tests the library workspace excluding the CLI on 1.88 and the
  CLI package on 1.89. Signposts: `CLI-DES`.
- [ ] Implement the versioned JSON application envelope, stdout/stderr stream
  invariants, stable exit classes, and global renderer options. Acceptance:
  snapshots cover success and failure for human and JSON modes, and subprocess
  output cannot leak beside a JSON document. Signposts: `CLI-DES`, `ADR2-4`.
- [ ] Implement `cargo theorem context --json` from OrthoConfig's compact agent
  context and add agent-native policy checks to CI. Acceptance: the versioned
  context includes every command path, mutation boundary, output mode,
  pagination rule, and exit class without reading project state or the network.
  Signposts: `CLI-DES`.

### Step 3A.2: implement Cargo project discovery and read-only commands

Dependencies: step 3A.1 and phases 1 to 3.

In scope: Cargo metadata, package selection, theorem/action inspection, bounded
results, and composed project checks.

Out of scope: modifying project files or installing proof tools.

- [ ] Implement workspace and package resolution through the stable
  `cargo metadata` JSON protocol with `--manifest-path`, `--package`,
  `--workspace`, and `--exclude`. Acceptance: fixtures cover virtual
  workspaces, ambiguous names, excluded members, and explicit manifests;
  ambiguity diagnostics enumerate valid package identifiers. Signposts:
  `CLI-DES`.
- [ ] Implement bounded top-level `list` and `get` theorem commands plus
  `action list` and `action get`, using stable ordering, `--limit`, and opaque
  cursors. Acceptance: JSON snapshots contain stable theorem IDs, paths,
  backends, action names, signatures, and next-cursor metadata without
  unbounded source or log content. Signposts: `CLI-DES`, `NMR-1`, `NMR-2`.
- [ ] Implement read-only `check` composition for schema, aliases, action
  signatures, build integration, and configured backend pins. Acceptance:
  `check` never performs installation or network mutation, and every failing
  component produces a stable diagnostic code and application exit class.
  Signposts: `CLI-DES`, `DES-6`, `DES-7`.

### Step 3A.3: implement mutation plans, scaffolding, and build integration

Dependencies: step 3A.2 and phase 3.

In scope: reusable build services, atomic project plans, theorem/action
scaffolding, and managed `build.rs` integration.

Out of scope: arbitrary Rust source repair and proof generation from prose.

- [ ] Extract build discovery and suite rendering behind a reusable
  `theoremc-build` library API, including fallible direct execution and the
  stable build-script boundary
  `theoremc::build::emit_suite_from_env_or_exit()`. Acceptance: the root build
  script and direct CLI path use the same discovery and rendering fixtures.
  Signposts: `CLI-DES`, `DES-7`.
- [ ] Implement an immutable project mutation plan with create, edit,
  unchanged, conflict, and manual-action entries. Apply plans under a
  workspace lock with temporary siblings, flush, atomic rename, and a recovery
  journal. Acceptance: property tests prove repeat application is idempotent,
  and failure-injection tests leave either the old or complete new state.
  Signposts: `CLI-DES`.
- [ ] Implement `init`, top-level `create`, and `action create` with `--dry-run`
  and narrowly scoped `--force` handling. Acceptance: scaffolds are schema
  valid, do not invent assertions, do not generate panic or `todo!()` stubs by
  default, and refuse unsafe overwrites. Signposts: `CLI-DES`, `TFS-1`,
  `DES-5`.
- [ ] Implement `build-script install`, `check`, `run`, and `delete` using
  parsed Rust source spans and managed calls rather than blind string
  replacement. Acceptance: fixtures cover absent, generated, already managed,
  and complex existing build scripts; unsafe edits become structured conflicts.
  Signposts: `CLI-DES`, `DES-7`.
- [ ] Implement `generate --kind suite|harness|metadata` with deterministic
  selection and explicit delivery for large artefacts. Acceptance: direct
  generation matches build-script output byte for byte for the same inputs.
  Signposts: `CLI-DES`, `DES-7`.

### Step 3A.4: implement backend providers and prover-tool parity

Dependencies: step 3A.1. This step may proceed in parallel with phase 4, but
high-level theorem execution depends on both.

In scope: provider capabilities, lock data, managed installations, Kani and
Verus parity, low-level execution, and migration adapters.

Out of scope: backend-neutral theorem lowering beyond Kani.

- [ ] Define a backend provider contract for discovery, release resolution,
  install planning, health checks, argument-vector execution, result parsing,
  and replay planning. Inject one command runner for timeouts, cancellation,
  stream capture, and test doubles; shell command strings are prohibited.
  Acceptance: a fake provider exercises every capability without process-global
  state. Signposts: `CLI-DES`.
- [ ] Implement `theoremc.toml` backend intent, a deterministic
  `theoremc.lock`, content-addressed platform cache entries, and `backend list`,
  `get`, `install`, `check`, `sync`, `update`, and `delete`. Acceptance:
  installation mutations support `--dry-run`, pin checks never install, and the
  lock omits machine-specific paths. Signposts: `CLI-DES`.
- [ ] Port Kani installation, setup, semantic-version checking, command
  override, harness execution, and concrete playback from
  `rust-prover-tools`. Acceptance: parity scenarios cover matching and
  mismatched pins, missing installations, setup failure, and version parsing.
  Signposts: `CLI-DES`, `DES-8`.
- [ ] Port Verus release-target selection, bounded download, checksum
  verification, safe extraction, binary discovery, Rust toolchain preparation,
  and raw proof-file execution from `rust-prover-tools`. Acceptance: default
  tests use fake executables and local archives; opt-in jobs cover real
  releases. Signposts: `CLI-DES`.
- [ ] Implement `backend run <backend>` and one-release legacy environment
  migration for the existing Kani and Verus variables. Acceptance: migration
  diagnostics identify the replacement setting, and `--passthrough-exit-code`
  is isolated as an advanced compatibility option. Signposts: `CLI-DES`.

### Step 3A.5: validate the public CLI contract

Dependencies: steps 3A.1 to 3A.4.

In scope: public help, context, JSON schemas, bounded behaviour, mutation
safety, migration parity, and agent-native policy.

Out of scope: full theorem-suite execution, reports, and detached run jobs.

- [ ] Add snapshot and schema tests for human help, `context --json`, JSON
  success/error envelopes, exit classes, choice-enumerating diagnostics, and
  pagination. Acceptance: every public command declares interaction, mutation,
  output, and bounded-response metadata. Signposts: `CLI-DES`.
- [ ] Add behavioural and end-to-end tests for direct invocation, Cargo external
  subcommand invocation, configuration precedence, dry runs, project locks,
  atomic writes, and fake backend processes. Acceptance: default tests require
  neither a network nor real prover installations. Signposts: `CLI-DES`.
- [ ] Add command-for-command migration fixtures for all four public
  `rust-prover-tools` workflows. Acceptance: each legacy invocation has a
  documented `cargo theorem` equivalent and equivalent externally observable
  backend behaviour. Signposts: `CLI-DES`.

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
  presence is reflected in run output and reported reachability data. Signposts:
  `DES-8`, `TFS-1`, `ADR-4`.

### Step 4.2: implement `call`, `must`, and `maybe` operational semantics

Dependencies: step 4.1 and phase 2.

In scope: call binding behaviour, failure obligations for `must`, and symbolic
branching for `maybe`.

Out of scope: probabilistic branching semantics.

- [ ] Implement `call` semantics with `as` binding rules, including rejection
  of unbound fallible results in `Do`. Acceptance: semantic tests cover `()`,
  value-returning, `Result`, and `Option` action signatures. Signposts: `TFS-4`,
  `DES-4`, `DES-8`.
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

## Phase 5: execution, reporting, and stable theorem identity

Outcome: `cargo theorem` runs produce durable, actionable artefacts with stable
IDs across renames and moves.

### Step 5.0: implement theorem execution orchestration and the run ledger

Dependencies: phase 4 and steps 3A.2 to 3A.4.

In scope: theorem selection, immutable execution plans, backend orchestration,
policy application, durable jobs, idempotency, and stable application outcomes.

Out of scope: hosted execution and report dashboarding.

- [ ] Implement shared selection by theorem ID, path, tag, package, backend, and
  changed revision, with stable ordering and explicit empty-selection policy.
  Acceptance: property tests cover category intersection, repeated-filter
  union, aliases, exclusions, and deterministic order. Signposts: `CLI-DES`,
  `NMR-2`.
- [ ] Implement top-level `run` as resolve, validate, pin-check, plan, execute,
  parse, policy, persist, and render stages. Acceptance: `run` never installs a
  backend, records backend-native outcomes before applying evidence policy, and
  maps every final state to a stable application exit class. Signposts:
  `CLI-DES`, `DES-8`, `DES-9`.
- [ ] Persist every foreground and detached run under
  `.theoremc/runs/<run-id>/` with an append-only bounded index, captured logs,
  input digest, parent link, and canonical plan. Acceptance: crash-recovery
  tests leave inspectable terminal or resumable records rather than orphaned
  processes. Signposts: `CLI-DES`.
- [ ] Implement blocking-by-default execution, `--no-wait`, `--timeout`,
  bounded parallelism, cancellation, and optional idempotency keys. Acceptance:
  duplicate active submissions return the existing job when inputs match and
  fail with both digests when they differ. Signposts: `CLI-DES`.
- [ ] Implement `jobs list`, `get`, `cancel`, and `prune` with pagination,
  bounded log excerpts, mutation previews, and explicit retention rules.
  Acceptance: list/get remain bounded and prune cannot delete records outside
  its structured plan. Signposts: `CLI-DES`.

### Step 5.1: implement theorem run result model and report outputs

Dependencies: step 5.0.

In scope: the `cargo theorem` canonical run model and output formats (human
report plus CI artefacts).

Out of scope: dashboard hosting.

- [ ] Implement a canonical theorem run record that includes theorem ID,
  metadata, assumptions, step outcomes, assertion outcomes, witness outcomes,
  evidence config, backend provenance, diagnostics, artefacts, and final status.
  Acceptance: serialized fixtures round-trip without field loss and distinguish
  invariant fields from wall-clock or run-identity fields. Signposts: `DES-9`,
  `TFS-1`, `TFS-6`, `CLI-DES`.
- [ ] Implement Markdown/HTML report rendering from the canonical run record.
  Acceptance: golden snapshots cover pass, fail, unreachable, undetermined,
  timeout, cancellation, and expected-failure examples. Signposts: `DES-9`,
  `CLI-DES`.
- [ ] Implement JUnit XML and Cucumber JSON emitters for CI integration.
  Acceptance: schema validation tests pass for both formats and no renderer
  parses terminal output. Signposts: `DES-9`, `CLI-DES`.
- [ ] Implement `report create <run-id>` with repeatable formats and explicit
  delivery for large or multiple artefacts. Acceptance: JSON application mode
  returns artefact references without mixing report bytes into stdout.
  Signposts: `CLI-DES`.

### Step 5.2: implement stable external theorem IDs and alias migration

Dependencies: phase 4 and step 3A.2.

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

Dependencies: steps 5.0 and 5.1, plus step 3A.4.

In scope: Kani failure replay orchestration and report attachment of playback
artefacts.

Out of scope: automated source rewriting workflows.

- [ ] Integrate Kani concrete playback execution for failed harnesses and
  capture generated replay artefacts as a child run. Acceptance: a failing
  theorem integration test produces linked source, command, log, and provenance
  artefacts without rewriting application source. Signposts: `DES-8`, `DES-9`,
  `CLI-DES`.
- [ ] Surface playback metadata and retrieval paths in human and CI reports.
  Acceptance: report snapshots include replay references only when available.
  Signposts: `DES-9`, `CLI-DES`.

### Step 5.4: implement profiles, delivery, feedback, and migration completion

Dependencies: steps 3A.5 and 5.1. Reusable OrthoConfig contracts are preferred;
tracked temporary adapters may cover soft dependencies until they ship.

In scope: persistent configuration overlays, artefact routing, local feedback,
legacy command migration, and retirement gates.

Out of scope: mandatory network services.

- [ ] Implement named profiles with secret redaction and precedence between
  project files and environment variables, plus bounded `profile list`, `get`,
  `save`, and guarded `delete`. Acceptance: context exposes profile names and
  non-secret fields only, and profile mutations support `--dry-run`.
  Signposts: `CLI-DES`.
- [ ] Implement atomic `stdout` and `file:<path>` delivery, leaving
  `webhook:<url>` behind an explicit later capability. Acceptance: unknown
  schemes enumerate valid choices and JSON mode returns delivery metadata
  rather than mixed payloads. Signposts: `CLI-DES`.
- [ ] Implement privacy-bounded local JSON Lines feedback with optional
  diagnostic attachment and no implicit source, environment, proof, or log
  capture. Acceptance: tests prove sensitive fields remain absent unless
  explicitly attached. Signposts: `CLI-DES`.
- [ ] Publish migration guidance and cross-platform parity results for all
  `rust-prover-tools` workflows, then mark the Python CLI maintenance-only.
  Acceptance: archival cannot occur until Kani and Verus parity fixtures pass
  on every supported host. Signposts: `CLI-DES`.

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

### Step 6.2: enforce schema architectural boundaries

Dependencies: phase 1 and step 6.1.

In scope: schema-layer visibility boundaries, dependency-graph enforcement, and
lint-based forbidden-edge checks.

Out of scope: splitting schema layers into separate crates.

- [ ] Introduce and stabilize explicit schema-layer module boundaries (domain,
  raw adapter, validator, loader, diagnostics) using non-public internals and
  curated re-exports. Acceptance: public `schema` exports include only approved
  API types and loader entry points, and contract tests prove raw adapter
  internals are not importable by consumers. Signposts: `DES-3.2`, `DES-10.1`,
  `ADR3-1`, `ADR3-2`.
- [ ] Add CI acyclicity checks with `cargo modules graph --acyclic --lib`.
  Acceptance: current module graph passes, and a synthetic cycle fixture causes
  the architecture check to fail. Signposts: `DES-10`, `ADR3-3`.
- [ ] Enforce explicit imports by denying wildcard imports in CI. Acceptance:
  schema modules fail lint checks when wildcard imports are introduced, and CI
  runs with `-D clippy::wildcard_imports` enabled. Signposts: `DES-10`,
  `ADR3-3`.
- [ ] Implement `theoremc_arch_lint` Dylint rules for forbidden import edges
  (for example, `schema::types` importing `schema::raw`). Acceptance: lint
  fixtures demonstrate expected failures for forbidden edges and no failures
  for allowed edges. Signposts: `DES-10`, `ADR3-4`.
- [ ] Add optional `cargo-deny` policy checks for architecture-sensitive
  dependencies (for example, constraining YAML adapter dependencies to adapter
  contexts). Acceptance: dependency policy checks pass in CI and fail on
  deliberate policy violations in fixtures. Signposts: `DES-10`, `ADR3-5`.

### Step 6.3: provide examples and authoring guidance

Dependencies: phases 1 to 5.

In scope: runnable examples, theorem authoring guidance, and maintenance
checklists.

Out of scope: production deployment templates.

- [ ] Create end-to-end example crates (`account`, `hnsw`) that demonstrate
  action exports, theorem files, generated harness behaviour, `cargo theorem`
  project checks, backend synchronization, execution, and reports. Acceptance:
  examples compile and execute through the documented theorem workflow.
  Signposts: `DES-3`, `DES-5`, `DES-8`, `CLI-DES`.
- [ ] Write and publish user-facing guidance for theorem authoring rules,
  especially explicit references, witness policy, expected status usage,
  project initialization, backend pins, and run artefacts. Acceptance: docs
  contain copy-paste-ready examples that match implementation semantics.
  Signposts: `TFS-5`, `ADR-3`, `ADR-4`, `DES-4`, `CLI-DES`.
- [ ] Add a contributor checklist that requires parser fixtures, codegen
  snapshots, command-context snapshots, run-record fixtures, and report
  snapshots for behavioural changes. Acceptance: pull request template and
  contributor docs reference the checklist explicitly. Signposts: `DES-6`,
  `DES-7`, `DES-9`, `CLI-DES`.

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
- [ ] Extend `cargo theorem` outputs to always include stable diagnostic code
  and arguments, include required English fallback text, and attach localized
  text only when a localizer is configured. Acceptance: application JSON and
  report snapshots for Markdown, HTML, JUnit XML, and Cucumber JSON confirm
  invariant machine fields and optional localized projection fields.
  Signposts: `DES-9`, `DES-6.5`, `ADR2-2`, `ADR2-4`, `CLI-DES`.
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

- Execute phases in order, except where the dependencies below explicitly
  permit parallel work.
- Complete steps 3A.1 and 3A.2 after the existing compile-time foundation;
  step 3A.4 may proceed in parallel with phase 4 because tool management does
  not depend on completed theorem lowering.
- Do not implement high-level `cargo theorem run` until both Kani execution
  semantics and the backend provider/pin-check contracts are stable.
- Do not start report rendering or alias migration before the Kani execution
  model is stable and the canonical run ledger exists.
- Treat vacuity policy implementation as a release gate, not an optional
  enhancement.
- Keep backend installation explicit: `check`, `build-script run`, and top-level
  `run` must never install or update tools.
- Do not retire `rust-prover-tools` until Kani and Verus parity fixtures pass on
  every supported host and migration guidance has shipped.
- Land schema architecture boundary checks before broadening examples and
  contributor workflow guidance.
- Start localization integration only after core diagnostics, application JSON,
  and canonical run records are structured and stable.
