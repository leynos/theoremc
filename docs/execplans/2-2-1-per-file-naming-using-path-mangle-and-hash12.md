# Step 2.2.1: per-file module naming using path_mangle and hash12

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Implement the first acceptance item of Roadmap Phase 2, Step 2.2: per-file
module naming using `path_mangle(path_stem(P))` and `hash12(P)`.

After this change, library consumers can transform a `.theorem` file path into
a deterministic, collision-resistant Rust module name of the form:

```plaintext
__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}
```

This module name is stable across builds, human-recognizable, and injective for
practical inputs because the 12-character blake3 hash suffix disambiguates
paths that sanitize to the same mangled stem (e.g., `my-file.theorem` and
`my_file.theorem` both mangle to `my_file` but have different hashes).

Observable success:

- `theoremc::mangle::mangle_module_path("theorems/bidirectional.theorem")`
  returns a `MangledModule` with a deterministic module name starting with
  `__theoremc__file__` and ending with a 12-character hex hash.
- Snapshot tests confirm deterministic names for mixed separators,
  punctuation-heavy paths, digit-leading stems, deeply nested directories, and
  Unicode edge cases.
- Behaviour-driven development (BDD) scenarios cover the happy path, separator
  normalization, and punctuation-heavy collision disambiguation.
- `make check-fmt`, `make lint`, and `make test` pass.
- `docs/theoremc-design.md`, `docs/users-guide.md`, and the relevant roadmap
  checkbox are updated.

This fulfils signposts `NMR-1`, `ADR-2`, and `DES-7` for the per-file module
naming slice. Harness naming and theorem-key collision detection remain
follow-up steps (roadmap items 2.2.2 and 2.2.3).

## Constraints

- Scope is limited to roadmap item 2.2.1 (per-file module naming). Do not
  implement harness naming (2.2.2) or theorem-key collision detection (2.2.3)
  in this change.
- Do not modify the existing `schema` module or its public API. The new
  functions extend the existing `mangle` module.
- The `collision` module was updated to use `CanonicalActionName` newtypes
  introduced during a follow-up refactoring pass.
- All mangling functions assume the input path `P` is the literal path string
  that would be passed to a future `theorem_file!("P")` macro invocation. No
  filesystem access or canonicalization is performed; the input is a plain
  string.
- The `path_mangle` algorithm must follow the specification in
  `docs/name-mangling-rules.md` §1 exactly:
  1. Replace `/` and `\` with `__`.
  2. Replace any character not in `[A-Za-z0-9_]` with `_`.
  3. Collapse consecutive `_` to a single `_`.
  4. Lowercase the result.
  5. If the result starts with a digit, prefix `_`.
- `path_stem(P)` removes a trailing `.theorem` extension if present;
  otherwise returns `P` unchanged.
- The existing `hash12` function (blake3, 12 lowercase hex chars) is reused
  directly.
- The generated module name format is:
  `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}`. Note: `hash12`
  receives the **original** path `P`, not the mangled stem. This is important
  because two paths that differ only in extension (or differ only in characters
  lost during mangling) must produce different hashes.
- Keep source files under 400 lines per the repository code-size rule.
- Use `rstest-bdd` v0.5.0 for behavioural coverage.
- Required quality gates: `make check-fmt`, `make lint`, `make test`.
- Documentation updates are required in: `docs/theoremc-design.md` (new
  §6.7.6), `docs/users-guide.md` (new "Per-file module naming" section), and
  `docs/roadmap.md` (mark first checkbox of Step 2.2 done).
- en-GB-oxendict spelling and grammar in all documentation and comments.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 12 files or more
  than 600 net lines, stop and escalate with a narrowed split.
- Interface: if a public API signature change to the `schema` or `collision`
  modules is required, stop and escalate.
- Dependencies: if a new external dependency (other than those already in
  `Cargo.toml`) is needed, stop and escalate.
- Iterations: if any quality gate fails more than 5 consecutive fix attempts,
  stop, and escalate with logs.
- Ambiguity: if the `path_mangle` specification leaves a case genuinely
  ambiguous (e.g., empty input), document the interpretation in `Decision Log`
  and proceed with the most defensive choice.

## Risks

- Risk: the collapse-consecutive-underscores rule in `path_mangle` makes it
  possible for two very different paths to produce the same mangled stem.
  Severity: low. Likelihood: medium. Mitigation: the `hash12(P)` suffix
  disambiguates. Unit tests explicitly cover such pairs (e.g., `my-file` vs
  `my_file`, `a/b` vs `a\b`).

- Risk: Clippy lint `string_slice = "deny"` or `indexing_slicing = "deny"`
  triggered by character manipulation. Severity: medium. Likelihood: medium.
  Mitigation: use iterator-based character processing and `String::push` /
  `String::push_str` rather than slice indexing. The existing `hash12` already
  handles this with `.get(..12).unwrap_or_default()`.

- Risk: the `path_mangle` step 3 (collapse consecutive underscores) interacts
  with step 1 (replace separators with `__`), potentially collapsing the double
  underscore to a single one. Severity: high. Likelihood: high. Mitigation: the
  spec says, "collapse consecutive `_` to a single `_`". This means `a/b`
  becomes `a__b` after step 1, then `a_b` after step 3. This is by design — the
  hash suffix provides the real disambiguator. Golden tests must verify the
  exact result.

- Risk: Clippy `cognitive_complexity` or `too_many_lines` triggered by the
  `path_mangle` function if logic is inlined. Severity: low. Likelihood: low.
  Mitigation: decompose into small helper functions for each step.

## Progress

- [x] Draft ExecPlan for Step 2.2.1.
- [x] Milestone 0: baseline verification (all existing tests pass).
- [x] Milestone 1: compute golden values for representative paths.
- [x] Milestone 2: implement `path_stem`, `path_mangle`, and
  `mangle_module_path` in `src/mangle.rs` with unit tests.
- [x] Milestone 3: add BDD feature file and test runner.
- [x] Milestone 4: update design docs, user's guide, and roadmap.
- [x] Milestone 5: run full quality gates and capture logs.

## Surprises & discoveries

- The `make fmt` markdown linter catches duplicate heading names across the
  entire document. "Building-block functions" was already used in the action
  mangling section, so the new per-file section required a distinct heading
  ("Path mangling functions").
- Existing tests extracted cleanly to `src/mangle_tests.rs` via the
  `#[path = ...]` pattern. The `Golden` struct needed renaming to
  `ActionGolden` to distinguish it from `ModuleGolden`.

## Decision log

- **2026-03-01:** `path_mangle` inlines all five steps in a single function
  body (three short loops). No decomposition was needed because the function
  stayed well within Clippy's cognitive-complexity threshold.
- **2026-03-01:** `MODULE_PREFIX` is a private `const` (not `pub`). Callers
  use `MangledModule::module_name()` rather than assembling the prefix
  themselves.
- **2026-03-01:** User's guide section renamed from "Building-block functions"
  to "Path mangling functions" to avoid duplicate-heading lint violation with
  the existing action mangling section.

## Outcomes & retrospective

Implementation completed with:

- 3 new public functions (`path_stem`, `path_mangle`, `mangle_module_path`)
  and 1 new public struct (`MangledModule`).
- 32 new unit tests and 3 BDD scenarios (total suite: 361 tests, 0 failures).
- No new dependencies; reuses existing `blake3`.
- All quality gates pass: `make check-fmt`, `make lint`, `make test`,
  `make markdownlint`.
- 7 files changed, 0 files deleted. Well within the 12-file tolerance.

## Context and orientation

The theoremc crate currently has three top-level modules:

- `src/schema/` — schema types, deserialization, validation, and diagnostics
  for `.theorem` documents.
- `src/mangle.rs` — action name mangling for deterministic, injective
  resolution of canonical action names into Rust identifiers.
- `src/collision.rs` — mangled-identifier collision detection across loaded
  theorem documents.

The `mangle` module already provides the building blocks this task reuses:

- `hash12(value: &str) -> String` — computes the first 12 lowercase hex
  characters of the blake3 digest of `value`.
- `segment_escape`, `action_slug`, `mangle_action_name` — action-level
  mangling (not directly reused here, but the module is the natural home for
  all mangling logic).

The normative specification for per-file module naming lives in
`docs/name-mangling-rules.md` §1 ("Per-file module name"). The algorithm is
also summarized in `docs/theorem-file-specification.md` §7.3.

### Key files

- `src/mangle.rs` — implementation target (extend with new functions).
- `src/lib.rs` — crate root (no changes needed; `pub mod mangle` already
  declared).
- `Cargo.toml` — dependency manifest (no changes needed; blake3 already
  present).
- `tests/action_mangle_bdd.rs` — existing BDD runner for action mangling
  (style reference for the new BDD runner).
- `tests/features/action_mangle.feature` — existing BDD feature file (style
  reference).
- `docs/name-mangling-rules.md` — normative specification.
- `docs/theoremc-design.md` — design document (add implementation decisions).
- `docs/users-guide.md` — user-facing API documentation.
- `docs/roadmap.md` — roadmap (mark checkbox done).

### Terminology

- `P` — the literal path string for a `.theorem` file, relative to the crate
  root (e.g., `"theorems/bidirectional.theorem"`).
- `path_stem(P)` — `P` with a trailing `.theorem` extension removed if
  present.
- `path_mangle(S)` — the 5-step sanitization algorithm that transforms a
  path stem into a valid Rust identifier fragment.
- `hash12(P)` — the first 12 lowercase hex characters of the blake3 digest
  of the original path string `P`.
- Module name — the generated Rust module identifier:
  `__theoremc__file__{path_mangle(path_stem(P))}__{hash12(P)}`.

## Plan of work

### Stage A: baseline and golden-value computation (no code changes)

Run the existing test suite to confirm the baseline is green. Then use a
temporary test or a short Rust snippet to compute blake3 `hash12` values for
the representative path strings listed in Milestone 1. Record these golden
values for hardcoding into tests.

Go/no-go: existing suite passes and golden values are recorded.

### Stage B: implement core functions and unit tests

Extend `src/mangle.rs` with three new public functions and one new public
struct. Keep the file under 400 lines; if needed, extract into a
`src/mangle_module.rs` sibling file wired via `#[path = ...]` (following the
`collision.rs` / `collision_tests.rs` pattern) or into a `src/mangle/`
directory module.

New types and functions:

1. `path_stem(path: impl AsRef<Utf8Path>) -> PathStem` — removes trailing
   `.theorem` if present.
2. `path_mangle(stem: &PathStem) -> String` — applies the 5-step
   sanitization.
3. `MangledModule` — struct holding `stem`, `mangled_stem`, `hash`, and
   `module_name`.
4. `mangle_module_path(path: impl AsRef<Utf8Path>) -> MangledModule` —
   composite entry point.

Unit tests cover:

- `path_stem` — extension removal, no extension, double extension, empty
  string.
- `path_mangle` — separator replacement, non-alphanumeric replacement,
  underscore collapse, lowercasing, digit-leading prefix. Parameterized with
  `rstest #[case]`.
- `mangle_module_path` — golden tests using the `Golden` struct pattern
  (established in the action-mangling tests). Cover at least: simple path,
  nested directory path, Windows-style backslash path, punctuation-heavy path,
  digit-leading stem, path with no `.theorem` extension.
- Disambiguation: two paths that mangle to the same stem produce different
  `module_name` values because their `hash12(P)` values differ.

Go/no-go: `cargo test --lib -- mangle` passes all new and existing tests.

### Stage C: BDD behavioural tests

Create `tests/features/module_naming.feature` with scenarios:

1. Simple paths produce deterministic module names.
2. Mixed separators produce stable, human-recognizable names.
3. Punctuation-heavy paths are disambiguated by hash.

Create `tests/module_naming_bdd.rs` following the established pattern from
`tests/collision_bdd.rs`.

Go/no-go: `cargo test --test module_naming_bdd` passes all scenarios.

### Stage D: documentation, roadmap, and quality gates

Update:

- `docs/theoremc-design.md` — add §6.7.6 documenting implementation
  decisions for Step 2.2.1.
- `docs/users-guide.md` — add "Per-file module naming" section with API
  documentation and examples.
- `docs/roadmap.md` — mark the first checkbox of Step 2.2 done.

Run `make check-fmt`, `make lint`, `make test` (with `set -o pipefail` and
`tee`) to validate all gates.

Go/no-go: all three gates pass with zero errors and zero warnings.

## Concrete steps

Run from repository root: `/home/user/project`.

1. Baseline verification:

   ```shell
   set -o pipefail
   make test 2>&1 | tee /tmp/2-2-1-baseline-test.log
   ```

   Expected signal: existing suite passes (327+ tests, 0 failures).

2. Compute golden hash values for representative paths. Run a temporary test
   or use `blake3` in a Rust playground to hash each path string. Record the
   12-character hex prefixes. Representative paths:

   - `"theorems/bidirectional.theorem"`
   - `"theorems/nested/deep/path.theorem"`
   - `"theorems\\windows\\style.theorem"`
   - `"theorems/my-file.theorem"`
   - `"theorems/my_file.theorem"`
   - `"theorems/123_digit_leading.theorem"`
   - `"no_extension"`
   - `"theorems/UPPER-case.theorem"`

3. Implement `path_stem`, `path_mangle`, `MangledModule`, and
   `mangle_module_path` in `src/mangle.rs`. Add unit tests below the existing
   `#[cfg(test)]` section (or extract tests to a sibling file if the 400-line
   limit would be exceeded).

4. Run module-level tests:

   ```shell
   set -o pipefail
   cargo test --lib -- mangle 2>&1 | tee /tmp/2-2-1-mangle-test.log
   ```

   Expected signal: all mangle tests pass.

5. Create BDD feature file `tests/features/module_naming.feature` and
   BDD test runner `tests/module_naming_bdd.rs`.

6. Run BDD tests:

   ```shell
   set -o pipefail
   cargo test --test module_naming_bdd 2>&1 | tee /tmp/2-2-1-bdd-test.log
   ```

   Expected signal: all BDD scenarios pass.

7. Update documentation files.

8. Run formatting gate:

   ```shell
   set -o pipefail
   make check-fmt 2>&1 | tee /tmp/2-2-1-check-fmt.log
   ```

   Expected signal: formatter check exits 0.

9. Run lint gate:

   ```shell
   set -o pipefail
   make lint 2>&1 | tee /tmp/2-2-1-lint.log
   ```

   Expected signal: Clippy + rustdoc exit 0 with no denied warnings.

10. Run full test suite:

    ```shell
    set -o pipefail
    make test 2>&1 | tee /tmp/2-2-1-test.log
    ```

    Expected signal: all tests pass (existing + new).

11. Review logs for failure markers:

    ```shell
    grep -E "error:|FAILED|failures:" /tmp/2-2-1-*.log
    ```

    Expected signal: no failure markers found.

## Validation and acceptance

Acceptance behaviours:

- `mangle_module_path("theorems/bidirectional.theorem")` returns a
  `MangledModule` whose `module_name()` matches
  `__theoremc__file__theorems_bidirectional__{hash12("theorems/bidirectional.theorem")}`.
   (The exact hash is determined in Milestone 1.)

- `mangle_module_path("theorems/my-file.theorem")` and
  `mangle_module_path("theorems/my_file.theorem")` produce different
  `module_name()` values because their `hash12` suffixes differ, even though
  their mangled stems are identical.

- `mangle_module_path("theorems\\windows\\style.theorem")` produces the
  same mangled stem as `mangle_module_path("theorems/windows/style.theorem")`
  would if path separators were normalized, but different `module_name()`
  values because `hash12` operates on the original path string.

- `mangle_module_path("theorems/123_digit.theorem")` produces a mangled
  stem starting with `_` (because the spec says to prefix `_` if the result
  starts with a digit).

- All existing schema, action-mangling, and collision tests continue to pass
  (no regressions).

Quality criteria:

- Tests: all existing and new unit/BDD tests pass.
- Lint: `make lint` passes with zero warnings.
- Format: `make check-fmt` passes.
- Final verification: `make test` passes after docs updates and roadmap tick.

## Idempotence and recovery

- All steps are idempotent; rerunning commands is safe.
- If a gate fails, inspect `/tmp/2-2-1-*.log`, apply minimal corrective
  edits, and rerun only the failing gate before rerunning the full gate
  sequence.
- The implementation is largely additive. The `collision` module received minor
  updates to adopt `CanonicalActionName` newtypes from a follow-up refactoring
  pass; its public API is unchanged.

## Artefacts and notes

New artefacts:

- `tests/module_naming_bdd.rs` — BDD test runner.
- `tests/features/module_naming.feature` — BDD feature file.
- Possibly `src/mangle_module.rs` or `src/mangle_module_tests.rs` if the
  400-line limit requires extraction.

Updated artefacts:

- `src/mangle.rs` — extended with `path_stem`, `path_mangle`,
  `MangledModule`, `mangle_module_path`, and unit tests.
- `docs/theoremc-design.md` — added §6.7.6 implementation decisions.
- `docs/users-guide.md` — added "Per-file module naming" section.
- `docs/roadmap.md` — marked first checkbox of Step 2.2 done.

## Interfaces and dependencies

### Public API additions (`theoremc::mangle`)

In `src/mangle.rs`, define:

```rust
/// The result of mangling a `.theorem` file path into a per-file
/// Rust module name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangledModule { /* private fields */ }

impl MangledModule {
    /// The original path stem (P with `.theorem` removed).
    pub fn stem(&self) -> &str;
    /// The sanitized stem after `path_mangle`.
    pub fn mangled_stem(&self) -> &str;
    /// The 12-character blake3 hash of the original path.
    pub fn hash(&self) -> &str;
    /// The full generated module name.
    pub fn module_name(&self) -> &str;
}

/// Removes a trailing `.theorem` extension from `path`, if present.
pub fn path_stem(path: impl AsRef<Utf8Path>) -> PathStem;

/// Sanitizes a path stem into a Rust-identifier-safe fragment.
///
/// Algorithm (per `docs/name-mangling-rules.md`):
/// 1. Replace `/` and `\` with `__`.
/// 2. Replace any character not in `[A-Za-z0-9_]` with `_`.
/// 3. Collapse consecutive `_` to a single `_`.
/// 4. Lowercase the result.
/// 5. If the result starts with a digit, prefix `_`.
pub fn path_mangle(stem: &PathStem) -> String;

/// Mangles a `.theorem` file path into a [`MangledModule`].
///
/// Produces a deterministic, collision-resistant Rust module name
/// of the form:
/// `__theoremc__file__{path_mangle(path_stem(path))}__{hash12(path)}`
pub fn mangle_module_path(path: impl AsRef<Utf8Path>) -> MangledModule;
```

### Dependencies

No new dependencies required. The existing `blake3 = "1.8.3"` crate provides
the `hash12` building block.
