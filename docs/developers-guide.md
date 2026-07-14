# Developer's guide

**Status:** living document **Audience:** maintainers and contributors to the
theoremc crate

## Problem statement

This guide exists to help contributors understand theoremc's internal
architecture and maintain its design constraints. The crate follows layered
boundaries and capability-oriented patterns to keep test generation predictable
and maintainable. Without clear architectural guidance, incremental changes can
erode these boundaries and make the codebase harder to reason about.

## Key architecture decision records (ADRs)

- [ADR-001: Theorem symbol stability and non-vacuity
  policy](adr-001-theorem-symbol-stability-and-non-vacuity-policy.md) — governs
  theorem naming and vacuity defaults
- [ADR-002: Library-first internationalization and localization with
  Fluent](adr-002-library-first-internationalization-and-localization-with-fluent.md)
  — establishes internationalization (i18n) strategy
- [ADR-003: Architectural boundary
  enforcement](adr-003-architectural-boundary-enforcement.md) — enforces
  layered schema boundaries and anti-corruption constraints

## Scope

This guide covers the build system, internal architecture, contributor
workflows, and extension points for the theoremc crate. For user-facing
behaviour and public API documentation, see the [user's guide](users-guide.md).
For high-level design rationale, see the
[design specification](theoremc-design.md). Normative references listed in the
design specification take precedence if wording diverges.

## 1. Build system overview

### 1.1 Build dependencies

The root-level `build.rs` script runs during `cargo build` and depends on three
crates declared under `[build-dependencies]` in `Cargo.toml`:

**Table:** Build dependencies

| Crate       | Purpose                                                      |
| ----------- | ------------------------------------------------------------ |
| `camino`    | UTF-8 path types for cross-platform path handling            |
| `cap-std`   | Capability-oriented filesystem access                        |
| `thiserror` | Derive macro for `BuildDiscoveryError` and `BuildSuiteError` |

These are separate from the library's `[dependencies]` and the test-only
`[dev-dependencies]`. Cargo compiles them for the host toolchain, not the
target.

### 1.2 Build script entrypoint (`build.rs`)

The build script performs discovery and suite generation:

1. reads `CARGO_MANIFEST_DIR` from the environment (set by Cargo),
2. delegates to `build_discovery::discover_theorem_inputs()`,
3. writes `OUT_DIR/theorem_suite.rs` via `build_suite::write_theorem_suite()`,
   containing `theorem_file!("path/to/file.theorem");` invocations for each
   discovered theorem,
4. emits `cargo::rustc-cfg=theoremc_has_theorems` when any theorems are
   discovered (used by conditional lint expectations in the generated suite
   bridge), and
5. prints `cargo::rerun-if-changed=` lines for each watched directory and
   discovered theorem file.

The discovery and suite modules are shared between `build.rs` and the library's
test suite via separate `#[path = "..."]` attributes for each module
(`#[path = "src/build_discovery.rs"]` for discovery and
`#[path = "src/build_suite.rs"]` for suite generation). Rust does not support
wildcards in `#[path]`, so multiple attributes must be listed. This keeps the
build script small without exporting a new public API surface.

### 1.3 Build discovery module (`src/build_discovery.rs`)

The `BuildDiscovery` struct returned by `discover_theorem_inputs()` carries two
ordered vectors:

- `theorem_files` — crate-relative `.theorem` file paths, sorted
  lexicographically and normalized to forward slashes.
- `watched_directories` — directories emitted as
  `cargo::rerun-if-changed` targets, including the root `theorems` directory
  and any nested subdirectories containing theorem files.

The module exposes its API as `pub(crate)` only. It is not part of the public
library surface.

#### Error handling

`BuildDiscoveryError` is an internal `thiserror`-derived enum with two variants:

- `Io` — wraps a `std::io::Error` together with a human-readable
  operation label and the path that failed.
- `TheoremRootNotDirectory` — the `theorems` path exists but is not a
  directory.

An absent `theorems/` directory is not an error; it returns a root-only watch
set so Cargo can detect when the directory is created later.

#### Suite generation (Step 3.1.2)

Step 3.1.1 (see
[`execplans/3-1-1-build-rs-scanning-of-theorems.md`](execplans/3-1-1-build-rs-scanning-of-theorems.md))
owns discovery and Cargo invalidation. Step 3.1.2 adds suite generation via
`build_suite::write_theorem_suite()`, which writes `OUT_DIR/theorem_suite.rs`
containing `theorem_file!("...")` invocations. The handoff is deliberately
narrow: `build.rs` produces an ordered crate-relative file list plus rerun
metadata; `build_suite` renders deterministic suite contents; and the hidden
`__theoremc_generated_suite` module in `src/lib.rs` imports the real
`theorem_file!` proc macro before including the generated suite. Step 3.2.1
keeps the generated callsites unchanged while moving the per-file expansion
logic into `crates/theoremc-macros`.

## 2. Module architecture

The crate follows the layer boundaries enforced by Architecture Decision Record
(ADR) [ADR-003](adr-003-architectural-boundary-enforcement.md):

**Table:** Module layers and responsibilities

| Layer         | Crate             | Modules                                            | Responsibility                                                                |
| ------------- | ----------------- | -------------------------------------------------- | ----------------------------------------------------------------------------- |
| Domain        | `theoremc-core`   | `canonical_action_name.rs`                         | Shared canonical action-name grammar and newtype                              |
| Schema        | `theoremc-core`   | `schema/`                                          | YAML deserialization and semantic validation                                  |
| Mangle        | `theoremc-core`   | `mangle*.rs`                                       | Deterministic identifier generation                                           |
| Cross-cutting | `theoremc-core`   | `collision.rs`                                     | Collision detection across schema and mangle                                  |
| Proc-macro    | `theoremc-macros` | `lib.rs`                                           | Proc-macro entry points, theorem-file loading delegation, and code generation |
| Lowering      | `theoremc`        | `arg_lowering.rs`                                  | Conversion of semantic values to Rust token trees                             |
| Build         | `theoremc`        | `build_discovery.rs`, `build_suite.rs`, `build.rs` | Theorem file discovery, suite generation, and Cargo change tracking           |

The schema layer must not import from `mangle`, and vice versa. Shared domain
concepts that both layers need belong in neutral top-level modules. For
canonical action names, `canonical_action_name.rs` owns the grammar, typed
failure reasons, and `CanonicalActionName` newtype. Schema code wraps that
typed result into schema diagnostics, and mangle code accepts either
`&CanonicalActionName` or checked `try_*` string APIs. Do not add another
string parser in either layer.

Public schema domain objects must carry canonical action names as
`CanonicalActionName`, not `String`. Raw YAML adapters may deserialize action
names as strings or `serde_saphyr::Spanned<String>` so diagnostics can retain
source locations, but raw-to-domain conversion is the boundary where those
strings are validated. Validation, collision detection, and macro query code
should treat `TheoremDoc.actions` and `ActionCall.action` as already canonical;
use `CanonicalActionName::as_str()` only when rendering messages, generated
identifiers, or user-facing text.

### 2.1 Schema validation diagnostics

Semantic validators must return typed validation errors rather than formatted
diagnostics. `schema::validate` owns the semantic rule check and returns a
`ValidationError` that combines a `ValidationPath` with a `ValidationKind`. The
path identifies the source field or collection entry that failed, while the
kind identifies the rule violation. Human-readable reason strings are rendered
from that typed error at the loader boundary.

The raw schema adapter owns source-location mapping. `RawTheoremDoc` maps a
`ValidationPath` to the best available `serde_saphyr` span and falls back to
the theorem name span when a semantic rule has no precise field location. Do
not locate diagnostics by parsing the public reason string; wording is for
users and snapshots, while paths are the stable machine contract between
validation and loading.

When adding a validation rule:

1. add or reuse the smallest `ValidationPath` variant that identifies the
   field or aggregate responsible for the failure,
2. add or reuse a `ValidationKind` variant for the semantic rule,
3. render the public reason in `ValidationError::reason()`,
4. map the path in `RawTheoremDoc::location_for_validation_path()`, and
5. add `rstest` coverage for the path mapping plus behavioural or snapshot
   coverage when the rendered diagnostic is externally observable.

### 2.2 Action probe generation

Compile-time action probes are generated by `theoremc-macros` from validated
`Actions` declarations in `TheoremDoc`. The macro must not infer signatures from
`crate::theorem_actions`, because doing so would accept drift rather than
detect it. The implementation uses
`theoremc_core::collision::referenced_actions` to walk action references in the
same order as collision detection, then uses
`theoremc_core::mangle::mangle_action_name` to resolve each canonical action to
its Rust export. This path receives validated `CanonicalActionName` values from
the schema domain model; string inputs must use checked `try_*` APIs before
reaching identifier-sensitive code.

Keep these conventions when extending probe generation:

- action signature parsing belongs at schema loading time where possible, so
  invalid theorem-side contracts remain schema diagnostics;
- generated Rust probes should fail only for owner-crate issues such as missing
  exports, inaccessible exports, or signature drift;
- duplicate declarations for the same referenced action in one macro expansion
  must be identical;
- probe emission is ordinary non-Kani Rust so drift fails during normal
  compilation, not only under `cargo kani`.

### 2.3 Build system API

Contributors extending the build system should note the following internal API
surface:

```rust
pub(crate) struct BuildDiscovery {
    theorem_files: Vec<Utf8PathBuf>,
    watched_directories: Vec<Utf8PathBuf>,
}

pub(crate) fn discover_theorem_inputs(
    manifest_dir: &Utf8Path,
) -> Result<BuildDiscovery, BuildDiscoveryError>;

pub(crate) fn render_theorem_suite<'a>(
    theorem_files: impl IntoIterator<Item = &'a Utf8Path>,
) -> String;

pub(crate) fn write_theorem_suite(
    out_dir: &Utf8Dir,
    discovery: &BuildDiscovery,
) -> Result<(), BuildSuiteError>;
```

Accessors return iterators over `&Utf8Path`:

- `theorem_files()` — discovered `.theorem` files in sorted order.
- `watched_directories()` — directories emitted for Cargo invalidation.
- `rerun_paths()` — watched directories followed by theorem files, in the
  exact order emitted by `build.rs`.

## 3. Contributor workflows

### 3.1 Quality gates

Before committing any change, run the following gates. Use the Makefile targets
where available, and run the direct Cargo invocations for specialized checks:

**Table:** Quality gates and their commands

| Gate                 | Command                                                                                            | What it checks                                                     |
| -------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Formatting           | `make check-fmt`                                                                                   | `cargo fmt --all -- --check`                                       |
| Linting              | `make lint`                                                                                        | Clippy with `-D warnings` plus rustdoc                             |
| Acyclicity           | `cargo modules graph --acyclic --lib`                                                              | Checks for cycles in module dependencies                           |
| Wildcard imports     | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::wildcard_imports` | Flags wildcard imports to keep dependency edges explicit           |
| Architecture linting | `cargo dylint theoremc_arch_lint --all -- -D warnings`                                             | Flags schema layer boundary and other architecture rule violations |
| Tests                | `make test`                                                                                        | nextest test binaries plus Cargo doctests                          |
| Markdown lint        | `make markdownlint`                                                                                | markdownlint-cli2 on all `.md` files                               |
| Mermaid diagrams     | `make nixie`                                                                                       | Validates Mermaid blocks in Markdown                               |
| Formatting fix       | `make fmt`                                                                                         | `cargo fmt --all` plus mdformat                                    |

When documentation changes are in scope, `make fmt`, `make markdownlint`, and
`make nixie` must also pass.

`make test` is intentionally nextest-backed for test binaries, followed by a
Cargo doctest pass for Rustdoc examples. It runs
`cargo nextest run --workspace --all-targets --all-features`, then
`cargo test --workspace --all-features --doc`. Override `NEXTEST_FLAGS` only
with options supported by cargo-nextest, and use `DOCTEST_FLAGS` for doctest
scope.

Capture long command output through `tee` with `set -o pipefail` to avoid
losing truncated results:

```sh
set -o pipefail; make lint | tee /tmp/make-lint.log
```

### 3.2 Test conventions

- Use `rstest` fixtures for shared setup.
- Replace duplicated tests with `#[rstest(...)]` parameterized cases.
- Behavioural tests use `rstest-bdd` v0.5.0 with `.feature` files under
  `tests/features/`.
- Use `googletest` assertions when matcher-style checks make intent clearer,
  especially for collection length, substring, and predicate assertions.
- Use `pretty_assertions` for equality checks where a diff is more useful than
  a boolean failure, such as full documents, generated output, and ordered
  vectors.
- Use `insta` snapshot tests when a multivariant textual output format needs
  stable review, for example diagnostics or macro expansion output.
- Use `proptest` for invariants over a generated input space, such as path,
  name, or mangling rules. Keep generated cases focused on the invariant and
  retain example-based tests for named edge cases.
- Keep integration fixture helpers in `tests/common/mod.rs`. The common module
  owns fixture loading, fixture source identifiers, successful fixture loading
  assertions, and error-fragment assertions. BDD step functions should return
  `Result` and delegate those reusable concerns to `tests/common` instead of
  panicking locally.
- Test files use `#[cfg(test)] #[path = "..._tests.rs"] mod tests;` to
  keep implementation files under 400 lines.
- Integration tests under `tests/` are separate crates and inherit
  package lint policy. Note that `expect_used = "deny"` fires in integration
  tests but not in `#[cfg(test)]` modules.

### 3.3 File size limits

No single code file may exceed 400 lines. When a module and its tests grow
beyond this limit, extract tests into a sibling `*_tests.rs` file using the
`#[path = ...]` attribute.

### 3.4 Extending the build system

To add new build-time discovery or generation:

1. Add the logic to `src/build_discovery.rs` (or a new sibling module)
   and keep it testable without spawning Cargo.
2. Wire the new logic into `build.rs` via the shared `#[path = ...]`
   inclusion.
3. Add direct unit tests covering edge cases (missing directories,
   permission errors, deterministic ordering).
4. Add behavioural tests in `tests/` using temporary fixture crates when
   the feature interacts with Cargo's build-script protocol.
5. Update `docs/theoremc-design.md` §7 and this guide.

Step 3.1.2 extends this pattern by adding suite generation (`build_suite.rs`)
to the build script. Step 3.2.1 keeps that generated `theorem_file!("...")`
callsite unchanged, but the hidden `__theoremc_generated_suite` module in
`src/lib.rs` now imports the real proc macro re-exported by the root facade.

The live workspace split is:

- `crates/theoremc-core` for shared schema, mangling, and collision logic,
- `crates/theoremc-macros` for proc-macro expansion, and
- the root `theoremc` crate for the public API plus build integration.

#### 3.4.1 `theoremc-core` and proc-macro boundary

The following items are exported from `theoremc-core` and form the stable
internal interface between the core library and the proc-macro crate:

**Table:** `theoremc-core` stable internal API

| Symbol                                | Kind   | Purpose                                                                                                                                             |
| ------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `load_theorem_file_from_manifest_dir` | `fn`   | Opens a crate-relative `.theorem` file via `cap_std`, validates it through the shared schema loader, and returns one `TheoremDoc` per YAML document |
| `TheoremFileLoadError`                | `enum` | Typed error covering all failure modes: `OpenManifestDir`, `InvalidTheoremPath`, `ReadTheoremFile`, `EmptyTheoremFile`, `InvalidTheoremFile`        |

The proc-macro crate exposes the companion expansion boundary:

**Table:** `theoremc-macros` stable internal API

| Symbol          | Kind         | Purpose                                                    |
| --------------- | ------------ | ---------------------------------------------------------- |
| `theorem_file!` | `proc macro` | Expands validated theorem documents into generated modules |

`theorem_file!` must fail macro expansion with the diagnostic rendered from the
shared loader error when the path is invalid, the file cannot be read, the file
is empty, or schema validation fails. It must call
`load_theorem_file_from_manifest_dir` for IO, path validation, and schema
diagnostics rather than re-implementing those behaviours locally.

If a theorem document omits `Evidence.kani`, the macro fails expansion with
`MissingKaniEvidence` for that theorem rather than generating a harness stub.
The generated Kani module remains behind `#[cfg(kani)]`, and each harness gets
its `#[kani::proof]` and `#[kani::unwind(n)]` attributes from the validated
Kani evidence for that theorem document.

The mutual invariant is that `TheoremFileLoadError` variants are the canonical
error types at the macro/runtime boundary. The proc macro may render those
errors into compile diagnostics, but it must not introduce a parallel taxonomy
for theorem-file loading failures.

When theorem expansion behaviour changes, prefer testing it in two layers:

1. direct proc-macro unit tests in `crates/theoremc-macros`, and
2. fixture-crate behavioural tests in `tests/theorem_file_macro_bdd.rs`.

## 4. Filesystem and path conventions

The crate uses `cap_std` and `camino` in place of `std::fs` and `std::path` for
capability-oriented filesystem access and reliable UTF-8 path handling. New
filesystem code should follow this convention.

Discovered theorem paths are normalized to forward-slash crate-relative form
(`theorems/nested/example.theorem`) regardless of host platform separator. This
normalization is important because downstream name mangling rules assume stable
path identity.

## 5. Lint and error handling policy

- Clippy warnings are denied (`-D warnings`).
- `missing_docs = "deny"` requires doc comments on all public items.
- `expect_used` and `unwrap_used` are denied in production code.
- Prefer `thiserror`-derived error enums for any condition callers might
  inspect. Use `eyre::Report` only at the application boundary.
- Lint suppressions must be tightly scoped and include a reason string.
