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

- [ADR-0001: Theorem symbol stability and non-vacuity
  policy](adrs/adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md) —
  governs theorem naming and vacuity defaults
- [ADR-0002: Library-first internationalization and localization with
  Fluent](
  adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md
  ) — establishes internationalization (i18n) strategy
- [ADR-0003: Architectural boundary enforcement](
  adrs/adr-0003-architectural-boundary-enforcement-for-schema-layers.md) —
  enforces layered schema boundaries and anti-corruption constraints
- [ADR-0008: Workspace build-support
  boundary](adrs/adr-0008-workspace-build-support-boundary.md) — assigns
  discovery and generated-suite writing to the internal build-support crate

## Scope

This guide covers the build system, internal architecture, contributor
workflows, and extension points for the theoremc crate. For user-facing
behaviour and public API documentation, see the [user's guide](users-guide.md).
For high-level design rationale, see the
[design specification](theoremc-design.md). For file and directory ownership,
see the [repository layout](repository-layout.md). Normative references listed
in the design specification take precedence if wording diverges.

## 1. Build system overview

### 1.1 Build dependencies

The root-level `build.rs` script runs during `cargo build`. Its only direct
build dependency is the workspace-internal `theoremc-build-support` crate,
declared under `[build-dependencies]` in the root `Cargo.toml`:

**Table:** Build dependencies

| Crate                    | Purpose                                               |
| ------------------------ | ----------------------------------------------------- |
| `theoremc-build-support` | Discovery, suite generation, and Cargo rerun metadata |

The support crate owns its build implementation dependencies: `camino`,
`cap-std`, and `thiserror` are in its `[dependencies]`; `rstest` and `tempfile`
are test-only `[dev-dependencies]`. Some of these crates also appear in the
root package's `[dev-dependencies]` for root tests; that does not make them
direct dependencies of `build.rs`. Cargo compiles the support crate and its
normal dependencies for the host toolchain, not the target.

### 1.2 Build script entrypoint (`build.rs`)

The minimal build-script entrypoint calls
`theoremc_build_support::prepare_build_script()`. The support crate:

1. reads `CARGO_MANIFEST_DIR` and `OUT_DIR` from Cargo's environment,
2. delegates to `theoremc_build_support::discover_theorem_inputs()`,
3. writes `OUT_DIR/theorem_suite.rs` via
   `theoremc_build_support::write_theorem_suite()`, containing
   `theorem_file!("path/to/file.theorem");` invocations for each discovered
   theorem,
4. returns `BuildScriptOutput` with theorem presence and ordered rerun paths.

The root `build.rs` adapter registers the theorem cfg with
`cargo::rustc-check-cfg`, emits `cargo::rustc-cfg=theoremc_has_theorems` when
the support output contains theorems, and prints `cargo::rerun-if-changed=`
lines from the returned paths.

Discovery and suite generation belong to the workspace-internal
`theoremc-build-support` crate. `build.rs` uses this crate as a build
dependency, and direct unit tests live with it. Root behavioural tests copy it
as a path dependency into temporary fixture crates, so they exercise the same
Cargo crate boundary as the production build script. Do not share build-support
source via `#[path]`: build support may move internally without coupling the
build script, the root library, and tests to file placement.

### 1.3 Build-support discovery module (`crates/theoremc-build-support/src/discovery.rs`)

The `BuildDiscovery` struct returned by `discover_theorem_inputs()` carries two
ordered vectors:

- `theorem_files` — crate-relative `.theorem` file paths, sorted
  lexicographically and normalized to forward slashes.
- `watched_directories` — directories emitted as
  `cargo::rerun-if-changed` targets, including the root `theorems` directory
  and any nested subdirectories containing theorem files.

The build-support crate exposes this API only for workspace build integration.
It is not re-exported from the public `theoremc` library surface, so downstream
consumers must not depend on it.

#### Error handling

`BuildDiscoveryError` is a workspace-internal, `thiserror`-derived error enum
with two variants:

- `Io` — wraps a `std::io::Error` together with a human-readable
  operation label and the path that failed.
- `TheoremRootNotDirectory` — the `theorems` path exists but is not a
  directory.

An absent `theorems/` directory is not an error; it returns a root-only watch
set so Cargo can detect when the directory is created later.

#### Suite generation (Step 3.1.2)

Step 3.1.1 (see
[`execplans/3-1-1-build-rs-scanning-of-theorems.md`](execplans/3-1-1-build-rs-scanning-of-theorems.md))
introduced discovery and Cargo invalidation; Step 3.1.2 added generated-suite
writing. ADR-0008 records the current ownership boundary: the
`theoremc-build-support` crate performs discovery and writes
`OUT_DIR/theorem_suite.rs` with `theorem_file!("...")` invocations. The root
`build.rs` adapter emits the cfg and rerun metadata from the returned
`BuildScriptOutput`. The hidden `__theoremc_generated_suite` module in
`src/lib.rs` imports the real `theorem_file!` proc macro before including the
generated suite. Step 3.2.1 keeps the generated callsites unchanged while the
per-file expansion logic belongs to `crates/theoremc-macros`.

## 2. Module architecture

The crate follows the layer boundaries enforced by Architecture Decision Record
(ADR)
[ADR-0003](adrs/adr-0003-architectural-boundary-enforcement-for-schema-layers.md):

**Table:** Module layers and responsibilities

| Layer         | Crate                    | Modules                                                | Responsibility                                                                       |
| ------------- | ------------------------ | ------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| Domain        | `theoremc-core`          | `canonical_action_name.rs`                             | Shared canonical action-name grammar and newtype                                     |
| Schema        | `theoremc-core`          | `schema/loader/`, `schema/raw/`, `schema/validate*.rs` | YAML loading, span-aware conversion, duplicate-key diagnostics, and validation       |
| Mangle        | `theoremc-core`          | `mangle*.rs`                                           | Deterministic identifier generation                                                  |
| Cross-cutting | `theoremc-core`          | `collision.rs`                                         | Collision detection across schema and mangle                                         |
| Proc-macro    | `theoremc-macros`        | `lib.rs`, `query.rs`, `render.rs`, `error.rs`          | Macro entry points, semantic query phase, token rendering, and expansion diagnostics |
| Lowering      | `theoremc`               | `arg_lowering.rs`                                      | Test-gated prototype for converting semantic values to Rust token trees              |
| Build         | `theoremc-build-support` | `discovery.rs`, `suite.rs`; root `build.rs`            | Theorem file discovery, suite generation, and Cargo change tracking                  |

The schema loader entry point is `schema/loader/mod.rs`. It coordinates YAML
deserialization, duplicate-key checking, raw-to-domain conversion, semantic
validation, and action-collision checks. Its `duplicate` child is private, and
`check_duplicate_theorem_keys` is `pub(super)`, so the loader can call it while
modules outside the loader subtree cannot. The loader declares its sibling test
files with `#[path = "../loader_tests.rs"]`,
`#[path = "../loader_action_tests.rs"]`,
`#[path = "../loader_diagnostic_tests.rs"]`, and
`#[path = "../loader_duplicate_tests.rs"]`.

The raw span adapter is defined in `schema/raw/mod.rs`, which privately declares
`schema/raw/decode.rs` and re-exports `RawDocDecodeError` as `pub(crate)`.
`decode.rs` holds only that error enum and its location, parameter, and binding
accessors. The `pub(super)` conversion helpers perform raw-to-domain conversion
for `Actions`, `Let` bindings, and `Do` steps from `schema/raw/raw_convert.rs`,
consuming the raw mirror types in `schema/raw_action.rs` and
`schema/raw_evidence.rs`. Raw span-location tests stay in the sibling
`schema/raw_location_tests.rs` file.

The raw step and binding enums in `schema/raw_action.rs` are mappings with
exactly one variant key. They dispatch on that key through
`schema/raw_single_key.rs` rather than deriving `#[serde(untagged)]`, because
serde buffers untagged variants through a `ContentDeserializer`, which discards
every `serde_saphyr` span nested inside the variant. Keeping the dispatch means
the `Spanned` action names survive into the conversion diagnostics. The
`raw_single_key` machinery is crate-private and scoped to the raw adapters: a
new raw shape that is a single-key mapping should reuse it, but nothing outside
the raw adapters should depend on it.

Rust-type lifetime unit tests live in `schema/rust_type_tests.rs`. Property
tests remain in `schema/rust_type_property_tests.rs`, which
`schema/rust_type.rs` registers through its `property_tests` module.

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

Public schema domain types under `schema/types.rs` carry no serde derives.
Spelling, aliases, defaults, and unknown-key policy belong to the raw adapter in
`schema/raw/mod.rs`, `schema/raw_action.rs`, and `schema/raw_evidence.rs`,
which mirrors each domain type and converts into it explicitly. When adding a
YAML-visible field, extend the raw mirror and the conversion helper rather than
deriving serde traits on the public type; that keeps the domain API independent
of the YAML implementation as ADR-003 requires. The identifier newtypes remain
the one deliberate exception: `TheoremName` and `ForallVar` implement
`Deserialize` through a shared `impl_identifier_deserialize!` macro so
validation happens during decoding and cannot be bypassed.

### Accepted harness-planning boundary (pending implementation)

[ADR-0011](adrs/adr-0011-vertical-slice-first-roadmap-sequencing.md) makes a
real version 1 Kani proof and counterexample the next release gate. Step 4.1
introduces the internal `HarnessPlan` in `theoremc-core`; the Kani renderer
remains in `theoremc-macros`, as specified by
[ADR-0006](adrs/adr-0006-backend-neutral-harness-planning-boundary.md). Schema
decoding, semantic validation, planning, and token rendering retain separate
ownership. All calls, `must` operations, and later law primaries share resolved
invocations. The plan retains source provenance and stable obligation identity
without Kani syntax, tokens, or generated local names as identity. Opaque Rust
predicates are explicitly backend-constrained.

Step 4.4 then adds versioned raw proof-obligation adapters, dedicated domain
law types, semantic validation, Law IR, and lowering into that same plan.
[ADR-0007](adrs/adr-0007-verus-backend-preservation-invariants.md) governs
backend capability checking, evidence distinctions, and additive future Verus
bindings. The current assertion-only domain and raw lists remain unchanged
until that step; the public API break requires release and migration
documentation. See
[RFC 0001](rfcs/rfc-0001-semantic-law-templates-over-composable-primaries.md#implementation-sequence)
and the
[roadmap](roadmap.md#phase-4-working-kani-proof-path-safety-policy-and-semantic-laws)
for implementation acceptance criteria.

### 2.1 Schema validation diagnostics

Semantic validators must return typed validation failures rather than rendered
diagnostics. `schema::validate` owns the semantic rule check and returns a
`ValidationFailure` whose optional `ValidationReasonKind` identifies the field
or collection entry that failed; the human-readable reason is carried alongside
it. `IndexedValidationField` distinguishes a repeated section's primary value
field (`assert`, `expr`, or `cover`) from its `because` field. The loader
renders the public diagnostic from that typed failure at its boundary.

The raw schema adapter owns source-location mapping.
`RawTheoremDoc::location_for_validation_reason` maps a `ValidationReasonKind`
to the best available `serde_saphyr` span and falls back to the theorem-name
span when a semantic rule has no precise field location. Do not locate
diagnostics by parsing the public reason string; wording is for users and
snapshots, while reason kinds are the stable machine contract between
validation and loading.

When adding a validation rule:

1. add or reuse the smallest `ValidationReasonKind` variant that identifies the
   field or aggregate responsible for the failure,
2. describe the semantic rule in the failure's reason text,
3. map the reason kind in
   `RawTheoremDoc::location_for_validation_reason()`, and
4. add `rstest` coverage for the mapping plus behavioural or snapshot coverage
   when the rendered diagnostic is externally observable.

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
  must have the same parameter names in order and equivalent parsed types;
  preserve the first declaration as the reference and retain both declaring
  theorem identities and signature summaries in conflict diagnostics;
- probe emission is ordinary non-Kani Rust so drift fails during normal
  compilation, not only under `cargo kani`.

Referenced-type probes use the same layering.
`theoremc_core::collision::referenced_types` walks each theorem document in
deterministic first-seen order, visiting `Forall` first, then `Actions.params`,
then `Actions.returns`, with canonical-token-stream deduplication.
`schema::rust_type::{parse, canonical_token_stream, parse_with_free_named_lifetime}`
owns Rust type parsing and comparison so schema validation, action signature
equivalence, and macro probe generation do not drift apart.
`parse_with_free_named_lifetime` parses once and performs the schema-validation
lifetime analysis for free named lifetime parameters. The macro query phase in
`query.rs` still uses `syn::parse_str` directly for probe emission rather than
routing through the shared parser; `render.rs` performs no parsing of its own.

Macro tests should keep the layers distinct:

- unit tests in `crates/theoremc-macros` assert the emitted token shape and
  deduplication behaviour;
- trybuild-style compile checks cover generated Rust diagnostics where stable;
- fixture-crate BDD tests in `tests/theorem_file_macro_bdd.rs` assert the
  theorem-author workflow using stable rustc fragments.

### 2.3 Build system API

The support crate exposes this API across a Cargo crate boundary to the root
build script and fixture crates:

```rust
pub fn prepare_build_script() -> Result<BuildScriptOutput, BuildScriptError>;

pub fn discover_theorem_inputs(
    manifest_dir: &Utf8Path,
) -> Result<BuildDiscovery, BuildDiscoveryError>;

pub fn write_theorem_suite(
    out_dir: &Utf8Dir,
    discovery: &BuildDiscovery,
) -> Result<(), BuildSuiteError>;
```

These items are `pub` in `theoremc-build-support` because callers are in
separate crates. `BuildScriptOutput` also exposes `has_theorems()` and
`rerun_paths()` accessors; its fields remain private. The public visibility
does not make the support crate part of the public `theoremc` library API.

`BuildDiscovery` keeps its fields private and provides iterators over
`&Utf8Path`:

- `theorem_files()` — discovered `.theorem` files in sorted order.
- `watched_directories()` — directories emitted for Cargo invalidation.
- `rerun_paths()` — watched directories followed by theorem files, in the
  exact order emitted by `build.rs`.

These public items support workspace build integration, the support crate's own
unit tests, and fixture crates that verify the same boundary. They are not a
downstream dependency contract. Do not include support source with `#[path]` or
depend directly on private implementation details such as
`render_theorem_suite`.

## 3. Contributor workflows

### 3.1 Rust toolchain policy

The workspace minimum supported Rust version (MSRV) is Rust 1.89, declared via
the root `Cargo.toml` `rust-version` field and repeated in each workspace
package. This floor is required by `serde-saphyr` 2.0.0 and `ortho_config`
0.10.0; it also permits the use of let chains in conditionals. The decision to
raise the reusable libraries from Rust 1.88 is recorded in
[ADR-0010](adrs/adr-0010-reusable-library-msrv-for-serde-saphyr-2-0.md).

The workspace sets `resolver = "3"` so Cargo prefers dependency versions
compatible with the declared MSRV. That is what keeps the dev-only
`cargo-platform` on 0.3.2 instead of 0.3.3, which needs Rust 1.91, so the MSRV
check can cover every target, including tests.

`make msrv` runs
`RUSTFLAGS="" cargo +1.89 check --workspace --all-targets --all-features
--locked`.
The empty `RUSTFLAGS` displaces the nightly-only `-Zthreads=8` flag and the
mold linker from the repository's Cargo configuration, which a stable toolchain
refuses before it checks any package. The test suite still runs once, in
`make test`.

### 3.2 Quality gates

Before committing any change, run the following gates. Use the Makefile targets
where available, and run the direct Cargo invocations for specialized checks:

**Table:** Quality gates and their commands

| Gate                 | Command                                                                                            | What it checks                                                               |
| -------------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Formatting           | `make check-fmt`                                                                                   | Checks Rust and Markdown table formatting                                    |
| Linting              | `make lint`                                                                                        | rustdoc, Clippy, then Whitaker, each over the whole workspace                |
| Type checking        | `make typecheck`                                                                                   | `cargo check --workspace --all-targets --all-features`                       |
| Acyclicity           | `cargo modules graph --acyclic --lib`                                                              | Checks for cycles in module dependencies                                     |
| Wildcard imports     | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::wildcard_imports` | Flags wildcard imports to keep dependency edges explicit                     |
| Architecture linting | planned: `theoremc_arch_lint` (see the roadmap)                                                    | Will flag schema layer boundary and other architecture rule violations       |
| Tests                | `make test`                                                                                        | nextest test binaries plus Cargo doctests                                    |
| Markdown lint        | `make markdownlint`                                                                                | markdownlint-cli2 on all `.md` files                                         |
| Mermaid diagrams     | `make nixie`                                                                                       | Validates Mermaid blocks in Markdown                                         |
| Spelling             | `make spelling`                                                                                    | Regenerates `typos.toml`, then checks spelling and shared phrase corrections |
| ADR/RFC names        | `make check-adr-rfc-names`                                                                         | Checks canonical filenames, unique serials, titles, and local record links   |
| Formatting fix       | `make fmt`                                                                                         | `cargo fmt --all`, then mdtablefix and markdownlint-cli2 `--fix`             |

When documentation changes are in scope, `make fmt`, `make markdownlint`, and
`make nixie` must also pass.

The style guide's tested-example loader is upstream tooling guidance and is not
a current Theoremc gate. The `make spelling` gate runs Typos and the shared
phrase corrections. The migration-guide naming rule applies to releases that
change observable behaviour; the existing project migration guide follows that
convention.

#### ADR/RFC documentation checker

`make check-adr-rfc-names` enforces the record naming contract in
`docs/documentation-style-guide.md`. It runs the `tools/` pytest suite and then
the checker command, both under a uv-managed Python 3.13 interpreter, and needs
only `uv` on `PATH`.

The checker is split along a port-and-adapter boundary:

- `tools/adr_rfc_names.py` holds the pure rules: filename and heading patterns,
  slug generation, Markdown destination extraction, and the `RecordReader`
  port. Everything crossing that port is a domain value: `RecordFile` and
  `MarkdownDocument` carry repository relative paths and text, and `LinkFacts`
  carries the two containment answers the rules cannot derive themselves. The
  module imports no filesystem path type and touches no files; fitness tests in
  `tools/test_check_adr_rfc_names.py` enforce both.
- `tools/adr_rfc_repository.py` implements that port as `PathRecordReader` and
  owns Markdown discovery, file reads, root resolution, and link containment,
  including symlink and encoded-path handling. It resolves the repository root
  strictly, once, in its constructor, so discovery, reads, and containment
  compare against the same real path, and a missing root or symlink loop
  becomes a read error rather than a silently unresolved path.
- `tools/check_adr_rfc_names.py` is the composition root. It builds the adapter
  inside its guarded region, so a root that cannot resolve becomes a
  `DocumentationCheck.read_error` before any discovery runs, and returns a
  value whose `read_error` field reports an unreadable file instead of raising.

The rules report canonical filenames, unique serials, heading serials, title
slugs, and local ADR/RFC links that escape the repository or are missing. A
malformed local destination becomes an error diagnostic rather than a
traceback. Because the adapter answers containment, a test double can supply
`LinkFacts` without a filesystem.

`make lint` is the aggregate entry point, locally and in CI. It runs three
stages in order — rustdoc, Clippy, then Whitaker — and recurses through
`$(MAKE)` for each, so the stages stay sequential even under `make -j`, and a
failing stage stops the aggregate with its own exit status. All three cover the
whole workspace: `LINT_PACKAGES` defaults to `--workspace`, `CARGO_FLAGS` adds
`--all-targets --all-features`, and the rustdoc stage adds
`--all-features --no-deps`. The scope is named once because Cargo selects only
the root package when no package flag is given, which is how the gates used to
under-check. Four tests hold this shape. `tests/lint_makefile_contract.rs`
reads the recipes as text, and `tests/lint_ci_contract.rs` reads the workflow
and `dylint.toml`. `tests/lint_gate_execution.rs` runs the recipes against stub
executables to prove each stage runs in order and propagates its failure.
`tests/lint_workspace_execution.rs` writes a two-package workspace and runs the
real `cargo doc` and `cargo clippy` invocations, so a finding in the non-root
member is shown to fail the stage rather than merely being passed the right
arguments.

`make lint-whitaker` runs the Whitaker Dylint suite through the `whitaker`
wrapper. Install it with `whitaker-installer` so that `whitaker` is on `PATH`,
and never call `cargo dylint` directly. The recipe sets
`DYLINT_RUSTFLAGS="-D warnings"` so findings fail the gate, and assigns an empty
`RUSTFLAGS`, because Whitaker runs under the toolchain its installer manages
and takes none of the development standard (no `-Zthreads=8`, no `mold`, no
Cranelift). CI installs Whitaker through the shared `install-whitaker` action,
pinned to a revision concordat QG-002 approves; that pin fixes the installer,
while the lint suite itself follows Whitaker's rolling release. The root
`dylint.toml` carries no exclusions, because the suite is clean over every
package, target, and feature set. If a finding ever proves irreducible, add the
narrowest key that works — `excluded_paths` before `excluded_crates` — and
record the reason beside it; `no_std_fs_operations` cannot be suppressed in
source. Whitaker does not replace the custom `theoremc_arch_lint` architecture
rules, which are still planned.

`make test` is intentionally nextest-backed for test binaries, followed by a
Cargo doctest pass for Rustdoc examples. It runs
`cargo nextest run --workspace --all-targets --all-features`, then
`cargo test --workspace --all-features --doc`. Override `NEXTEST_FLAGS` only
with options supported by cargo-nextest, and use `DOCTEST_FLAGS` for doctest
scope.

CI runs `make check-fmt`, the markdownlint-cli2 action, `make spelling`,
`make lint`, `make typecheck`, and `make test`. A separate coverage measurement
step precedes `make test` and installs `cargo-nextest`.

Capture long command output through `tee` with `set -o pipefail` to avoid
losing truncated results:

```sh
set -o pipefail; make lint | tee /tmp/make-lint.log
```

#### The spelling gate

`make spelling` runs the pinned `typos-config-builder gate`, which regenerates
the tracked `typos.toml` from the live shared en-GB-oxendict dictionary on the
`main` branch of `leynos/agent-helper-scripts`, merges the repository's
`typos.local.toml`, and then runs Typos and the shared phrase corrections.
Because the dictionary is live, `typos.toml` changes whenever shared policy
changes; CI runs the gate rather than a drift check, and the regenerated file
is committed. `typos.local.toml` holds narrow repository-specific exceptions
only: exact or full-line patterns for identifiers and terms the policy cannot
correct, never bare accepted words. Its one file exclusion covers the spelling
policy's own fixture data, whose purpose is to contain rejected spellings; the
builder rejects broad exclusions. Never edit `typos.toml` by hand.

The target needs `uv`, taken from the `UV` make variable (`UV ?= uv`), so a
contributor without `uv` on `PATH` can run `make UV=/path/to/uv spelling`. `uv`
provisions the pinned Python 3.14 interpreter, so no separate Python
installation is needed. The whole invocation lives in `SPELLING_COMMAND`, so
the recipe and its overrides stay in one place:

- `SPELLING_BUILDER` (default
  `git+https://github.com/leynos/typos-config-builder.git`) and
  `SPELLING_BUILDER_REF` (default `v0.1.3`) select the builder revision.
- `SPELLING_REPOSITORY` (default `.`) is the repository the gate inspects.
- `SPELLING_SCOPE` (default `all`) is the file scope the gate checks.

`make spelling SPELLING_REPOSITORY=/path/to/checkout` therefore checks another
working tree without editing the recipe. The same command exports
`UV_CACHE_DIR=.uv-cache` and `UV_TOOL_DIR=.uv-tools`, so the gate's uv cache
and installed tools stay inside the repository; both directories are
git-ignored. The builder inspects git-tracked files only: stage a new document
before running the gate because an unstaged file is invisible to it locally
while CI checks it once committed.

Two integration tests guard the wiring and the overlay.
`spelling_gate_contract` reads the Makefile and CI workflow to check the pinned
command, its defaults, the recipe, and the ordering after Markdown lint.
`spelling_command_contract` runs the recipe against a stub `uv` and needs no
network. `spelling_policy` runs the real pinned builder against temporary
tracked fixtures: one exercises every `typos.local.toml` pattern and must pass,
and one adds a misspelling and must fail. The fixture prose lives under
`tests/fixtures/spelling/`, which the overlay excludes from the repository gate
because it must contain the spellings the gate rejects; the copy staged in the
temporary repository is not excluded, so the patterns are still exercised. The
test sets `TYPOS_CONFIG_BUILDER_OFFLINE`, so the dictionary comes from the
builder's bundled snapshot. The builder package itself is fetched from the
pinned tag on first use, so the first run needs network or a warm uv cache;
`make spelling` warms the same cache, and CI runs the gate before the test
suite.

### 3.3 Test conventions

- Use `rstest` fixtures for shared setup.
- Replace duplicated tests with `#[rstest(...)]` parameterized cases.
- Behavioural tests use `rstest-bdd` v0.5.0 with `.feature` files under
  `tests/features/`.
- Add `insta` snapshot tests when requirements depend on output-format
  consistency across multiple variants.
- Use `googletest` assertions when matcher-style checks make intent clearer,
  especially for collection length, substring, and predicate assertions.
- Use `pretty_assertions` for equality checks where a diff is more useful than
  a boolean failure, such as full documents, generated output, and ordered
  vectors.
- Use `proptest` for invariants over a generated input space, such as path,
  name, or mangling rules. Keep generated cases focused on the invariant and
  retain example-based tests for named edge cases.
- Keep shared property strategies under a `#[cfg(test)]` test-support module
  owned by the domain module whose invariants they generate. Only unit tests
  within that crate may reuse them; they are not production helpers or an
  integration-test API.
- Keep integration fixture helpers in `tests/common/mod.rs`. The common module
  owns fixture loading, fixture source identifiers, successful fixture loading
  assertions, and error-fragment assertions. BDD step functions should return
  `Result` and delegate those reusable concerns to `tests/common` instead of
  panicking locally.
- Test files use `#[cfg(test)] #[path = "..._tests.rs"] mod tests;` to
  keep implementation files under 400 lines.
- Cargo build-script rerun tests must use explicit fixture mtime helpers such
  as `FixtureCrate::write_with_advanced_mtime()` after an initial build rather
  than sleeping for a filesystem timestamp tick.
- Fixture crates that tests build with `cargo` share one warm target directory,
  `theoremc-fixture-builds` inside `$CARGO_TARGET_DIR` (or the repository's
  `target`), so third-party dependencies compile once per run instead of once
  per fixture. Each fixture's package gets a short unique name (`fx_` and a
  run-and-sequence suffix; short because Kani derives a file name from the
  crate name), which keeps its build artefacts apart in that directory;
  `generated_suite_contents` reads only the directories named after the
  fixture. The workspace `Cargo.lock` is copied into each fixture so every
  fixture resolves the versions the workspace already built, and the fixture's
  `cargo` runs with `CARGO_NET_OFFLINE=true`, with `CARGO_BUILD_TARGET_DIR` and
  `CARGO_BUILD_BUILD_DIR` removed so a caller's environment cannot move the
  artefacts or share a lock. A fixture that needs a private target must say so
  here and in its helper.
- Tests that start a child `cargo` or `rustc` (the fixture suites, the macros
  crate's trybuild suite and the `rustc` compile-fail unit test) run in the
  `nested-cargo-builds` nextest group (`.config/nextest.toml`), one at a time,
  with an explicit `slow-timeout` and a `global-timeout` for the run. Add a new
  test of that kind to the group's filter.
- Integration tests under `tests/` are separate crates and inherit
  package lint policy. Note that `expect_used = "deny"` fires in integration
  tests but not in `#[cfg(test)]` modules.

`reference_provenance.rs` is a test-local child module of
`reference_selection.rs`; its `pub(super)` `Provenance::assert_reported` helper
is available only to the parent test module. It expands a fixture expected to
fail, compares all five expected values for exact equality with the typed
`MacroExpansionError::ConflictingActionSignature` fields, and checks each value
as a substring in the rendered message. It returns the whole rendered message
so reference-selection cases can compare diagnostics across fixtures.

### 3.4 File size limits

No single code file may exceed 400 lines. When a module and its tests grow
beyond this limit, extract tests into a sibling `*_tests.rs` file using the
`#[path = ...]` attribute.

### 3.5 Dependency requirements

Use Cargo's default caret-compatible version requirements for third-party
crates, but spell the lower bound as an explicit patch version such as
`1.11.0`, not a broad major-only or minor-only floor such as `1` or `0.2`. When
tightening a manifest requirement to match this policy, prefer the version
already resolved in `Cargo.lock` and avoid `cargo update` unless Cargo requires
the lockfile to change.

There is no `[workspace.dependencies]` table. Each manifest declares its own
pinned version, and shared third-party crates repeat the same version string
across the manifests that need them.

#### 3.5.1 `&str`-backed newtype wrappers

Thin `&'a str` wrappers such as `FixtureName`, `ExpectedFragment`, and
`YamlKey` derive `the_newtype::Newtype` plus `derive_more`'s `From` and `Into`
rather than repeating a `new`/`as_str` pair. The shared `schema::StrNewtype`
trait carries one blanket implementation over
`Newtype<Inner = &'a str> + From<&'a str> + Into<&'a str> + Copy`, so a new
wrapper needs three derives and nothing else:

```rust
#[derive(Debug, Clone, Copy, Newtype, From, Into)]
pub struct MyWrapper<'a>(&'a str);
```

Bring `StrNewtype` into scope wherever `new` or `as_str` is called. Because
`the-newtype-macros` resolves the trait path through `proc-macro-crate`, any
crate that derives `Newtype` must also depend on `the-newtype` directly; a
transitive dependency is not enough.

`newt-hype` is not suitable for these wrappers. Its `newtype!` macro emits a
type alias for a shared generic base struct, which collapses distinct wrappers
into one type and exposes `inner` instead of `as_str`.

Wrappers over other borrowed text work the same way. `YamlLine` in
`schema::loader_decode_location` wraps one line of the source document and
carries the line classifiers (indentation, section headers, mapping keys) as
methods. Where a module threads the same `&str` parameter through many helpers,
prefer a wrapper whose methods answer the questions the module keeps asking;
this both names the domain concept and keeps the module's string-argument ratio
low enough for the CodeScene advisory gate.

#### 3.5.2 `syn` version alignment

Every direct `syn` requirement in the workspace is pinned to the same version,
currently `3.0.3`, and the pinned `prettyplease` is the release built against
it, currently `0.3.0`. The two move together: `prettyplease` is not a
`syn`-generic library, so `prettyplease::unparse` accepts only the `syn::File`
of the `syn` release it was compiled against. A workspace that satisfies the
preprocessor with one major and the printer with another produces two distinct
`syn::File` types and fails at the call site with
`error[E0308]: mismatched types` and the note "there are multiple different
versions of crate `syn` in the dependency graph". Bumping one without the other
is therefore never a patch-level change.

Alignment is a per-crate property, not a workspace-wide one. Transitive
dependents still pull their own `syn` onto the graph, and that is expected:
`derive_more-impl`, `thiserror-impl`, `rstest_macros`, `googletest_macro`,
`rust-embed-impl`, `displaydoc`, and `tracing-attributes` all build against
`syn 2`, while `the-newtype-macros` builds against `syn 1`. Those copies are
internal to each proc macro, and their duplicate `syn::File` types never meet
ours, because a proc macro consumes and emits token streams rather than sharing
an AST with its caller. Only a direct `syn` requirement that meets
`prettyplease::unparse` in the same crate has to agree with the pinned
`prettyplease`.

When changing the pinned versions, edit every direct requirement together,
regenerate `Cargo.lock`, and rebuild. `cargo tree -i syn@<major>` reports which
crates still resolve a `syn` major other than the pinned one, and confirms that
the survivors are transitive; `cargo tree -i syn` alone is ambiguous while more
than one major is present. The `theoremc-macros` `insta` golden snapshot
catches an accidental change in printed output across the upgrade.

### 3.6 Extending the build system

To add new build-time discovery or generation:

1. Add the logic to `crates/theoremc-build-support` (or a new sibling module)
   and keep it testable without spawning Cargo.
2. Expose the narrow internal API from `theoremc-build-support` and wire it
   into `build.rs` as a normal build dependency.
3. Add direct unit tests covering edge cases (missing directories,
   permission errors, deterministic ordering).
4. Add behavioural tests in `tests/` using temporary fixture crates when
   the feature interacts with Cargo's build-script protocol.
5. Update `docs/theoremc-design.md` §7 and this guide.

The current implementation keeps discovery and suite writing in
`crates/theoremc-build-support`; the root `build.rs` adapter emits cfg and
rerun metadata from `BuildScriptOutput`. Step 3.2.1 keeps the generated
`theorem_file!("...")` callsite unchanged, while the hidden
`__theoremc_generated_suite` module in `src/lib.rs` imports the real proc macro
re-exported by the root facade.

The live workspace split is:

- `crates/theoremc-core` for shared schema, mangling, and collision logic,
- `crates/theoremc-build-support` for build-time discovery and suite generation,
- `crates/theoremc-macros` for proc-macro expansion, and
- the root `theoremc` crate for the public API plus build integration.

#### 3.6.1 `theoremc-core` and proc-macro boundary

The following items are exported from `theoremc-core` and form the stable
internal interface between the core library and the proc-macro crate:

**Table:** `theoremc-core` stable internal API

| Symbol                                | Kind   | Purpose                                                                                                                                                            |
| ------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `load_theorem_file_from_manifest_dir` | `fn`   | Opens a crate-relative `.theorem` file via `cap_std`, validates it through the shared schema loader, and returns one `TheoremDoc` per YAML document                |
| `TheoremFileLoadError`                | `enum` | Typed error covering all failure modes: `OpenManifestDir`, `InvalidTheoremPath`, `ReadTheoremFile`, `EmptyTheoremFile`, `InvalidTheoremFile`                       |
| `collision::referenced_types`         | `fn`   | Returns distinct Rust type strings referenced by `Forall`, `Actions.params`, and `Actions.returns` in deterministic first-seen order, with canonical deduplication |

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

### 3.7 CodeScene coverage publication

Main owns CodeScene. `.github/workflows/coverage-main.yml` is the one
publisher: on each push to `main` it measures coverage, writes the ratchet
baseline, and uploads the report. Pull requests measure coverage in `ci.yml`
and ratchet it against that baseline, but never contact CodeScene. The shared
uploader verifies the `cs-coverage` archive against a committed digest, so the
artefact is pinned. What cannot be pinned is the CodeScene API the tool calls,
and the tool refuses to run when that API's answers change shape. Moving the
call to `main` keeps such a change off every pull request's critical path.

- The pull-request lane sets `with-ratchet: "true"` and
  `publish-artefact: "false"`, and holds no CodeScene step, client, host, or
  credential. The rule covers every workflow a pull request can start,
  including local reusable workflows those workflows call. A local composite
  action runs in its caller's job, but the contract reads workflow files only,
  so any step running one in either closure is reported rather than read as
  compliant.
- The publisher's check step runs one exact command,
  `echo "available=${{ secrets.CS_ACCESS_TOKEN != '' }}" >> "$GITHUB_OUTPUT"`,
  and carries nothing but a name and an id. The upload step runs only when that
  output is `true` and the ref is `refs/heads/main`, and passes the secret
  straight to the uploader's `access-token` input. No `env` block holds the
  token, because the uploader is a composite action that hands its step's `env`
  to the nested steps it runs.
- The publisher's concurrency group is
  `${{ github.workflow }}-${{ github.ref }}`, never cancelled. Runs for `main`
  never overlap, and a newer trigger replaces an older pending run rather than
  queueing behind it. GitHub does not promise to start runs in trigger order,
  so this does not guarantee commit order. A manual re-run of an older run
  keeps its SHA and its run id: it republishes that commit's coverage to
  CodeScene, but replaces no ratchet baseline unless the original run saved
  none.
- No workflow names the retired installer digest, `installer-checksum` or
  `CODESCENE_CLI_SHA256`.
- Merges made by the Dependabot automerge workflow with `GITHUB_TOKEN` fire no
  push, so they reach the publisher only through a later push or a dispatch.
- A dispatch that replaces a pending push uploads the same or a newer commit.
  generate-coverage saves the baseline only on a push, so the baseline can lag
  by more than one commit until a later push saves it.

`make test-workflow-contracts` holds this shape over the repository's own
workflows by running `cv005-contracts check`, the shared contract library in
`leynos/shared-actions` (`packages/cv005-contracts`), from a full commit named
by `CV005_CONTRACTS_REF` in the Makefile; CI runs it in a "Check the CV-005
contracts" step after setting up uv. A fix to the rules is therefore a pin
bump. The target needs `uv`, which fetches the Python 3.13 the library runs
under. The repository's parameters are in `.github/cv005.toml`: its
`repository` name and the `[selection]` the baseline measures, which the
publisher's generator must carry and every pull-request lane must match. The
library's own suite proves each rule refuses the shape it exists to refuse, so
this repository keeps no copy of the readers or the refusal cases. Workflows
are read strictly: a duplicate key, or a workflow declaring both a quoted and
an unquoted `on` key, is refused rather than silently resolved, and a reading
failure exits 2 rather than passing.

### 3.8 `cargo theorem` application boundary

The [`cargo theorem` CLI foundation](cargo-theorem-cli-design.md) adds a
dedicated `crates/cargo-theorem/` application package. Its initial registered
command is `context --json`; later roadmap slices add operational commands. The
package owns the Cargo external-subcommand entry point, OrthoConfig command and
configuration types, dispatch, application rendering, and exit-code mapping.
Reusable theorem, project, execution, and report services remain in library
packages rather than moving into the CLI.

The `cargo-theorem` package uses `ortho_config` and `cargo-orthohelp` 0.10.0.
The CLI's `agent_context.rs` calls cargo-orthohelp at runtime to convert
generated command metadata into the agent-context schema. Keep both
dependencies in the application package. The facade, `theoremc-core`,
`theoremc-macros`, and build-support crates remain configuration-free and do
not depend on application configuration, but every package now declares the
same Rust 1.89 MSRV. CI and local validation run one workspace check:

```console
RUSTFLAGS="" cargo +1.89 check --workspace --all-targets --all-features --locked
```

The empty `RUSTFLAGS` matters: repository Cargo configuration carries the
nightly-only `-Zthreads=8`, which stable Rust refuses before checking a
package. An assigned `RUSTFLAGS` displaces the configuration's flags, so the
empty assignment allows the stable checks to run without the nightly-only
setting.

Configuration, CLI parsing, and process error handling belong to
`crates/cargo-theorem`. Its OrthoConfig-derived types resolve files, profiles,
environment, and selected command settings before handing validated values to
application services. The application resolves the file chain once because
OrthoConfig 0.10.0's selected-subcommand helper cannot consume the canonical
`theoremc.toml` filename or explicit `--config` path. An application-owned
`SelectedSubcommandMerge` adapter passes only the selected `cmds.<name>` values
from that resolved chain. Tests inject `MapEnv` and explicit file sources
rather than changing the test process environment. The `cargo-theorem` entry
point uses a single `theorem` subcommand wrapper so Cargo dispatch and direct
`cargo-theorem theorem ...` invocation share one parser. Do not add OrthoConfig
or `ortho_l10n` to the facade or reusable theorem crates.

Backend integration uses narrow ports. Backend lifecycle concerns (discovery,
resolution, installation, and health checks) must remain separate from backend
execution and replay concerns. A shared command-runner abstraction owns
argument-vector process creation, timeout and cancellation handling, output
capture, and test doubles; providers supply backend-specific plans and parsers.
Public backend arguments remain ordinary argument-vector values. Secret inputs
must not be accepted through CLI input or placed in process argv: providers
declare them separately and supply them through an injected protected channel.
Only redacted references to those inputs may be displayed, logged, or persisted.

Build integration belongs to the reusable `theoremc-build` boundary. It owns
theorem discovery, suite rendering, and the planned Cargo manifest and
`build.rs` integration services. The injector may update a project, while
`build-script run` calls the reusable service directly. Commands that invoke
Cargo, such as `check --compile` and `run`, execute the package's normal
`build.rs`; their plans and context must make that side effect explicit.

The CLI's caller-facing contract is documented separately from its design. The
[`cargo theorem` user's guide](cargo-theorem-cli-users-guide.md) is the
task-oriented guide for installation, project initialization, theorem creation,
checking, backend management, execution, reports, JSON output, exit classes,
and migration from `rust-prover-tools`. It states its proposed status until the
commands it describes ship. The design document remains the normative source,
and the guide defers to it on any conflict.

Observability follows the workspace-wide `metrics` and `tracing` policy. The
CLI design fixes the metric names, kinds, and label sets, and requires a
matching `describe_*` declaration for each. Metric labels are drawn only from
closed enumerations the CLI or a provider descriptor controls; run IDs, paths,
theorem IDs, and raw error text are event fields, never labels. Libraries emit
instrumentation but never install global subscribers or recorders.

## 4. Filesystem and path conventions

The crate uses `cap_std` and `camino` in place of `std::fs` and `std::path` for
capability-oriented filesystem access and reliable UTF-8 path handling. New
filesystem code should follow this convention.

Discovered theorem paths are normalized to forward-slash crate-relative form
(`theorems/nested/example.theorem`) regardless of host platform separator. This
normalization is important because downstream name mangling rules assume stable
path identity.

String-level path formatting that feeds proc-macro expansion or fixture Cargo
manifests belongs in `theoremc_core::path_format`. Use
`normalize_path_separators` instead of open-coded `replace('\\', "/")`, and use
`toml_basic_string_value` when writing a normalized path into a TOML basic
string. Keep filesystem validation in `theorem_file.rs` behind the named path
violation classifier so tests can assert the rejected path class rather than a
single opaque invalid-path boolean.

## 5. Lint and error handling policy

- Clippy warnings are denied (`-D warnings`).
- `missing_docs = "deny"` requires doc comments on all public items.
- `expect_used` and `unwrap_used` are denied in production code.
- Prefer `thiserror`-derived error enums for any condition callers might
  inspect. Use `eyre::Report` only at the application boundary.
- Lint suppressions must be tightly scoped and include a reason string.

## The build standard

Development, test, lint, and typecheck builds use the parallel `rustc` frontend
(`-Zthreads=8`) and, on Linux, the `mold` linker (`-Clink-arg=-fuse-ld=mold`).
These are defaults in `.cargo/config.toml`, which Cargo discovers on its own,
so a bare `cargo build` gets them. `mold` ships for Linux only, so the linker
flag lives in a Linux-only table and macOS and Windows keep their platform
linker. Cargo selects one `rustflags` source rather than merging them, so every
source repeats the same flags apart from the linker.

An assigned `RUSTFLAGS` replaces the configuration's flags, so the Makefile
recipes that set it compose the standard's flags onto any inherited value (CI's
`setup-rust` exports one). Two builds are deliberately excluded: coverage
assigns `RUSTFLAGS` without the fast flags, because a measurement should not
depend on them, and the release recipe and workflow keep the platform linker,
because they assign `RUSTFLAGS` (even an empty value displaces the
configuration). Cargo has no per-profile `rustflags`, so a direct
`cargo build --release` takes the configuration's flags unless `RUSTFLAGS` is
assigned too.

On Linux, install `mold` before building: the configuration names it, so a
build without it fails at link time. CI installs it through `setup-rust`'s
`install-mold` input. `tests/build_standard_contract.rs` holds the standard. It
reads the configuration sources, the commands `make -n` prints for each
development target on a Linux host and a macOS host (each keeping the caller's
own `RUSTFLAGS`) and for the release target (the coverage exclusion is checked
in the workflow steps) on a Linux host, and the `setup-rust` steps of the CI
workflows (each must pass `install-mold`), so a flag lost through a recipe or
workflow edit fails there. The decision is recorded in
[ADR-0005](adrs/adr-0005-adopt-the-rust-build-standard-for-development-builds.md).
The contract runs `make -n`, so a direct `cargo test` command requires GNU
make to be available on the `PATH`. It fails when `make` is missing instead of
skipping, so a missing tool cannot read as a pass.

### Cranelift

Exception: Cranelift is not the development-profile backend. The estate adopts
it only where the full suite passes under it, and that is not shown here on the
pinned `nightly-2026-01-30` (measured 2026-10-02): four tests fail under LLVM
on this host as well and one more under Cranelift, so the suite is not shown to
pass under it. Revisit on the next toolchain bump: measure the whole suite
under the backend, with CI's environment, and adopt it if every test passes.

Recorded state: the pin is `nightly-2026-11-05` and no measurement has been
taken on it, so the exception stands on the 2026-10-02 result. The review is
deferred to 1 April 2027, when the whole suite is re-measured under both
backends.

### Development build footprint

The development profile sets `incremental = false` and `debug = 0` in
`Cargo.toml`, so a dev build keeps no incremental state or debug information.
To debug, set `CARGO_PROFILE_DEV_DEBUG=true`; to rebuild incrementally, set
`CARGO_INCREMENTAL=1`. `.cargo/config.toml` also sets
`[unstable] embed-metadata = false`, the native Cargo key for nightlies from
2026-07-31.
