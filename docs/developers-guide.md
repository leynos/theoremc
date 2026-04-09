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
  — establishes internationalisation (i18n) strategy
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

| Crate | Purpose |
| ----------- | ------------------------------------------------- |
| `camino` | UTF-8 path types for cross-platform path handling |
| `cap-std` | Capability-oriented filesystem access |
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
metadata; `build_suite` renders deterministic suite contents; and the temporary
`__theoremc_generated_suite` bridge module in `src/lib.rs` provides a validating
`theorem_file!` macro via `include_str!`. In Step 3.2, the bridge will be
replaced by proc-macro-based per-file expansion while keeping the same generated
callsites.

## 2. Module architecture

The crate follows the layer boundaries enforced by Architecture Decision Record
(ADR) [ADR-003](adr-003-architectural-boundary-enforcement.md):

**Table:** Module layers and responsibilities

| Layer | Modules | Responsibility |
| ------------- | -------------------------------------------------- | ------------------------------------------------------------------- |
| Schema | `schema/` | YAML deserialization and semantic validation |
| Mangle | `mangle*.rs` | Deterministic identifier generation |
| Cross-cutting | `collision.rs` | Collision detection across schema and mangle |
| Lowering | `arg_lowering.rs` | Conversion of semantic values to Rust token trees |
| Build | `build_discovery.rs`, `build_suite.rs`, `build.rs` | Theorem file discovery, suite generation, and Cargo change tracking |

The schema layer must not import from `mangle`, and vice versa. The `collision`
module exists as a separate top-level module specifically to orchestrate both
without violating this boundary. The `mangle_validate` module duplicates
action-name validation rules to preserve independence from the schema layer.

### 2.1 Build system API

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

Before committing any change, run the following gates. The Makefile wraps each
underlying command:

**Table:** Quality gates and their Makefile commands

| Gate | Command | What it checks |
| ---------------- | ------------------- | -------------------------------------- |
| Formatting | `make check-fmt` | `cargo fmt --all -- --check` |
| Linting | `make lint` | Clippy with `-D warnings` plus rustdoc |
| Tests | `make test` | `cargo test --workspace` |
| Markdown lint | `make markdownlint` | markdownlint-cli2 on all `.md` files |
| Mermaid diagrams | `make nixie` | Validates Mermaid blocks in Markdown |
| Formatting fix | `make fmt` | `cargo fmt --all` plus mdformat |

When documentation changes are in scope, `make fmt`, `make markdownlint`, and
`make nixie` must also pass.

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
to the build script. The `__theoremc_generated_suite` bridge module in
`src/lib.rs` defines a temporary `theorem_file!` macro that validates theorem
paths at compile time via `include_str!`. This bridge will be replaced with
real proc-macro-based per-file expansion in Step 3.2 while keeping the same
generated `theorem_file!(...)` callsite.

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
