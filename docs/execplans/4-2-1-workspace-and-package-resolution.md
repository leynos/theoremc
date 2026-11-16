# Task 4.2.1: Workspace and package resolution through `cargo metadata`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting design review and user approval)

Branch: `4-2-1-workspace-and-package-resolution`

Roadmap item: `4.2.1` in `docs/roadmap.md`.

## Purpose / big picture

`cargo theorem` is theoremc's Cargo external subcommand. Cargo runs
`cargo-theorem theorem <args>` from the caller's current directory. It does not
say which workspace or which package the caller means. Every later project
command needs that answer. These are `list`, `get`, `check`, `generate`,
`init`, the scaffolding commands, and `run`. Today the application assumes the
project root is `"."` (`crates/cargo-theorem/src/cli.rs`, `execute`). It has no
way to name a package.

This task adds one reusable resolution service. It turns four Cargo-style
selectors into a deterministic, bounded, ordered set of workspace members:

- `--manifest-path <path>` chooses the `Cargo.toml` that anchors resolution.
- `--package <spec>`, which may be repeated, chooses named workspace members.
- `--workspace` chooses every workspace member.
- `--exclude <spec>`, which may be repeated and requires `--workspace`, removes
  members from a workspace selection.

The service reads the workspace through the stable `cargo metadata` JSON
protocol (format version 1) using the `cargo_metadata` crate. A *package
specification* (spec) is the string a caller types to name a package. A
*workspace member* is a package that Cargo lists in `workspace_members`.
*Default members* are the members Cargo itself would select when no selector is
given. Cargo reports these in `workspace_default_members`, computed relative to
the current directory or the explicit manifest.

Consider this fixture, a virtual workspace (a root `Cargo.toml` with
`[workspace]` but no `[package]`):

```plaintext
fixture/
  Cargo.toml          [workspace] members = ["app", "foo", "foo_bar", "foo-bar"],
                      exclude = ["vendor/foo"], default-members = ["app", "foo"]
  app/                package app 0.1.0, depends on vendor/foo by path
  foo/                package foo 0.1.0
  foo_bar/            package foo_bar 0.1.0
  foo-bar/            package foo-bar 0.1.0
  vendor/foo/         package foo 0.2.0, not a workspace member
```

After the change, a later command can call the service. From `fixture/` with no
selectors it receives `app` and `foo` in name order. With `--package foo` it
receives the member `foo 0.1.0`, never the excluded `vendor/foo`. With
`--workspace --exclude foo-bar` it receives `app`, `foo` and `foo_bar`. With
`--package fop` it receives a usage diagnostic with code `package.unknown`.
That diagnostic enumerates the valid package identifiers `app`, `foo`,
`foo-bar` and `foo_bar`. A command that needs exactly one package, run from
`fixture/` without selectors, receives a `selection.ambiguous` diagnostic. It
enumerates `app` and `foo` and tells the caller to pass `--package`.

No command is registered with these selectors in this task. The command line
users see does not change. `cargo theorem context --json` stays byte-for-byte
equivalent and still advertises only `context`. The first consumers arrive with
roadmap tasks 4.2.3 (`list`, `get`, `action list`, `action get`) and 4.2.4
(`check`). They flatten the selector arguments into their own argument structs
and map the structured diagnostics through the JSON envelope delivered by
4.1.4. The service is therefore delivered as a documented, tested, crate-public
library surface of the `cargo-theorem` package (see Decision D3). Its
observable contract is exercised through that public boundary against the real
`cargo` executable.

This task closes the open questions that the design leaves implicit in
`docs/cargo-theorem-cli-design.md` §6.4:

- what a package identifier is;
- which exit class an ambiguous or unknown package uses;
- what "eligible members" means;
- how `cargo metadata` runs within bounded time and output; and
- which Cargo behaviours theoremc keeps or deliberately tightens.

These answers are recorded in a new ADR and in §6.4.

## Constraints

- Implementation begins only after the user approves this plan.
- Selection must use `cargo metadata --format-version=1 --no-deps`, parsed
  through the `cargo_metadata` crate (design §6.4). Do not parse `Cargo.toml`
  directly to discover members. Do not invoke `cargo pkgid`, which resolves
  dependencies and may write `Cargo.lock`.
- Resolution is read-only. It must not create, modify, or delete any file in
  the inspected project, including `Cargo.lock` and `target/`. It must not
  trigger network access. That includes rustup toolchain auto-installation (see
  Decision D8).
- Processes are created only through argument vectors (design §16). There is
  no shell and no string interpolation. Every child process has a deadline,
  bounded retained stdout and stderr, and guaranteed reaping (AGENTS.md
  "Architecture and operational boundaries").
- Production code must not read the process environment except through the
  existing injected `ortho_config::EnvSource` seam. `clippy.toml` already denies
  `std::env::var`, `var_os`, `set_var`, and `set_current_dir`. Tests must not
  mutate the process environment. Child processes may be configured through
  `Command::env`.
- The selectors are command-line inputs, not configuration values (design
  §6.5). They must not become OrthoConfig fields or `THEOREMC_*` variables.
- Do not register the selectors on any command, and do not change the agent
  context, in this task (design §6.6: never advertise unimplemented handlers).
- Keep OrthoConfig, Clap, `cargo_metadata`, and process code out of the facade
  and the reusable library crates (`theoremc`, `theoremc-core`,
  `theoremc-macros`, `theoremc-build-support`) (ADR-0009, design §4.1).
- Every file stays under 400 lines. Every module starts with a `//!` comment.
  Every public item has Rustdoc with an example. Doctests use the fake runner or
  `no_run` so they never spawn Cargo.
- Lints are not silenced except as a documented last resort with item-scoped
  `#[expect(..., reason = "...")]`. Do not use `.expect()` or `.unwrap()`
  outside tests. Every assertion carries a diagnostic message.
- Error types use `thiserror`. Large payloads are boxed. No `eyre` or `anyhow`
  appears in library-facing signatures.
- Workspace MSRV is Rust 1.89 (ADR-0010). `cargo_metadata` 0.23.1 declares
  Rust 1.86, so it is compatible. `cargo-util-schemas` declares Rust 1.97 and
  is excluded for that reason (Decision D5).
- Completed ExecPlans are historical and must not be edited.

## Tolerances (exception triggers)

The comparison base is `origin/main` at `4c05966`. Count with
`git diff --numstat origin/main...HEAD`, excluding
`docs/execplans/4-2-1-workspace-and-package-resolution.md` and `Cargo.lock`.
`Cargo.lock` is generated. It is checked only for unexpected new crates, see
the Dependencies trigger. Limits measure net lines (additions minus deletions).
Classify by whole file:

- *Code* is `*.rs` files, `*.feature` files, and test data under
  `crates/cargo-theorem/tests/`.
- *Configuration* is `Cargo.toml` files and `.config/nextest.toml`.
- *Documentation* is `docs/**/*.md` except this plan.

The survey behind these numbers is in `Context and orientation`.

- Files: stop and escalate if more than 40 non-ExecPlan files change.
- Code: stop and escalate if code exceeds 2,900 net lines. The estimate is
  about 1,150 production lines and 1,450 test lines.
- Configuration: stop and escalate if configuration exceeds 60 net lines. The
  estimate is about 30.
- Documentation: stop and escalate if documentation exceeds 600 net lines. The
  estimate is about 420.
- Interface: stop and escalate if any public interface beyond the
  `cargo_theorem::workspace` and `cargo_theorem::process` modules described in
  `Interfaces and dependencies` is needed. The same applies to any change to
  existing CLI flags, the agent context, or the JSON envelope.
- Dependencies: stop and escalate if a dependency beyond the authorized set is
  needed, or if `Cargo.lock` gains a crate that is not already locked. The
  authorized set is:
  - runtime: `cargo_metadata`, `camino`, `semver`;
  - development: `googletest`, `pretty_assertions`, `insta`, `proptest`,
    `rstest-bdd`, `rstest-bdd-macros`.
- Architecture: stop and escalate if the work must spawn any process outside
  `cargo_theorem::process`. The same applies if it must place Cargo or process
  code in a library crate, or if it must contradict ADR-0009 or design §§6.3 to
  6.6.
- Iterations: stop and escalate if a focused test still fails after five
  fix attempts.
- Time: stop and escalate if any milestone takes more than eight working
  hours.
- Ambiguity: stop and present options if Cargo's observed behaviour
  contradicts an axiom in `Verification plan`.

Enforce each category independently. Spare capacity in one category never
offsets another.

## Risks

- Risk: `ortho_config` 0.10.0's `OrthoConfig` derive may reject a
  `#[command(flatten)]` field of a non-OrthoConfig `clap::Args` struct, even
  when it carries `#[serde(skip)]`. If so, later commands cannot flatten the
  selector arguments as planned. Severity: medium. Likelihood: medium.
  Mitigation: EP-M3 compiles a test-only command struct that derives
  `OrthoConfig` and flattens `WorkspaceSelectorArgs`. If the derive refuses,
  the fallback is to expose `WorkspaceSelectorArgs::augment` through Clap's
  `Args` trait so a hand-written command can merge it. Record the result in
  `Surprises & discoveries`.
- Risk: the deadline and bounded-capture runner can deadlock or leak a child
  when a process fills one pipe while the runner waits on the other. Severity:
  high. Likelihood: medium. Mitigation: drain both pipes on dedicated threads,
  apply the deadline to the whole operation, and kill and reap on any early
  exit. EP-M2 tests cover stdout overflow, stderr flooding, a hanging child,
  and a missing program, using real `sh` children on Unix.
- Risk: a grandchild process inherits the pipes and keeps them open after the
  child is killed. The reader threads then never see end-of-file. Severity:
  medium. Likelihood: low. The strace probe shows `cargo metadata --no-deps`
  executes only the rustup proxy and `cargo`. Mitigation: reader joins are also
  bounded by the deadline. Threads still blocked after the kill are detached,
  and the runner returns `TimedOut`. Detach is the documented cleanup policy
  because standard library pipes cannot be interrupted. Record the residual
  risk in the developers' guide.
- Risk: `cargo_metadata` enables `serde_json`'s `unbounded_depth` feature
  through feature unification. The CLI's JSON parsing then loses its recursion
  limit. Severity: low. Likelihood: high. The feature will be enabled.
  Mitigation: retained stdout is capped at 16 MiB before parsing. Cargo's TOML
  parser bounds nesting in `package.metadata`. The input originates from local
  Cargo, not the network. Record this in ADR-0014.
- Risk: the `CARGO` environment variable is absent when `cargo-theorem theorem`
  is run directly. `cargo` then resolves through the rustup proxy, which may
  honour the inspected project's `rust-toolchain.toml` and install a toolchain.
  Severity: medium. Likelihood: medium. Mitigation: set `RUSTUP_AUTO_INSTALL=0`
  on the child only (Decision D8). A missing toolchain then becomes an
  `external_tool_failure` diagnostic rather than a download.
- Risk: real-Cargo fixture tests become slow or contend for Cargo locks under
  nextest. Severity: low. Likelihood: low. Metadata without dependencies
  compiles nothing and wrote nothing in probes. Mitigation: place the
  integration binary in the existing `nested-cargo-builds` nextest group, as
  `docs/developers-guide.md` §3.3 requires for tests that start a child `cargo`.
- Risk: roadmap 4.1.4 (JSON envelope) is not complete. The structured
  diagnostics therefore have no CLI renderer yet. Severity: low. Likelihood:
  certain. Mitigation: the diagnostic carries every envelope field from design
  §8.3 (`code`, `class`, `message_fallback`, `arguments`, `valid_values`).
  Insta snapshots fix its shape so 4.1.4 and 4.2.3 can render it without
  re-deriving data.

## Progress

- [x] (2026-11-16 19:05Z) Reconnaissance: Wyvern survey of the crate, tooling,
  and docs; Cargo behaviour probes; external research on `cargo_metadata`,
  package ID specifications, and rustup.
- [x] (2026-11-16 19:40Z) Initial draft of this plan.
- [ ] Design review by the expert panel applied.
- [ ] Plan approved by the user.
- [ ] `EP-M1` Decision records and the pure selection kernel.
- [ ] `EP-M2` Bounded process runner and the `cargo metadata` adapter.
- [ ] `EP-M3` Resolution service, selector arguments, and real-Cargo fixtures.
- [ ] `EP-M4` Guides, roadmap closure, and delivery.

## Surprises & discoveries

- Observation: Cargo refuses a workspace that contains two members with the
  same package name. Evidence: probe 1 in `Artefacts and notes`. Both
  `cargo metadata` and `cargo check -p foo` fail with
  ``two packages named `foo` in this workspace``. Impact: a bare name can never
  be ambiguous among members. The fixtures for "ambiguous names" therefore
  cover the cases that remain. These are a member and a non-member sharing a
  name, names that differ only by `-` versus `_`, and single-target commands
  facing several default members (Decision D2).
- Observation: `workspace_default_members` already reflects Cargo's default
  selection relative to the current directory or explicit manifest. Evidence:
  probe 3. From inside member `b`, or with `--manifest-path b/Cargo.toml`, the
  default is `[b]`. From a virtual root with `default-members = ["a"]`, it is
  `[a]`. Without `default-members`, it is every member. From a non-virtual
  root, it is the root package. Impact: theoremc does not re-implement Cargo's
  default-selection rules. It trusts that field (AXIOM-default-members).
- Observation: `cargo metadata --no-deps` creates no `Cargo.lock` and no
  `target/`, and starts no `rustc`. Evidence: probes 3 and 5. The `find`
  snapshot is unchanged, and the strace shows only `execve` of the rustup proxy
  and `cargo`. Impact: read-only resolution is achievable with `--no-deps`. A
  regression that drops `--no-deps` is detectable by the creation of
  `Cargo.lock` in a fixture with a path dependency.
- Observation: Cargo only warns for an `--exclude` that matches nothing, and
  rejects `--exclude` without `--workspace`. Evidence: probe 2. Impact:
  Decision D4 tightens the first behaviour to an error and keeps the second.

## Decision log

- D1. Decision: the pure selection kernel, the `cargo metadata` adapter, and
  the bounded command runner live in the `cargo-theorem` package as the
  feature-grouped modules `workspace` and `process`. Rationale: design §4.1
  keeps Cargo-metadata and process-control dependencies out of the facade.
  Design §5 lets project and runner services begin as feature-grouped modules
  if they respect dependency direction. The planned `theoremc-build` library
  (roadmap 4.3.1) is a build dependency of every consumer. Putting
  `cargo_metadata` there would compile it into every downstream build. The
  strongest alternative was a new `theoremc-project` crate. It was rejected
  because no second package consumes selection, and a new crate adds MSRV,
  layout, and publishing surface without a consumer. Extraction trigger: move
  both modules to a library crate when a package other than `cargo-theorem`
  needs them. Date/Author: 2026-11-16, planning agent.
- D2. Decision: a package specification is one of four forms:
  - `name`;
  - `name@version`;
  - `name:version`, where the version may be partial (`1`, `1.4`, or `1.4.3`
    with optional pre-release); or
  - a complete package ID exactly as `cargo metadata` prints it.

  Matching is exact against workspace members only. There is no case folding, no
  `-`/`_` normalization, no prefix matching, and no matching of non-member
  packages. A spec containing `*`, `?`, or `[` is rejected as
  `package.glob_unsupported`. "Valid package identifiers" in diagnostics are
  member names, because Cargo guarantees they are unique within a workspace.
  Rationale: this follows the Cargo package ID specification grammar for the
  forms agents actually type. Exactness makes the `foo-bar`/`foo_bar` and
  member/non-member cases deterministic. Partial URL specs (for example
  `file:///path/foo` without `#version`) are deferred because they require
  parsing opaque IDs, which Cargo documents as not stable. Date/Author:
  2026-11-16, planning agent.
- D3. Decision: expose the modules as `pub mod workspace` and `pub mod process`
  from `crates/cargo-theorem/src/lib.rs`. Document that this visibility serves
  later in-crate commands and the crate's own tests. It does not establish a
  downstream contract. This mirrors ADR-0008's statement about
  `theoremc-build-support`. Rationale: no registered command consumes selection
  in this task, so private items would fail `dead_code` under `-D warnings`.
  The alternatives were `#[expect(dead_code)]` on unfinished code, or
  registering a placeholder command that design §6.6 forbids. The public
  boundary also lets the integration and BDD suites exercise the promise where
  it is owned. Date/Author: 2026-11-16, planning agent.
- D4. Decision: selection rules.
  - No selector yields Cargo's default members.
  - `--package` may repeat. Its result is the union of the matches, sorted by
    name and deduplicated.
  - `--workspace` yields every member. In this task every member is eligible;
    later commands may narrow eligibility explicitly.
  - `--exclude` requires `--workspace`.
  - `--package` conflicts with `--workspace`.
  - Every spec that matches no member is an error, including `--exclude` specs.
    Cargo only warns for those.
  - An empty final selection is an error.
  - Every unmatched spec is reported in command-line order.

  Rationale: an agent-native CLI should fail closed on typos. A silently
  ignored exclusion would widen a later mutation. The stricter rule is
  documented as a deliberate difference from Cargo in ADR-0014 and the CLI
  user's guide. Date/Author: 2026-11-16, planning agent.
- D5. Decision: parse specs and partial versions with a small in-crate parser
  built on `semver`, rather than `cargo-util-schemas::PackageIdSpec`. Rationale:
  `cargo-util-schemas` 0.14.3 declares Rust 1.97, above the workspace MSRV of
  1.89. It also pulls in `toml`, `url`, `jiff`, and other dependencies for a
  grammar subset under 150 lines. Date/Author: 2026-11-16, planning agent.
- D6. Decision: exit classes and diagnostic codes, mapped onto design §8.4:
  - usage (2):
    - `package.unknown`, `package.glob_unsupported`, and
      `package.invalid_spec`;
    - `selection.conflict`, `selection.empty`, and `selection.ambiguous`;
    - `manifest.not_found` and `manifest.invalid_name`;
  - project state (4): `workspace.not_found` and `workspace.metadata_failed`,
    the latter when Cargo ran but exited non-zero, as for an invalid manifest
    or duplicate member names;
  - external tool failure (8): `cargo.unavailable`, `cargo.timed_out`,
    `cargo.output_limit`, `cargo.metadata_invalid`, and
    `cargo.unsupported_version`, the last when `workspace_default_members` is
    absent because Cargo is older than 1.71.

  Diagnostics enumerate at most 50 choices, matching the default list limit in
  design §8.5. They also report the total count in `arguments`. Rationale:
  design §8.3's own example classifies an unknown enumerated value as `usage`
  with `valid_values`. A `cargo` exit status does not distinguish a broken
  manifest from a Cargo crash without parsing localized stderr. Theoremc does
  not parse stderr. It classifies a completed Cargo run that exits non-zero as
  project state, and a failure to obtain a well-formed result as an external
  tool failure. Date/Author: 2026-11-16, planning agent.
- D7. Decision: add a minimal synchronous `process::CommandRunner` port with
  one production adapter, `SystemCommandRunner`. It covers argument vectors,
  the working directory, environment overrides, a deadline, fatal stdout
  limits, truncating stderr limits, and kill-and-reap cleanup. Protected inputs
  (design §12.1) are not part of this task. Backend work (roadmap 4.4) extends
  this same port rather than adding a second one. Rationale:
  `cargo_metadata::MetadataCommand::exec` uses `Command::output`, which has no
  deadline and no output bound. Design §12.1 and the developers' guide §3.8
  require one shared runner that owns process creation, timeouts, bounded
  capture, and test doubles. This is the first production process in the
  application, so it introduces that port at its smallest useful size. The
  adapter uses `MetadataCommand::parse` on the bounded stdout. Date/Author:
  2026-11-16, planning agent.
- D8. Decision: the `cargo metadata` invocation contract.
  - Program: the injected `CARGO` value, which Cargo sets for external
    subcommands, falling back to `cargo`.
  - Argument vector: `metadata --format-version=1 --no-deps --offline`, plus
    `--manifest-path <path>` when explicit.
  - Working directory: the caller's start directory.
  - Child-only environment: `RUSTUP_AUTO_INSTALL=0`.
  - Deadline: 120 seconds.
  - Stdout: at most 16 MiB retained; exceeding it is fatal.
  - Stderr: at most 64 KiB retained; excess is truncated and counted.

  Rationale: `--no-deps` makes the call read-only (see probes). `--offline` is
  a defensive no-network assertion. The limits follow the per-process stream
  bound in design §8.6. Rustup 1.28.1 documents `RUSTUP_AUTO_INSTALL=0` as the
  opt-out from automatic toolchain installation. Date/Author: 2026-11-16,
  planning agent.
- D9. Decision: defer `tracing` spans and `metrics` emission for project
  resolution to roadmap 4.5.4, which owns the metric registry and structured
  events. Rationale: the workspace has no `tracing` or `metrics` dependency and
  no subscriber or recorder in the application. AGENTS.md requires that new
  telemetry be shown to produce a non-zero signal through the collection path,
  and that path does not exist yet. Adding uncollected spans would be
  unverifiable. The service returns structured outcomes, such as member and
  selection counts and stderr truncation, that 4.5.4 can instrument at the
  application boundary. Date/Author: 2026-11-16, planning agent.
- D10. Decision: no Verus proof and no Kani harness for this task. Verification
  uses:
  - named `rstest` tables;
  - a bounded exhaustive test against an independent bitmask oracle;
  - `proptest` properties; and
  - integration and BDD tests against real Cargo.

  Rationale: see `Formal verification decision`. Date/Author: 2026-11-16,
  planning agent.
- D11. Decision: the selector arguments are long options only, matching design
  §6.4 and the user's guide. Cargo's `-p` short form is not added in this task.
  Rationale: the design lists long forms only. A short flag would reserve `-p`
  across every future project command. That choice belongs to the commands that
  adopt the selectors. Date/Author: 2026-11-16, planning agent.

## Outcomes & retrospective

Not started. Complete at each milestone boundary and at completion.

## Context and orientation

The inspected base revision is `4c05966`
(`Chore(deps): bump rust-toolchain (#255)`). The workspace members are
`crates/test-helpers`, `crates/theoremc-build-support`, `crates/cargo-theorem`,
`crates/theoremc-core`, and `crates/theoremc-macros`. The root `theoremc`
package is also a member. The toolchain is `nightly-2026-11-16`
(`rust-toolchain.toml`), and the declared MSRV is 1.89.

### The application package today

`crates/cargo-theorem/` builds the `cargo-theorem` executable.

- `src/main.rs` calls `cargo_theorem::run(std::env::args_os())`.
- `src/lib.rs` declares private modules `agent_context`, `cli`, `config`, and
  `error`, and re-exports only `run`.
- `src/cli.rs` (299 lines) defines the Clap tree:
  - `CargoEntry` → `CargoCommand::Theorem(TheoremArgs)`.
  - `TheoremArgs` flattens `config::GlobalOptions` and adds `--profile`.
  - `Commands` has one variant, `Context(ContextArgs)`.
- `run_with_sources_and_io` parses with `ortho_config::parse_localized_command`.
- `execute` resolves configuration with
  `ortho_config::load_globals_and_merge_selected_subcommand_with_sources`,
  passing `let project_root = Path::new(".")`.
- `src/cli_diagnostics.rs` (178 lines) renders Clap and application errors.
  The JSON diagnostic has `schema_version`, `kind`, `command`, `status`, and
  `error.{code, class, message_fallback}`. It does not yet render `arguments` or
  `valid_values`; that is roadmap 4.1.4.
- `src/error.rs` defines `ApplicationError` with three variants:
  - `Configuration`, which exits 3;
  - `Serialization`, which exits 10;
  - `Output`, which exits 10.

  Exit classes 4 to 9 are not yet produced.
- `src/config/mod.rs` exposes `process_sources()`, which returns the injected
  `ortho_config::SharedEnvSource` and `SharedScanEnvSource`. Tests inject
  `ortho_config::MapEnv`.

No production code in the workspace spawns a process. Nothing depends on
`cargo_metadata` directly. `Cargo.lock` already contains
`cargo_metadata 0.23.1` (via `rstest-bdd-harness`) and 0.18.1 (via
`cargo-orthohelp`), together with `camino 1.2.6` and `semver 1.0.28`.
`cargo-orthohelp` 0.10.0 has a private `metadata.rs` with exact-name package
lookup, but it is not reusable because the module is private to its binary.

### Tests and tooling

- The crate's tests are:
  - `src/cli_tests.rs` and `src/cli_tests/runner.rs`, in-process with
    `MapEnv`;
  - `src/config_tests.rs`;
  - `tests/entry_points.rs` with `tests/entry_points/{process,support,
    agent_context}.rs`, which run the built binary through an isolated `PATH`
    and `env_clear()`.
- Its dev-dependencies are `anyhow`, `cap-std`, `rstest`, and `tempfile`.
- Across the workspace:
  - `rstest-bdd` 0.6.0 is used with `.feature` files under the root
    `tests/features/` and binding files `tests/<name>_bdd.rs`, for example
    `tests/build_discovery_bdd.rs`.
  - `insta` is used only in `theoremc-macros`.
  - `googletest`, `pretty_assertions`, and `proptest` are used in the root,
    `theoremc-core`, and `theoremc-macros`.
- `.config/nextest.toml` defines the `nested-cargo-builds` group with
  `max-threads = 1` for tests that start a child `cargo`. The group is assigned
  by `binary(...)` filters.
- Gates (Makefile):
  - `make check-fmt` runs `cargo fmt --all -- --check` and `mdtablefix`.
  - `make lint` runs, in order:
    - rustdoc with `-D warnings`;
    - Clippy, with every target and feature and `-D warnings`;
    - Whitaker.
  - `make test` runs `cargo fetch --locked`, `cargo nextest run --workspace
    --all-targets --all-features`, and doctests.
  - Also available: `make markdownlint`, `make nixie`, `make spelling`, `make
    msrv`, and `make check-agent-native`.
- There is no Verus tooling in the repository. Kani runs only through
  `make kani` against generated harnesses.

### Documentation to update

- `docs/cargo-theorem-cli-design.md` §6.4 is the normative selection
  contract; §5 holds the package responsibilities.
- `docs/cargo-theorem-cli-users-guide.md`, lines 65 to 79, describes the
  selectors in the present tense while also saying project commands are not yet
  accepted. Fix that inconsistency.
- `docs/developers-guide.md` §3.8 is the application boundary section and acts
  as the component architecture document for `cargo-theorem`. §3.3 covers test
  conventions.
- ADRs live in `docs/adrs/`. The next number is 0014.
- `docs/contents.md` and `docs/repository-layout.md` index documents and
  crates.
- `docs/users-guide.md` covers the library and macro only. It needs no change
  because no library or command behaviour changes (see EP-M4).

### Skills and references for the implementer

Load `rust-router` first. Then load:

- `rust-types-and-apis` for the spec and selection types;
- `rust-errors` for the error enums;
- `rust-unit-testing` for rstest, googletest, and insta;
- `proptest`;
- `hexagonal-architecture` for the port and adapter split;
- `domain-cli-and-daemons` for the process runner;
- `arch-decision-records` for ADR-0014;
- `execplans` to keep this plan current.

Repository references:

- `docs/rust-testing-with-rstest-fixtures.md`;
- `docs/rstest-bdd-users-guide.md`;
- `docs/reliable-testing-in-rust-via-dependency-injection.md`;
- `docs/rust-doctest-dry-guide.md`;
- `docs/complexity-antipatterns-and-refactoring-strategies.md`;
- `docs/ortho-config-users-guide.md`;
- `docs/developers-guide.md`;
- `docs/theoremc-design.md` (DES);
- `docs/theorem-file-specification.md` (TFS), for context only. Theorem files
  are not read in this task.

### Budget survey

The production estimate is about 1,150 lines:

| Area                       | Estimated lines |
| -------------------------- | --------------- |
| Spec parsing               | 180             |
| Model and translation      | 200             |
| Selection kernel           | 180             |
| Errors and diagnostics     | 200             |
| Cargo adapter              | 150             |
| Selector arguments         | 70              |
| Runner                     | 220             |
| Exit-class enum and wiring | 60              |

The test estimate is about 1,450 lines:

| Area                                | Estimated lines |
| ----------------------------------- | --------------- |
| Spec tables                         | 200             |
| Kernel tables and exhaustive oracle | 330             |
| Properties                          | 160             |
| Fake-runner adapter tests           | 180             |
| Runner tests                        | 180             |
| Fixture builder                     | 120             |
| Integration tests                   | 220             |
| BDD steps                           | 60              |

Comparable plans give the history:

- `docs/execplans/3-1-1-build-rs-scanning-of-theorems.md` set "roughly 10
  changed files or 600 net lines" and never revised it.
- `docs/execplans/3-1-2-generate-out-dir-theorem-suite-rs.md` set "roughly 12
  changed files or 700 net lines" and never revised it.
- `docs/execplans/4-1-2-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli.md`
  set no line budget. Its only size issue was a 400-line breach in `cli.rs`,
  fixed by splitting tests.

No plan shows a history of repeated increases. This task is about three times
larger than 3.1.x because it adds a process boundary and an exhaustive oracle.
The code limit therefore carries about 12% contingency over the estimate.

## Conformance basis

Upstream artefacts and the revision inspected (`4c05966`):

- `docs/roadmap.md` task 4.2.1 and step 4.2.
- `docs/cargo-theorem-cli-design.md` §§4.1, 5, 6.3 to 6.6, 8.3 to 8.6, 12.1,
  15, 16, 17, and 18.
- `docs/developers-guide.md` §§3.3, 3.8, and 3.9.
- ADR-0008 (build-support boundary), ADR-0009 (OrthoConfig CLI
  configuration), and ADR-0010 (MSRV).
- AGENTS.md is the governing standard.
- No Terms of Reference document exists for this work.

Requirements derived for this plan:

- `REQ-selectors`: the service accepts `--manifest-path`, repeated `--package`,
  `--workspace`, and repeated `--exclude` (design §6.4).
- `REQ-protocol`: workspace data comes from `cargo metadata` format version 1
  through `cargo_metadata` (design §6.4).
- `REQ-virtual`: virtual workspaces resolve correctly, including default
  members (roadmap success).
- `REQ-ambiguity`: ambiguity and unknown-package diagnostics enumerate valid
  package identifiers, bounded (roadmap success, design §§8.3 and 15).
- `REQ-exclude`: excluded members never appear in a workspace selection
  (roadmap success).
- `REQ-manifest`: an explicit manifest anchors resolution independently of the
  current directory (roadmap success, design §6.3).
- `REQ-single`: mutation commands can demand exactly one unambiguous target
  (design §6.4).
- `REQ-readonly`: resolution writes nothing and uses no network (design §§16
  and 10.3).
- `REQ-bounded`: the Cargo child has a deadline and bounded output (design §8.6,
  AGENTS.md).
- `REQ-runner`: processes are created only through a shared, injectable
  command runner (design §12.1, developers' guide §3.8).
- `REQ-noadvertise`: no unimplemented command or flag is advertised (design
  §6.6).

Gaps:

- `GAP-root`: the application uses `"."` as project root rather than the
  Cargo workspace root (design §6.3). This task supplies the `workspace_root`.
  Wiring it into configuration discovery happens with the first project
  command, 4.2.3 or 4.2.4, because no registered command performs project
  discovery today.
- `GAP-envelope`: `arguments` and `valid_values` are not rendered. 4.1.4 owns
  that work.

Trace links:

```plaintext
design §6.4 -> REQ-selectors -> EP-M3 -> workspace::selectors tests, workspace_resolution BDD
design §6.4 -> REQ-protocol -> EP-M2 -> cargo adapter argv test, workspace_resolution real-cargo tests
roadmap 4.2.1 -> REQ-virtual -> EP-M1, EP-M3 -> kernel table "virtual_root_defaults", BDD "virtual workspace defaults"
roadmap 4.2.1 -> REQ-ambiguity -> EP-M1, EP-M3 -> INV-diagnostic tests, insta snapshots, BDD "ambiguous names"
roadmap 4.2.1 -> REQ-exclude -> EP-M1, EP-M3 -> INV-exclusion exhaustive test, BDD "excluded members"
roadmap 4.2.1 -> REQ-manifest -> EP-M3 -> BDD "explicit manifest"
design §6.4 -> REQ-single -> EP-M1 -> PackageSelection::require_single tests
design §16 -> REQ-readonly -> EP-M3 -> INV-readonly fixture snapshot test
design §8.6 -> REQ-bounded -> EP-M2 -> runner overflow, flood, and timeout tests
design §12.1 -> REQ-runner -> EP-M2 -> fake-runner adapter tests
design §6.6 -> REQ-noadvertise -> EP-M3 -> agent-context regression test
```

## Verification plan

### Abstract model

Let `M` be a finite set of workspace members. Each member `m` has:

- a name `n(m)`;
- a semantic version `v(m)`;
- an opaque package ID `i(m)`;
- a manifest path.

Cargo guarantees that `n` is injective on `M` (AXIOM-unique-names) and that `i`
is injective. Let `D ⊆ M` be Cargo's default members for the chosen anchor
(AXIOM-default-members).

A spec `s` denotes a match set `μ(s) ⊆ M`:

- `name` matches `{m | n(m) = name}`;
- `name@p` and `name:p` match `{m | n(m) = name ∧ p ⊑ v(m)}`, where `p ⊑ v`
  means every component present in the partial version `p` equals the
  corresponding component of `v`;
- an ID matches `{m | i(m) = id}`.

By injectivity, `|μ(s)| ≤ 1` for every spec.

A request `R` is one of three forms:

- `Default`;
- `Packages(S)`, where `S` is a non-empty list of specs;
- `Workspace(E)`, where `E` is a possibly empty list of specs.

Define the unmatched specs `U(R)` as the specs `s` in `S` or `E`, in their
command-line order and deduplicated, for which `μ(s) = ∅`. The selection is:

```plaintext
sel(Default)       = D
sel(Packages(S))   = ⋃_{s ∈ S} μ(s)
sel(Workspace(E))  = M \ ⋃_{e ∈ E} μ(e)
```

The checks apply in this order:

1. A malformed or glob spec is a usage error raised during parsing, before
   metadata is loaded.
2. If `U(R) ≠ ∅`, the outcome is `Unknown(U(R), names(M))`.
3. If the selection is empty, the outcome is `Empty`.
4. Otherwise the outcome is the selection, as a list sorted by `n` with no
   duplicates.

`single(Sel)` is the unique member when `|Sel| = 1`, and
`Ambiguous(sorted names(Sel))` otherwise. Every enumeration is truncated to its
first 50 entries in name order, and its total count is reported.

The metadata translation `τ` maps a `cargo_metadata::Metadata` value to a
snapshot `(root, M, D)`:

- `M` is the set of packages whose `id` appears in `workspace_members`;
- `D` is the set of `workspace_default_members`.

`τ` fails with `cargo.metadata_invalid` in any of these cases:

- a member ID has no package;
- a default ID is not a member;
- names repeat;
- a version fails to parse.

It fails with `cargo.unsupported_version` when the default members are missing.

### Invariants and lemmas

- `INV-subset` (`REQ-exclude`, `REQ-ambiguity`): every selected member is in
  `M`. A non-member package never appears, even when it shares a name with a
  member.
- `INV-default` (`REQ-virtual`, `REQ-manifest`): `sel(Default) = D` exactly.
- `INV-package`: `sel(Packages(S))` equals the union of the match sets.
- `INV-exclusion` (`REQ-exclude`): `sel(Workspace(E)) ∩ μ(e) = ∅` for every
  `e ∈ E`. Also `M \ ⋃ μ(E) ⊆ sel(Workspace(E))`.
- `INV-order`: an `Ok` selection is strictly increasing by name. It is
  invariant under permutation of the members and of `S` and `E`, and under
  duplication of specs.
- `INV-nonempty`: an `Ok` selection is never empty.
- `INV-diagnostic` (`REQ-ambiguity`): `Unknown` lists exactly the unmatched
  specs in command-line order. Its `valid_values` are the first `min(50, |M|)`
  member names in ascending order, and its count is `|M|`. `Ambiguous` lists
  the sorted names of the offending selection under the same bound. Every
  listed value names a member.
- `INV-exact`: a spec matches only by exact string equality of the name and
  ID, and by componentwise equality of the version. Specs that differ by case,
  by `-` versus `_`, or by prefix do not match unless they equal another
  member's name.
- `INV-readonly` (`REQ-readonly`): resolution leaves the fixture file tree
  byte-identical and creates no `Cargo.lock` or `target/`.
- `INV-bounded` (`REQ-bounded`):
  - retained stdout is at most 16 MiB and retained stderr at most 64 KiB;
  - the runner returns within the deadline plus a bounded grace period;
  - the child is always reaped or killed before return.
- `L-roundtrip`: for every member `m` of any valid snapshot, the specs `n(m)`,
  `n(m)@v(m)`, `n(m):v(m)`, and `i(m)` each parse, and each has match set `{m}`.
- `L-translation`: `τ` succeeds on every metadata document satisfying the
  consistency conditions above. When it succeeds, it preserves names, versions,
  IDs, and the membership of `D`.

### Axioms (trusted, not verified here)

- `AXIOM-unique-names`: Cargo rejects a workspace whose members share a
  package name. Source: probe 1, Cargo 1.100.0.
- `AXIOM-default-members`: Cargo 1.71 and later emits
  `workspace_default_members` computed by its package-selection rules relative
  to the working directory or `--manifest-path`. Source: the `cargo metadata`
  documentation, the `cargo_metadata` 0.23.1 documentation, and probe 3.
- `AXIOM-no-deps-readonly`: `cargo metadata --no-deps` performs no dependency
  resolution, writes no `Cargo.lock`, and needs no network. Source: the
  `cargo metadata` documentation and probe 5. EP-M3's `INV-readonly` test
  re-checks this against the real interface on every run.
- `AXIOM-format-v1`: within format version 1, Cargo only adds fields and enum
  values, and package IDs are opaque. Source: the `cargo metadata`
  "Compatibility" section.
- `AXIOM-cargo-env`: Cargo sets `CARGO` for external subcommands. Source: the
  Cargo environment-variable reference. The fallback is `cargo` on `PATH`.
- `AXIOM-rustup-auto-install`: rustup 1.28.1 and later honours
  `RUSTUP_AUTO_INSTALL=0`. Source: the rustup 1.28.1 announcement and the
  rustup book.
- `AXIOM-cargo-metadata-parse`: `cargo_metadata::MetadataCommand::parse`
  faithfully deserializes format version 1.

### Methods per obligation

- Obligation: `INV-subset`, `INV-default`, `INV-package`, `INV-exclusion`,
  `INV-order`, `INV-nonempty`, and `INV-diagnostic` over the kernel.
  - Method: bounded exhaustive test against an independent oracle, plus named
    `rstest` tables.
  - Rationale: the kernel is a finite set computation. Exhaustive enumeration
    of every snapshot with up to four members is cheap (well under a second)
    and strictly stronger than sampling for that bound. Errors that need more
    than four members are confined to the 50-entry truncation, which a
    separate named test covers with 120 members.
  - Domain:
    - members `{0..k}` for `k ≤ 4`, with names drawn from a fixed alphabet that
      includes the `foo-bar`/`foo_bar` pair;
    - every subset of default members;
    - every request: `Default`; `Packages` over every non-empty subset of
      member names plus one unknown name; `Workspace` over every subset of
      excludes plus one unknown name.
  - Artefact:
    `crates/cargo-theorem/src/workspace/selection_exhaustive_tests.rs`.
    The oracle works on `u8` bitmasks with set algebra and shares no code with
    the kernel.
  - Evidence: `cargo nextest run -p cargo-theorem -E
    'test(/selection_exhaustive/)'` passes. The test prints and asserts its
    case count and its per-class counts.
  - Non-vacuity:
    - Assert that each outcome class (`Ok`, `Unknown`, `Empty`, and single
      `Ambiguous`) occurs at least 100 times.
    - Assert that every request form is reached.
    - Seeded faults, applied temporarily and reverted:
      1. ignore exclusions, which `INV-exclusion` must reject;
      2. keep Cargo's member order instead of sorting, which must fail under
         shuffled member input;
      3. normalize `-` to `_` in matching, which `INV-exact` must reject
         through the `foo-bar`/`foo_bar` pair.

      Record the three red transcripts in `Artefacts and notes`.
- Obligation: `L-roundtrip` and `INV-exact`.
  - Method: `proptest`.
  - Rationale: the spec grammar has an unbounded input space of names,
    versions, and IDs.
  - Domain:
    - names matching `[a-z][a-z0-9_-]{0,15}`;
    - versions from generated `(u64, u64, u64)` with optional pre-release;
    - IDs formed in both shapes Cargo prints: `path+file:///…/name#version`
      and `path+file:///…#name@version`.
  - Artefact: `crates/cargo-theorem/src/workspace/spec_property_tests.rs`.
  - Evidence: the test passes with the default 256 cases. Any regression file
    lands under `proptest-regressions/` and is committed.
  - Non-vacuity:
    - Classify generated names by whether they contain `-` or `_`, and assert
      both classes appear.
    - The mutation property derives a near-miss spec (case flip, `-`/`_` swap,
      or a dropped trailing character) and asserts it does not match, unless it
      equals another member's name. A seeded case-insensitive matcher must fail
      this test.
- Obligation: `L-translation` and the `τ` failure cases.
  - Method: named `rstest` cases over JSON documents.
  - Artefact: `crates/cargo-theorem/tests/data/metadata/*.json`, small
    documents derived from probe output. The table lives in
    `src/workspace/metadata_tests.rs`.
  - Evidence: each document yields the expected snapshot or diagnostic code.
  - Non-vacuity: one valid document per Cargo ID shape, plus one document for
    each failure case (missing default members, a default that is not a
    member, a member without a package, a duplicate name, and a bad version).
- Obligation: `INV-bounded`.
  - Method: runner tests with real `/bin/sh` children (`#[cfg(unix)]`).
  - Rationale: pipe and kill semantics are the platform's. Only real processes
    exercise them.
  - Cases:
    1. normal exit with captured output;
    2. stdout of limit plus one byte gives `StdoutLimitExceeded`, and the child
       is reaped;
    3. a 1 MiB stderr flood under a 1 KiB limit succeeds with `truncated =
       true` and `bytes_seen = 1048576`;
    4. `sleep 30` under a 200 ms deadline returns `TimedOut` in under 5
       seconds;
    5. a missing program gives `Spawn`;
    6. a non-zero exit is returned as data, not an error.
  - Artefact: `crates/cargo-theorem/src/process/runner_tests.rs`.
  - Non-vacuity:
    - Case 2 asserts that the child process ID is no longer alive after
      return.
    - Seeded fault: removing the kill call must make case 4 exceed its 5 second
      bound.
- Obligation: the adapter error mapping and argument-vector contract (D6, D8).
  - Method: named tests with a fake `CommandRunner` that records the
    `CommandSpec` and returns scripted outcomes.
  - Artefact: `crates/cargo-theorem/src/workspace/cargo_adapter_tests.rs`.
  - Non-vacuity:
    - The recorded argument vector must contain `--no-deps`.
    - Every `CommandError` and exit outcome maps to its own code, and each
      mapping is asserted individually.
- Obligation: `INV-readonly`, `INV-default` against real Cargo,
  `REQ-manifest`, `REQ-virtual`, and the ambiguous-name fixtures.
  - Method: integration and BDD tests that run real `cargo metadata` (from
    `env!("CARGO")`) on fixture workspaces in temporary directories.
  - Artefacts:
    - `crates/cargo-theorem/tests/workspace_resolution.rs`;
    - `crates/cargo-theorem/tests/workspace_resolution/fixture.rs`, the fixture
      builder;
    - `crates/cargo-theorem/tests/workspace_resolution_bdd.rs`;
    - `crates/cargo-theorem/tests/features/workspace_selection.feature`.
  - Non-vacuity:
    - The read-only test uses the fixture with a path dependency. With the
      `--no-deps` flag removed, Cargo writes `Cargo.lock` and the test fails.
      Exercise this once as a seeded fault and record it.
    - The ambiguous-names fixture asserts that a non-member `vendor/foo`
      package exists on disk, so the precondition is real.

### Formal verification decision

Verus is not warranted. The kernel's obligations are set identities over finite
member sets with exact string equality. A Verus proof would model sets and
strings, then show the implementation computes `D`, `⋃μ(S)`, and `M \ ⋃μ(E)`.
That nearly restates the specification, so it gives little evidence beyond the
independent oracle. It would also add a toolchain, a CI lane, and a proof
directory that the repository does not have.

The genuinely risky properties are not ones Verus can reach:

- Cargo's default-member semantics, which are an axiom;
- the read-only behaviour of `cargo metadata`, which is an axiom checked
  against the real tool;
- pipe and kill behaviour, which belongs to the platform.

Kani is not warranted either. The kernel manipulates heap strings and ordered
collections, which Kani explores poorly. The bounded exhaustive test gives the
same "every case within the bound" guarantee over an abstract bitmask domain,
with an independent oracle.

What stronger methods would add is an unbounded statement over member counts
above four. Two observations narrow the residual gap:

- The kernel's per-member logic is uniform: there is no counter, index, or
  threshold that changes behaviour above four members.
- The only size-dependent behaviour, 50-entry truncation, has its own named
  test.

This decision is recorded as D10.

### Suggested BDD features

Proposed feature file:
`crates/cargo-theorem/tests/features/workspace_selection.feature`. The binding
module is `crates/cargo-theorem/tests/workspace_resolution_bdd.rs`, which uses
rstest-bdd 0.6.0. Steps build fixtures, call
`cargo_theorem::workspace::resolve_selection` with a `CargoMetadataSource` over
`SystemCommandRunner`, and assert on names or diagnostic codes and values.

```gherkin
Feature: Workspace and package selection
  Cargo projects are resolved through cargo metadata into an ordered,
  bounded package selection.

  # REQ-virtual, EP-M3
  Scenario: Virtual workspace defaults follow default-members
    Given the virtual workspace fixture
    When packages are resolved from the workspace root without selectors
    Then the selected packages are "app, foo"

  # REQ-manifest, EP-M3
  Scenario: An explicit member manifest anchors the default selection
    Given the virtual workspace fixture
    When packages are resolved from an unrelated directory with manifest "foo_bar/Cargo.toml"
    Then the selected packages are "foo_bar"
    And the workspace root is the fixture root

  # REQ-exclude, EP-M3
  Scenario: Excluded members never appear in a workspace selection
    Given the virtual workspace fixture
    When packages are resolved with "--workspace --exclude foo-bar --exclude app"
    Then the selected packages are "foo, foo_bar"

  # REQ-ambiguity, INV-exact, EP-M3
  Scenario: A name shared with a non-member selects only the member
    Given the virtual workspace fixture
    When packages are resolved with "--package foo"
    Then the selected packages are "foo"
    And the selected package "foo" has version "0.1.0"

  # REQ-ambiguity, INV-exact, EP-M3
  Scenario: Hyphen and underscore names are distinct
    Given the virtual workspace fixture
    When packages are resolved with "--package foo_bar"
    Then the selected packages are "foo_bar"

  # REQ-ambiguity, INV-diagnostic, EP-M3
  Scenario: Unknown packages enumerate the valid identifiers
    Given the virtual workspace fixture
    When packages are resolved with "--package fop"
    Then resolution fails with code "package.unknown" and class "usage"
    And the valid values are "app, foo, foo-bar, foo_bar"

  # REQ-single, INV-diagnostic, EP-M3
  Scenario: A single-target command rejects several default members
    Given the virtual workspace fixture
    When exactly one package is required from the workspace root without selectors
    Then resolution fails with code "selection.ambiguous" and class "usage"
    And the valid values are "app, foo"

  # REQ-readonly, EP-M3
  Scenario: Resolution leaves the project unchanged
    Given the virtual workspace fixture
    When packages are resolved with "--workspace"
    Then the fixture file tree is unchanged
    And no "Cargo.lock" file exists in the fixture
```

## Plan of work

### `EP-M0` feasibility probe

No new probe is needed. Planning-time probes answered every feasibility
question: duplicate names, default-member semantics, read-only behaviour, and
child processes. Their transcripts are in `Artefacts and notes`. The remaining
uncertainty is the `OrthoConfig` flatten interaction, which EP-M3 resolves as
its first red test.

### `EP-M1` decision records and the pure selection kernel

The assigned items are `REQ-virtual`, `REQ-exclude`, `REQ-ambiguity`, and
`REQ-single` at kernel level, decisions D1 to D6 and D10 to D11, and obligations
`INV-subset` to `INV-exact`, `L-roundtrip`, and `L-translation`.

First, record the decisions before the code that depends on them:

1. Write `docs/adrs/adr-0014-workspace-package-selection.md`.
   - Status: Accepted on approval of this plan.
   - Contents: Context; Decision covering D1 to D9 in summary, with the Cargo
     differences explicit; Consequences, including the
     `serde_json/unbounded_depth` note.
2. Update `docs/cargo-theorem-cli-design.md` §6.4 with the resolved contract:
   - the spec grammar;
   - the selection rules;
   - the error codes and classes;
   - the 50-choice bound;
   - the `cargo metadata` invocation contract.

   Reference ADR-0014 from §6.4.
3. Update §5 to state that `workspace` and `process` begin as feature-grouped
   modules of `cargo-theorem`.
4. Index ADR-0014 in `docs/contents.md`.

Then work test-first on the kernel.

- Red: add `crates/cargo-theorem/src/workspace/mod.rs` declaring the
  submodules. Add `spec_tests.rs` with `rstest` tables for these cases:
  - `name`, `name@1`, `name@1.4`, `name@1.4.3`, `name@1.4.3-alpha.1`,
    `name:1.4`, and both full-ID shapes, which are accepted;
  - the empty string, `@1`, `foo@`, `foo@x`, `foo@1.2.3.4`, which are
    rejected as `package.invalid_spec`;
  - `foo*`, `f?o`, `[f]oo`, which are rejected as `package.glob_unsupported`.

  Add `selection_tests.rs` with named cases over the virtual fixture snapshot:
  - default;
  - an explicit package;
  - repeated and duplicate packages;
  - workspace;
  - workspace with excludes;
  - an unknown package, with several unknowns reported in order;
  - an unknown exclude;
  - empty after exclusion;
  - `require_single` with one and with two members;
  - truncation at 50 with 120 members.

  Run `cargo nextest run -p cargo-theorem -E 'test(/workspace::/)'`. Expect
  compile failure: the unresolved items `PackageSpec`, `select`, and so on.
- Green: implement the following.
  - `workspace/spec.rs`: `PackageSpec`, `PartialVersion`, `PackageSpec::parse`,
    and `PackageSpec::matches`.
  - `workspace/model.rs`:
    - `PackageName` and `PackageId` newtypes;
    - `WorkspaceMember`;
    - `WorkspaceSnapshot`, whose constructor validates name uniqueness and that
      defaults are members.
  - `workspace/selection.rs`:
    - `SelectionRequest`, `select`, and `PackageSelection`;
    - `PackageSelection::members` and `PackageSelection::require_single`.
  - `workspace/error.rs`:
    - `SelectionError`;
    - `Diagnostic`, with `code`, `class`, `message_fallback`, `arguments`,
      `valid_values`, and `Serialize`.
  - `src/exit_class.rs`: the `ExitClass` enum for every class in §8.4, with
    `code()` and `as_str()`. Change `ApplicationError::exit_code` to use
    `ExitClass`. Its existing tests must still pass unchanged.
  - Declare `pub mod workspace;` and `pub mod exit_class;` in `src/lib.rs`, with
    module documentation stating D3.

  Rerun the focused tests and expect them to pass.
- Add the exhaustive oracle test and the proptest properties, then run them.
  Apply each seeded fault from the `Verification plan` in turn. Observe the
  failure, record the transcript, and revert. Confirm that `git diff` shows no
  fault code.
- Add `workspace/metadata.rs` with the translation `τ`, from
  `cargo_metadata::Metadata` to `WorkspaceSnapshot`, and its JSON-document
  tests. This brings in `cargo_metadata` as a dependency of `cargo-theorem`.
- Refactor: keep each file under 400 lines, and split test modules with
  `#[path]` as `cli_tests.rs` does. Then run the milestone gates.

### `EP-M2` bounded process runner and the `cargo metadata` adapter

The assigned items are `REQ-bounded`, `REQ-runner`, `REQ-protocol`, decisions
D7 and D8, and obligation `INV-bounded`.

- Red: add `process/runner_tests.rs` with the six real-process cases, under
  `#[cfg(unix)]`. Add `workspace/cargo_adapter_tests.rs` with a recording fake
  runner. It asserts:
  - the exact argument vector, both with and without a manifest;
  - the working directory;
  - `RUSTUP_AUTO_INSTALL=0`;
  - the limits;
  - the program taken from an injected `CARGO`, with `cargo` as the fallback
    when it is absent or empty (using `ortho_config::MapEnv`).

  It also asserts each outcome mapping:
  - `Spawn` gives `cargo.unavailable`;
  - `TimedOut` gives `cargo.timed_out`;
  - `StdoutLimitExceeded` gives `cargo.output_limit`;
  - a non-zero exit gives `workspace.metadata_failed`, with a bounded stderr
    excerpt in `arguments`;
  - invalid JSON gives `cargo.metadata_invalid`;
  - missing default members gives `cargo.unsupported_version`.

  Expect compile failure.
- Green, part one: implement `process/mod.rs` with `CommandSpec`,
  `OutputLimits`, `CommandOutput`, `CapturedStream`, `CommandError`, and the
  `CommandRunner` trait. Then implement `process/system.rs` with
  `SystemCommandRunner`:
  1. Spawn with piped stdout and stderr and null stdin.
  2. Start one reader thread per pipe. The stdout reader reads up to the limit
     plus one byte and reports overflow. The stderr reader retains up to its
     limit and drains and counts the rest.
  3. Wait on a channel with `recv_timeout`, computed against a single
     `Instant` deadline.
  4. On overflow or deadline, call `Child::kill` and then `Child::wait`.
  5. Join the readers with the remaining grace period (2 seconds). Readers
     still blocked are detached and documented.
- Green, part two: implement `workspace/cargo_adapter.rs` with
  `CargoMetadataSource<R: CommandRunner>`,
  `CargoProgram::from_env(&dyn EnvSource)`, and `ManifestAnchor`.
  `ManifestAnchor` has two forms:
  - `Discover { start_dir }`;
  - `Explicit { start_dir, manifest_path }`.

  Also implement the `WorkspaceMetadataSource` port trait in
  `workspace/metadata.rs`. The adapter validates explicit manifests before
  spawning. A missing path gives `manifest.not_found`. A path whose file name
  is not `Cargo.toml` gives `manifest.invalid_name`. Both are checked through
  `cap_std::fs_utf8::Dir::open_ambient_dir` on the parent directory. For
  discovery, the adapter checks for a `Cargo.toml` in the start directory and
  its ancestors; finding none gives `workspace.not_found`.
- Declare `pub mod process;` in `src/lib.rs`.
- Run the seeded "no kill" fault once, record it, and revert. Run the
  milestone gates.

### `EP-M3` resolution service, selector arguments, and real-Cargo fixtures

The assigned items are `REQ-selectors`, `REQ-manifest`, `REQ-readonly`,
`REQ-noadvertise`, the end-to-end evidence for `REQ-virtual`, `REQ-exclude`, and
`REQ-ambiguity`, and obligation `INV-readonly`.

- Red, flatten compatibility: add a test-only command struct in
  `workspace/selectors_tests.rs`. It derives `clap::Args`, `Deserialize`,
  `Serialize`, and `OrthoConfig`, and flattens `WorkspaceSelectorArgs` with
  `#[serde(skip)]`. It parses through a test `Parser`. If the derive rejects
  the struct, follow the mitigation in `Risks`, record the discovery, and
  continue.
- Red, selector table: add `selectors_tests.rs` with `rstest` cases.
  - These parse into the expected `SelectionRequest` and `ManifestAnchor`:
    - no flags;
    - `--package a --package b`;
    - `--workspace --exclude a`;
    - `--manifest-path x/Cargo.toml`.
  - These are rejected:
    - `--exclude a` without `--workspace`, a Clap usage error;
    - `--package a --workspace`, a conflict;
    - `--package 'f*'`, which gives `package.glob_unsupported` from
      `into_request`.
- Red, integration tests: add the fixture builder
  `tests/workspace_resolution/fixture.rs`.
  - It writes manifests and empty `src/lib.rs` files into a `tempfile::TempDir`
    through `cap_std::fs_utf8`.
  - Every fixture declares `[workspace]`, so Cargo never walks up into the
    theoremc workspace.
  - It provides `snapshot_tree()`, which returns a sorted list of
    path-and-content pairs.

  Add `tests/workspace_resolution.rs` with real-Cargo cases:
  - the BDD scenarios' cases, as plain tests;
  - a non-virtual root, whose default is the root package only;
  - running from inside a member directory;
  - a malformed member manifest, giving `workspace.metadata_failed` with a
    stderr excerpt;
  - a directory without `Cargo.toml`, giving `workspace.not_found`;
  - `INV-readonly`.

  Add the `.feature` file and the BDD bindings. Add an `insta` snapshot test,
  `tests/workspace_resolution/diagnostics.rs`. It snapshots the JSON
  serialization and the `Display` text of the `package.unknown`,
  `selection.ambiguous`, `selection.empty`, and truncated (120-member)
  diagnostics. Temporary paths are redacted with `insta` filters.

  Add an agent-context regression to `tests/entry_points/agent_context.rs`. It
  asserts that the emitted context contains none of `manifest-path`, `package`,
  `workspace`, and `exclude`.

  Run
  `cargo nextest run -p cargo-theorem -E
  'binary(=workspace_resolution) | binary(=workspace_resolution_bdd)'`.
  Expect failure on the unresolved `resolve_selection` and
  `WorkspaceSelectorArgs`.
- Green: implement `workspace/selectors.rs` with `WorkspaceSelectorArgs`, a
  Clap `Args` struct with long options only. Its
  `into_request(self, start_dir)` returns a `ResolutionInput`, which pairs the
  anchor with the request, or a `SelectionError`. Implement `workspace/mod.rs`
  with
  `resolve_selection(source: &impl WorkspaceMetadataSource, input:
  &ResolutionInput) -> Result<ResolvedSelection, ResolutionError>`.
  `ResolvedSelection` carries `workspace_root`, the `PackageSelection`, and
  the stderr truncation facts. Add the integration binaries to the
  `nested-cargo-builds` override filter in `.config/nextest.toml`, as
  `binary(=workspace_resolution) | binary(=workspace_resolution_bdd)`. Add the
  dev-dependencies.
- Refactor, then run the milestone gates.

### `EP-M4` guides, roadmap closure, and delivery

- Update `docs/cargo-theorem-cli-users-guide.md`:
  - Rewrite the selectors section. It must state that selectors apply to
    project commands as they ship, starting with `list`, `get`, and `check`.
  - Document the spec forms, repeated `--package`, the `--exclude` rules, the
    "unknown exclude is an error" difference from Cargo, ordering, the
    ambiguity and unknown diagnostics with an example, and exit classes 2, 4,
    and 8 for selection failures.
- Update `docs/developers-guide.md` §3.8:
  - the `workspace` and `process` modules;
  - the port and adapter split;
  - the reuse policy: every project command resolves packages only through
    `workspace::resolve_selection`, and nothing in `cargo-theorem` creates a
    process except through `process::CommandRunner`;
  - the detach policy for blocked reader threads;
  - the extraction trigger from D1.
- Update `docs/developers-guide.md` §3.3 with the fixture-workspace builder,
  the rule that every fixture declares `[workspace]`, and the nextest group
  entry.
- Update `docs/repository-layout.md` (the `crates/cargo-theorem/` row) and
  `docs/contents.md` (index this ExecPlan).
- Add a `docs/CHANGELOG.md` entry if the file keeps an unreleased section.
- Leave `docs/users-guide.md` unchanged. It documents the library, and this
  task changes no library or command behaviour. Record that decision in the
  outcomes.
- Mark roadmap 4.2.1 done in `docs/roadmap.md` (`- [x] 4.2.1.`) only after all
  acceptance criteria pass.
- Run every gate. Then complete `Outcomes & retrospective` and set the status
  to COMPLETE.

## Milestones and plateaus

- `EP-M1`: decisions recorded, and the kernel, spec parser, translation, and
  exit-class enum complete and public.
  - Requirements: `REQ-virtual`, `REQ-exclude`, `REQ-ambiguity`, and
    `REQ-single`, at kernel level.
  - Acceptance: the exhaustive, table, and property tests pass. The seeded
    faults are recorded. The gates pass.
  - Conformance:
    - The kernel imports no Clap, OrthoConfig, or process code. Check with
      `rg -n 'clap|ortho_config|std::process'
      crates/cargo-theorem/src/workspace/{spec,model,selection}.rs`, which must
      print nothing.
    - ADR-0014 is accepted.
    - Design §6.4 is updated.
  - Recovery: revert the milestone commits. Nothing outside `cargo-theorem`
    and docs changes.
  - Remaining gaps: no Cargo invocation yet.
  - Compatibility: none.
- `EP-M2`: the runner and adapter are complete.
  - Requirements: `REQ-bounded`, `REQ-runner`, and `REQ-protocol`.
  - Acceptance: the runner and fake-runner tests pass. The "no kill" fault is
    recorded. The gates pass.
  - Conformance: `rg -n 'Command::new' crates/cargo-theorem/src` matches only
    `process/system.rs`.
  - Recovery: revert the milestone commits. The kernel from EP-M1 stands
    alone.
  - Remaining gaps: no real-Cargo evidence yet.
  - Compatibility: none.
- `EP-M3`: the service and selectors are complete, with real-Cargo evidence.
  - Requirements: all `REQ-*` items.
  - Acceptance: the integration, BDD, snapshot, and agent-context tests pass.
    The read-only seeded fault is recorded. The gates pass.
  - Conformance:
    - The agent context is unchanged.
    - No selector is registered on `Commands`.
    - The trust boundary is unchanged: Cargo is invoked with `--no-deps`, and
      no files are written.
  - Recovery: revert the milestone commits.
  - Remaining gaps: `GAP-root` and `GAP-envelope`, owned by 4.2.3, 4.2.4, and
    4.1.4.
  - Compatibility: none.
- `EP-M4`: the guides are accurate, the roadmap item is marked done, and the
  plan is COMPLETE.
  - Acceptance: every gate passes, including markdownlint, nixie, and
    spelling.
  - Conformance: every upstream artefact is reconciled.
  - Recovery: documentation commits revert independently.
  - Compatibility: none.

## Concrete steps

Run every command from the worktree root,
`/home/leynos/.lody/repos/github---leynos---theoremc/worktrees/<id>`. Use
`set -o pipefail` whenever output is piped through `tee`.

Focused loops, by milestone:

```plaintext
EP-M1 red:   cargo nextest run -p cargo-theorem -E 'test(/workspace::/)'  -> error[E0432]: unresolved import
EP-M1 green: cargo nextest run -p cargo-theorem -E 'test(/workspace::/)'  -> all passed
EP-M2 red:   cargo nextest run -p cargo-theorem -E 'test(/process::|cargo_adapter/)' -> compile failure
EP-M2 green: same command -> all passed (runner cases skip only on non-Unix hosts)
EP-M3 red:   cargo nextest run -p cargo-theorem \
               -E 'binary(=workspace_resolution) | binary(=workspace_resolution_bdd)' -> compile failure
EP-M3 green: same command -> all passed
Snapshots:   cargo insta test -p cargo-theorem --review   (accept only after reading each snapshot)
```

Milestone gates, run sequentially:

```plaintext
set -o pipefail
make fmt
make check-fmt   2>&1 | tee /tmp/check-fmt-theoremc-4-2-1-workspace-and-package-resolution.out
make lint        2>&1 | tee /tmp/lint-theoremc-4-2-1-workspace-and-package-resolution.out
make test        2>&1 | tee /tmp/test-theoremc-4-2-1-workspace-and-package-resolution.out
make msrv        2>&1 | tee /tmp/msrv-theoremc-4-2-1-workspace-and-package-resolution.out
make check-agent-native 2>&1 | tee /tmp/agent-native-theoremc-4-2-1-workspace-and-package-resolution.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-4-2-1-workspace-and-package-resolution.out
make nixie       2>&1 | tee /tmp/nixie-theoremc-4-2-1-workspace-and-package-resolution.out
make spelling    2>&1 | tee /tmp/spelling-theoremc-4-2-1-workspace-and-package-resolution.out
```

Delegate full gate runs to the `scrutineer` agent where available. A known
local baseline failure is the Kani BDD test, which fails on clean `main`
because of a rustup library-path interaction. CI skips it because Kani is not
installed there. Record it as a baseline failure if it appears, not as a
regression.

`make check-fmt` checks only uncommitted Markdown with `mdtablefix --git`.
Before pushing documentation commits, run `mdtablefix --check` on the changed
files directly.

## Validation and acceptance

The task is accepted when all of the following hold:

1. `make check-fmt`, `make lint`, `make test`, `make msrv`,
   `make check-agent-native`, `make markdownlint`, `make nixie`, and
   `make spelling` all pass.
2. The BDD feature `workspace_selection.feature` passes all eight scenarios
   against real Cargo. Before implementation, its binding module fails to
   compile, which is the recorded red evidence.
3. The exhaustive kernel test reports at least 100 cases for each outcome
   class. It fails under each of the three seeded faults, whose transcripts are
   recorded.
4. The real-Cargo read-only test passes. With the `--no-deps` flag removed it
   fails because `Cargo.lock` appears, which is recorded.
5. The insta snapshots show that `package.unknown` lists
   `["app","foo","foo-bar","foo_bar"]` with class `usage`. They also show that
   the 120-member diagnostic lists 50 values with a total count of 120.
6. `cargo theorem context --json` still lists exactly one command path,
   `context`, and none of the selector flags.

Quality criteria:

- Tests: every test above passes under `make test`.
- Verification: `INV-*` and `L-*` are discharged as described in
  `Methods per obligation`. The axioms are recorded in ADR-0014.
- Lint and typecheck: `make lint` and `make msrv` are clean.
- Performance: no benchmark. Each real-Cargo fixture call completes in under 2
  seconds locally.
- Security: Cargo runs with `--no-deps --offline` and
  `RUSTUP_AUTO_INSTALL=0`. Processes are created only through argument vectors.

## Idempotence and recovery

Every step can be re-run. Fixtures live in fresh temporary directories that are
deleted when their `TempDir` drops. Seeded faults are applied by hand,
observed, and reverted. Check `git diff --stat` before committing to confirm
that no fault remains. If a milestone fails its gates, fix the issue forward
within the iteration tolerance, or revert that milestone's commits. Earlier
milestones remain valid plateaus. Proptest regression files are committed only
when they record a real, fixed failure.

## Artefacts and notes

The planning-time probes ran Cargo 1.100.0 (`3acc79c0c 2026-11-05`) and rustup
1.29.3 in throwaway `/tmp` directories.

Probe 1 (duplicate member names):

```plaintext
$ cargo metadata --no-deps --format-version 1
error: two packages named `foo` in this workspace:
- /tmp/ws-probe.jElz/a/foo/Cargo.toml
- /tmp/ws-probe.jElz/b/foo/Cargo.toml
```

Probe 2 (a member and an excluded path package share the name `foo`):

```plaintext
metadata --no-deps ids: ['path+file:///tmp/ws-probe2.WfZj/app#0.1.0', 'path+file:///tmp/ws-probe2.WfZj/foo#0.1.0']
$ cargo pkgid foo
error: specification `foo` is ambiguous
help: re-run this command with one of the following specifications
  foo@0.1.0
  foo@0.2.0
$ cargo check -p foo            -> Checking foo v0.1.0 (/tmp/ws-probe2.WfZj/foo)
$ cargo check --exclude foo     -> error: --exclude can only be used together with --workspace
$ cargo check -p fop            -> error: package ID specification `fop` did not match any packages
                                   help: a package with a similar name exists: `foo`
$ cargo check --workspace --exclude nope
warning: excluded package(s) `nope` not found in workspace `/tmp/ws-probe2.WfZj`
```

Probe 3 (default-member semantics, `default-members = ["a"]`):

```plaintext
root:            default: ['path+file:///tmp/ws-probe3.FqAP/a#0.1.0']
--manifest-path b/Cargo.toml: workspace_root=/tmp/ws-probe3.FqAP default: [...b#0.1.0]
cwd=b:           default: [...b#0.1.0]
cwd=a/sub:       default: [...a#0.1.0]
no default-members (virtual): [...a#0.1.0, ...b#0.1.0]
non-virtual root: default: ['path+file:///tmp/ws-probe4.OaBH#root@0.1.0']
--manifest-path b (a directory): error: manifest path `b` is a directory but expected a file
no Cargo.toml anywhere: error: could not find `Cargo.toml` in `/tmp/tmp.l6d7ZyACR3` or any parent directory
--no-deps without Cargo.lock: exit 0, no Cargo.lock created; --locked also exits 0
```

Probe 5 (read-only behaviour and child processes):

```plaintext
find snapshot before/after `cargo metadata --no-deps --offline`: tree unchanged
strace execve: /home/leynos/.cargo/bin/cargo (rustup proxy), .../toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo
```

Probe 6 (members whose names differ only by `-` versus `_`):

```plaintext
members = ["foo-bar", "foo_bar"]: metadata ids ['path+file:///tmp/ws-probe6.Zsx0/foo-bar#0.1.0',
'path+file:///tmp/ws-probe6.Zsx0/foo_bar#0.1.0'], no warnings
```

External references consulted with Firecrawl:

- the Cargo book, `cargo metadata` (output format, compatibility, and
  `--no-deps`);
- the Cargo book, Package ID Specifications (grammar and examples);
- the Cargo book, `cargo check` package selection (default-member rules, glob
  support, and `--exclude` requiring `--workspace`);
- docs.rs, `cargo_metadata` 0.23.1 (`MetadataCommand`, `Metadata`,
  `WorkspaceDefaultMembers`, and rust-version 1.86);
- docs.rs, `cargo-util-schemas` 0.14.3 (rust-version 1.97);
- the rustup book and the rustup 1.28.1 announcement
  (`RUSTUP_AUTO_INSTALL`).

## Interfaces and dependencies

The authorized dependencies for `crates/cargo-theorem/Cargo.toml` are:

- runtime:
  - `cargo_metadata = "0.23.1"`;
  - `camino = "1.2.2"`;
  - `semver = "1.0.26"`;
  - `cap-std = { version = "4.0.2", features = ["fs_utf8"] }`, moved from
    dev-dependencies to dependencies for manifest validation;
- development:
  - `googletest = "0.14.3"`;
  - `pretty_assertions = "1.4.1"`;
  - `insta = { version = "1", features = ["json", "filters"] }`, at the version
    already locked by `theoremc-macros`;
  - `proptest = { version = "1.11.0", default-features = false, features =
    ["std"] }`;
  - `rstest-bdd = "0.6.0"`;
  - `rstest-bdd-macros = "0.6.0"`.

Every one of these is already present in `Cargo.lock`.

The public surface at the end of EP-M3 is sketched below. Names are binding;
field details may be refined if recorded in `Decision log`.

```rust
// crates/cargo-theorem/src/exit_class.rs
pub enum ExitClass { Success, Usage, Configuration, ProjectState, BackendUnavailable,
    VerificationMismatch, VerificationIncomplete, ExternalToolFailure, DeliveryFailure, Internal }
impl ExitClass { pub const fn code(self) -> u8; pub const fn as_str(self) -> &'static str; }

// crates/cargo-theorem/src/process/mod.rs
pub struct CommandSpec { pub program: std::ffi::OsString, pub args: Vec<std::ffi::OsString>,
    pub current_dir: camino::Utf8PathBuf, pub env: Vec<(std::ffi::OsString, std::ffi::OsString)> }
pub struct OutputLimits { pub stdout_bytes: usize, pub stderr_bytes: usize, pub deadline: std::time::Duration }
pub struct CapturedStream { pub bytes: Vec<u8>, pub bytes_seen: u64, pub truncated: bool }
pub struct CommandOutput { pub exit_code: Option<i32>, pub success: bool,
    pub stdout: Vec<u8>, pub stderr: CapturedStream }
#[derive(Debug, thiserror::Error)]
pub enum CommandError { Spawn { .. }, TimedOut { .. }, StdoutLimitExceeded { .. }, Io { .. } }
pub trait CommandRunner { fn run(&self, spec: &CommandSpec, limits: &OutputLimits)
    -> Result<CommandOutput, CommandError>; }
pub struct SystemCommandRunner;            // process/system.rs, the only `Command::new` site

// crates/cargo-theorem/src/workspace/
pub struct PackageName(String); pub struct PackageId(String);
pub struct WorkspaceMember { /* name, version: semver::Version, id, manifest_path: Utf8PathBuf */ }
pub struct WorkspaceSnapshot { /* root: Utf8PathBuf, members (by name), default_members */ }
pub enum PackageSpec { Name(PackageName), NameVersion(PackageName, PartialVersion), Id(PackageId) }
pub enum SelectionRequest { Default, Packages(Vec<PackageSpec>) /* non-empty */,
    Workspace { exclude: Vec<PackageSpec> } }
pub enum ManifestAnchor { Discover { start_dir: Utf8PathBuf },
    Explicit { start_dir: Utf8PathBuf, manifest_path: Utf8PathBuf } }
pub struct ResolutionInput { pub anchor: ManifestAnchor, pub request: SelectionRequest }
pub struct PackageSelection { /* non-empty, sorted by name */ }
impl PackageSelection { pub fn members(&self) -> &[WorkspaceMember];
    pub fn require_single(&self) -> Result<&WorkspaceMember, SelectionError>; }
pub fn select(snapshot: &WorkspaceSnapshot, request: &SelectionRequest) -> Result<PackageSelection, SelectionError>;
pub trait WorkspaceMetadataSource {
    fn load(&self, anchor: &ManifestAnchor) -> Result<WorkspaceSnapshot, MetadataError>; }
pub struct CargoMetadataSource<R: CommandRunner> { /* runner, program: CargoProgram, limits */ }
pub struct CargoProgram(std::ffi::OsString);
impl CargoProgram { pub fn from_env(env: &dyn ortho_config::EnvSource) -> Self; }
#[derive(clap::Args)] pub struct WorkspaceSelectorArgs { /* --manifest-path, --package, --workspace, --exclude */ }
impl WorkspaceSelectorArgs {
    pub fn into_request(self, start_dir: Utf8PathBuf) -> Result<ResolutionInput, SelectionError>; }
pub struct ResolvedSelection { /* workspace_root, selection, stderr_truncated */ }
pub fn resolve_selection(source: &impl WorkspaceMetadataSource, input: &ResolutionInput)
    -> Result<ResolvedSelection, ResolutionError>;
pub struct Diagnostic { pub code: &'static str, pub class: ExitClass, pub message_fallback: String,
    pub arguments: serde_json::Map<String, serde_json::Value>, pub valid_values: Vec<String> }
// SelectionError, MetadataError, ResolutionError each expose `fn diagnostic(&self) -> Diagnostic`.
```

Ownership and reuse policy, also recorded in developers' guide §3.8:

- The kernel files (`spec.rs`, `model.rs`, `selection.rs`, `error.rs`) are
  pure. They depend only on `semver`, `camino`, `serde`, and `thiserror`.
- `metadata.rs` owns the port and the anti-corruption translation from
  `cargo_metadata`. No other module touches `cargo_metadata` types.
- `cargo_adapter.rs` is the only code that knows Cargo's command line.
- `process/system.rs` is the only process-creation site in `cargo-theorem`.
- Later commands reuse these only through `resolve_selection`,
  `PackageSelection::require_single`, and `WorkspaceSelectorArgs`. A parallel
  implementation, or reparsing of `Cargo.toml` for membership, violates the
  design.

## Revision note

- 2026-11-16: Initial draft from the Wyvern reconnaissance, Cargo probes, and
  Firecrawl research. Awaiting the expert design review and user approval.
