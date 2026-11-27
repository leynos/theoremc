# Architectural decision record (ADR) 0014: Environment seam boundaries

## Status

Accepted. Process-environment reads happen only at named composition roots,
which capture values once and inject them into pure logic; tests never mutate
the parent process environment.

## Date

2026-11-23

## Decision metadata

- Deciders: theoremc maintainers
- Technical story: issue #101, replacing ambient process-environment access and
  mutation with injected seams
- Related records:
  - [ADR-0008](adr-0008-workspace-build-support-boundary.md).
  - [ADR-0009](adr-0009-orthoconfig-cli-configuration.md).
  - [ADR-0012](adr-0012-shared-warm-target-for-fixture-crates.md).
  - [ADR-0013](adr-0013-workspace-complete-lint-gate.md).

## Context

Several components legitimately need a value only the process environment
holds. A Cargo build script learns its crate root and output directory from
`CARGO_MANIFEST_DIR` and `OUT_DIR`; a `theorem_file!` expansion learns the
invoking crate's manifest directory from `CARGO_MANIFEST_DIR`; an integration
test that launches a child with a cleared environment must copy the host's
`PATH` and Rust toolchain home into it. None of these values can be computed
from the code under test.

The problem is not the read. It is that the read used to be reachable from
ordinary logic, so arranging a scenario meant changing global state. The
earlier test helper `EnvGuard` in `crates/test-helpers` took a process-wide
mutex, wrote `set_var`/`remove_var`, and restored the previous values from
`Drop`. Under Rust 2024 those writes are `unsafe`, so the helper forced a
crate-wide exception to `unsafe_code = "forbid"`, and the mutex only
coordinated code that chose to use it. Any other test reading the same variable
still raced.

[ADR-0013](adr-0013-workspace-complete-lint-gate.md) made `make lint` cover
every workspace package and feature set, which made a mechanical prohibition
practical: a rule enforced by the compiler is worth more than a rule recorded
in prose.

## Decision

Adopt the environment seam taxonomy that `leynos/netsuke` recorded in its
ADR-008, and prohibit ambient access mechanically.

### The prohibition

`clippy.toml` disallows `std::env::var`, `var_os`, `vars`, `vars_os`,
`set_var`, and `remove_var` with reasons that state the remedy, and
`[workspace.lints.clippy]` sets `disallowed_methods = "deny"`. Every workspace
member inherits the baseline through `[lints] workspace = true`, so no crate
opts out. `[workspace.lints.rust]` sets `unsafe_code = "forbid"`, which the
removal of `EnvGuard` makes compatible with the policy.

A sanctioned read carries an item-scoped
`#[expect(clippy::disallowed_methods, reason = "...")]`. `allow` is never used:
Clippy reports an unfulfilled `#[expect]`, so an exception that has outlived
its site turns into a warning and the backlog removes itself.

The source is Netsuke's
[ADR-008: environment seam taxonomy](https://github.com/leynos/netsuke/blob/main/docs/adr-008-environment-seam-taxonomy.md).
That document was not reachable while this record was written, so the taxonomy
below follows the description in issue #101, which cites it. The prohibition
reproduces the six `disallowed-methods` entries in the `clippy.toml` that
Netsuke publishes at that repository.

### The taxonomy

The smallest seam that carries the input wins. In increasing order:

1. **An explicit value.** When one variable reaches one caller, pass the value
   itself. `BuildScriptContext` carries the manifest and output directories
   into discovery and suite writing; the trybuild harness resolves its target
   directory once and passes it to staging; the fixture helper takes an
   optional base path. No trait, no closure, no ambient read downstream.

2. **A narrow closure for one variable and one caller.** The proc-macro reads
   `CARGO_MANIFEST_DIR` through `manifest_dir_from_reader(|name| ...)`, which
   maps a `Result<String, VarError>` to `Utf8PathBuf` or `MissingManifestDir`.
   The composition root is one annotated function, `manifest_dir_from_env`,
   that supplies `env::var`. A trait object here would be more machinery than
   the single read supports.

3. **`mockable::Env` when a growing group of variables is shared.** The CLI
   already takes its environment through OrthoConfig's injected sources, and
   its tests supply `MapEnv`. That is the existing form of this rung; do not
   add a second, parallel environment trait beside it.

4. **An `Arc`-wrapped reader only at a `Send + Sync` registration boundary.**
   OrthoConfig's `SharedEnvSource` is this rung. Nothing in this repository
   needs a new one today; adding `Arc` without such a boundary is overhead.

5. **`Command::env_clear` plus explicit `Command::env` values for subprocess
   tests.** A child process has its own environment, so configuring it is not a
   global mutation. The entry-point tests build every child from a captured
   `HostEnv` and never write the parent's variables or current directory.

### Approved composition roots

Exactly these sites perform a direct process-environment read, each with a
reasoned `#[expect]`:

- `BuildScriptContext::from_cargo_env` in
  `crates/theoremc-build-support/src/build_script.rs`, for `CARGO_MANIFEST_DIR`
  and `OUT_DIR`.
- `manifest_dir_from_env` in `crates/theoremc-macros/src/query.rs`, for
  `CARGO_MANIFEST_DIR`.
- `HostEnv::capture` in `crates/cargo-theorem/tests/entry_points/process.rs`,
  for `PATH`, `RUSTUP_HOME`, `HOME`, and `USERPROFILE`.
- `ambient_target_base` in `tests/common/fixture_isolation.rs`, for
  `CARGO_TARGET_DIR`.
- `kani_is_required` in `tests/theorem_file_macro_bdd/cargo_runner.rs`, for
  `THEOREMC_REQUIRE_KANI`.
- The OrthoConfig derive on `GlobalOptions`, whose expansion reads the process
  environment; `crates/cargo-theorem/src/config/mod.rs` declares that module
  under the expectation.

The trybuild target is resolved from `cargo metadata` rather than from
`CARGO_TARGET_DIR`, because trybuild itself builds under the metadata's
`target_directory`. Reading one source of truth keeps the staged fixtures where
the child compiler looks.

### Test arrangement

No test mutates the parent process environment. A scenario that needs a
variable supplies a stub reader, an explicit value, or a child process
environment. Because no shared mutable state remains, no environment mutex and
no serialized test group is needed; the suite runs in the default parallel
group.

## Consequences

- A new ambient read fails `make lint` in Make and in CI, with the configured
  reason in the diagnostic. The rule does not depend on a reviewer noticing.
- `unsafe_code = "forbid"` holds workspace-wide with no crate-wide exception,
  because the only reason for one was the deleted `EnvGuard`.
- The logic behind each boundary is a pure function of explicit input, so a
  test can exercise absence, malformed values, and unrelated directories
  without serialization.
- Adding a variable to an existing boundary means extending the value that
  boundary already carries, not adding another read. Adding a genuinely new
  boundary is an ADR-level change: record it here, with its reason.
- The `#[expect]` convention trades a permanent allowance for a self-clearing
  exception. When a boundary disappears, its stale expectation reports itself.
