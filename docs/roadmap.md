# Theoremc development roadmap

This roadmap translates the settled theoremc design, the `cargo theorem` CLI
design, the accepted RFCs, and the architectural decision records (ADRs) into
an outcome-oriented delivery sequence. It does not promise dates. Each phase
carries one testable idea in the Goals, Ideas, Steps, Tasks (GIST) sense: the
steps underneath a phase are workstreams that validate or falsify that idea,
and each task is a review-sized execution unit with observable acceptance
criteria.

The primary design document is [theoremc-design.md](theoremc-design.md). The
command-line surface is defined by
[cargo-theorem-cli-design.md](cargo-theorem-cli-design.md). Decision records
live in [docs/adrs/](adrs/) and RFCs in [docs/rfcs/](rfcs/).

The document follows the roadmap grammar edited by `mapsplice`: phases are
`## N.` headings, steps are `### N.M.` headings, and tasks are `- [ ] N.M.K.`
checklist items. Every task that depends on work outside its own position names
that work in a `Requires` bullet using dotted task (`N.M.K`) or step (`N.M`)
anchors, so `mapsplice` can rewrite the references when the roadmap is
restructured. ExecPlans for roadmap tasks are named
`docs/execplans/N-M-K-<slug>.md` after the task they implement.

## Sources

Tasks cite these sources with `§` section references. ADRs are cited by number.

- [theoremc-design.md](theoremc-design.md): architecture, theorem format,
  action model, parsing, build integration, the Kani backend, reporting, and
  enforcement.
- [cargo-theorem-cli-design.md](cargo-theorem-cli-design.md): Cargo-native
  command surface, project integration, backend management, execution,
  reporting, and agent-native contracts.
- [theorem-file-specification.md](theorem-file-specification.md): document
  model, step and action schemas, value forms, and evidence schema.
- [name-mangling-rules.md](name-mangling-rules.md): action, harness, and stable
  external theorem identifier rules.
- [RFC 0001](rfcs/rfc-0001-semantic-law-templates-over-composable-primaries.md):
  versioned proof obligations and semantic law templates.
- [ADR-0001](adrs/adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md):
  injective action and harness naming (decisions 1 and 2), explicit
  `{ ref: ... }` semantics (decision 3), the non-vacuity witness policy
  (decision 4), and stable external IDs with migration aliases (decision 5).
- [ADR-0002](adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md):
  injected localizers (decision 1), stable diagnostic codes with English
  fallback (decision 2), Fluent layering (decision 3), deterministic English
  for machine-facing artefacts (decision 4), and deferred keyword
  internationalization (decision 5).
- [ADR-0003](adrs/adr-0003-architectural-boundary-enforcement-for-schema-layers.md):
  schema layer contract (decision 1), module visibility (decision 2), CI
  architecture checks (decision 3), Dylint forbidden-edge checks (decision 4),
  and optional `cargo-deny` policy (decision 5).
- [ADR-0004](adrs/adr-0004-theorem-side-action-signatures.md): theorem-side
  action signatures.
- [ADR-0006](adrs/adr-0006-backend-neutral-harness-planning-boundary.md):
  shared backend-neutral harness plan and resolved invocations.
- [ADR-0007](adrs/adr-0007-verus-backend-preservation-invariants.md): law
  meaning, backend bindings, identity, and evidence preservation.
- [ADR-0008](adrs/adr-0008-workspace-build-support-boundary.md): workspace
  build support boundary.
- [ADR-0009](adrs/adr-0009-orthoconfig-cli-configuration.md): OrthoConfig CLI
  configuration.
- [ADR-0010](adrs/adr-0010-reusable-library-msrv-for-serde-saphyr-2-0.md):
  reusable library minimum supported Rust version.
- [ADR-0011](adrs/adr-0011-vertical-slice-first-roadmap-sequencing.md): the
  vertical slice before laws and rich reporting.

## Sequencing rules

- Execute phases in order, except where `Requires` bullets explicitly permit
  parallel work. Step 4.4 may proceed in parallel with phase 5 because tool
  management does not depend on completed theorem lowering.
- Do not implement high-level `cargo theorem run` until both Kani execution
  semantics and the backend provider and pin-check contracts are stable.
- Make the real version 1 Kani proof and counterexample in step 5.1 the next
  release gate. Complete operational breadth and minimal evidence policy in
  steps 5.2 and 5.3, then integrate RFC 0001 in step 5.4 before rich reporting.
  Laws reuse the shared harness plan, renderer, and runner; they do not delay
  the slice.
- Start rich report rendering only after step 5.4, stable Kani execution, and
  the canonical run ledger. Complete alias migration before alias-aware
  selection or execution.
- Treat vacuity policy implementation as a release gate, not an optional
  enhancement.
- Keep backend installation explicit: `check`, `build-script run`, and
  top-level `run` must never install or update tools.
- Do not retire `rust-prover-tools` until Kani and Verus parity fixtures pass
  on every supported host and migration guidance has shipped.
- Land schema architecture boundary checks before broadening examples and
  contributor workflow guidance.
- Start localization integration only after core diagnostics, application
  JSON, and canonical run records are structured and stable.

## 1. Strict theorem documents with source-located diagnostics

Idea: if theorem documents are deserialized strictly, validated semantically,
and diagnosed with source locations before any code generation exists, every
later slice can trust its input and authors get actionable feedback from the
first release.

This foundational phase settles the document model that every later phase
consumes. It is complete.

### 1.1. Prove that strict deserialization rejects malformed theorem documents

This step answers whether `serde`-driven schema types can enforce the document
model exactly, including aliases and unknown-key rejection, without a bespoke
parser. See theorem-file-specification.md §§1-3 and theoremc-design.md §6.

- [x] 1.1.1. Implement `TheoremDoc` and subordinate schema types with
  `serde(deny_unknown_fields)` and TitleCase plus lower-case aliases.
  - See theorem-file-specification.md §§1-3 and theoremc-design.md §6.1.
  - Success: unit tests prove unknown keys and wrong scalar types fail
    deserialization with actionable errors.
- [x] 1.1.2. Implement `.theorem` multi-document loading (`---` separation)
  into an ordered in-memory collection.
  - Requires 1.1.1.
  - See theorem-file-specification.md §1 and theoremc-design.md §4.1.
  - Success: parser tests cover one-document and many-document files with
    stable document ordering.
- [x] 1.1.3. Enforce theorem identifier lexical rules
  (`^[A-Za-z_][A-Za-z0-9_]*$`) and Rust keyword rejection.
  - Requires 1.1.1.
  - See theorem-file-specification.md §2 and theoremc-design.md §6.2.
  - Success: validation tests reject reserved keywords and invalid ASCII
    identifiers with line and column diagnostics.

### 1.2. Enforce the semantic rules that serde cannot express

This step answers which document rules need a post-deserialization validator,
and whether that validator can stay independent of Rust typechecking. See
theorem-file-specification.md §§3-6 and theoremc-design.md §§4, 6.2.

- [x] 1.2.1. Validate required fields and non-empty constraints for `Theorem`,
  `About`, `Prove`, and Kani evidence requirements.
  - Requires 1.1.1.
  - See theorem-file-specification.md §§3, 6 and theoremc-design.md §6.4.
  - Success: negative tests cover each missing or empty field and confirm
    deterministic error messages.
- [x] 1.2.2. Parse `Assume.expr`, `Prove.assert`, and `Witness.cover` as
  `syn::Expr` and reject statement blocks.
  - Requires 1.1.1.
  - See theorem-file-specification.md §3 and theoremc-design.md §6.5.
  - Success: tests demonstrate single-expression acceptance and block-style
    rejection.
- [x] 1.2.3. Enforce `Step` and `LetBinding` shape rules: `Let` allows only
  `call` or `must`, and `maybe` requires `because` and nested `do`.
  - Requires 1.1.1.
  - See theorem-file-specification.md §4 and theoremc-design.md §4.4.
  - Success: validation tests cover each invalid variant combination.
- [x] 1.2.4. Enforce non-vacuity defaults: `Witness` is required unless
  `allow_vacuous: true` carries a non-empty `vacuity_because`.
  - Requires 1.2.1.
  - See theorem-file-specification.md §6, theoremc-design.md §8.4, and
    ADR-0001 decision 4.
  - Success: tests cover valid and invalid vacuity declarations and the
    default-failure behaviour.

### 1.3. Make parser and validator failures source-located and regression-gated

This step answers whether diagnostics can carry stable file, line, and column
positions across both deserialization and semantic failures. See
theoremc-design.md §6.3.

- [x] 1.3.1. Wrap parser and validator failures in structured diagnostics that
  include source file, line, and column.
  - Requires 1.1, 1.2.
  - See theoremc-design.md §6.3 and theorem-file-specification.md §1.
  - Success: snapshot tests assert stable, source-located diagnostic output for
    representative failures.
- [x] 1.3.2. Build a fixture suite of valid and invalid `.theorem` files
  covering aliases, nested `maybe`, `must` semantics preconditions, and the
  witness policy.
  - Requires 1.3.1.
  - See theorem-file-specification.md §§1-6 and ADR-0001 decision 4.
  - Success: fixtures run in continuous integration (CI) and gate parser and
    validator regressions.

## 2. Injective action resolution and deterministic naming

Idea: if action references, generated modules, and harness names are injective
and stable functions of their sources, theorem files can bind to Rust code at
compile time without runtime registries or rename drift.

This phase is complete.

### 2.1. Prove that action names mangle injectively into Rust paths

This step answers whether dot-separated action names can be validated, mangled,
and resolved into `crate::theorem_actions` without collisions. See
name-mangling-rules.md "Action name mangling", theoremc-design.md §5.2, and
ADR-0001 decision 1.

- [x] 2.1.1. Implement canonical action-name validation for dot-separated
  segments with per-segment identifier and keyword rules.
  - Requires 1.1, 1.2, 1.3.
  - See name-mangling-rules.md "Action name mangling" and
    theorem-file-specification.md §4.
  - Success: tests reject malformed names and reserved-keyword segments.
- [x] 2.1.2. Implement action mangling (`segment_escape`, `action_slug`,
  `hash12(blake3)`) and canonical path resolution into `crate::theorem_actions`.
  - Requires 2.1.1.
  - See name-mangling-rules.md "Action name mangling", theoremc-design.md §5.2,
    and ADR-0001 decision 1.
  - Success: golden tests cover representative names and underscore edge
    cases.
- [x] 2.1.3. Fail compilation on duplicate canonical action names and
  duplicate mangled identifiers, reporting all colliding sources.
  - Requires 2.1.2.
  - See name-mangling-rules.md "Action name mangling" and theoremc-design.md
    §5.2.
  - Success: integration tests prove both collision classes are detected
    before backend execution.

### 2.2. Make generated module and harness names stable across builds

This step answers whether per-file modules and harnesses can be named
deterministically from theorem paths and names, so downstream tooling can rely
on them. See name-mangling-rules.md "Harness name mangling (Kani MVP)" and
ADR-0001 decision 2.

- [x] 2.2.1. Implement per-file module naming using `path_mangle(path_stem(P))`
  and `hash12(P)`.
  - Requires 2.1.2.
  - See name-mangling-rules.md "Harness name mangling (Kani MVP)" and
    theoremc-design.md §7.
  - Success: snapshot tests confirm deterministic names for mixed separators
    and punctuation-heavy paths.
- [x] 2.2.2. Implement harness naming
  `theorem__{theorem_slug(T)}__h{hash12(P#T)}` with deterministic CamelCase to
  snake_case conversion.
  - Requires 2.2.1.
  - See name-mangling-rules.md "Harness name mangling (Kani MVP)" and
    ADR-0001 decision 2.
  - Success: tests cover acronym runs, numeric boundaries, and already-snake
    identifiers.
- [x] 2.2.3. Enforce duplicate theorem-key rejection (`P#T`) at build time.
  - Requires 2.2.2.
  - See name-mangling-rules.md "Harness name mangling (Kani MVP)" and
    theorem-file-specification.md §1.
  - Success: integration tests prove collisions fail with actionable theorem
    source diagnostics.

### 2.3. Make argument values explicit so new bindings cannot change meaning

This step answers whether YAML argument values can lower to Rust expressions
without implicit reference inference. See theorem-file-specification.md §5,
theoremc-design.md §5.5, and ADR-0001 decision 3.

- [x] 2.3.1. Implement argument decoding so plain YAML strings are always
  literals and variable references require `{ ref: name }`.
  - Requires 1.2, 2.1.
  - See theorem-file-specification.md §5 and theoremc-design.md §5.5.
  - Success: tests prove adding a new binding cannot alter existing literal
    argument semantics.
- [x] 2.3.2. Implement the optional `{ literal: "text" }` wrapper and reject
  sentinel wrappers with invalid value types such as `{ literal: 42 }`.
  - Requires 2.3.1.
  - Single-key maps whose key is not a recognized sentinel pass through as
    struct-literal candidates per theorem-file-specification.md §5.3.
  - See theorem-file-specification.md §5 and ADR-0001 decision 3.
  - Success: parser tests cover valid wrapper use and deterministic rejection
    cases.
- [x] 2.3.3. Implement struct-literal synthesis from YAML maps based on action
  parameter types, plus recursive list lowering to `vec![...]`.
  - Requires 2.3.2.
  - See theorem-file-specification.md §5 and theoremc-design.md §5.5.
  - Success: compile-fail tests show type mismatches are surfaced by Rust
    compilation.

## 3. Always-connected compile-time integration

Idea: if theorem files are discovered by the build script, expanded by a
proc-macro, and probed against action signatures during ordinary compilation,
theorems cannot silently drift from the code they describe.

This phase is complete.

### 3.1. Prove that the build script discovers theorems and invalidates reliably

This step answers whether Cargo's build-script contract can keep theorem files
connected to compilation without a separate tool. See theoremc-design.md §7.1
and ADR-0008.

- [x] 3.1.1. Implement `build.rs` scanning of `theorems/**/*.theorem` and
  emission of `cargo::rerun-if-changed` lines for directory and file paths.
  - Requires 1.1.2, 2.2.1.
  - See theoremc-design.md §7.1.
  - Success: build integration tests confirm theorem edits trigger rebuilds
    reliably.
- [x] 3.1.2. Generate `OUT_DIR/theorem_suite.rs` with one `theorem_file!(...)`
  invocation per discovered path, then include it from crate code.
  - Requires 3.1.1.
  - See theoremc-design.md §7.1.
  - Success: empty, single-file, and multi-file suites compile
    deterministically.

### 3.2. Expand each theorem file into a stable per-file module

This step answers whether `theorem_file!` can emit stable module scaffolding
and harness stubs that later backend work fills in. See theoremc-design.md §7.2.

- [x] 3.2.1. Implement macro expansion that emits the stable per-file module,
  includes theorem source content via `include_str!`, and builds one harness
  entry per theorem document.
  - Requires 3.1.2, 2.2.2.
  - See theoremc-design.md §7.2 and ADR-0001 decision 2.
  - Success: macro-expansion snapshots remain stable across repeated builds.
- [x] 3.2.2. Gate generated Kani harnesses with `#[cfg(kani)]` and emit the
  required `#[kani::proof]` and `#[kani::unwind(n)]` attributes from evidence.
  - Requires 3.2.1, 1.2.1.
  - See theoremc-design.md §§7.2, 8 and theorem-file-specification.md §6.
  - Success: non-Kani `cargo build` succeeds and Kani-targeted builds discover
    harnesses.

### 3.3. Detect action and type drift at compile time

This step answers whether generated probe bindings can turn signature and type
drift into ordinary compile errors in the theorem owner crate. See
theoremc-design.md §7.3 and ADR-0004.

- [x] 3.3.1. Emit typed action probes (`let _: fn(...) -> ... = ...;`) for
  every referenced action to force signature compatibility at compile time.
  - Requires 3.2.1, 2.1.3.
  - See theoremc-design.md §§5.4, 7.3 and ADR-0004.
  - Success: signature drift causes compile failure in the theorem owner
    crate.
- [x] 3.3.2. Emit referenced-type probes for generated struct-literal
  synthesis and step bindings to surface missing-type and moved-type breakages
  early.
  - Requires 3.3.1, 2.3.3.
  - See theoremc-design.md §§5.5, 7.3.
  - Success: compile-fail tests validate predictable drift diagnostics.

## 4. One Cargo-native entry point for project integration

Idea: if theoremc ships one Cargo-native, agent-readable command for project
inspection, scaffolding, build integration, and proof-backend management,
adopters and agents can integrate theorems without bespoke scripts or the
separate `rust-prover-tools` CLI.

This phase builds the `cargo theorem` spine on the completed compile-time
foundation. Backend management may proceed in parallel with phase 5; theorem
execution in phase 6 depends on both. See cargo-theorem-cli-design.md §§1-10,
12 and §19.

### 4.1. Establish the Cargo subcommand and OrthoConfig spine

This step answers whether one package can own the command tree, layered
configuration, and agent-native metadata without leaking application
dependencies into the reusable libraries. Its outcome fixes the contracts that
every later command extends. See cargo-theorem-cli-design.md §§4-6, 8, 14 and
ADR-0009.

- [x] 4.1.1. Add a dedicated `crates/cargo-theorem` package and
  `cargo-theorem` executable, keep application-only dependencies outside the
  root theoremc facade, and remove the placeholder root `src/main.rs`.
  - Requires 1.3, 3.1.
  - See cargo-theorem-cli-design.md §§4.1, 5.
  - Success: both `cargo theorem --help` and `cargo-theorem theorem --help`
    execute the same command tree in end-to-end tests, and no unrelated
    `theoremc` binary remains.
- [x] 4.1.2. Adopt `ortho_config` 0.10.0 for CLI, environment, TOML file,
  named profile, and selected-subcommand merging, using source-injection APIs,
  `SelectedSubcommandMerge`, and `OrthoConfigSubcommandDocs`.
  - Requires 4.1.1.
  - Declare command behaviour metadata and validate the package with
    `cargo orthohelp --check-agent-native`.
  - See cargo-theorem-cli-design.md §6 and ADR-0009.
  - Success: injected-source tests prove global defaults < files < profile <
    environment < flags; command settings remain under `cmds.<command>` with
    defaults < files < environment < flags; the selected-command adapter uses
    the same resolved file chain as global settings, including explicit
    `--config`; profiles reject command namespaces as v0.10.0 requires; and
    generated recursive metadata matches the Clap command tree.
- [x] 4.1.3. Declare Rust 1.89 for every workspace package: `cargo-theorem`
  requires it for `ortho_config` 0.10.0, and the reusable libraries require it
  for `serde-saphyr` 2.0.0.
  - Requires 4.1.2.
  - See cargo-theorem-cli-design.md §4.2, ADR-0009, and ADR-0010.
  - Success: CI runs one Rust 1.89 workspace check.
- [ ] 4.1.4. Implement the versioned JSON application envelope,
  stdout/stderr stream invariants, stable exit classes, and global renderer
  options.
  - Requires 4.1.2.
  - See cargo-theorem-cli-design.md §§8.2-8.4 and ADR-0002 decision 4.
  - Success: snapshots cover success and failure for human and JSON modes, and
    subprocess output cannot leak beside a JSON document.
- [x] 4.1.5. Implement `cargo theorem context --json` from OrthoConfig's
  compact agent context and add agent-native policy checks to CI.
  - Requires 4.1.2.
  - Follow-on slices add their paths and metadata to the same generated
    context.
  - See cargo-theorem-cli-design.md §§6.6, 14, 15.
  - Success: the versioned context includes every command path and its
    available behaviour metadata registered in the foundation tree, without
    advertising later unimplemented commands or reading project state or the
    network.

### 4.2. Inspect Cargo projects through bounded read-only commands

This step answers whether theorem and action inspection can be served from
Cargo metadata with stable ordering and bounded responses. Its outcome provides
the package selection that scaffolding, generation, and execution reuse. See
cargo-theorem-cli-design.md §§6.4, 8.5, 9.3, 10.3.

- [ ] 4.2.1. Implement workspace and package resolution through the stable
  `cargo metadata` JSON protocol with `--manifest-path`, `--package`,
  `--workspace`, and `--exclude`.
  - Requires 4.1.2.
  - See cargo-theorem-cli-design.md §6.4.
  - Success: fixtures cover virtual workspaces, ambiguous names, excluded
    members, and explicit manifests; ambiguity diagnostics enumerate valid
    package identifiers.
- [ ] 4.2.2. Implement bounded top-level `list` and `get` theorem commands
  plus `action list` and `action get`, using stable ordering, `--limit`, and
  opaque cursors.
  - Requires 4.2.1, 4.1.4, 2.2.2.
  - See cargo-theorem-cli-design.md §§8.5, 9.3 and name-mangling-rules.md
    "Stable external theorem identifiers".
  - Success: JSON snapshots contain stable theorem IDs, paths, backends,
    action names, signatures, and next-cursor metadata without unbounded
    source or log content.
- [ ] 4.2.3. Implement read-only `check` composition for schema, action
  signatures, and build integration.
  - Requires 4.2.1, 4.1.4, 3.3.2.
  - Alias and backend-pin components join `check` in 6.1.2 and 4.4.2.
  - See cargo-theorem-cli-design.md §10.3 and theoremc-design.md §§6, 7.
  - Success: `check` never performs installation or network mutation, and
    every failing component produces a stable diagnostic code and application
    exit class.

### 4.3. Mutate projects through atomic plans

This step answers whether scaffolding and build-script management can be
expressed as previewable, idempotent mutation plans that never leave a
half-written project. See cargo-theorem-cli-design.md §§9, 10, 16 and ADR-0008.

- [ ] 4.3.1. Extract build discovery and suite rendering behind a reusable
  `theoremc-build` library API, including fallible direct execution and the
  stable build-script boundary `theoremc::build::emit_suite_from_env_or_exit()`.
  - Requires 4.1.1, 3.1.2.
  - See cargo-theorem-cli-design.md §10.1, theoremc-design.md §7.1, and
    ADR-0008.
  - Success: the root build script and the direct CLI path use the same
    discovery and rendering fixtures.
- [ ] 4.3.2. Implement an immutable project mutation plan with create, edit,
  unchanged, conflict, and manual-action entries, applied under a workspace
  lock with temporary siblings, flush, atomic rename, and a recovery journal.
  - Requires 4.2.1.
  - See cargo-theorem-cli-design.md §§9.1, 16.
  - Success: property tests prove repeat application is idempotent, and
    failure-injection tests leave either the old or the complete new state.
- [ ] 4.3.3. Implement `init`, top-level `create`, and `action create` with
  `--dry-run` and narrowly scoped `--force` handling.
  - Requires 4.3.2.
  - See cargo-theorem-cli-design.md §§9.1-9.2, theorem-file-specification.md
    §§1-3, and theoremc-design.md §5.
  - Success: scaffolds are schema valid, do not invent assertions, do not
    generate panic or `todo!()` stubs by default, and refuse unsafe
    overwrites.
- [ ] 4.3.4. Implement `build-script install`, `check`, `run`, and `delete`
  using parsed Rust source spans and managed calls rather than blind string
  replacement.
  - Requires 4.3.1, 4.3.2.
  - See cargo-theorem-cli-design.md §§10.1-10.2 and theoremc-design.md §7.1.
  - Success: fixtures cover absent, generated, already managed, and complex
    existing build scripts; unsafe edits become structured conflicts.
- [ ] 4.3.5. Implement `generate --kind suite|harness|metadata` with
  deterministic selection and explicit delivery for large artefacts.
  - Requires 4.3.1, 4.2.1.
  - See cargo-theorem-cli-design.md §10.3.
  - Success: direct generation matches build-script output byte for byte for
    the same inputs.

### 4.4. Manage proof backends with prover-tool parity

This step answers whether Kani and Verus installation and low-level execution
can move behind one injected provider contract with deterministic pins. Its
outcome unblocks theorem execution in step 6.2 and the retirement of
`rust-prover-tools`. This step may proceed in parallel with phase 5. See
cargo-theorem-cli-design.md §12.

- [ ] 4.4.1. Define a backend provider contract for discovery, release
  resolution, install planning, health checks, argument-vector execution,
  result parsing, and replay planning.
  - Requires 4.1.2.
  - Inject one command runner for timeouts, cancellation, stream capture, and
    test doubles; shell command strings are prohibited.
  - See cargo-theorem-cli-design.md §12.1.
  - Success: a fake provider exercises every capability without
    process-global state.
- [ ] 4.4.2. Implement `theoremc.toml` backend intent, a deterministic
  `theoremc.lock`, content-addressed platform cache entries, and `backend list`,
  `get`, `install`, `check`, `sync`, `update`, and `delete`.
  - Requires 4.4.1, 4.2.3, 4.3.2.
  - Extend `check` with the configured backend-pin component.
  - See cargo-theorem-cli-design.md §§12.2-12.3.
  - Success: installation mutations support `--dry-run`, pin checks never
    install, and the lock omits machine-specific paths.
- [ ] 4.4.3. Port Kani installation, setup, semantic-version checking, command
  override, harness execution, and concrete playback from `rust-prover-tools`.
  - Requires 4.4.2.
  - See cargo-theorem-cli-design.md §12.4 and theoremc-design.md §8.
  - Success: parity scenarios cover matching and mismatched pins, missing
    installations, setup failure, and version parsing.
- [ ] 4.4.4. Port Verus release-target selection, bounded download, checksum
  verification, safe extraction, binary discovery, Rust toolchain preparation,
  and raw proof-file execution from `rust-prover-tools`.
  - Requires 4.4.2.
  - See cargo-theorem-cli-design.md §12.5.
  - Success: default tests use fake executables and local archives; opt-in
    jobs cover real releases.
- [ ] 4.4.5. Implement `backend run <backend>` and one-release legacy
  environment migration for the existing Kani and Verus variables.
  - Requires 4.4.3, 4.4.4.
  - See cargo-theorem-cli-design.md §§6.3, 12.3.
  - Success: migration diagnostics identify the replacement setting, and
    `--passthrough-exit-code` is isolated as an advanced compatibility option.

### 4.5. Lock the public CLI contract with end-to-end suites

This step answers whether the command surface built in steps 4.1 to 4.4 holds
its documented contracts in combination: help, context, JSON schemas, bounded
behaviour, mutation safety, and migration parity. Full theorem-suite execution,
reports, and detached run jobs are out of scope. See
cargo-theorem-cli-design.md §§15, 17, 18.

- [ ] 4.5.1. Add the public contract suite: snapshot and schema tests for
  human help, `context --json`, JSON success and error envelopes, exit classes,
  choice-enumerating diagnostics, and pagination across every public command.
  - Requires 4.1, 4.2, 4.3, 4.4.
  - See cargo-theorem-cli-design.md §§8, 15.
  - Success: every public command declares interaction, mutation, output, and
    bounded-response metadata, and the suite fails when one does not.
- [ ] 4.5.2. Add the end-to-end combinatorial suite for direct invocation,
  Cargo external subcommand invocation, configuration precedence, dry runs,
  project locks, atomic writes, and fake backend processes.
  - Requires 4.1, 4.2, 4.3, 4.4.
  - See cargo-theorem-cli-design.md §18.
  - Success: default tests require neither a network nor real prover
    installations.
- [ ] 4.5.3. Add command-for-command migration fixtures for all four public
  `rust-prover-tools` workflows.
  - Requires 4.4.5.
  - See cargo-theorem-cli-design.md §§12.4-12.5, 18.
  - Success: each legacy invocation has a documented `cargo theorem`
    equivalent and equivalent externally observable backend behaviour.
- [ ] 4.5.4. Implement the metric registry and its emitted structured events.
  - Requires 4.1, 4.2, 4.3, 4.4.
  - See cargo-theorem-cli-design.md §§17.1-17.3.
  - Success: tests assert every registered metric is emitted with its
    documented kind and label set, every metric carries a `describe_*`
    declaration, and no unbounded value reaches a label.

## 5. A working Kani proof path with explicit non-vacuity

Idea: if a version 1 theorem can produce a real Kani proof or counterexample
through a backend-neutral harness plan, with non-vacuity enforced by default,
then operational breadth and schema version 2 semantic laws can reuse that path
instead of building parallel renderers.

This phase follows ADR-0011: deliver the vertical slice first, then broaden
semantics, then add RFC 0001 laws. Implementation plans must cite ADR-0006. See
theoremc-design.md §8.

### 5.1. Deliver the Kani vertical slice through the shared harness plan

This step answers whether a small version 1 subset can lower into a shared
`HarnessPlan`, render to Kani, and yield one real proof and one real
counterexample through normal build integration. Per ADR-0008,
`theoremc-build-support` remains limited to build-script preparation,
discovery, and suite writing; `HarnessPlan` belongs in `theoremc-core` per
ADR-0006. Schema version 2 laws, rich reports, and non-Kani backends are out of
scope. See theoremc-design.md §§8.1-8.4, ADR-0006, and ADR-0011.

- [ ] 5.1.1. Introduce the internal backend-neutral `HarnessPlan` in
  `theoremc-core` with resolved invocations, source provenance, stable
  obligation identities, and explicitly opaque Rust predicates.
  - Requires 3.3.2.
  - See ADR-0006, ADR-0007, ADR-0008, and ADR-0011.
  - Success: plan fixtures contain no token streams, Kani syntax, or generated
    local names as semantic identities.
- [ ] 5.1.2. Lower the supported version 1 subset into the shared plan:
  ordered typed `Forall`, `Assume`, `Let`, infallible `call` with bound results,
  `Prove`, and `Witness`.
  - Requires 5.1.1.
  - See theorem-file-specification.md §§1-4, ADR-0006, and ADR-0011.
  - Success: unit tests cover binding order and identity; all source
    constructs lacking implemented lowering fail deterministically, including
    fallible calls, `must`, and `maybe` until supported.
- [ ] 5.1.3. Promote the argument-lowering prototype into the owned
  proc-macro rendering path, and render plan operations as symbolic bindings,
  assumptions, invocations, assertions with rationales, and covers.
  - Requires 5.1.2, 2.3.3, 3.2.2.
  - See theorem-file-specification.md §5, theoremc-design.md §§5.5, 8.1, and
    ADR-0006.
  - Success: generated-code snapshots verify literals, references, lists,
    struct values, and declared borrowing without implicit clones or default
    construction; the Kani renderer does not consume schema types directly.
- [ ] 5.1.4. Add a small longhand idempotence example using a pure
  shared-reference primary with an owned result, plus a deliberately false
  variant, executed by real Kani.
  - Requires 5.1.3.
  - Retain the example for version 2 plan-equivalence fixtures.
  - See theoremc-design.md §§7, 8, ADR-0001 decision 4, ADR-0006, and
    ADR-0011.
  - Success: fixture crates discover theorems through normal build scripts;
    ordinary `cargo build` needs no Kani dependencies; `cargo kani list`
    discovers stable harness names; real Kani proves one claim and produces a
    counterexample for the other while preserving theorem and obligation
    identity; explicit witnesses are emitted and checked; and at least one CI
    path or maintainer gate executes Kani, so mocked commands or snapshots
    alone do not complete this task.

### 5.2. Expand `call`, `must`, and `maybe` operational semantics

This step answers whether operational breadth can be added in independent
increments through the same resolved-invocation model and Kani renderer.
Probabilistic branching semantics and a second direct renderer are out of
scope. See theorem-file-specification.md §4 and theoremc-design.md §§4.4, 8.2.

- [ ] 5.2.1. Complete value-returning and unit-returning `call`, then bound
  fallible `call` with explicit results; enforce `as` binding rules and reject
  unbound non-unit results in `Do`.
  - Requires 5.1.4.
  - See theorem-file-specification.md §4, theoremc-design.md §§4.4, 8.2,
    ADR-0006, and ADR-0011.
  - Success: semantic tests cover `()`, owned values, `Result`, and `Option`
    signatures.
- [ ] 5.2.2. Add `must` over `Result`, then `Option`, then infallible
  documentary `must`, as independently tested increments.
  - Requires 5.2.1.
  - See theorem-file-specification.md §4, theoremc-design.md §8.2, and
    ADR-0011.
  - Success: real harness tests produce counterexamples for failed success
    obligations and retain source identity; successful cases yield the
    unwrapped value.
- [ ] 5.2.3. Add symbolic `maybe` with nested steps through shared plan
  branching.
  - Requires 5.2.1.
  - See theorem-file-specification.md §4, theoremc-design.md §8.2, and
    ADR-0011.
  - Success: Kani explores both branches under bounded settings and nested
    obligations retain structural identities.

### 5.3. Enforce expected results and non-vacuity at run time

This step answers whether backend outcomes can be associated with stable
obligation identities and checked against declared evidence before any rich
reporting exists. Markdown, HTML, JUnit, Cucumber, playback, and full CLI
orchestration are out of scope. See theoremc-design.md §§8.3-8.4 and ADR-0011.

- [ ] 5.3.1. Implement a minimal runner that associates backend outcomes with
  stable theorem and obligation identities.
  - Requires 5.1, 5.2.
  - See ADR-0006, ADR-0007, and ADR-0011.
  - Success: fixtures distinguish `proved`, `falsified`, `unreachable`,
    `undetermined`, `unsupported`, and `execution_error`; resource exhaustion
    is `undetermined` with reason `resource_exhaustion`.
- [ ] 5.3.2. Compare the actual status with `Evidence.kani.expect` and fail
  mismatches.
  - Requires 5.3.1.
  - See theorem-file-specification.md §6, theoremc-design.md §8.3, and
    ADR-0001 decision 4.
  - Success: integration tests cover SUCCESS, FAILURE, UNREACHABLE, and
    UNDETERMINED; unreachable and undetermined fail by default unless
    explicitly expected and justified.
- [ ] 5.3.3. Enforce explicit witnesses at run time unless both
  `allow_vacuous: true` and a non-empty `vacuity_because` justify the override.
  - Requires 5.3.1, 1.2.4.
  - See theorem-file-specification.md §6, theoremc-design.md §8.4, and
    ADR-0001 decision 4.
  - Success: validation and runtime tests reject a missing rationale and
    distinguish unreachable witnesses from successful proofs.

### 5.4. Integrate RFC 0001 semantic laws through the proven Kani path

This step answers whether schema version 2 laws can lower through semantic Law
IR into the existing `HarnessPlan`, renderer, and runner without a second path.
Verus bindings or rendering, rich reporting, and implicit purity proofs or
cloning adapters are out of scope. Work must cite ADR-0006 and ADR-0007. See
RFC 0001.

- [ ] 5.4.1. Freeze omitted `Schema` as the version 1 compatibility profile
  and add explicit version 2 selection with raw and public `ProofObligation`
  variants.
  - Requires 5.1, 5.2, 5.3.
  - Document the breaking `Vec<Assertion>` to `Vec<ProofObligation>` release
    boundary and migration.
  - See RFC 0001, theorem-file-specification.md §1, and ADR-0011.
  - Success: parser fixtures preserve existing assertions in both versions,
    reject laws in omitted or version 1 documents, reject unsupported
    versions, and locate malformed or ambiguous obligation shapes.
- [ ] 5.4.2. Add domain types for `idempotent`, `round_trip`, `equivalent`,
  `refines`, `preserves`, and `homomorphism`, plus common primary invocation
  and relation types outside raw adapters.
  - Requires 5.4.1.
  - Extend referenced-action, conflict, and type probes to law primaries.
  - See RFC 0001, ADR-0006, and ADR-0007.
  - Success: parameterized tests validate action signatures, roles, fixed
    arguments, modes, shared-reference bound parameters, composition, and
    boolean relations; ordinary rustc composition probes cover type
    equivalence beyond structural validation.
- [ ] 5.4.3. Lower validated laws into semantic Law IR, then the existing
  `HarnessPlan`, reusing resolved invocations, the Kani renderer, and the
  minimal runner.
  - Requires 5.4.2, 5.2.3.
  - See RFC 0001, ADR-0006, and ADR-0007.
  - Success: fixtures retain parent and child obligation identity, rationale,
    provenance, and backend constraints without Kani syntax in either semantic
    representation; complete capability checking rejects unsupported
    operations before partial emission.
- [ ] 5.4.4. Add the end-to-end law suite: positive and negative Kani
  integration cases for all six laws, including a counterexample for every
  comparison law and unreachable preconditions for `preserves`.
  - Requires 5.4.3.
  - See RFC 0001, ADR-0001 decision 4, ADR-0006, ADR-0007, and ADR-0011.
  - Success: longhand version 1 and structured version 2 examples yield
    semantically equivalent plans; generated precondition covers supplement
    explicit witnesses, and unreachable checks never count as proved.
- [ ] 5.4.5. Publish versioned law examples for normalization, codecs,
  reference equivalence, refinement, state transitions, and collection
  homomorphisms.
  - Requires 5.4.4.
  - Record future Verus binding design as additive follow-up work in 9.1.1.
  - See RFC 0001 and ADR-0007.
  - Success: guidance shows explicit adapters for mutation-heavy APIs and
    explains custom-relation weakness, witness requirements, bounded evidence,
    and when opaque assertions are appropriate.

## 6. Durable theorem runs with stable identity and reports

Idea: if `cargo theorem run` records every execution in a canonical,
identity-stable run ledger, then human reports, CI artefacts, and
counterexample playback can all be projections of one record instead of parsers
of terminal output.

This phase connects the Kani path from phase 5 to the command spine from phase
4 and replaces the reporting scope formerly assigned to `theoremd`. See
cargo-theorem-cli-design.md §§11, 13 and theoremc-design.md §9.

### 6.1. Give theorems stable external identities across renames

This step answers whether canonical theorem IDs and an alias graph can keep run
history and selection stable when files move. Automatic alias file editing is
out of scope. See name-mangling-rules.md "Stable external theorem identifiers"
and ADR-0001 decision 5.

- [ ] 6.1.1. Implement canonical external theorem ID generation as
  `{normalized_path(P)}#{T}` with path normalization rules.
  - Requires 4.2.1, 5.3.1.
  - See name-mangling-rules.md "Stable external theorem identifiers",
    theoremc-design.md §9, and ADR-0001 decision 5.
  - Success: tests cover path separator normalization and leading `./`
    removal.
- [ ] 6.1.2. Implement alias file loading from
  `theorems/theorem-id-aliases.yaml` and deterministic resolution of deprecated
  IDs to canonical IDs, and add the alias component to `check`.
  - Requires 6.1.1, 4.2.3.
  - See name-mangling-rules.md "Stable external theorem identifiers" and
    ADR-0001 decision 5.
  - Success: tests cover direct aliases and multi-hop alias chains.
- [ ] 6.1.3. Detect and reject alias cycles and ambiguous resolutions at load
  time.
  - Requires 6.1.2.
  - See name-mangling-rules.md "Stable external theorem identifiers" and
    ADR-0001 decision 5.
  - Success: cycle and ambiguity fixtures fail with actionable diagnostics.

### 6.2. Orchestrate theorem execution into a durable run ledger

This step answers whether selection, planning, backend execution, and evidence
policy can run as one staged pipeline whose every outcome is persisted and
mapped to a stable exit class. Hosted execution and report dashboarding are out
of scope. See cargo-theorem-cli-design.md §§11.1-11.2, 13.1.

- [ ] 6.2.1. Implement shared selection by theorem ID, path, tag, package,
  backend, and changed revision, with stable ordering and an explicit
  empty-selection policy.
  - Requires 6.1.3, 4.2.1.
  - See cargo-theorem-cli-design.md §11.1 and name-mangling-rules.md "Stable
    external theorem identifiers".
  - Success: property tests cover category intersection, repeated-filter
    union, aliases, exclusions, and deterministic order.
- [ ] 6.2.2. Implement top-level `run` as resolve, validate, pin-check, plan,
  execute, parse, policy, persist, and render stages.
  - Requires 6.2.1, 5.3.3, 4.4.3, 4.3.1.
  - See cargo-theorem-cli-design.md §11.2 and theoremc-design.md §§8, 9.
  - Success: `run` never installs a backend, records backend-native outcomes
    before applying evidence policy, and maps every final state to a stable
    application exit class.
- [ ] 6.2.3. Persist every foreground and detached run under
  `.theoremc/runs/<run-id>/` with an append-only bounded index, captured logs,
  input digest, parent link, and canonical plan.
  - Requires 6.2.2.
  - See cargo-theorem-cli-design.md §13.1.
  - Success: crash-recovery tests leave inspectable terminal or resumable
    records rather than orphaned processes.
- [ ] 6.2.4. Implement blocking-by-default execution, `--no-wait`,
  `--timeout`, bounded parallelism, cancellation, and optional idempotency keys.
  - Requires 6.2.3.
  - See cargo-theorem-cli-design.md §§8.6, 11.2.
  - Success: duplicate active submissions return the existing job when inputs
    match and fail with both digests when they differ.
- [ ] 6.2.5. Implement `jobs list`, `get`, `cancel`, and `prune` with
  pagination, bounded log excerpts, mutation previews, and explicit retention
  rules.
  - Requires 6.2.4.
  - See cargo-theorem-cli-design.md §§8.6, 13.1.
  - Success: `list` and `get` remain bounded, and `prune` cannot delete
    records outside its structured plan.

### 6.3. Project the canonical run record into human and CI reports

This step answers whether one serialized run record can carry everything that
Markdown, HTML, JUnit XML, and Cucumber JSON need, so no renderer parses
terminal output. Dashboard hosting is out of scope. Work must cite ADR-0007.
See cargo-theorem-cli-design.md §§11.3-11.4 and theoremc-design.md §9.

- [ ] 6.3.1. Implement a canonical theorem run record with theorem ID,
  metadata, assumptions, step outcomes, generic proof obligations, law parent
  and child identities and rationales, witnesses, backend-indexed evidence,
  provenance, diagnostics, artefacts, and final status.
  - Requires 6.2.3, 5.4.3.
  - See cargo-theorem-cli-design.md §11.3, theoremc-design.md §9,
    theorem-file-specification.md §§1, 6, RFC 0001, and ADR-0007.
  - Success: serialized fixtures round-trip without field loss, preserve the
    six terminal statuses and the resource-exhaustion reason, distinguish
    bounded Kani from future deductive Verus evidence, and separate invariant
    fields from run identity and time.
- [ ] 6.3.2. Implement Markdown and HTML report rendering from the canonical
  run record.
  - Requires 6.3.1.
  - See cargo-theorem-cli-design.md §11.4 and theoremc-design.md §9.
  - Success: golden snapshots cover pass, fail, unreachable, undetermined,
    timeout, cancellation, and expected-failure examples.
- [ ] 6.3.3. Implement JUnit XML and Cucumber JSON emitters for CI
  integration.
  - Requires 6.3.1.
  - See cargo-theorem-cli-design.md §11.4 and theoremc-design.md §9.
  - Success: schema validation tests pass for both formats, and no renderer
    parses terminal output.
- [ ] 6.3.4. Implement `report create <run-id>` with repeatable formats and
  explicit delivery for large or multiple artefacts.
  - Requires 6.3.2, 6.3.3.
  - See cargo-theorem-cli-design.md §11.4.
  - Success: JSON application mode returns artefact references without mixing
    report bytes into stdout.

### 6.4. Replay counterexamples as linked child runs

This step answers whether Kani concrete playback can be captured as a child run
and surfaced in reports without rewriting application source. Automated source
rewriting workflows are out of scope. See theoremc-design.md §8.5 and
cargo-theorem-cli-design.md §11.4.

- [ ] 6.4.1. Integrate Kani concrete playback execution for failed harnesses
  and capture generated replay artefacts as a child run.
  - Requires 6.2.3, 4.4.3.
  - See theoremc-design.md §8.5 and cargo-theorem-cli-design.md §11.4.
  - Success: a failing theorem integration test produces linked source,
    command, log, and provenance artefacts without rewriting application
    source.
- [ ] 6.4.2. Surface playback metadata and retrieval paths in human and CI
  reports.
  - Requires 6.4.1, 6.3.2, 6.3.3.
  - See theoremc-design.md §9 and cargo-theorem-cli-design.md §11.4.
  - Success: report snapshots include replay references only when available.

### 6.5. Complete profiles, delivery, feedback, and prover-tool migration

This step answers whether the remaining operator workflows can ship on
OrthoConfig's reusable contracts and whether `rust-prover-tools` can be
retired. Reusable OrthoConfig contracts are preferred; tracked temporary
adapters may cover soft dependencies until they ship. Mandatory network
services are out of scope. See cargo-theorem-cli-design.md §§6, 13.2-13.3.

- [ ] 6.5.1. Implement named profiles with secret redaction and precedence
  between project files and environment variables, plus bounded `profile list`,
  `get`, `save`, and guarded `delete`.
  - Requires 4.5.1, 4.3.2.
  - See cargo-theorem-cli-design.md §§6.3, 6.5.
  - Success: context exposes profile names and non-secret fields only, and
    profile mutations support `--dry-run`.
- [ ] 6.5.2. Implement `stdout` and `file:<path>` delivery, leaving
  `webhook:<url>` behind an explicit later capability.
  - Requires 6.3.4.
  - See cargo-theorem-cli-design.md §13.2.
  - Success: `stdout` is validated before the first byte is written and
    carries partial-write semantics, while `file:<path>` is atomic through a
    temporary sibling and rename; unknown schemes enumerate valid choices, and
    JSON mode returns delivery metadata rather than mixed payloads.
- [ ] 6.5.3. Implement privacy-bounded local JSON Lines feedback with optional
  diagnostic attachment and no implicit source, environment, proof, or log
  capture.
  - Requires 4.5.1.
  - See cargo-theorem-cli-design.md §13.3.
  - Success: tests prove sensitive fields remain absent unless explicitly
    attached.
- [ ] 6.5.4. Publish migration guidance and cross-platform parity results for
  all `rust-prover-tools` workflows, then mark the Python CLI maintenance-only.
  - Requires 4.5.3, 6.2.2.
  - See cargo-theorem-cli-design.md §§12.4-12.5, 19.
  - Success: archival cannot occur until Kani and Verus parity fixtures pass
    on every supported host.

## 7. Guardrails, examples, and adoption

Idea: if theoremc's own guarantees are enforced by lints and architecture
checks, and adopters have runnable examples and authoritative guidance, the
system stays hard to bypass and easy to adopt as it grows.

See theoremc-design.md §§10, 12.3.

### 7.1. Flag proof code that bypasses theoremc

This step answers whether opt-in lints can distinguish theoremc-generated
harnesses from handwritten Kani code with zero false positives. Mandatory lint
enforcement for all adopters is out of scope. See theoremc-design.md §10.

- [ ] 7.1.1. Implement theorem-generated marker attributes and the metadata
  needed by enforcement lints.
  - Requires 5.1.3.
  - See theoremc-design.md §§7.2, 10.
  - Success: generated harnesses carry stable markers for lint
    identification.
- [ ] 7.1.2. Implement Dylint rules that flag raw `kani::assume` and unmarked
  `#[kani::proof]` usage outside theoremc-generated modules.
  - Requires 7.1.1.
  - See theoremc-design.md §10.
  - Success: a lint test crate demonstrates the expected warnings and zero
    false positives in generated modules.

### 7.2. Enforce schema architectural boundaries

This step answers whether the schema layer contract can be held by module
visibility, CI graph checks, and lints without splitting layers into separate
crates. See theoremc-design.md §§3.2, 10.1-10.3 and ADR-0003.

- [ ] 7.2.1. Introduce and stabilize explicit schema-layer module boundaries
  (domain, raw adapter, validator, loader, diagnostics) using non-public
  internals and curated re-exports.
  - Requires 1.3.2.
  - See theoremc-design.md §§3.2, 10.1 and ADR-0003 decisions 1 and 2.
  - Success: public `schema` exports include only approved API types and
    loader entry points, and contract tests prove raw adapter internals are
    not importable by consumers.
- [ ] 7.2.2. Add CI acyclicity checks with
  `cargo modules graph --acyclic --lib`.
  - Requires 7.2.1.
  - See theoremc-design.md §10.3 and ADR-0003 decision 3.
  - Success: the current module graph passes, and a synthetic cycle fixture
    causes the architecture check to fail.
- [ ] 7.2.3. Enforce explicit imports by denying wildcard imports in CI.
  - Requires 7.2.1.
  - See theoremc-design.md §10.3 and ADR-0003 decision 3.
  - Success: schema modules fail lint checks when wildcard imports are
    introduced, and CI runs with `-D clippy::wildcard_imports` enabled.
- [ ] 7.2.4. Implement `theoremc_arch_lint` Dylint rules for forbidden import
  edges, such as `schema::types` importing `schema::raw`.
  - Requires 7.2.1, 7.1.2.
  - See theoremc-design.md §10.2 and ADR-0003 decision 4.
  - Success: lint fixtures demonstrate the expected failures for forbidden
    edges and no failures for allowed edges.
- [ ] 7.2.5. Add optional `cargo-deny` policy checks for
  architecture-sensitive dependencies, such as constraining YAML adapter
  dependencies to adapter contexts.
  - Requires 7.2.1.
  - See theoremc-design.md §10.2 and ADR-0003 decision 5.
  - Success: dependency policy checks pass in CI and fail on deliberate
    policy violations in fixtures.

### 7.3. Publish runnable examples and authoritative guidance

This step answers whether a new adopter can go from an empty crate to a
reported proof using only the published examples and guides. The small
first-slice example belongs to 5.1.4 and does not wait for this step.
Production deployment templates are out of scope. See theoremc-design.md §§3-5,
8 and cargo-theorem-cli-design.md §18.

- [ ] 7.3.1. Create end-to-end example crates (`account`, `hnsw`) that
  demonstrate action exports, theorem files, generated harness behaviour,
  `cargo theorem` project checks, backend synchronization, execution, and
  reports.
  - Requires 6.3.4, 4.5.2, 7.2.1.
  - See theoremc-design.md §§3, 5, 8 and cargo-theorem-cli-design.md §18.
  - Success: examples compile and execute through the documented theorem
    workflow in CI.
- [ ] 7.3.2. Write and publish user-facing guidance for theorem authoring
  rules, especially explicit references, witness policy, expected status usage,
  project initialization, backend pins, and run artefacts.
  - Requires 6.2.2, 4.3.3, 4.4.2, 5.3.3.
  - See theorem-file-specification.md §5, theoremc-design.md §4, ADR-0001
    decisions 3 and 4, and cargo-theorem-cli-design.md §§9, 12.
  - Success: the guides contain copy-paste-ready examples that match
    implementation semantics.
- [ ] 7.3.3. Bring the
  [`cargo theorem` user's guide](cargo-theorem-cli-users-guide.md) to parity
  with every shipped command: installation and first use, project
  initialization, theorem creation, checking, backend installation rules,
  execution, reports, jobs, profiles, JSON output, exit classes, bounded
  results, and migration from `rust-prover-tools`.
  - Requires 6.5.4.
  - See cargo-theorem-cli-design.md §§7, 18.
  - Success: every released command has a documented task path, and the guide
    states a proposed status for any command that is not yet stable.
- [ ] 7.3.4. Add a contributor checklist that requires parser fixtures,
  codegen snapshots, command-context snapshots, run-record fixtures, and report
  snapshots for behavioural changes.
  - Requires 6.3.2, 4.5.1.
  - See theoremc-design.md §§6, 7, 9 and cargo-theorem-cli-design.md §18.
  - Success: the pull request template and contributor documentation reference
    the checklist explicitly.

## 8. Library-first localization with deterministic machine output

Idea: if diagnostics are modelled as stable codes and structured arguments with
English fallback, theoremc can render localized human text through injected
Fluent localizers while every machine-facing artefact stays byte-identical
across locales.

See theoremc-design.md §§6.8-6.10 and ADR-0002.

### 8.1. Model diagnostics as stable codes and arguments

This step answers whether every parser and validator diagnostic can be
expressed as a code, structured arguments, and a source location with required
English fallback text. Locale negotiation and user interface (UI)-specific
message composition are out of scope. See theoremc-design.md §§6.8, 6.10 and
ADR-0002 decision 2.

- [ ] 8.1.1. Define a canonical diagnostic payload type containing code,
  structured arguments, source location, and required English fallback text.
  - Requires 1.3.1.
  - See theoremc-design.md §§6.3, 6.8, 6.10 and ADR-0002 decision 2.
  - Success: parser and validator diagnostics are emitted through this model
    with snapshot-backed stability tests.
- [ ] 8.1.2. Define the compatibility policy for diagnostic codes and argument
  schemas.
  - Requires 8.1.1.
  - See theoremc-design.md §6.8 and ADR-0002 decision 2.
  - Success: contributor documentation and tests guard against accidental
    breaking changes in existing codes and argument keys.

### 8.2. Render diagnostics through an injected Fluent localizer

This step answers whether a library-safe localizer contract can layer consumer
catalogues over embedded defaults without process-wide locale state. See
theoremc-design.md §§6.8-6.9 and ADR-0002 decisions 1 and 3.

- [ ] 8.2.1. Introduce a library-level `Localizer` trait, or equivalent, that
  renders diagnostic messages from code and structured arguments.
  - Requires 8.1.1.
  - See theoremc-design.md §6.8 and ADR-0002 decision 1.
  - Success: theoremc APIs accept localization as injected context and never
    read locale environment variables.
- [ ] 8.2.2. Embed theoremc `en-US` Fluent resources and expose them for
  consumer loader composition.
  - Requires 8.2.1.
  - See theoremc-design.md §6.9 and ADR-0002 decision 3.
  - Success: integration tests show host applications can load theoremc
    Fluent assets into an existing Fluent language loader.
- [ ] 8.2.3. Implement an optional Fluent-backed localizer that layers
  consumer catalogues over theoremc defaults with deterministic fallback for
  missing keys and formatting failures.
  - Requires 8.2.2.
  - See theoremc-design.md §6.9 and ADR-0002 decision 3.
  - Success: tests cover consumer hit, default fallback, and formatting-error
    fallback paths.

### 8.3. Keep machine-facing artefacts locale-invariant

This step answers whether compile-time diagnostics, application JSON, and
reports can carry localized projections without changing their machine fields.
Parser keyword localization is out of scope. See theoremc-design.md §6.8,
theoremc-design.md §9, and ADR-0002 decision 4.

- [ ] 8.3.1. Keep proc-macro and code-generation diagnostics deterministic
  English.
  - Requires 8.1.1.
  - See theoremc-design.md §6.8 and ADR-0002 decision 4.
  - Success: compile-fail and snapshot tests confirm output stability across
    host locale changes.
- [ ] 8.3.2. Extend `cargo theorem` diagnostic objects to always include the
  stable diagnostic code and arguments and the required English fallback text,
  attaching localized text only when a localizer is configured.
  - Requires 8.2.3, 6.3.4, 4.1.4.
  - See theoremc-design.md §§6.8, 9, ADR-0002 decisions 2 and 4, and
    cargo-theorem-cli-design.md §8.3.
  - Success: application JSON and report snapshots for Markdown, HTML, JUnit
    XML, and Cucumber JSON confirm invariant machine fields on diagnostic
    objects and optional localized projection fields; successful envelopes do
    not require diagnostic fields.
- [ ] 8.3.3. Add the cross-locale end-to-end suite proving machine-facing
  artefacts are identical across locales while localized human-facing strings
  vary only in localized fields.
  - Requires 8.3.1, 8.3.2.
  - See theoremc-design.md §§6.8, 9 and ADR-0002 decision 4.
  - Success: the suite runs the same theorem set under at least two locales
    and fails on any machine-field difference.

### 8.4. Guard the deferral of parser keyword internationalization

This step answers whether canonical schema keywords can be held stable in this
release line while localized keywords remain a future ADR. Implementation of
localized theorem schema keys is out of scope. See theoremc-design.md §4.7 and
ADR-0002 decision 5.

- [ ] 8.4.1. Document and enforce that `.theorem` top-level keys remain
  canonical in this release line.
  - Requires 8.3.3.
  - See theoremc-design.md §§4, 4.7 and ADR-0002 decision 5.
  - Success: parser tests reject unsupported localized key synonyms, and the
    documentation explains the deferral policy with its migration
    implications.

## 9. Deferred extensions after the core release

Idea: if the core Kani-backed theorem workflow is already trustworthy and
boring to operate, the project can evaluate broader backends and operator
extensions on their product value instead of letting them destabilize the main
release.

These items are compatible with the design but explicitly do not block the
first stable release. See theoremc-design.md §13 and
cargo-theorem-cli-design.md §20.

### 9.1. Evaluate deductive and model-checking backends

This step answers whether law meaning, identity, and evidence survive a second
backend. See theoremc-design.md §13, RFC 0001, and ADR-0007.

- [ ] 9.1.1. Decide on and design Verus bindings for schema version 2 laws
  that preserve law meaning and explicit executable, specification, and proof
  modes.
  - Requires 5.4.5, 6.3.1.
  - See theoremc-design.md §13, RFC 0001, and ADR-0007.
  - Success: an accepted ADR or RFC defines the binding as an additive
    extension of version 2 law meaning.
- [ ] 9.1.2. Evaluate Stateright and Alloy integration against the
  `HarnessPlan` boundary.
  - Requires 9.1.1.
  - See theoremc-design.md §13 and ADR-0006.
  - Success: a recorded decision states whether either backend graduates to
    roadmap work.

### 9.2. Evaluate operator extensions to the CLI

This step answers which deferred CLI extensions earn a place after the core CLI
is stable. See cargo-theorem-cli-design.md §20.

- [ ] 9.2.1. Decide whether webhook delivery and upstream feedback endpoints
  graduate from the explicit later capability.
  - Requires 6.5.2, 6.5.3.
  - See cargo-theorem-cli-design.md §§13.2-13.3, 20.
  - Success: a recorded decision defines the delivery capability or rejects
    it.
- [ ] 9.2.2. Evaluate remote execution providers, shared job ledgers,
  proof-result caching across workspaces, and incremental execution from
  semantic dependency graphs.
  - Requires 6.2.5.
  - See cargo-theorem-cli-design.md §20.
  - Success: a recorded decision states which extensions graduate to roadmap
    work.
- [ ] 9.2.3. Evaluate MCP server generation from agent context, automatic
  shell completion installation, and signed backend lock files or provenance
  attestations.
  - Requires 4.5.1, 4.4.2.
  - See cargo-theorem-cli-design.md §§14, 20.
  - Success: a recorded decision states which extensions graduate to roadmap
    work.

### 9.3. Evaluate localized theorem schema keywords

This step answers whether localized `.theorem` keywords justify a schema change
after the canonical-keyword guard has shipped. See theoremc-design.md §4.7 and
ADR-0002 decision 5.

- [ ] 9.3.1. Record a follow-up ADR deciding whether parser keyword
  internationalization proceeds.
  - Requires 8.4.1.
  - See theoremc-design.md §4.7 and ADR-0002 decision 5.
  - Success: an accepted ADR either defines localized keyword support with a
    migration path or confirms the canonical-only policy.
