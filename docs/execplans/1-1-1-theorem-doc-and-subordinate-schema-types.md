# Implement TheoremDoc and subordinate schema types

This ExecPlan is a living document. The sections `Constraints`, `Tolerances`,
`Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

## Purpose / big picture

After this change a library consumer can call
`theoremc::schema::load_theorem_docs(yaml_text)` to deserialize one or more
YAML (YAML Ain't Markup Language) theorem documents from a single string
into a `Vec<TheoremDoc>`. Every document is strictly validated at
deserialization time: unknown keys are
rejected, required fields must be present, scalar types must match, TitleCase
and lowercase key aliases both work, theorem identifiers must match
`^[A-Za-z_][A-Za-z0-9_]*$` and must not be Rust reserved keywords.

This is Roadmap Phase 1, Step 1.1 — the first real code in the repository.
It implements the schema structs, key-alias support, unknown-key rejection,
and multi-document file loading described in `TFS-1` and `DES-6`. Code
generation and backend emission are explicitly out of scope.

Observable success: running `make test` passes and the new tests demonstrate
that unknown keys, wrong scalar types, invalid identifiers, and Rust keywords
all fail deserialization with actionable error messages. Valid `.theorem`
documents (including multi-document files using `---` separators) deserialize
into correctly populated `TheoremDoc` structs.

## Constraints

- All code must pass `make check-fmt`, `make lint`, and `make test`.
- Clippy lints are aggressive (see `Cargo.toml` `[lints.clippy]`): no
  `unwrap`, no `expect`, no indexing, no panics in result functions, no
  missing docs, etc.
- No `unsafe` code.
- No file longer than 400 lines.
- Module-level (`//!`) doc comments on every module.
- Public APIs documented with rustdoc (`///`).
- Comments in en-GB-oxendict spelling.
- Use `thiserror` for error enums (not `eyre` in library code).
- Use `serde-saphyr` for YAML deserialization (not `serde_yaml`).
- Caret requirements only for dependency versions.
- Edition 2024, nightly-2026-01-30 toolchain.
- This plan must not modify paths outside `src/schema/`, `src/lib.rs`,
  `src/main.rs`, `tests/`, `docs/`, and `Cargo.toml`.

## Tolerances (exception triggers)

- Scope: if implementation requires more than 15 new files or 2000 net lines
  of code, stop and escalate.
- Dependencies: the approved dependency set is serde, serde-saphyr, indexmap,
  thiserror (runtime) and rstest, rstest-bdd (dev). If a new dependency is
  required, stop and escalate.
- Iterations: if a test or lint failure persists after 5 attempts, stop and
  escalate.
- Ambiguity: the normative spec is `docs/theorem-file-specification.md`. If
  the spec is ambiguous on a point that materially affects deserialization,
  document the ambiguity in `Decision Log` and escalate.

## Risks

- Risk: `serde-saphyr` v0.0.17 may not support `from_multiple` or may behave
  unexpectedly with `deny_unknown_fields`.
  Severity: medium. Likelihood: low.
  Mitigation: prototype in Milestone 0. Fall back to manual YAML document
  splitting if `from_multiple` is absent.

- Risk: `rstest-bdd` v0.5.0 may not compile on nightly-2026-01-30.
  Severity: low. Likelihood: low.
  Mitigation: prototype in Milestone 0. Fall back to `rstest` parameterized
  tests with BDD-style naming.

- Risk: `deny_unknown_fields` and `#[serde(untagged)]` cannot coexist on the
  same container (known serde limitation).
  Severity: medium. Likelihood: certain.
  Mitigation: place `deny_unknown_fields` on inner structs, not on untagged
  enum containers. See Decision D1.

- Risk: `serde-saphyr` has no `Value` type; the spec skeleton references
  `serde_saphyr::Value` which does not exist.
  Severity: medium. Likelihood: certain.
  Mitigation: define a custom `TheoremValue` enum. See Decision D2.

## Progress

- [x] (2026-02-09) Write ExecPlan document.
- [x] (2026-02-09) Milestone 0: scaffold and de-risk.
- [x] (2026-02-09) Milestone 1: core schema types.
- [x] (2026-02-09) Milestone 2: identifier validation.
- [x] (2026-02-09) Milestone 3: multi-document loading.
- [x] (2026-02-09) Milestone 4: comprehensive tests (131 tests passing).
- [x] (2026-02-09) Milestone 5: documentation and finalization.

## Surprises & discoveries

- Observation: Clippy's `expect_used` lint is automatically suppressed in
  `#[test]` functions (via `allow-expect-in-tests`), so `#[expect]`
  annotations for it are unfulfilled and cause their own lint error.
  Evidence: `unfulfilled-lint-expectations` error when `#[expect(clippy::
  expect_used)]` was placed on test functions.
  Impact: use `.expect()` freely in `#[test]` functions without annotation.

- Observation: `rstest-bdd` v0.5.0 lacks concrete usage examples in its
  API documentation. The step registration and scenario runner API is
  higher-level than needed for simple deserialization acceptance tests.
  Evidence: docs.rs page for rstest-bdd 0.5.0 lists macros and types but
  no runnable examples.
  Impact: used `rstest` parameterized tests with BDD-style naming instead.

- Observation: `missing_crate_level_docs` has been renamed to
  `rustdoc::missing_crate_level_docs` in current nightly.
  Evidence: warning during `cargo doc --no-deps`.
  Impact: moved lint configuration from `[lints.rust]` to
  `[lints.rustdoc]` section in `Cargo.toml`.

## Decision log

- D1: `deny_unknown_fields` + `untagged` enum conflict.
  `LetBinding` and `Step` use `#[serde(untagged)]`. Serde does not allow
  `deny_unknown_fields` on untagged enum containers. Place
  `deny_unknown_fields` on inner structs (`ActionCall`, `MaybeBlock`) instead.
  The single-key variant structure (`call:`, `must:`, `maybe:`) is validated
  structurally by serde's untagged matching.
  Date: 2026-02-09.

- D2: custom `TheoremValue` instead of `serde_json::Value`.
  `serde-saphyr` has no `Value` type. The spec notes "a wrapper around
  `serde_saphyr::Value` is likely useful so a project-specific `Value` enum
  can enforce 'no nulls'". We define `TheoremValue` with variants
  `Bool(bool)`, `Integer(i64)`, `Float(f64)`, `String(String)`,
  `Sequence(Vec<TheoremValue>)`, `Mapping(IndexMap<String, TheoremValue>)`.
  This avoids pulling in `serde_json` for a YAML-only project and enforces
  no-null at the type level.
  Date: 2026-02-09.

- D3: `KaniExpectation` as a proper enum.
  The spec lists exactly four valid values for `Evidence.kani.expect`. Model
  as `enum KaniExpectation { Success, Failure, Unreachable, Undetermined }`
  with `#[serde(rename = "SUCCESS")]` etc. This catches invalid values at
  deserialization time rather than needing post-hoc string validation.
  Date: 2026-02-09.

- D4: module location `src/schema/`.
  Step 1.1 is purely deserialization. The design doc's eventual `parser/`
  module is for the full parsing + validation pipeline (Step 1.2+). For now,
  `src/schema/` is clearer and can be moved later.
  Date: 2026-02-09.

- D5: `ActionCall.as` field naming.
  `as` is a Rust keyword. Use field name `as_binding` with
  `#[serde(rename = "as", default)]`.
  Date: 2026-02-09.

- D6: library crate alongside binary.
  Create `src/lib.rs` so schema types are testable as a library. `src/main.rs`
  remains the binary entry point.
  Date: 2026-02-09.

- D7: rstest-bdd v0.5.0.
  Available on crates.io (released 2026-02-06). Prototype in Milestone 0 to
  confirm toolchain compatibility. Fallback: `rstest` parameterized tests.
  Date: 2026-02-09.

## Outcomes & retrospective

All milestones completed successfully. The implementation delivers:

- 11 schema types (`TheoremDoc`, `Assumption`, `Assertion`, `WitnessCheck`,
  `LetBinding`, `Step`, `MaybeBlock`, `ActionCall`, `Evidence`,
  `KaniEvidence`, `KaniExpectation`) plus `TheoremValue` and `SchemaError`.
- Multi-document loading via `serde_saphyr::from_multiple`.
- Identifier validation (ASCII pattern + Rust keyword rejection).
- 131 tests: 33 unit (identifier + loader), 65 parameterized BDD-style
  (rstest), 33 integration tests (fixture-based).
- 14 fixture files (5 valid, 9 invalid).
- All quality gates pass: `make check-fmt`, `make lint`, `make test`.

Lessons learned:

- Serde's `deny_unknown_fields` restriction with `untagged` enums is a
  known limitation. Placing the annotation on inner structs is a reliable
  workaround that preserves the spirit of strict deserialization.
- `serde-saphyr` is solid for YAML deserialization but requires a custom
  `Value` type; this is a one-time cost and the resulting type is better
  suited to the project's needs (no-null enforcement).
- Nightly Rust's aggressive clippy lints (cognitive complexity, shadow,
  indexing) require careful test structuring — split assertions across
  functions to stay under complexity thresholds.

## Context and orientation

The `theoremc` project is a Rust-based formal verification framework. It
compiles human-readable `.theorem` YAML files into Kani proof harnesses. The
repository is currently a skeleton: a single `src/main.rs` stub printing
"Hello from Theorem Compiler!", a `Cargo.toml` with aggressive lints but no
dependencies, and extensive design documentation in `docs/`.

Key reference documents:

- `docs/theorem-file-specification.md` — normative schema spec (TFS-1).
  Section 8 provides a pseudocode Rust struct skeleton.
- `docs/theoremc-design.md` — architecture and design (DES-6: parsing and
  validation).
- `docs/roadmap.md` — phased implementation plan. Step 1.1 is the first
  checkbox.
- `AGENTS.md` — coding standards and quality gates.

The `.theorem` file format is YAML. A single file may contain one or more YAML
documents separated by `---`. Each document describes one theorem with
sections: `Theorem`, `About`, `Tags`, `Given`, `Forall`, `Assume`, `Witness`,
`Let`, `Do`, `Prove`, and `Evidence`. Keys use TitleCase canonically but
lowercase aliases are accepted.

Toolchain: Rust nightly-2026-01-30 (edition 2024). Build: `make check-fmt`,
`make lint`, `make test`.

## Plan of work

### Milestone 0: scaffold and de-risk

Add production and dev dependencies to `Cargo.toml`. Create `src/lib.rs` with
a placeholder module declaration. Confirm that `serde-saphyr` compiles and
that its multi-document API (`from_multiple` or equivalent) works. Confirm
that `rstest-bdd` v0.5.0 compiles on the project toolchain. Run quality gates
to confirm the scaffold is clean.

Dependencies to add:

    [dependencies]
    serde = { version = "1", features = ["derive"] }
    serde-saphyr = "0.0.17"
    indexmap = { version = "2", features = ["serde"] }
    thiserror = "2"

    [dev-dependencies]
    rstest = "0.26"
    rstest-bdd = "0.5.0"

### Milestone 1: core schema types

Create the `src/schema/` module tree:

`src/schema/error.rs` — define `SchemaError` using `thiserror::Error`:

    #[derive(Debug, thiserror::Error)]
    pub enum SchemaError {
        #[error("YAML deserialization failed: {0}")]
        Deserialize(String),
        #[error("invalid identifier '{identifier}': {reason}")]
        InvalidIdentifier { identifier: String, reason: String },
    }

`src/schema/value.rs` — define `TheoremValue`:

    #[derive(Debug, Clone, PartialEq)]
    pub enum TheoremValue {
        Bool(bool),
        Integer(i64),
        Float(f64),
        String(String),
        Sequence(Vec<TheoremValue>),
        Mapping(IndexMap<String, TheoremValue>),
    }

With a hand-written `impl<'de> serde::Deserialize<'de> for TheoremValue` that
walks the YAML value types and rejects null.

`src/schema/types.rs` — define all schema structs exactly following
`docs/theorem-file-specification.md` section 8, with these serde attributes:

- `TheoremDoc`: `#[serde(deny_unknown_fields)]` with `rename`/`alias` on each
  field for TitleCase + lowercase.
- `Assumption`, `Assertion`, `WitnessCheck`: `#[serde(deny_unknown_fields)]`.
- `LetBinding`: `#[serde(untagged)]` enum (no `deny_unknown_fields` on the
  enum itself). Variants: `Call { call: ActionCall }`,
  `Must { must: ActionCall }`.
- `Step`: `#[serde(untagged)]` enum. Variants: `Call { call: ActionCall }`,
  `Must { must: ActionCall }`, `Maybe { maybe: MaybeBlock }`.
- `MaybeBlock`: `#[serde(deny_unknown_fields)]` with `#[serde(rename = "do")]`
  for the `do_steps` field.
- `ActionCall`: `#[serde(deny_unknown_fields)]` with
  `#[serde(rename = "as", default)]` for `as_binding`.
- `Evidence`: `#[serde(deny_unknown_fields)]` with optional `kani`, `verus`,
  `stateright` fields.
- `KaniEvidence`: `#[serde(deny_unknown_fields)]`.
- `KaniExpectation`: enum with `#[serde(rename = "SUCCESS")]` etc.

`src/schema/mod.rs` — declare submodules (`error`, `value`, `types`,
`identifier`, `loader`) and re-export public API.

`src/lib.rs` — declare `pub mod schema`.

### Milestone 2: identifier validation

`src/schema/identifier.rs` — define:

- `is_valid_identifier(s: &str) -> bool`: regex match for
  `^[A-Za-z_][A-Za-z0-9_]*$`.
- `is_rust_keyword(s: &str) -> bool`: check against the complete Rust keyword
  list from the language reference.
- `validate_identifier(s: &str) -> Result<(), SchemaError>`: combines both
  checks.

Integrate validation into the loading path so `TheoremDoc.theorem` and
`Forall` map keys are validated after deserialization.

### Milestone 3: multi-document loading

`src/schema/loader.rs` — define:

    pub fn load_theorem_docs(
        input: &str,
    ) -> Result<Vec<TheoremDoc>, SchemaError>

Uses `serde_saphyr::from_multiple::<TheoremDoc>(input)` (or equivalent) to
handle `---`-separated documents. After deserialization, runs identifier
validation on theorem names and Forall keys.

If `from_multiple` does not exist in serde-saphyr, manually split on YAML
document boundaries (`---` lines) and call `serde_saphyr::from_str` on each.

### Milestone 4: comprehensive tests

Create test fixtures under `tests/fixtures/`:

- `valid_minimal.theorem` — smallest valid document (Theorem, About, Prove,
  Evidence with Kani and Witness).
- `valid_full.theorem` — document using all sections.
- `valid_multi.theorem` — two documents separated by `---`.
- `valid_lowercase.theorem` — all keys in lowercase aliases.
- `invalid_unknown_key.theorem` — document with an unrecognized top-level key.
- `invalid_wrong_type.theorem` — `Tags: foo` instead of `Tags: [foo]`.
- `invalid_missing_theorem.theorem` — missing required `Theorem` field.
- `invalid_keyword_name.theorem` — theorem name is a Rust keyword.
- `invalid_bad_identifier.theorem` — theorem name with invalid characters.

Integration tests in `tests/`:

- `tests/schema_deser.rs` — happy-path tests loading valid fixtures and
  asserting struct contents.
- `tests/schema_unhappy.rs` — unhappy-path tests loading invalid fixtures and
  asserting error messages.
- `tests/multi_document.rs` — multi-document loading tests.

Behavioural tests using `rstest-bdd` (or `rstest` parameterized fallback):

- Given/When/Then scenarios for key acceptance criteria:
  - "Given a theorem file with an unknown key, when loaded, then
    deserialization fails with an error mentioning the unknown key."
  - "Given a theorem file with lowercase aliases, when loaded, then
    deserialization succeeds identically to TitleCase."
  - "Given a theorem name that is a Rust keyword, when loaded, then
    validation rejects it."

Unit tests within `src/schema/identifier.rs`:

- Valid identifiers: `"Foo"`, `"_bar"`, `"Baz123"`.
- Invalid identifiers: `"123abc"`, `""`, `"foo-bar"`, `"foo bar"`.
- Rust keywords: `"fn"`, `"let"`, `"match"`, `"type"`.
- Non-keywords that look close: `"lets"`, `"types"`, `"True"`.

### Milestone 5: documentation and finalization

Create `docs/users-guide.md` with initial content covering the schema:
document structure, required and optional fields, key aliases, identifier
rules, and multi-document files. Include a minimal `.theorem` example.

Update `docs/roadmap.md`: change the first Step 1.1 checkbox from `[ ]` to
`[x]`.

Update `docs/theoremc-design.md`: add a subsection under §6 recording
decisions D1–D7.

Update `docs/contents.md`: add entries for `users-guide.md` and
`execplans/`.

Run final `make check-fmt && make lint && make test`.

## Concrete steps

All commands run from `/home/user/project`.

Milestone 0:

    mkdir -p docs/execplans
    # Write ExecPlan file (this document)
    # Edit Cargo.toml to add dependencies
    # Create src/lib.rs
    make check-fmt && make lint && make test

Milestone 1:

    mkdir -p src/schema
    # Create error.rs, value.rs, types.rs, mod.rs
    # Update lib.rs
    make check-fmt && make lint && make test

Milestone 2:

    # Create identifier.rs
    # Wire validation into loader
    make check-fmt && make lint && make test

Milestone 3:

    # Create loader.rs
    make check-fmt && make lint && make test

Milestone 4:

    mkdir -p tests/fixtures
    # Create fixture .theorem files
    # Create test files
    make check-fmt && make lint && make test

Milestone 5:

    # Create/update documentation files
    make check-fmt && make lint && make test

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New tests cover: valid deserialization (happy),
  unknown keys (unhappy), wrong types (unhappy), missing fields (unhappy),
  lowercase aliases (happy), multi-document (happy), identifier validation
  (happy + unhappy), keyword rejection (unhappy).
- Lint: `make lint` passes with zero warnings (Clippy pedantic + custom
  denials).
- Format: `make check-fmt` passes.

Quality method:

    set -o pipefail
    make check-fmt 2>&1 | tee /tmp/check-fmt.log
    make lint 2>&1 | tee /tmp/lint.log
    make test 2>&1 | tee /tmp/test.log

Expected: all three commands exit 0.

## Idempotence and recovery

All steps are additive and re-runnable. No destructive operations. If a
milestone fails, fix the issue and re-run `make check-fmt && make lint &&
make test` from the repo root. The project can be reset to its initial state
via `git checkout -- .` (all changes are tracked).

## Artifacts and notes

Key file paths (all relative to repo root):

- `src/lib.rs` — library crate root
- `src/schema/mod.rs` — schema module root
- `src/schema/types.rs` — `TheoremDoc` and subordinate types
- `src/schema/value.rs` — `TheoremValue` enum
- `src/schema/identifier.rs` — identifier validation
- `src/schema/loader.rs` — multi-document loading
- `src/schema/error.rs` — error types
- `tests/schema_deser.rs` — deserialization tests
- `tests/schema_unhappy.rs` — rejection tests
- `tests/multi_document.rs` — multi-doc tests
- `tests/fixtures/*.theorem` — YAML fixtures

## Interfaces and dependencies

Production dependencies:

- `serde` 1.x with `derive` feature — serialization framework.
- `serde-saphyr` 0.0.17 — panic-free YAML deserializer.
- `indexmap` 2.x with `serde` feature — insertion-ordered maps for `Forall`,
  `Let`, and `ActionCall.args`.
- `thiserror` 2.x — ergonomic error enum derivation.

Dev dependencies:

- `rstest` 0.26.x — test fixtures and parameterization.
- `rstest-bdd` 0.5.0 — BDD-style behavioural tests.

Public API surface (in `theoremc::schema`):

    pub fn load_theorem_docs(input: &str)
        -> Result<Vec<TheoremDoc>, SchemaError>;

    pub struct TheoremDoc { /* TitleCase fields, see types.rs */ }
    pub struct Assumption { pub expr: String, pub because: String }
    pub struct Assertion { pub assert_expr: String, pub because: String }
    pub struct WitnessCheck { pub cover: String, pub because: String }
    pub enum LetBinding { Call { call: ActionCall },
                          Must { must: ActionCall } }
    pub enum Step { Call { call: ActionCall },
                    Must { must: ActionCall },
                    Maybe { maybe: MaybeBlock } }
    pub struct MaybeBlock { pub because: String,
                            pub do_steps: Vec<Step> }
    pub struct ActionCall { pub action: String,
                            pub args: IndexMap<String, TheoremValue>,
                            pub as_binding: Option<String> }
    pub struct Evidence { pub kani: Option<KaniEvidence>,
                          pub verus: Option<TheoremValue>,
                          pub stateright: Option<TheoremValue> }
    pub struct KaniEvidence { pub unwind: u32,
                              pub expect: KaniExpectation,
                              pub allow_vacuous: bool,
                              pub vacuity_because: Option<String> }
    pub enum KaniExpectation { Success, Failure, Unreachable, Undetermined }
    pub enum TheoremValue { Bool(bool), Integer(i64), Float(f64),
                            String(String), Sequence(Vec<TheoremValue>),
                            Mapping(IndexMap<String, TheoremValue>) }
    pub enum SchemaError { Deserialize(String),
                           InvalidIdentifier { identifier, reason },
                           ValidationFailed { theorem, reason } }

    pub fn validate_identifier(s: &str) -> Result<(), SchemaError>;
