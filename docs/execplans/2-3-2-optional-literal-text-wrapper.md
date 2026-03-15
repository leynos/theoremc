# Step 2.3.2: optional `{ literal: "text" }` wrapper and sentinel key rejection

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

After this change, theoremc recognizes `{ literal: "text" }` as a sentinel
wrapper that forces string literal interpretation, parallel to the existing
`{ ref: name }` wrapper that forces variable reference interpretation[^1]. A
theorem author who writes `args: { label: { literal: "graph" } }` gets
`ArgValue::Literal(LiteralValue::String("graph"))` -- exactly the same result
as writing `label: "graph"` directly, but with explicit intent documented in
the YAML source[^2].

Invalid literal wrappers (where the value is not a string, such as
`{ literal: 42 }` or `{ literal: true }`) are rejected with actionable error
messages at load time. The sentinel dispatch is unified: single-key maps whose
key is `"ref"` or `"literal"` are intercepted as wrappers; all other maps
(including multi-key maps that happen to contain `ref` or `literal` among
their keys) pass through as `ArgValue::RawMap` for future struct-literal
synthesis (Step 2.3.3)[^3].

Observable success: loading a `.theorem` file containing
`{ literal: "graph" }` produces
`ArgValue::Literal(LiteralValue::String("graph"))`. Loading a file containing
`{ literal: 42 }` fails with an error message containing `"literal value must
be a string"`. Running `make check-fmt`, `make lint`, and `make test` all
pass. The `docs/users-guide.md` reference to `{ literal: "text" }` is
upgraded from "(future)" to documented behaviour.

## Constraints

- ADR-003 boundary rule: domain types (public API) must not import from raw,
  validator, or loader modules. All changes stay within
  `src/schema/arg_value.rs` (domain layer) and `src/schema/raw_action.rs` (raw
  adapter layer). No new cross-boundary imports are introduced.
- Source files must not exceed 400 lines. Current line counts:
  `arg_value.rs` 243, `arg_value_tests.rs` 166, `raw_action.rs` 196,
  `arg_decode_bdd.rs` 276. There is headroom in all files.
- Use `rstest` for unit tests, `rstest-bdd` v0.5.0 (`rstest_bdd_macros`) for
  Behaviour-Driven Development (BDD) scenarios.
- Comments and documentation must use en-GB-oxendict spelling ("-ize" / "-yse"
  / "-our").
- No new external dependencies beyond what is already in `Cargo.toml`.
- Existing public API surfaces must remain additive-only. The new
  `ArgDecodeError::NonStringLiteralValue` variant is additive. The existing
  `is_ref_wrapper` function is private and can be replaced.
- The `SchemaError` enum's existing variants must not be removed or have their
  fields changed.
- Quality gates: `make check-fmt`, `make lint`, `make test`, and
  `make markdownlint` must pass.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 12 files or more than
  400 net lines of code, stop and escalate with a narrowed split.
- Dependencies: if a new external dependency beyond what is already in
  `Cargo.toml` is needed, stop and escalate.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop and escalate with logs.
- Ambiguity: if the treatment of a specific YAML value form is unclear in a
  concrete scenario, stop and present options.
- Interface: if any existing public API item must be removed (not extended),
  stop and escalate.

## Risks

- Risk: Adding the `NonStringLiteralValue` variant to `ArgDecodeError` is a
  public enum change. Any downstream consumer matching exhaustively will see a
  compile error.
  Severity: low.
  Likelihood: low (the enum already had 4 variants added in Step 2.3.1;
  consumers are expected to handle new variants).
  Mitigation: this is an additive change, consistent with Step 2.3.1
  precedent.

- Risk: The unified `classify_sentinel` refactoring accidentally changes
  behaviour for existing ref-wrapper edge cases.
  Severity: medium.
  Likelihood: low (the existing test suite has 16 unit tests and 5 BDD
  scenarios covering ref behaviour).
  Mitigation: all existing tests must pass before and after the refactoring.
  The refactoring is a structural change to `decode_mapping` internals only.

## Progress

- [x] Stage A: preflight verification (existing tests pass). 355 tests.
- [x] Stage B: add `NonStringLiteralValue` error variant and handle it in
  `remap_with_prefix`.
- [x] Stage C: add `LITERAL_KEY`, `SentinelKind`, `classify_sentinel`,
  `decode_literal_target`, and refactor `decode_mapping`.
- [x] Stage D: add unit tests for literal wrapper in `arg_value_tests.rs`.
  10 new unit tests (365 total).
- [x] Stage E: add BDD fixture files, scenarios, and step implementations.
  2 new BDD scenarios (367 total).
- [x] Stage F: update documentation (`users-guide.md`, `theoremc-design.md`,
  `roadmap.md`).
- [x] Stage G: final quality gate and commit. All 4 gates pass (367 tests).

## Surprises & discoveries

- Observation: Clippy rejects `.expect()` in production code due to the
  project's `clippy::expect_used` deny policy.
  Evidence: `make lint` failed on `map.into_values().next().expect(...)` in
  `decode_mapping`.
  Impact: used `let Some(value) = ... else { return ... }` pattern instead,
  consistent with the original `decode_mapping` implementation.

## Decision log

- Decision: Use a `SentinelKind` enum and `classify_sentinel` helper instead
  of adding a second `is_literal_wrapper` predicate alongside
  `is_ref_wrapper`.
  Rationale: a unified dispatch is cleaner, eliminates the chance of the two
  predicates falling out of sync, and makes it trivial to add future sentinel
  keys. The existing `is_ref_wrapper` function is private, so removing it has
  no API impact.
  Date/Author: 2026-03-05 / plan author.

- Decision: `{ literal: "" }` (empty string) is accepted as valid. Unlike
  `{ ref: "" }` which is rejected because empty is not a valid identifier, an
  empty string is a legitimate string literal.
  Rationale: the purpose of `{ literal: ... }` is to force string literal
  interpretation. Rejecting empty strings would be surprising to users.
  Date/Author: 2026-03-05 / plan author.

- Decision: Multi-key maps containing `literal` or `ref` as one of their keys
  pass through as `ArgValue::RawMap`, per TFS-5 section 5.3. The "reject
  ambiguous wrapper maps" acceptance criterion is satisfied by rejecting
  single-key `{ literal: <non-string> }` maps deterministically rather than
  silently passing them through as `RawMap`.
  Rationale: TFS-5 section 5.3 says only single-key sentinel maps are
  intercepted. Multi-key maps are always struct literal candidates. A map like
  `{ literal: 42 }` that looks like it intends to be a literal wrapper but has
  the wrong value type must not silently become a struct literal candidate --
  it must be an error.
  Date/Author: 2026-03-05 / plan author.

- Decision: Name the new error variant `NonStringLiteralValue` (not
  `NonStringLiteralTarget`).
  Rationale: semantic accuracy -- the `literal` wrapper carries a "value"
  while the `ref` wrapper carries a "target" (an identifier name).
  Date/Author: 2026-03-05 / plan author.

## Outcomes & retrospective

Implementation completed successfully. All acceptance criteria met:

- `{ literal: "text" }` wrappers are recognized and decoded as
  `ArgValue::Literal(LiteralValue::String(...))`.
- Invalid literal wrappers (`{ literal: 42 }`, `{ literal: true }`, etc.)
  produce actionable `NonStringLiteralValue` errors.
- Multi-key maps containing `literal` pass through as `ArgValue::RawMap`.
- The unified `classify_sentinel` dispatch replaced `is_ref_wrapper` cleanly;
  all 16 existing ref-related unit tests continue to pass.
- 12 new tests total (10 unit + 2 BDD), bringing the count from 355 to 367.
- `docs/users-guide.md` upgraded `{ literal: "text" }` from "(future)" to
  documented behaviour.
- Roadmap Step 2.3.2 checkbox marked as done.
- The roadmap's original "reject ambiguous wrapper maps containing unsupported
  sentinel keys" wording was clarified during review. The rejection criterion
  applies to sentinel wrappers with invalid value types (e.g.
  `{ literal: 42 }`), not to unrecognized single-key maps (e.g.
  `{ frobnicate: "value" }`). Unrecognized single-key maps pass through as
  `ArgValue::RawMap` — struct-literal candidates per TFS-5 §5.3. The roadmap
  wording and `classify_sentinel` doc comment were updated to reflect this
  design decision.

Lesson learned: the `clippy::expect_used` deny policy requires using
`let Some(...) = ... else { return ... }` instead of `.expect()` even in
cases where the precondition is provably satisfied. This is consistent with
the project's zero-panic policy.

## Context and orientation

The `theoremc` library parses `.theorem` files (YAML) into strongly-typed Rust
structures. Action calls within theorems have `args` maps where each value is
decoded from raw YAML (`TheoremValue`) into a semantic `ArgValue`. The
decoding logic lives in `src/schema/arg_value.rs`.

Key files and their roles (pre-change baseline — see stages B–D for
the changes introduced by this plan):

`src/schema/arg_value.rs` (243 lines) is the core module. It contains:

- `ArgDecodeError` enum (4 variants: `EmptyRefTarget`, `InvalidIdentifier`,
  `ReservedKeyword`, `NonStringRefTarget`). Each variant carries a `param:
  String` field for diagnostic context.
- `ArgValue` enum (4 variants: `Literal`, `Reference`, `RawSequence`,
  `RawMap`).
- `LiteralValue` enum (4 variants: `Bool`, `Integer`, `Float`, `String`).
- `decode_arg_value(param_name, TheoremValue) -> Result<ArgValue,
  ArgDecodeError>` is the public entry point. It dispatches scalars to
  `Literal`, sequences to `RawSequence`, and mappings to the private
  `decode_mapping` function.
- `decode_mapping(param_name, map)` is private. It checks `is_ref_wrapper` and
  either validates the ref target or returns `ArgValue::RawMap`.
- `is_ref_wrapper(map)` is a private predicate checking
  `map.len() == 1 && map.contains_key("ref")`.
- `decode_ref_target(param_name, value)` is private. It validates that the
  value is a non-empty string matching `^[A-Za-z_][A-Za-z0-9_]*$` and not a
  Rust keyword.
- `non_string_kind(value)` is a private const fn returning human-readable kind
  labels for error messages.
- `const REF_KEY: &str = "ref"` is the existing sentinel key constant.

`src/schema/arg_value_tests.rs` (166 lines) contains 16 unit tests included
via the `#[path]` attribute. Tests cover scalar decoding, valid ref decoding,
invalid ref decoding (empty, keyword, invalid identifier, non-string), and
pass-through forms (single-key non-ref map, multi-key map with ref, empty
map, sequence).

`src/schema/raw_action.rs` (196 lines) contains raw serde types and
conversion functions. The `remap_with_prefix` function (line 177) exhaustively
matches all 4 `ArgDecodeError` variants to prepend breadcrumb context for
nested `maybe.do` steps. Any new variant added to `ArgDecodeError` must be
handled in this function, or Clippy's exhaustive-match lint will fail
compilation.

`src/schema/raw.rs` (391 lines) contains `RawDocDecodeError` which wraps
`ArgDecodeError` as `#[source]`. This does not need modification because it
wraps `ArgDecodeError` generically.

`src/schema/mod.rs` re-exports `ArgDecodeError`, `ArgValue`, `LiteralValue`
from `arg_value`. No changes needed because the new error variant is part of
the existing `ArgDecodeError` enum.

`tests/arg_decode_bdd.rs` (276 lines) contains 5 BDD scenarios with helpers
`load_ok`, `load_err`, `first_let_args`, `first_do_arg`, and
`assert_is_string_literal`.

`tests/features/arg_decode.feature` (25 lines) is the Gherkin feature file.

`tests/fixtures/` contains YAML fixture files used by BDD tests.

`docs/users-guide.md` (442 lines) has a "Value forms in arguments" section
starting at line 244. Line 298 lists `{ literal: "text" }` as "explicit string
literal (future)".

`docs/theoremc-design.md` contains section 6.7.6 "Implementation decisions
(Step 2.3.1)" at line 857. The new section 6.7.9 for Step 2.3.2 will be
inserted after the Step 2.2.2 section (6.7.8).

`docs/roadmap.md` has the Step 2.3.2 checkbox at line 239.

## Plan of work

### Stage A: preflight verification

Run `make check-fmt`, `make lint`, and `make test` to confirm the working tree
is clean and all existing tests pass. This establishes the baseline.

### Stage B: add new error variant and breadcrumb handling

In `src/schema/arg_value.rs`, add a new variant to the `ArgDecodeError` enum
after the existing `NonStringRefTarget` variant (line 65):

```rust
/// The `literal` value is not a string (e.g. an integer or boolean).
#[error(
    "argument '{param}': literal value must be a string, \
     not {kind}"
)]
NonStringLiteralValue {
    /// Argument parameter name.
    param: String,
    /// Human-readable kind label (e.g. "an integer").
    kind: &'static str,
},
```

In `src/schema/raw_action.rs`, add a new match arm in `remap_with_prefix`
after the `NonStringRefTarget` arm (after line 193):

```rust
ArgDecodeError::NonStringLiteralValue { param, kind } => {
    ArgDecodeError::NonStringLiteralValue {
        param: format!("{prefix}: {param}"),
        kind,
    }
}
```

Validation: `make lint` and `make test` pass with no regressions.

### Stage C: unified sentinel dispatch and literal decoding

In `src/schema/arg_value.rs`, make the following changes:

**Add the literal key constant** after the existing `REF_KEY` (after
line 17):

```rust
/// The sentinel YAML map key that identifies an explicit string literal.
const LITERAL_KEY: &str = "literal";
```

**Add a private `SentinelKind` enum** (after the constants, before
`ArgDecodeError`):

```rust
/// Discriminates recognized sentinel map keys for dispatch.
enum SentinelKind {
    /// The `{ ref: <Identifier> }` sentinel.
    Ref,
    /// The `{ literal: <String> }` sentinel.
    Literal,
}
```

**Replace `is_ref_wrapper`** (lines 191-193) with a unified
`classify_sentinel` function:

```rust
/// Classifies a single-key map as a recognized sentinel wrapper, or
/// returns `None` for maps that should pass through as struct literal
/// candidates.
fn classify_sentinel(map: &IndexMap<String, TheoremValue>) -> Option<SentinelKind> {
    if map.len() != 1 {
        return None;
    }
    let key = map.keys().next()?;
    match key.as_str() {
        REF_KEY => Some(SentinelKind::Ref),
        LITERAL_KEY => Some(SentinelKind::Literal),
        _ => None,
    }
}
```

**Add `decode_literal_target`** after `decode_ref_target`:

```rust
/// Validates the `literal` target value and produces an
/// `ArgValue::Literal(LiteralValue::String(...))`.
fn decode_literal_target(
    param_name: &str,
    value: TheoremValue,
) -> Result<ArgValue, ArgDecodeError> {
    let TheoremValue::String(s) = value else {
        return Err(ArgDecodeError::NonStringLiteralValue {
            param: param_name.to_owned(),
            kind: non_string_kind(&value),
        });
    };
    Ok(ArgValue::Literal(LiteralValue::String(s)))
}
```

Unlike `decode_ref_target`, there is no empty-string check, no identifier
validation, and no keyword check. An empty string is a valid string literal.

**Rewrite `decode_mapping`** (lines 174-188) to use the unified dispatch:

```rust
/// Decodes a YAML mapping into a sentinel wrapper (`Reference` or
/// `Literal`) if the map has exactly one recognized sentinel key, or a
/// `RawMap` for all other maps (struct literal candidates).
fn decode_mapping(
    param_name: &str,
    map: IndexMap<String, TheoremValue>,
) -> Result<ArgValue, ArgDecodeError> {
    let Some(kind) = classify_sentinel(&map) else {
        return Ok(ArgValue::RawMap(map));
    };

    // `classify_sentinel` confirmed exactly one key, so the iterator
    // always yields a value.
    let Some(value) = map.into_values().next() else {
        return Ok(ArgValue::RawMap(IndexMap::new()));
    };
    match kind {
        SentinelKind::Ref => decode_ref_target(param_name, value),
        SentinelKind::Literal => decode_literal_target(param_name, value),
    }
}
```

**Update doc-comments:**

- Module-level doc (line 1-9): mention `{ literal: <String> }` maps.
- `ArgValue::RawMap` doc (line 92-94): remove "`{ literal: ... }` wrapper
  recognition" from the "future" note.
- `LiteralValue::String` doc (line 118-120): change from "in future steps"
  to present tense.
- `decode_arg_value` doc (lines 123-159): add bullet points for the literal
  wrapper decoding rules and errors.

Validation: `make check-fmt`, `make lint`, and `make test` pass. All 16
existing unit tests and 5 BDD scenarios still pass.

### Stage D: unit tests

In `src/schema/arg_value_tests.rs`, add the following after the "Pass-through
forms" section (after line 144):

A `// -- Literal wrapper decoding` section with:

- `valid_literal_wrapper_decodes_as_string_literal` -- rstest parametric with
  cases: `"hello"`, `""` (empty string), `"  spaces  "` (whitespace). Each
  constructs `IndexMap::from([("literal", TheoremValue::String(input))])` and
  asserts `ArgValue::Literal(LiteralValue::String(expected))`.

- `literal_with_non_string_value_is_rejected` -- rstest parametric with cases:
  `Integer(42)` / `"an integer"`, `Bool(true)` / `"a boolean"`,
  `Float(1.0)` / `"a float"`, `Sequence(vec![...])` / `"a sequence"`,
  `Mapping(IndexMap::from([...]))` / `"a mapping"`. Each asserts
  `ArgDecodeError::NonStringLiteralValue`.

- `multi_key_map_with_literal_is_raw_map` -- single test confirming a two-key
  map containing `"literal"` passes through as `ArgValue::RawMap`.

- `literal_error_message_includes_param_name` -- single test confirming the
  `Display` output of `NonStringLiteralValue` contains the parameter name.

Validation: `make test` passes with increased test count.

### Stage E: BDD fixture files, scenarios, and step implementations

Create two fixture files in `tests/fixtures/`:

`tests/fixtures/valid_arg_literal_wrapper.theorem`:

```yaml
Theorem: ArgLiteralWrapper
About: Explicit literal wrapper is decoded as a string literal
Let:
  result:
    call:
      action: label.set
      args:
        label: { literal: "graph" }
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
```

`tests/fixtures/invalid_arg_literal_non_string.theorem`:

```yaml
Theorem: ArgLiteralNonString
About: Non-string literal wrapper value is rejected
Let:
  result:
    call:
      action: label.set
      args:
        label: { literal: 42 }
Prove:
  - assert: 'true'
    because: trivially true
Evidence:
  kani:
    unwind: 1
    expect: SUCCESS
Witness:
  - cover: 'true'
    because: always reachable
```

In `tests/features/arg_decode.feature`, append two new scenarios:

```plaintext
  Scenario: Explicit literal wrapper is decoded as string literal
    Given a theorem file with an explicit literal wrapper
    Then loading succeeds and the argument is a string literal

  Scenario: Non-string literal wrapper value is rejected
    Given a theorem file with a non-string literal wrapper value
    Then loading fails with a literal type error message
```

In `tests/arg_decode_bdd.rs`, add step implementations and scenario wiring
following the existing pattern: `#[given("...")]` no-op functions,
`#[then("...")]` functions returning `Result<(), String>`, and
`#[scenario(path = "...", name = "...")]` wiring functions.

The "decoded as string literal" then-step loads
`valid_arg_literal_wrapper.theorem`, extracts the `"label"` arg from the
first Let binding, and asserts it equals
`ArgValue::Literal(LiteralValue::String("graph".into()))`.

The "rejected" then-step loads `invalid_arg_literal_non_string.theorem` and
asserts the error string contains `"literal value must be a string"`.

Validation: `make test` passes with new BDD scenarios included.

### Stage F: documentation updates

In `docs/users-guide.md`:

- Update the `ArgValue::RawMap` bullet (line 264-265) to remove the
  "`{ literal: ... }` wrapper" from the future note.
- After the "Invalid reference targets" section (around line 287), add a
  paragraph documenting the literal wrapper with examples of valid use and
  rejection messages.
- Update the "Supported YAML value forms" summary (line 298) to remove
  "(future)" from the `{ literal: "text" }` entry.
- Add `{ literal: "graph" }` to the YAML examples block alongside the existing
  `{ ref: graph }` example.

In `docs/theoremc-design.md`:

- Insert a new subsection `### 6.7.9 Implementation decisions (Step 2.3.2)`
  before section 6.8 (line 891), documenting: the `NonStringLiteralValue`
  error variant, the unified `classify_sentinel` dispatch replacing
  `is_ref_wrapper`, empty string acceptance in `{ literal: "" }`, and
  multi-key map pass-through behaviour.

In `docs/roadmap.md`:

- Change the Step 2.3.2 checkbox at line 239 from `[ ]` to `[x]`.

Validation: `make markdownlint` passes.

### Stage G: final quality gate and commit

Run all quality gates:

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/check-fmt-final.log
set -o pipefail; make lint 2>&1 | tee /tmp/lint-final.log
set -o pipefail; make test 2>&1 | tee /tmp/test-final.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/markdownlint-final.log
```

All must exit 0.

## Concrete steps

All commands are run from `/home/user/project`.

Preflight:

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/check-fmt-pre.log
set -o pipefail; make lint 2>&1 | tee /tmp/lint-pre.log
set -o pipefail; make test 2>&1 | tee /tmp/test-pre.log
```

After Stage C (implementation, before new tests):

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/check-fmt-impl.log
set -o pipefail; make lint 2>&1 | tee /tmp/lint-impl.log
set -o pipefail; make test 2>&1 | tee /tmp/test-impl.log
```

After Stage D+E (all tests added):

```bash
set -o pipefail; make test 2>&1 | tee /tmp/test-full.log
```

After Stage F (documentation):

```bash
set -o pipefail; make markdownlint 2>&1 | tee /tmp/markdownlint.log
```

Final gate (Stage G):

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/check-fmt-final.log
set -o pipefail; make lint 2>&1 | tee /tmp/lint-final.log
set -o pipefail; make test 2>&1 | tee /tmp/test-final.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/markdownlint-final.log
```

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New unit tests cover: valid literal wrappers
  (including empty string), non-string literal values (integer, boolean, float,
  sequence, mapping), multi-key maps containing `literal` key, and error
  message formatting. New BDD scenarios cover: valid literal wrapper fixture
  loading, and invalid literal wrapper rejection.
- Lint/typecheck: `make check-fmt` and `make lint` both exit 0.
- Documentation: `make markdownlint` exits 0. The `{ literal: "text" }` entry
  in `docs/users-guide.md` is no longer marked "(future)".
- Roadmap: the Step 2.3.2 checkbox in `docs/roadmap.md` is checked.

Quality method:

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/gate-fmt.log
set -o pipefail; make lint 2>&1 | tee /tmp/gate-lint.log
set -o pipefail; make test 2>&1 | tee /tmp/gate-test.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/gate-md.log
```

## Idempotence and recovery

All stages are idempotent. The refactoring of `decode_mapping` replaces the
existing function body entirely, so partial application is not a concern. If
a stage fails partway through, fix the issue and re-run the stage's validation
commands. If the implementation is abandoned, `git checkout -- .` restores the
working tree.

## Artefacts and notes

Expected error message for `{ literal: 42 }`:

```plaintext
argument 'label': literal value must be a string, not an integer
```

Expected `decode_mapping` after refactoring:

```rust
fn decode_mapping(
    param_name: &str,
    map: IndexMap<String, TheoremValue>,
) -> Result<ArgValue, ArgDecodeError> {
    let Some(kind) = classify_sentinel(&map) else {
        return Ok(ArgValue::RawMap(map));
    };

    // `classify_sentinel` confirmed exactly one key, so the iterator
    // always yields a value.
    let Some(value) = map.into_values().next() else {
        return Ok(ArgValue::RawMap(IndexMap::new()));
    };
    match kind {
        SentinelKind::Ref => decode_ref_target(param_name, value),
        SentinelKind::Literal => decode_literal_target(param_name, value),
    }
}
```

Expected `classify_sentinel`:

```rust
fn classify_sentinel(
    map: &IndexMap<String, TheoremValue>,
) -> Option<SentinelKind> {
    if map.len() != 1 {
        return None;
    }
    let key = map.keys().next()?;
    match key.as_str() {
        REF_KEY => Some(SentinelKind::Ref),
        LITERAL_KEY => Some(SentinelKind::Literal),
        _ => None,
    }
}
```

Expected `decode_literal_target`:

```rust
fn decode_literal_target(
    param_name: &str,
    value: TheoremValue,
) -> Result<ArgValue, ArgDecodeError> {
    let TheoremValue::String(s) = value else {
        return Err(ArgDecodeError::NonStringLiteralValue {
            param: param_name.to_owned(),
            kind: non_string_kind(&value),
        });
    };
    Ok(ArgValue::Literal(LiteralValue::String(s)))
}
```

## Interfaces and dependencies

No new external dependencies. All changes use existing crate dependencies
(`indexmap`, `thiserror`, `rstest`, `rstest_bdd_macros`).

New public interface additions (additive only):

In `src/schema/arg_value.rs`, the `ArgDecodeError` enum gains one new variant:

```rust
/// The `literal` value is not a string (e.g. an integer or boolean).
#[error(
    "argument '{param}': literal value must be a string, \
     not {kind}"
)]
NonStringLiteralValue {
    /// Argument parameter name.
    param: String,
    /// Human-readable kind label (e.g. "an integer").
    kind: &'static str,
},
```

New private interfaces:

```rust
const LITERAL_KEY: &str = "literal";

enum SentinelKind { Ref, Literal }

fn classify_sentinel(map: &IndexMap<String, TheoremValue>) -> Option<SentinelKind>;
fn decode_literal_target(param_name: &str, value: TheoremValue) -> Result<ArgValue, ArgDecodeError>;
```

Removed private interfaces:

```rust
fn is_ref_wrapper(map: &IndexMap<String, TheoremValue>) -> bool;
```

This function is replaced by `classify_sentinel` which provides unified
sentinel dispatch.

Files modified (11 total):

1. `src/schema/arg_value.rs` -- core logic changes
2. `src/schema/arg_value_tests.rs` -- new unit tests
3. `src/schema/raw_action.rs` -- new `remap_with_prefix` match arm
4. `tests/arg_decode_bdd.rs` -- new BDD scenarios
5. `tests/features/arg_decode.feature` -- new Gherkin scenarios
6. `tests/fixtures/valid_arg_literal_wrapper.theorem` -- new fixture (create)
7. `tests/fixtures/invalid_arg_literal_non_string.theorem` -- new fixture
   (create)
8. `docs/users-guide.md` -- documentation updates
9. `docs/theoremc-design.md` -- new section 6.7.9
10. `docs/roadmap.md` -- check Step 2.3.2 checkbox
11. `docs/execplans/2-3-2-optional-literal-text-wrapper.md` -- this ExecPlan
    (create)

[^1]: `TFS-5` (theorem-file-specification.md sections 5.2 and 5.3)
[^2]: `ADR-3` (adr-001-theorem-symbol-stability-and-non-vacuity-policy.md
    decision 3 -- "Literal wrappers remain supported")
[^3]: `DES-5` (theoremc-design.md section 5.5.1)
