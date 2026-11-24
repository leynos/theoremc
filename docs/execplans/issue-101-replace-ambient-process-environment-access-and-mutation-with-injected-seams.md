# Replace ambient process-environment access and mutation with injected seams

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: IN PROGRESS (execution authorized by the issue #101 task request)

Branch:
`issue-101-replace-ambient-process-environment-access-and-mutation-with-injected-seams`

Roadmap item: none. Issue [#101](https://github.com/leynos/theoremc/issues/101)
is the governing request; related work landed under #98 (lint baseline) and #99
(workspace-complete lint gate).

## Purpose / big picture

TheoremC currently lets a handful of places read the process environment
directly: the Cargo build script, the `theorem_file!` proc macro, and three
test harnesses. Some of those reads are unavoidable — a build script can only
learn the crate root from Cargo's environment — but the surrounding logic
should not need to know that. Issue #101 asks for the same environment-boundary
policy that `leynos/netsuke` adopted: capture the environment once at a narrow
composition root, pass the resulting values into pure logic, and never mutate
the parent test process to arrange a scenario.

After the change a contributor can observe three things. First, `std::env::var`,
`var_os`, `vars`, `vars_os`, `set_var` and `remove_var` are denied by Clippy
everywhere, so a new ambient read fails `make lint` unless it carries a reasoned
`#[expect]` at an approved boundary. Second, the build-script, proc-macro, and
test-harness logic can be exercised from explicit values alone, so the tests
run in parallel with no environment mutex and no serialized group. Third,
`unsafe_code = "forbid"` holds workswide with no crate-wide exception, because
the unsafe `EnvGuard` helper is gone.

The behaviour of theorem discovery, suite generation, macro expansion, fixture
builds, and the CLI entry points is unchanged; only the seam through which
those components receive their inputs changes.

## Constraints

- Do not modify `Makefile` or `.github/workflows/ci.yml`. The existing
  `make lint` aggregate already enforces the workspace baseline.
- Do not change `.config/nextest.toml`. In particular, do not add or remove
  resource-protection groups as part of this work.
- Keep the workspace baseline at `disallowed_methods = "deny"` and
  `unsafe_code = "forbid"`. No per-crate or per-module `allow`; sanctioned
  reads use item-scoped `#[expect(clippy::disallowed_methods, reason = "...")]`.
- Preserve existing error types and diagnostics: `BuildScriptError`,
  `MacroExpansionError::MissingManifestDir`, the trybuild snapshots, and the
  intentionally missing `missing_theorem` fixture.
- Preserve the observable behaviour of the Cargo rerun directives emitted by
  `build.rs`, including their order.
- Do not retroactively edit completed ExecPlans, ADRs, or the
  `docs/ortho-config` import.
- Documentation must follow the documentation style guide: en-GB-oxendict
  spelling, eightieth-column prose wrapping, dash bullets.
- Every `assert!`, `assert_eq!`, and `assert_ne!` carries a diagnostic message.
- Files must stay under 400 lines. Use `#[cfg(test)] #[path = "..._tests.rs"]`
  for colocated unit tests.
- Implementation proceeds without a further approval gate because the task
  request that supplied the coding plan also authorized implementation and PR
  delivery.

## Tolerances (exception triggers)

- Files: more than 20 non-ExecPlan files changed means stop and escalate.
- Code: more than 900 net lines of Rust (production plus tests) means stop and
  escalate.
- Configuration: more than 40 net lines of `clippy.toml`, `Cargo.toml`, and
  `Cargo.lock` means stop and escalate.
- Documentation: more than 500 net lines excluding ExecPlan files means stop and
  escalate.
- Interface: any public API change beyond `BuildScriptContext` (manifest and
  output paths), the `prepare_build_script` signature, and the new ADR means
  stop and escalate.
- Dependencies: any dependency outside `serde_json` and `tempfile` as workspace
  versioned dev-dependencies means stop and escalate.
- Iterations: three failed attempts at the same gate without a new diagnosis
  means stop and escalate.
- Ambiguity: where two defensible seam shapes exist and the choice changes a
  public signature, stop and present the options.

The ExecPlan file itself is excluded from every size count.

## Risks

- Risk: `cargo metadata` resolves a different target directory than the
  trybuild child Cargo uses, so staged fixtures land nowhere the compiler
  looks. Severity: medium Likelihood: low Mitigation: resolve metadata from the
  same working directory with the inherited environment, and add a unit test
  over stub metadata plus a staging-content test.
- Risk: the fixture target helper is shared by several BDD suites; an injected
  base that skips canonicalization changes the paths recorded in build logs.
  Severity: medium Likelihood: low Mitigation: keep canonicalization and the
  existing suffix logic untouched; test absolute, relative, and absent bases.
- Risk: `HostEnv` plumbed through every call site changes argument counts and
  trips `too_many_arguments`. Severity: low Likelihood: medium Mitigation: pass
  `&HostEnv` as the first parameter and keep the remaining arguments at their
  current count.
- Risk: the new Clippy execution test could inherit host flags and produce a
  false pass or fail. Severity: medium Likelihood: medium Mitigation: remove
  `RUSTFLAGS`, `RUSTDOCFLAGS`, and `CLIPPY_CONF_DIR`, set an explicit
  `CARGO_TARGET_DIR`, and run offline.

## Progress

- [x] (2026-11-23 17:30Z) Reconnaissance: confirmed #98/#234 landed the lint
  baseline, `clippy.toml` entries, `unsafe_code = "forbid"`, and the removal of
  `EnvGuard`/`ENV_LOCK`.
- [x] (2026-11-23 17:45Z) ExecPlan drafted.
- [x] (2026-11-23 19:10Z) `EP-M1` Build-script seam: `BuildScriptContext`,
  `from_cargo_env`, `from_lookup` (with `FnMut` so an ordering test can record
  reads), `prepare_build_script(&context)`, and the migrated tests. The one
  compile failure found was the `Fn`/`FnMut` mismatch, fixed.
- [x] (2026-11-23 19:10Z) `EP-M2` Test-side seams written: metadata-resolved
  trybuild target and staging module, injected fixture target base with
  selector tests, and `HostEnv` with unit tests. Callers threaded; a missed
  caller in `entry_points/agent_context.rs` was found by the workspace check
  and fixed.
- [x] (2026-11-23 19:10Z) `EP-M3` Enforcement proof and documentation written:
  `tests/lint_environment_contract.rs`, two Clippy execution cases, ADR-0014,
  developers' guide, `AGENTS.md`, ADR-0012 addendum, indexes.
- [x] (2026-11-23 19:25Z) Focused tests: `theoremc-build-support` 38/38,
  `cargo-theorem --test entry_points` plus
  `theoremc --test lint_environment_contract` 23/23, and all six
  `lint_workspace_execution` cases including the two new Clippy execution
  cases. Trybuild staging verified by the 11 `.theorem` files present under
  `target/tests/trybuild/theoremc-macros/tests/expand/`.
- [x] (2026-11-23 19:30Z) `tools/check_adr_rfc_names.py` passes: 15 records and
  105 local links.
- [x] (2026-11-24) `make check-fmt`, `make check-adr-rfc-names`,
  `make markdownlint`, `make nixie`, `make spelling`: pass.
- [x] (2026-11-24) `make lint`: passes. Four Clippy findings were fixed on the
  way: `missing_const_for_fn` on `BuildScriptContext::new`, `shadow_reuse` on
  the fixture base and two contract-test bindings, and
  `needless_borrows_for_generic_args` plus `unnecessary_wraps` and
  `panic_in_result_fn` in the entry-point tests.
- [x] (2026-11-24) Rebased onto `origin/main` `32ab0d9`, which bumped
  `ortho_config` and `cargo-orthohelp` to 0.11.0 and **trybuild** to 1.0.125.
  trybuild 1.0.125 keeps the `<target>/tests/trybuild/<crate>/` project
  layout, so the metadata-resolved staging is unchanged. The macros suite
  re-ran green on 1.0.125 (9/9), including the eight new staging tests.
- [ ] `make test`: contention-limited on this host. The stock run reported
  1018 of 1019 tests run, with one fixture-crate BDD test
  (`existing_theorem_files_are_discovered_recursively`) terminated at nextest's
  exact 600 s ceiling while the shared compile queue held more than 400
  requests. Re-run with only that ceiling raised, all three cases in the binary
  passed (1506 s, 572 s, 614 s), and a full-suite re-run with the same raised
  ceiling reached 1037 of 1053 tests with **zero failures** before the harness's
  own two-hour cap. The remaining tests are the macro BDD scenarios, which are
  being re-run in isolation. `.config/nextest.toml` is untouched; the stock gate
  needs a quieter host to finish, not a code change.
- [ ] CodeRabbit review at each major milestone; concerns cleared.
- [ ] Push and open the draft PR.

## Surprises & discoveries

- Observation: Issues #98 and #99 already landed most of the mechanical
  groundwork before this branch existed. Evidence: `clippy.toml` already lists
  the six Netsuke entries plus `std::env::set_current_dir`;
  `[workspace.lints.clippy]` already denies `disallowed_methods`;
  `[workspace.lints.rust]` already forbids `unsafe_code`;
  `crates/test-helpers/src/lib.rs` no longer defines `EnvGuard` or `ENV_LOCK`.
  Impact: the remaining work is the seam reshaping, the enforcement tests, and
  the documentation, not the lint adoption itself.

- Observation: `docs/adrs/adr-0013-workspace-complete-lint-gate.md` already
  exists, so the next free ADR number is 0014, not 0013 as the coding plan
  expected. Evidence: `ls docs/adrs/`. Impact: the new record is
  `docs/adrs/adr-0014-environment-seam-boundaries.md`; the plan records the
  deviation rather than reusing a taken number.

- Observation: trybuild 1.0.122 resolves its project directory from
  `cargo metadata`'s `target_directory`, at
  `<target>/tests/trybuild/<crate-name>/`. Evidence:
  `~/.cargo/registry/src/.../trybuild-1.0.122/src/run.rs` lines 112–168.
  Impact: staging fixtures under that same path from a metadata-resolved target
  matches what the compiler sees, which is the behaviour the ticket requested.

## Decision log

- D1. Decision: keep the proc-macro's direct `CARGO_MANIFEST_DIR` read as a
  single closure-plus-reader seam (`manifest_dir_from_env` calling
  `manifest_dir_from_reader(env::var)`), rather than introducing a shared
  trait. Rationale: one variable, one caller, and the existing pure mapping is
  already tested for `NotPresent`, `NotUnicode`, and a valid path. Netsuke's
  taxonomy reserves `mockable::Env` for a growing group of variables shared by
  many tests. Date/Author: 2026-11-23, implementing agent (pre-existing shape
  from #234, retained).

- D2. Decision: represent the build-script inputs as `BuildScriptContext` with
  explicit `manifest_dir` and `out_dir` fields, constructed either from Cargo's
  environment (`from_cargo_env`, the single annotated read) or from a lookup
  closure (`from_lookup`, crate-private and test-only). Rationale: it gives
  discovery and suite generation an explicit input structure as the ticket
  requested without introducing a universal environment service. Rejected
  alternative: keep the loose `prepare_build_script_with_env` plus
  `prepare_build_script_at` pair, which passes two bare paths and does not name
  the boundary. Date/Author: 2026-11-23, implementing agent.

- D3. Decision: add `serde_json.workspace = true` to
  `crates/theoremc-macros` dev-dependencies to parse `cargo metadata` in the
  trybuild harness. `tempfile` is already a dev-dependency there. Rationale:
  the metadata parser is a dev-only contract; promoting a helper into
  `test-helpers` would re-add the macro crate dev-dependency that the ticket
  asked to remove. Date/Author: 2026-11-23, implementing agent.

- D4. Decision: leave `kani_is_required` in
  `tests/theorem_file_macro_bdd/cargo_runner.rs` as an annotated host-input
  root. Rationale: it reads one operator-set CI switch, has no downstream
  policy, and its caller is the generated BDD harness itself. The ADR records
  it as an approved host boundary beside `HostEnv`. Date/Author: 2026-11-23,
  implementing agent.

- D5. Decision: number the new ADR 0014.
  Rationale: 0013 is taken by the workspace-complete lint gate. Date/Author:
  2026-11-23, implementing agent.

- D6. Decision: use parameterized unit tests plus one live Clippy execution for
  verification; do not add model checking or formal proof. Rationale: the
  change maps finite input classes to explicit values and error variants, with
  no algebraic or concurrency invariant. Stronger methods would leave the
  third-party Cargo and Clippy interfaces unverified anyway. Date/Author:
  2026-11-23, implementing agent.

## Outcomes & retrospective

Pending implementation.

## Context and orientation

TheoremC is a Cargo workspace with a hybrid root manifest: `Cargo.toml`
declares both `[workspace]` and `[package]`. The affected components are:

- The root `build.rs`, which calls
  `theoremc_build_support::prepare_build_script()` and emits
  `cargo::rustc-check-cfg`, `cargo::rustc-cfg`, and `cargo::rerun-if-changed`
  directives from the returned `BuildScriptOutput`.

- `crates/theoremc-build-support/src/build_script.rs`, which today holds
  `prepare_build_script()` (annotated read boundary),
  `prepare_build_script_with_env(reader)` (private lookup seam), and
  `prepare_build_script_at(manifest_dir, out_dir)` (private explicit-path step).
  `BuildScriptError` carries `Environment`, `OpenOutputDirectory`,
  `Discovery`, and `Suite` variants.

- `crates/theoremc-macros/src/query.rs`, which resolves the invoking crate's
  manifest directory through `manifest_dir_from_env()` and the testable
  `manifest_dir_from_reader(reader)`.

- `crates/theoremc-macros/tests/expand.rs`, whose `stage_fixture` copies each
  `.theorem` fixture into the trybuild project under the host target directory,
  reading `CARGO_TARGET_DIR` directly with a fallback to
  `<manifest>/../../target`.

- `tests/common/fixture_isolation.rs`, whose `shared_target_dir()` appends
  `theoremc-fixture-builds` to `CARGO_TARGET_DIR` or the repository `target`,
  creates it, and canonicalizes it. `tests/common/fixture_crate.rs` consumes it.

- `crates/cargo-theorem/tests/entry_points/process.rs`, which reads `PATH`,
  `RUSTUP_HOME`, `HOME`, and `USERPROFILE` in three separately annotated helpers
  (`rustup_home`, `isolated_path`, `cargo_binary`) and builds `env_clear`
  child commands.

The lint policy lives in `clippy.toml` and `[workspace.lints]` in `Cargo.toml`.
The workspace-wide gate is `make lint`, which runs `make lint-rustdoc`,
`make lint-clippy`, and `make lint-whitaker` in order. `make test` runs
`cargo nextest run --workspace --all-targets --all-features` and then the
doctests. `.config/nextest.toml` groups compiler-spawning tests into
`nested-cargo-builds`, one at a time.

Terms used in this plan: a *composition root* is the one place where a process
reads the real environment; a *seam* is the injected value, closure, or trait
through which downstream logic receives that input; a *stub reader* is a test
double that returns supplied values or errors instead of touching the process.

## Conformance basis

- Issue [#101](https://github.com/leynos/theoremc/issues/101): environment
  boundary policy, validated ambient sites, and acceptance criteria.
- `leynos/netsuke` ADR-008 environment seam taxonomy
  (`docs/adr-008-environment-seam-taxonomy.md`): explicit values,
  `mockable::Env` for a growing shared group, `Arc` readers at `Send + Sync`
  registration boundaries, `Command::env_clear` for subprocess tests. The
  upstream document was not reachable from this environment; the taxonomy is
  followed as the issue describes it.
- `leynos/netsuke` `clippy.toml`: the six `disallowed-methods` entries and
  reasons.
- ADR-0008 workspace build-support boundary,
  `docs/adrs/adr-0008-workspace-build-support-boundary.md`.
- ADR-0012 shared warm target for fixture crates,
  `docs/adrs/adr-0012-shared-warm-target-for-fixture-crates.md`.
- ADR-0013 workspace-complete lint gate,
  `docs/adrs/adr-0013-workspace-complete-lint-gate.md`.
- `AGENTS.md` environment-injection and process-wide-lock rules.

Requirements and gaps:

- `REQ-no-mutation`: no test or helper calls `set_var` or `remove_var`.
- `REQ-annotated-roots`: direct `var`/`var_os`/`vars`/`vars_os` calls exist only
  at reasoned, item-scoped composition roots.
- `REQ-explicit-inputs`: build discovery and suite generation accept explicit
  environment-derived inputs.
- `REQ-macro-target`: macro tests supply target-directory state without ambient
  process mutation.
- `REQ-clippy-entries`: `clippy.toml` carries the six Netsuke entries.
- `REQ-lint-levels`: both workspace lint levels are denied/forbidden.
- `REQ-no-unsafe-exception`: `unsafe_code = "forbid"` passes with no crate-wide
  exception.
- `REQ-parallel-tests`: no environment mutex or serialized group is needed.
- `REQ-gate-enforcement`: Make and CI enforce the rule through the existing
  aggregate.

Trace links:

```plaintext
REQ-explicit-inputs -> EP-M1 -> build_script_tests::preparation_uses_explicit_context
REQ-macro-target -> EP-M2 -> expand::compile_errors
REQ-annotated-roots -> EP-M2 -> fixture_isolation_tests::shared_target_*
REQ-annotated-roots -> EP-M2 -> entry_points (HostEnv)
REQ-clippy-entries -> EP-M3 -> lint_environment_contract::*
REQ-lint-levels -> EP-M3 -> lint_environment_contract::*
REQ-gate-enforcement -> EP-M3 -> lint_workspace_execution::*
REQ-no-mutation, REQ-no-unsafe-exception, REQ-parallel-tests -> EP-M1..M3 -> all of the above
```

## Verification plan

### Abstract model

The change is a re-plumbing of inputs. Model each affected component as a pure
function `f(inputs) -> Result<output, error>` plus one boundary
`boundary() -> inputs` that reads the process environment. The obligations are
that `f` is total over the modelled input classes, that every error variant is
reproduced, and that the boundary is the only reader.

### Invariants and lemmas

- `INV-single-boundary`: each affected crate has exactly one function that
  performs a direct environment read, and that function carries a reasoned
  `#[expect]`.
- `INV-explicit-context`: `prepare_build_script` performs discovery, opens the
  output directory, and writes the suite using only the supplied
  `BuildScriptContext`; it never reads the process.
- `INV-lookup-order`: `BuildScriptContext::from_lookup` reads
  `CARGO_MANIFEST_DIR` before `OUT_DIR` and stops at the first failure.
- `INV-error-fidelity`: `NotPresent` and `NotUnicode` map to
  `BuildScriptError::Environment` with the offending variable name; the macro
  maps any failure to `MissingManifestDir`.
- `INV-target`: staged trybuild fixtures live under
  `<target_directory>/tests/trybuild/theoremc-macros/tests/expand/<name>`, where
  `target_directory` comes from the same `cargo metadata` the harness runs.
- `INV-fixture-base`: the fixture target is
  `<base>/theoremc-fixture-builds`, canonicalized once, where `base` is the
  injected value or the repository `target`.
- `INV-host-env`: child commands are built from explicitly captured host values
  and always run with `env_clear` plus explicit `env` entries.

### Axioms (trusted, not verified here)

- `AXIOM-cargo-metadata`: `cargo metadata --no-deps --format-version=1` emits a
  JSON object with a `target_directory` string; trybuild 1.0.122 resolves its
  project from that value.
- `AXIOM-clippy`: the pinned Clippy reads `clippy.toml`'s `disallowed-methods`
  and reports the configured reason; an item-scoped `#[expect]` satisfies it.
- `AXIOM-child-env`: `Command::env_clear` and `Command::env` affect only the
  child process, so subprocess tests may use the real environment as input.

### Methods per obligation

- Obligation: `INV-explicit-context`, `INV-lookup-order`, `INV-error-fidelity`.
  Method: `rstest` parameterized unit tests with stub readers and temporary
  manifest/output directories. Rationale: finite partitions (`NotPresent`,
  `NotUnicode`, valid) with observable error values. Artefact:
  `crates/theoremc-build-support/src/build_script_tests.rs`. Evidence: focused
  `cargo nextest run -p theoremc-build-support`; the lookup order test fails if
  the reader order is reversed. Non-vacuity: each case supplies values or a
  concrete `VarError`; a valid case proves the reader is reached at all, and
  the error cases prove the mapping.

- Obligation: `INV-target`.
  Method: a pure parser test over stub JSON (a valid document, a missing key,
  and malformed JSON) plus an integration staging test. Rationale: the parser
  is finite; the staging path is the contract trybuild relies on. Artefact:
  `crates/theoremc-macros/tests/expand.rs` and a colocated test module.
  Evidence: `cargo nextest run -p theoremc-macros --test expand`; removing a
  staged file makes a compile-pass case fail for the intended reason.
  Non-vacuity: the valid metadata case must produce a path; the missing-key
  case must fail; the staging test asserts the copied files exist and their
  contents match.

- Obligation: `INV-fixture-base`.
  Method: parameterized tests over absolute, relative, and absent bases.
  Rationale: three documented behaviours, each observable. Artefact:
  `tests/common/fixture_isolation_tests.rs`. Evidence:
  `cargo nextest run -p theoremc --test build_discovery_bdd` exercises the
  helper through fixtures; the unit tests cover the selector. Non-vacuity: the
  relative case proves canonicalization happens; the absent case proves the
  fallback is used.

- Obligation: `INV-host-env`.
  Method: unit tests over a constructed `HostEnv` and inspection of
  `Command::get_envs`. Rationale: the child environment is the observable
  contract. Artefact:
  `crates/cargo-theorem/tests/entry_points/process_tests.rs` (or a colocated
  module) plus the existing entry-point integration tests. Evidence:
  `cargo nextest run -p cargo-theorem --test entry_points`. Non-vacuity: at
  least one case with every field set and one with none set, plus a
  missing-`PATH` case that must return the documented error.

- Obligation: the lint contract and its enforcement.
  Method: text contract test over `clippy.toml` and `Cargo.toml`, plus two
  execution tests that run Clippy in a throwaway workspace. Rationale: text
  alone cannot prove Clippy acts; execution alone cannot prove the workspace
  baseline is set. Artefact: `tests/lint_environment_contract.rs` and two cases
  in `tests/lint_workspace_execution.rs`. Evidence: the execution test fails if
  the `clippy.toml` is not copied, and the `#[expect]` case fails if the
  expectation is malformed. Non-vacuity: the denied case observes exit failure
  and the configured reason in stderr; the expected case observes success; the
  clean control proves the workspace itself is buildable.

### Formal verification decision

No new invariant is algebraic, arithmetic, or concurrency-sensitive. The change
is input plumbing plus error mapping, so parameterized tests and one live
Clippy execution are proportionate. Model checking or proof would add no
coverage over the finite input classes and would leave the third-party
Cargo/Clippy interfaces unverified regardless. Recorded as D6 in the decision
log.

## Plan of work

### `EP-M1` Build-script seam

In `crates/theoremc-build-support/src/build_script.rs`, introduce
`BuildScriptContext`:

```rust
pub struct BuildScriptContext {
    manifest_dir: Utf8PathBuf,
    out_dir: Utf8PathBuf,
}

impl BuildScriptContext {
    #[must_use]
    pub fn new(manifest_dir: Utf8PathBuf, out_dir: Utf8PathBuf) -> Self;
    #[must_use]
    pub fn manifest_dir(&self) -> &Utf8Path;
    #[must_use]
    pub fn out_dir(&self) -> &Utf8Path;
    pub fn from_cargo_env() -> Result<Self, BuildScriptError>;
    fn from_lookup(
        read: impl Fn(&'static str) -> Result<String, env::VarError>,
    ) -> Result<Self, BuildScriptError>;
}
```

`from_cargo_env` is the single annotated read: `Self::from_lookup(env::var)`.
`from_lookup` reads `CARGO_MANIFEST_DIR` then `OUT_DIR`, stopping at the first
failure, and maps each failure to `BuildScriptError::Environment`.
`prepare_build_script` becomes `prepare_build_script(&BuildScriptContext)`:
discovery, then open `context.out_dir()`, then write the suite, in that order.
Delete `prepare_build_script_with_env`, `prepare_build_script_at`, and
`required_cargo_path`. Export `BuildScriptContext` from
`crates/theoremc-build-support/src/lib.rs`.

Change the root `build.rs` to call `BuildScriptContext::from_cargo_env()?` and
then `prepare_build_script(&context)?`, keeping the directive order.

Migrate `build_script_tests.rs` to construct contexts explicitly and to use
`from_lookup` with stub readers. Keep the two filesystem-failure tests, which
already pass explicit paths. Add cases for lookup order, short-circuiting,
`NotUnicode`, and an unrelated temporary manifest and output directory.

### `EP-M2` Test-side seams

In `crates/theoremc-macros/tests/expand.rs`, add a `target_directory()` helper
that runs `env!("CARGO") metadata --no-deps --format-version=1` in the current
working directory with the inherited environment, checks the exit status, and
parses `target_directory` with `serde_json`. Pass the resolved `Utf8PathBuf`
and the fixture name into `stage_fixture`. Stage the eleven `.theorem` fixtures
under `<target>/tests/trybuild/theoremc-macros/tests/expand/<name>` before
registering the trybuild cases. Add unit tests for the JSON parser: valid,
missing key, and malformed.

In `tests/common/fixture_isolation.rs`, split `shared_target_dir` into a pure
`fixture_target_dir(base: Option<Utf8PathBuf>)` selector that appends
`theoremc-fixture-builds`, creates it, and canonicalizes it, plus one annotated
`ambient_target_base()` that reads `CARGO_TARGET_DIR`. `shared_target_dir`
becomes the composition of the two. Add unit tests for absolute, relative, and
absent bases.

In `crates/cargo-theorem/tests/entry_points/process.rs`, add
`HostEnv::capture()` reading `PATH`, `RUSTUP_HOME`, `HOME`, and `USERPROFILE`
once with one reasoned `#[expect]`. Change `rustup_home`, `isolated_path`,
`cargo_binary`, and `isolated_command` to take `&HostEnv`. Update
`crates/cargo-theorem/tests/entry_points.rs` to capture once per test. Add unit
tests over constructed `HostEnv` values and inspect `Command::get_envs`.

### `EP-M3` Enforcement proof and documentation

Add `tests/lint_environment_contract.rs`, which reads `clippy.toml` and
`Cargo.toml` as text and asserts the six entries, their reasons, and both
workspace lint levels. Add two `#[cfg(unix)]` cases to
`tests/lint_workspace_execution.rs` that copy `clippy.toml` into a throwaway
workspace, declare both lint levels, remove inherited `CLIPPY_CONF_DIR`, and
prove a direct `std::env::var` fails with the configured reason while a reasoned
`#[expect]` passes.

Create `docs/adrs/adr-0014-environment-seam-boundaries.md` with status, date,
context, decision, and consequences. Map `BuildScriptContext`, the
metadata-resolved trybuild target, the injected fixture base, and `HostEnv` to
explicit values; the macro mapping to a narrow closure for one variable and one
caller; `env_clear` child commands to isolated child processes; and `MapEnv`
CLI sources to the existing shared configuration source. Record the upstream
ADR's unreachability.

Add an environment-boundaries rule, the `#[expect]` convention, and the
approved roots to `docs/developers-guide.md`. Update `AGENTS.md` to forbid
parent-process environment mutation and point to the ADR. Index ADR-0014 in
`docs/contents.md` and `docs/repository-layout.md`.

## Concrete steps

Working directory: the worktree root.

```plaintext
Red:  cargo nextest run -p theoremc-build-support \
        --test build_script  # (unit tests compile with the crate)
Green: cargo nextest run -p theoremc-build-support
Focused macro: cargo nextest run -p theoremc-macros --test expand
Focused CLI: cargo nextest run -p cargo-theorem --test entry_points
Focused lints: cargo nextest run -p theoremc --test lint_environment_contract \
                 --test lint_workspace_execution
Gates, in order: make check-fmt; make lint; make test; make markdownlint; \
                 make nixie; make spelling
```

Capture long gate output with `tee` under `/tmp` and review it afterwards.

## Validation and acceptance

- `make check-fmt` passes.
- `make lint` passes with the six disallowed methods denied and no `allow`.
- `make test` passes; the new unit and integration cases run in the default
  parallel test group (no serialization).
- `make markdownlint`, `make nixie`, and `make spelling` pass.
- `tests/lint_workspace_execution.rs` proves a direct `std::env::var` fails
  under the copied `clippy.toml` and a reasoned `#[expect]` passes.
- `grep -rn 'env::set_var\|env::remove_var' --include=*.rs .` returns nothing.
- `grep -rn 'env::var' --include=*.rs .` returns only the annotated composition
  roots named in ADR-0014.

## Idempotence and recovery

Every step edits tracked files and is safe to re-run. Fixture staging recreates
directories with `create_dir_all` and overwrites files. Temporary directories
are removed by `tempfile`. If a gate fails, read the captured log, fix the
cited cause, and re-run only that gate before continuing.

## Artefacts and notes

Reconnaissance evidence: `git log --oneline main..HEAD` shows #98 and #99 work
already merged; `clippy.toml` contains the six entries; `test-helpers` no
longer defines `EnvGuard`.

## Interfaces and dependencies

New public interface in `theoremc-build-support`:

```rust
pub struct BuildScriptContext { /* manifest_dir, out_dir */ }
impl BuildScriptContext {
    pub fn new(manifest_dir: Utf8PathBuf, out_dir: Utf8PathBuf) -> Self;
    pub fn manifest_dir(&self) -> &Utf8Path;
    pub fn out_dir(&self) -> &Utf8Path;
    pub fn from_cargo_env() -> Result<Self, BuildScriptError>;
}
pub fn prepare_build_script(
    context: &BuildScriptContext,
) -> Result<BuildScriptOutput, BuildScriptError>;
```

Private test-only interfaces: `BuildScriptContext::from_lookup`,
`HostEnv::capture`, `fixture_target_dir`, and the trybuild metadata parser.
Authorized dependency changes: `serde_json.workspace = true` added to
`crates/theoremc-macros` dev-dependencies. No runtime dependency changes.

Reuse policy: `BuildScriptContext` owns the build-script input boundary;
`HostEnv` owns the `cargo-theorem` test host boundary; the fixture target
selector owns the warm-target path decision. No parallel environment reader may
be added outside ADR-0014's approved roots.

## Revision note

- 2026-11-23: Initial draft after reconnaissance. Recorded that #98/#99 had
  already landed the lint baseline, so the remaining work is the seam shaping,
  enforcement tests, and documentation. Renumbered the planned ADR to 0014.
