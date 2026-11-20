# Task 4.2.1: Workspace and package resolution through `cargo metadata`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (expert design review applied; awaiting user approval)

Branch: `4-2-1-workspace-and-package-resolution`

Roadmap item: `4.2.1` in `docs/roadmap.md`.

## Purpose / big picture

`cargo theorem` is theoremc's Cargo external subcommand. Cargo runs
`cargo-theorem theorem <args>` from the caller's current directory. It does not
say which workspace or which package the caller means. Every later project
command needs that answer:

- `list`, `get`, `check`, and `generate`;
- `init` and the scaffolding commands;
- `run`.

Today the application assumes the project root is `"."`. This happens in
`crates/cargo-theorem/src/cli.rs`, function `execute`. The application has no
way to name a package.

This task adds one reusable resolution service. It turns Cargo-style selectors
into a deterministic, bounded, ordered set of workspace members:

- `--manifest-path <path>` chooses the `Cargo.toml` that anchors resolution;
- `--package <spec>` chooses named workspace members and may be repeated;
- `--workspace` chooses every workspace member;
- `--exclude <spec>` removes members from a workspace selection. It may be
  repeated and requires `--workspace`.

The service reads the workspace through the stable `cargo metadata` JSON
protocol (format version 1), using the `cargo_metadata` crate. It runs Cargo
through a bounded command runner in a new `theoremc-runner` library crate. That
crate is the "command execution abstraction" that design §5 assigns to
`theoremc-runner`.

The following terms are used throughout:

- A *package specification* (spec) is the string a caller types to name a
  package.
- A *workspace member* is a package that Cargo lists in `workspace_members`.
- *Default members* are the members Cargo itself would select when no selector
  is given. Cargo reports them in `workspace_default_members`, computed
  relative to the current directory or the explicit manifest.

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

After the change, a later command can call the service and observe the
following behaviour:

- From `fixture/` with no selectors, it receives `app` and `foo`, in name
  order.
- With `--package foo`, it receives the member `foo 0.1.0`, never the excluded
  `vendor/foo`.
- With `--workspace --exclude foo-bar`, it receives `app`, `foo`, and
  `foo_bar`.
- With `--package fop`, it receives a usage diagnostic with code
  `package.unknown`. The diagnostic enumerates the valid package identifiers
  `app`, `foo`, `foo-bar`, and `foo_bar`, and suggests `foo`.
- A command that needs exactly one package, run from `fixture/` without
  selectors, receives a `selection.ambiguous` diagnostic. It enumerates `app`
  and `foo` and tells the caller to pass `--package`.

No command is registered with these selectors in this task. The command line
that users see does not change. `cargo theorem context --json` still advertises
only `context`.

The first consumers arrive with two later roadmap tasks. Task 4.2.3 adds `list`,
`get`, `action list`, and `action get`. Task 4.2.4 adds `check`. Those
commands will flatten the selector argument structs into their own arguments,
call the service once per invocation, and render its structured diagnostics
through the JSON envelope from task 4.1.4.

The service is therefore delivered as a documented, crate-public library
surface (Decision D3). Its observable contract is exercised through that public
boundary against the real `cargo` executable. The roadmap success criterion for
4.2.1 is thus evidenced at the library boundary rather than through a command,
as Decision D12 records. Command-level evidence follows in 4.2.3 and 4.2.4.

This task closes questions that `docs/cargo-theorem-cli-design.md` §6.4 leaves
implicit:

- what a package identifier is;
- which exit class an ambiguous or unknown package uses;
- what "eligible members" means;
- whether the selectors are global flags;
- how `cargo metadata` runs within bounded time and output;
- which Cargo behaviours theoremc keeps or deliberately tightens.

These answers are recorded in a new ADR, ADR-0014, and in design §6.4.

## Constraints

- Implementation begins only after the user approves this plan.
- Selection must use `cargo metadata --format-version=1 --no-deps`, parsed
  through the `cargo_metadata` crate (design §6.4).
  - Do not parse `Cargo.toml` to discover members.
  - Do not invoke `cargo pkgid`, which resolves dependencies and may write
    `Cargo.lock`.
- Resolution is read-only:
  - It must not create, modify, or delete any file in the inspected project,
    including `Cargo.lock` and `target/`.
  - It must not trigger network access, including rustup toolchain
    auto-installation on rustup 1.28.1 or later (Decision D8).
- Resolution depends only on three inputs, never on configuration values:
  - command-line selectors;
  - the injected `CARGO` environment value;
  - the caller's start directory.

  It must run before project-scope configuration discovery, because design §6.3
  needs the workspace root to find `theoremc.toml` (`INV-config-free`).
- Every process is created through `theoremc_runner::CommandRunner`, with an
  argument vector. There is no shell and no string interpolation (design §16).
  Every child has a deadline, bounded retained output, and guaranteed reaping
  (AGENTS.md, "Architecture and operational boundaries").
- Production code reads the process environment only through the existing
  injected `ortho_config::EnvSource` seam. `clippy.toml` denies `std::env::var`,
  `var_os`, `set_var`, and `set_current_dir`. Tests must not mutate the
  process environment. A child process's environment may be set through
  `Command::env`.
- The selectors are command-line inputs, not configuration values (design
  §6.5). They must not become OrthoConfig fields or `THEOREMC_*` variables.
  They are flattened per project command, not declared as Clap-global
  arguments, because `context` must not accept them (Decision D13).
- Do not register the selectors on any command in this task. Do not change the
  agent context either (design §6.6).
- Keep OrthoConfig, Clap, and `cargo_metadata` out of the facade and out of the
  reusable library crates (ADR-0009, design §4.1). `theoremc-runner` depends on
  none of them.
- Every file stays under 400 lines.
- Every module starts with a `//!` comment.
- Every public item has Rustdoc with an example. Doctests use a fake runner or
  `no_run`, so they never spawn Cargo.
- Lints are not silenced except as a documented last resort, using an
  item-scoped `#[expect(..., reason = "...")]`.
- Do not use `.expect()` or `.unwrap()` outside tests. Test helpers return
  `Result`.
- Every assertion carries a diagnostic message.
- Errors use `thiserror`. Public structs and enums that may grow carry
  `#[non_exhaustive]`.
- The workspace MSRV is Rust 1.89 (ADR-0010). The dependency picture is:
  - `cargo_metadata` 0.23.1 declares Rust 1.86, so it is compatible.
  - `wait-timeout` 0.2.1 is already locked.
  - `cargo-util-schemas` declares Rust 1.97, so it is excluded (Decision D5).
- Completed ExecPlans are historical and must not be edited.

## Tolerances (exception triggers)

The comparison base is `origin/main` at `4c05966`. Count changes with
`git diff --numstat origin/main...HEAD`. Exclude this plan file and
`Cargo.lock`.

`Cargo.lock` is generated. It is checked only for unexpected new crates, under
the Dependencies trigger.

Limits measure net lines (additions minus deletions). Classify each file whole:

- Code: `*.rs`, `*.feature`, and test data under `crates/*/tests/`.
- Configuration: every `Cargo.toml` and `.config/nextest.toml`.
- Documentation: `docs/**/*.md`, except this plan.

The survey behind these numbers is in `Context and orientation`.

- Files: stop and escalate if more than 55 non-ExecPlan files change.
- Code: stop and escalate if code exceeds 3,400 net lines. The estimate is
  about 1,350 production lines and 1,650 test lines.
- Configuration: stop and escalate if configuration exceeds 90 net lines. The
  estimate is about 60.
- Documentation: stop and escalate if documentation exceeds 650 net lines. The
  estimate is about 480.
- Interface: stop and escalate if a public surface is needed beyond the one
  authorized in `Interfaces and dependencies`, which is:
  - the new `theoremc-runner` crate;
  - the `cargo_theorem::workspace` module;
  - the `cargo_theorem::diagnostic` module.

  Also stop and escalate on any change to existing CLI flags, the agent
  context, or the JSON envelope rendering.
- Dependencies: stop and escalate if a dependency outside the authorized set is
  needed, or if `Cargo.lock` gains a crate that is not already locked.
- Architecture: stop and escalate if any of the following is needed:
  - a process created outside `theoremc-runner`;
  - Cargo, Clap, or OrthoConfig types in `theoremc-runner`;
  - Cargo code in a consumer-facing library crate;
  - a contradiction of ADR-0009 or of design §§6.3 to 6.6 beyond the
    amendments ADR-0014 records.
- Iterations: stop and escalate if a focused test still fails after five fix
  attempts.
- Time: stop and escalate if any milestone takes more than eight working hours.
- Ambiguity: stop and present options if Cargo's or rustup's observed behaviour
  contradicts an axiom in `Verification plan`.

Enforce each category independently. Spare capacity in one category never
offsets another.

## Risks

- Risk: `ortho_config` 0.10.0's `OrthoConfig` derive may reject a
  `#[command(flatten)]` field whose type is a non-OrthoConfig `clap::Args`
  struct, even when the field carries `#[serde(skip)]`.
  - Severity: medium. Likelihood: medium.
  - Mitigation: EP-M3 starts with a compile test of a test-only command struct
    that derives `OrthoConfig` and flattens `WorkspaceSelectorArgs`. If the
    derive refuses, later commands merge the selectors through Clap's
    `Args::augment_args` instead. Record the outcome in
    `Surprises & discoveries`.
- Risk: the runner hangs or leaks a child process.
  - A wrapper `CARGO` program can close its pipes and keep running.
  - A grandchild can hold the pipes after the child exits.
  - A child can fill one pipe while the runner waits on the other.
  - Severity: high. Likelihood: medium.
  - Mitigation:
    - Drain each pipe on its own thread.
    - Bound every wait by one `Instant` deadline, using
      `wait_timeout::ChildExt::wait_timeout` in slices of at most 25 ms. Each
      slice also checks the stdout overflow flag.
    - Kill and reap on overflow or deadline.
    - Bound the reader joins by a 2-second pipe grace after the child exits.
      If a pipe is still open after that grace, return `StreamsHeldOpen`.
  - EP-M2 runner cases 1 to 8 exercise each path with real `sh` children.
- Risk: reader threads that remain blocked after the pipe grace are detached
  and hold up to their stream limit in memory until the grandchild exits.
  - Severity: low for one-shot CLI calls. Severity: medium for the
    long-running jobs of roadmap 4.4.
  - Likelihood: low. Probe 5 shows `cargo metadata --no-deps` runs only the
    rustup proxy and `cargo`.
  - Mitigation: developers' guide §3.8 records that detaching is acceptable
    only for one-shot commands. Task 4.4.1 owns process-tree ownership,
    meaning process groups and group kill. This task deliberately avoids a
    separate process group so that terminal Ctrl-C still reaches the child
    (Decision D7).
- Risk: rustup older than 1.28.1 ignores `RUSTUP_AUTO_INSTALL=0`. When
  `cargo-theorem theorem` is run directly, `CARGO` is unset and `cargo`
  resolves through the rustup proxy. If the project pins an uninstalled
  toolchain, rustup may then download it.
  - Severity: medium. Likelihood: low, because current rustup is 1.29.3.
  - Mitigation:
    - ADR-0014 and the CLI user's guide state the minimum rustup version for
      the no-download guarantee.
    - When Cargo dispatches the subcommand, it sets `CARGO` to the toolchain's
      real `cargo`, which bypasses the proxy.
    - A real-proxy test pins a toolchain name that cannot be installed. It
      checks that the failure classifies quickly as `workspace.metadata_failed`.
- Risk: `cargo_metadata` turns on `serde_json`'s `unbounded_depth` feature
  through feature unification.
  - Severity: low. Likelihood: certain.
  - Mitigation:
    - Retained stdout is capped at 16 MiB before parsing.
    - Cargo's TOML parser bounds nesting in `package.metadata`.
    - The input comes from local Cargo.
    - ADR-0014 records this risk.
- Risk: `cargo_metadata::WorkspaceDefaultMembers` panics if it is dereferenced
  when Cargo did not emit the field (Cargo older than 1.71).
  - Severity: medium. Likelihood: low.
  - Mitigation: `τ` checks `is_available()` before any read. A JSON fixture
    without the field must produce `cargo.unsupported_version`, not a panic.
- Risk: real-Cargo fixture tests contend for locks, or hang for the
  `nested-cargo-builds` group's 600-second allowance.
  - Severity: low. Likelihood: low.
  - Mitigation: the binaries join that group, as developers' guide §3.3
    requires. Their own override sets `slow-timeout = { period = "10s",
    terminate-after = 3 }`.
- Risk: task 4.1.4 (the JSON envelope) is not complete, so the structured
  diagnostics have no CLI renderer yet.
  - Severity: low. Likelihood: certain.
  - Mitigation: `Diagnostic` carries every field of the envelope in design
    §8.3. Its pinned `arguments` keys are fixed by insta snapshots, so 4.1.4
    adopts it unchanged.
- Risk: creating `theoremc-runner` now fixes the runner's API before backends
  exercise it.
  - Severity: medium. Likelihood: medium.
  - Mitigation:
    - Keep the surface minimal and pre-1.0.
    - Align the field names with design §§8.6 and 12.1.
    - Record that 4.4.1 extends the same trait, adding protected inputs,
      cancellation, and file-streamed sinks, and updates every caller in the
      same change. No compatibility layer is introduced.

## Progress

- [x] (2026-11-16 19:05Z) Reconnaissance completed:
  - a Wyvern survey of the crate, the tooling, and the docs;
  - Cargo behaviour probes;
  - Firecrawl research on `cargo_metadata`, package ID specifications, and
    rustup.
- [x] (2026-11-16 19:40Z) Initial draft committed (`f33ef86`).
- [x] (2026-11-16 20:30Z) Expert design review completed. The three panels
  covered structure and alternatives, contracts and scaling, and failure modes
  and rigour. Their findings are applied in this revision; see the revision
  note.
- [ ] Plan approved by the user.
- [ ] `EP-M1` Decision records, diagnostics, and the pure selection kernel.
- [ ] `EP-M2` `theoremc-runner` and the `cargo metadata` adapter.
- [ ] `EP-M3` Resolution service, selector arguments, spans, and real-Cargo
  fixtures.
- [ ] `EP-M4` Guides, roadmap closure, and delivery.

## Surprises & discoveries

- Observation: Cargo refuses a workspace that contains two members with the
  same package name.
  - Evidence: probe 1. Both `cargo metadata` and `cargo check -p foo` fail with
    ``two packages named `foo` in this workspace``.
  - Impact: a bare name is never ambiguous among members. The "ambiguous names"
    fixtures cover the cases that remain:
    - a member and a non-member that share a name;
    - names that differ only by `-` versus `_`;
    - single-target commands that face several default members.
- Observation: `workspace_default_members` already gives Cargo's default
  selection relative to the current directory or the explicit manifest.
  - Evidence: probe 3.
    - From inside member `b`, or with `--manifest-path b/Cargo.toml`, the
      default is `[b]`.
    - From a virtual root with `default-members = ["a"]`, it is `[a]`.
    - Without `default-members`, it is every member.
    - From a non-virtual root, it is the root package.
  - Impact: theoremc does not re-implement Cargo's default-selection rules
    (`AXIOM-default-members`).
- Observation: `cargo metadata --no-deps` creates no `Cargo.lock` and no
  `target/` directory, and starts no `rustc`.
  - Evidence: probes 3 and 5. The `find` snapshot is unchanged. strace shows
    `execve` only for the rustup proxy and `cargo`.
  - Impact: read-only resolution is achievable. A regression that drops
    `--no-deps` makes Cargo write `Cargo.lock` in a fixture that has a path
    dependency, so the regression is detectable.
- Observation: Cargo only warns when an `--exclude` matches nothing. It rejects
  `--exclude` without `--workspace`.
  - Evidence: probe 2.
  - Impact: Decision D4 tightens the first behaviour into an error and keeps
    the second.
- Observation: members named `foo-bar` and `foo_bar` can coexist without a
  warning.
  - Evidence: probe 6.
  - Impact: the fixture pair is valid.

## Decision log

- D1. Decision: package placement.
  - The selection kernel, the `cargo metadata` adapter, and the selector
    arguments live in the `cargo-theorem` package as `pub mod workspace`.
  - The bounded command runner lives in a new library crate,
    `crates/theoremc-runner`.

  ADR-0014 amends developers' guide §3.8 ("Reusable … project … services remain
  in library packages") and design §5 ("The CLI must not become the only place
  where reusable project … logic exists") for workspace selection specifically.

  Rationale:
  - Workspace selection takes only CLI inputs, and every consumer is a CLI
    command. Build scripts obtain their package from `CARGO_MANIFEST_DIR`.
  - Design §4.1 keeps Cargo metadata dependencies out of the facade.
  - The planned `theoremc-build` (roadmap 4.3.1) is a dependency of every
    consumer.
  - The runner, by contrast, must be reachable from library-owned backend
    providers (design §12.1). A library cannot depend on `cargo-theorem`
    (design §5), so the runner gets the crate that design Table 1 already
    names.

  Rejected alternatives:
  - Both modules in `cargo-theorem`. Roadmap 4.4.1 could not then extend the
    runner.
  - A `theoremc-project` crate. It has no second consumer.

  Extraction trigger: move `workspace` into a library when a package other than
  `cargo-theorem` needs it.

  Date/Author: 2026-11-16, planning agent, revised after review.
- D2. Decision: a package specification has one of four forms:
  - `name`;
  - `name@version`;
  - `name:version`, where the version may be partial: `1`, `1.4`, or `1.4.3`,
    with an optional pre-release;
  - a complete package ID, exactly as `cargo metadata` prints it.

  Matching rules:
  - Matching is exact and considers workspace members only.
  - There is no case folding, no `-`/`_` normalization, no prefix matching,
    and no matching of non-members.
  - A non-ID spec containing `*`, `?`, or `[` is rejected as
    `package.glob_unsupported`. ID specs are exempt, because a path may contain
    `[`.
  - Names are the canonical identifiers in every output, because Cargo
    guarantees they are unique within a workspace.
  - `name@version` is the documented stable fully-qualified form. Package IDs
    are opaque, and their format changed in Cargo 1.77.

  Rationale: this is the subset of the Cargo package ID specification grammar
  that callers actually type. Exactness makes the `foo-bar`/`foo_bar` case and
  the member/non-member case deterministic. Partial URL specs are deferred,
  because they would require parsing opaque IDs.

  Date/Author: 2026-11-16, planning agent.
- D3. Decision: `workspace` and `diagnostic` are `pub` modules of the
  `cargo-theorem` library. Their types carry `#[non_exhaustive]` where they may
  grow.

  Rationale: the integration and BDD suites must exercise the contract across
  the crate boundary, as AGENTS.md "exercise the boundary that owns the
  promise" requires. The later commands in the same crate then consume the
  modules. Their documentation states that `pub` here establishes no downstream
  contract beyond pre-1.0 semver.

  Date/Author: 2026-11-16, planning agent, rationale restated after review.
- D4. Decision: selection rules.
  - With no selector, the result is Cargo's default members.
  - `--package` may repeat. The result is the union of the matches, sorted by
    name and deduplicated.
  - `--workspace` selects every member. In this task every member is eligible.
  - `--exclude` requires `--workspace`, enforced by Clap `requires`.
  - `--package` conflicts with `--workspace`, enforced by Clap
    `conflicts_with`.
  - Clap renders both of those violations as the existing `cli.usage`.
  - Every spec that matches no member is an error, including an `--exclude`
    spec, where Cargo only warns.
  - An empty final selection is an error.
  - Unmatched specs are reported in command-line order, bounded to 50 entries
    with a total count.

  Eligibility is decided now, for later commands, though not implemented here:
  - A command that narrows eligibility, such as to theoremc-enabled packages,
    applies the filter after selection.
  - `--package` and `--exclude` specs are always validated against all members,
    so narrowing never turns a working exclude into an error.
  - An explicit `--package` that names an ineligible member yields
    `package.ineligible`, class usage. The narrowing command introduces that
    code.

  Rationale: an agent-native CLI should fail closed on typos, because a
  silently ignored exclusion would widen a later mutation. ADR-0014 and the CLI
  user's guide document the stricter rule as a deliberate difference from Cargo.

  Date/Author: 2026-11-16, planning agent, refined after review.
- D5. Decision: parse specs and partial versions with a small in-crate parser
  built on `semver`, rather than `cargo-util-schemas::PackageIdSpec`.

  Rationale: `cargo-util-schemas` 0.14.3 declares Rust 1.97, above the
  workspace MSRV. It would also pull in `toml`, `url`, `jiff`, and others for a
  grammar subset under 150 lines.

  Date/Author: 2026-11-16, planning agent.
- D6. Decision: exit classes and diagnostic codes, mapped onto design §8.4 and
  defined by one `DiagnosticCode` enum whose `class()` method is the single
  mapping table.

  | Class                 | Exit code | Diagnostic codes                                                                                                                                               |
  | --------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | Usage                 | 2         | `package.unknown`, `package.glob_unsupported`, `package.invalid_spec`, `selection.empty`, `selection.ambiguous`, `manifest.not_found`, `manifest.invalid_name` |
  | Project state         | 4         | `workspace.metadata_failed`                                                                                                                                    |
  | External tool failure | 8         | `cargo.unavailable`, `cargo.terminated`, `cargo.timed_out`, `cargo.output_limit`, `cargo.metadata_invalid`, `cargo.unsupported_version`                        |

  Classification notes:
  - `workspace.metadata_failed` covers a Cargo run that completed with a
    non-zero exit code. Examples are an invalid manifest, duplicate member
    names, no `Cargo.toml` above the start directory, and an uninstalled
    pinned toolchain under `RUSTUP_AUTO_INSTALL=0`.
  - `cargo.terminated` means Cargo was killed by a signal.
  - `cargo.timed_out` also covers pipes held open past the grace period.
  - `cargo.unsupported_version` means `workspace_default_members` is absent.
  - Clap-level selector misuse remains `cli.usage`.

  Pinned `arguments` keys:
  - `specs` and `flag` (`package` or `exclude`) for unmatched specs;
  - `specs_total`;
  - `valid_values_total` and `valid_values_truncated`;
  - `suggestions`, at most 5, ranked by normalized-equality then edit distance
    of at most 2, then name;
  - `cargo_exit_code` and `stderr_excerpt`, the last 4 KiB as lossy UTF-8 cut
    on a character boundary;
  - `stderr_truncated`;
  - `manifest_path`.

  `valid_values` lists at most 50 member names in ascending order, matching the
  default list limit in design §8.5.

  Rationale:
  - Design §8.3's own example classifies an unknown enumerated value as `usage`
    with `valid_values`.
  - Theoremc does not parse Cargo's localized stderr. It therefore classifies a
    completed non-zero Cargo run as project state, and a failure to obtain any
    result as an external tool failure.
  - Suggestions keep the diagnostic useful in workspaces with more than 50
    members.

  Date/Author: 2026-11-16, planning agent, revised after review.
- D7. Decision: the `theoremc-runner` crate provides the `CommandRunner` trait
  and one production adapter, `SystemCommandRunner`. Its names follow design
  §12.1 and §8.6.
  - `CommandSpec` has `program: PathBuf`, `argv: Vec<OsString>`,
    `current_dir`, environment overrides, and per-stream `StreamLimit`. A
    stream limit has a byte bound and an `OverflowPolicy` of `Fail` or
    `Truncate`.
  - `CapturedStream` has `bytes`, `bytes_seen`, `bytes_retained`,
    `limit_bytes`, and `truncated`.
  - `CommandOutput` carries the exit code or the terminating signal, both
    captured streams, and the elapsed `Duration`.
  - There is one deadline per run, and no process group.
  - Out of scope for this task: protected inputs, cancellation tokens,
    file-streamed sinks, and process-tree ownership. Roadmap 4.4.1 extends the
    trait and updates every caller in the same change.

  Rationale: `cargo_metadata::MetadataCommand::exec` uses `Command::output`,
  which has no deadline and no bound on output. Design §12.1 and developers'
  guide §3.8 require one shared runner. This task is the first production
  process creation in the workspace. The adapter parses the bounded stdout with
  `MetadataCommand::parse`. Avoiding a process group keeps terminal Ctrl-C
  delivery for one-shot commands.

  Date/Author: 2026-11-16, planning agent, revised after review.
- D8. Decision: the `cargo metadata` invocation contract.
  - Program: the injected `CARGO` value, which Cargo sets for external
    subcommands, else `cargo`.
  - Argument vector: `metadata --format-version=1 --no-deps --offline`, plus
    `--manifest-path <path>` when one is given explicitly.
  - Working directory: the caller's start directory.
  - Environment set on the child only: `RUSTUP_AUTO_INSTALL=0` and
    `CARGO_TERM_COLOR=never`.
  - Deadline: 30 seconds.
  - Stdout: at most 16 MiB with policy `Fail`, because truncated JSON is
    useless.
  - Stderr: at most 64 KiB with policy `Truncate`.
  - The limits are fixed constants in this task. Design §8.6 lets
    configuration lower limits, but resolution must run before configuration
    discovery (`INV-config-free`), so a configurable deadline is deferred.
  - A relative `--manifest-path` is validated against the start directory.

  Rationale:
  - `--no-deps` makes the call read-only (probes 3 and 5), and `--offline`
    asserts that no network is needed.
  - Measured output is about 33 KB for six members in 0.15 s. 16 MiB therefore
    holds more than a thousand members.
  - A 30-second deadline bounds an agent's wait on a hang.
  - Rustup 1.28.1 documents `RUSTUP_AUTO_INSTALL=0`.

  Date/Author: 2026-11-16, planning agent, revised after review.
- D9. Decision: telemetry.
  - Add `tracing` spans now:
    - `theoremc_runner` emits `process.run`, with fields `program`
      (file-name component only), `exit_code`, `elapsed_ms`, `stdout_bytes`,
      `stderr_truncated`, and `outcome`;
    - `cargo_theorem::workspace` emits `project.resolve`, with fields
      `anchor` (`discover` or `explicit`), `request` (`default`, `packages`,
      or `workspace`), `members`, `selected`, and `outcome` (a diagnostic code
      or `ok`).
  - Spans carry no paths, specs, or raw stderr (design §17.2).
  - Verify the spans with a scoped `tracing::subscriber::with_default` test
    subscriber built on `tracing-subscriber`. Nothing global is installed.
  - Defer metrics to roadmap 4.5.4, which owns the metric registry.

  Rationale: design §17 requires spans for project resolution and child
  processes, and 4.5.4 covers only metrics and events. `tracing` 0.1.44 and
  `tracing-subscriber` 0.3.23 are already locked. Installing the application's
  subscriber stays with roadmap step 4.5.

  Date/Author: 2026-11-16, planning agent, revised after review.
- D10. Decision: no Verus proof and no Kani harness. Verification uses:
  - named `rstest` tables;
  - a bounded exhaustive test against an independent bitmask oracle;
  - `proptest` properties;
  - integration and BDD tests against real Cargo.

  Rationale: see `Formal verification decision`.

  Date/Author: 2026-11-16, planning agent, confirmed by review.
- D11. Decision: selector flags are long options only, matching design §6.4.
  Cargo's `-p` short form is not added.

  Rationale: a short flag would reserve `-p` across every future project
  command. That choice belongs to the commands that adopt the selectors.

  Date/Author: 2026-11-16, planning agent.
- D12. Decision: the roadmap criterion, "fixtures cover virtual workspaces,
  ambiguous names, excluded members, and explicit manifests; ambiguity
  diagnostics enumerate valid package identifiers", is evidenced at the library
  boundary in this task. No command consumes the service yet, and design §6.6
  forbids advertising one. Command-level JSON evidence follows in 4.2.3 and
  4.2.4, after 4.1.4.

  Rationale: a placeholder command would violate design §6.6. Folding this task
  into 4.2.4 would invert the roadmap's `Requires 4.2.1`. This decision needs
  explicit user acceptance at plan approval.

  Date/Author: 2026-11-16, planning agent.
- D13. Decision: two argument structs, flattened per command.
  - `WorkspaceSelectorArgs`: `--manifest-path`, repeated `--package`,
    `--workspace`, and repeated `--exclude`. It serves read-only and
    workspace-capable commands.
  - `SinglePackageArgs`: `--manifest-path` and at most one `--package`. It
    serves single-target mutation commands, so their help and agent context
    never advertise `--workspace`.

  Both structs produce a `ResolutionInput`. Neither is declared Clap-global.
  `takes_global_value` in `cli_diagnostics.rs` stays unchanged, and must learn
  any value-taking selector that is ever made global. Design §6.4 is reworded
  from "global selectors" to "common project-command selectors, accepted after
  the subcommand".

  Rationale: Clap-global arguments would be inherited by `context`, which
  design §6.6 forbids.

  Date/Author: 2026-11-16, planning agent, added after review.
- D14. Decision: `Diagnostic`, `DiagnosticCode`, and `ExitClass` live in an
  application-level `cargo_theorem::diagnostic` module, not inside `workspace`.
  - `ExitClass` serializes in `snake_case`, matching `as_str()`.
  - `ApplicationError::exit_code` is re-expressed through `ExitClass`, with no
    behaviour change.
  - Task 4.1.4 folds the existing `cli.usage`, `configuration.invalid`, and
    `application.internal` literals into `DiagnosticCode` when it renders the
    envelope.

  Rationale: future `backend.*` and `run.*` codes must not import a
  workspace-specific type. A single code enum makes it impossible to pair a
  code with the wrong class.

  Date/Author: 2026-11-16, planning agent, added after review.

## Outcomes & retrospective

Not started. Complete this section at each milestone boundary and at completion.

## Context and orientation

### Inspected base revision

The inspected base revision is `4c05966`, "Chore(deps): bump rust-toolchain
(#255)".

The workspace members are the root `theoremc` package and these crates:

- `crates/test-helpers`;
- `crates/theoremc-build-support`;
- `crates/cargo-theorem`;
- `crates/theoremc-core`;
- `crates/theoremc-macros`.

The toolchain is `nightly-2026-11-16` (`rust-toolchain.toml`). The declared
MSRV is 1.89.

### The application package today

`crates/cargo-theorem/` builds the `cargo-theorem` executable.

- `src/main.rs` calls `cargo_theorem::run(std::env::args_os())`.
- `src/lib.rs` declares the private modules `agent_context`, `cli`, `config`,
  and `error`. It re-exports only `run`.
- `src/cli.rs` (299 lines) defines the Clap tree:
  - `CargoEntry` holds `CargoCommand::Theorem(TheoremArgs)`.
  - `TheoremArgs` flattens `config::GlobalOptions` and adds `--profile`.
  - `Commands` has a single variant, `Context(ContextArgs)`.
  - `run_with_sources_and_io` parses with
    `ortho_config::parse_localized_command`.
  - `execute` calls `effective_settings`, which loads global configuration,
    then calls
    `ortho_config::load_globals_and_merge_selected_subcommand_with_sources`
    with `let project_root = Path::new(".")`.
- `src/cli_diagnostics.rs` (178 lines) renders Clap errors and application
  errors.
  - The JSON diagnostic has `schema_version`, `kind`, `command`, `status`, and
    `error.{code, class, message_fallback}`.
  - It does not render `arguments` or `valid_values`; task 4.1.4 owns that.
  - `takes_global_value` lists value-taking global flags by hand.
- `src/error.rs` defines `ApplicationError` and its exit codes:
  - `Configuration` exits 3;
  - `Serialization` exits 10;
  - `Output` exits 10.

  Exit classes 4 to 9 are not produced yet.
- `src/config/mod.rs`, function `process_sources()`, returns the injected
  `ortho_config::SharedEnvSource` and `SharedScanEnvSource`. Tests inject
  `ortho_config::MapEnv`.

### Dependencies and process creation

No production code in the workspace spawns a process. Nothing depends on
`cargo_metadata` directly.

`Cargo.lock` already contains:

- `cargo_metadata` 0.23.1 and 0.18.1;
- `camino` 1.2.6;
- `semver` 1.0.28;
- `wait-timeout` 0.2.1;
- `tracing` 0.1.44 and `tracing-subscriber` 0.3.23.

`cargo-orthohelp` 0.10.0 has a private `metadata.rs` that looks packages up by
exact name. It cannot be reused, because the module is private to its binary.

### Tests and tooling

The crate's existing tests are:

- `src/cli_tests.rs` and `src/cli_tests/runner.rs`, which run in-process with
  `MapEnv`;
- `src/config_tests.rs`;
- `tests/entry_points.rs` with
  `tests/entry_points/{process,support, agent_context}.rs`, which run the built
  binary through an isolated `PATH` with `env_clear()`.

The crate's dev-dependencies are `anyhow`, `cap-std`, `rstest`, and `tempfile`.

Across the workspace:

- `rstest-bdd` 0.6.0 is in use. The developers' guide §3.3 still says v0.5.0,
  which is stale.
- `.feature` files live under the root `tests/features/`, with bindings in
  `tests/<name>_bdd.rs`.
- `insta` is used only in `theoremc-macros`.
- `googletest`, `pretty_assertions`, and `proptest` are used in the root
  package, `theoremc-core`, and `theoremc-macros`.

`.config/nextest.toml` assigns tests that start a child `cargo` to the
`nested-cargo-builds` group (`max-threads = 1`) through `binary(...)` filters.
That group allows 600 seconds per test.

The Makefile gates are:

- `make check-fmt`: `cargo fmt --all -- --check` and `mdtablefix`.
- `make lint`: rustdoc with `-D warnings`, then Clippy over all targets and
  features with `-D warnings`, then Whitaker.
- `make test`: `cargo fetch --locked`, then
  `cargo nextest run --workspace --all-targets --all-features`, then doctests.
- Also available: `make markdownlint`, `make nixie`, `make spelling`,
  `make msrv`, and `make check-agent-native`.

There is no Verus tooling in the repository. Kani runs only through
`make kani`, against generated harnesses.

### Documentation to update

- `docs/cargo-theorem-cli-design.md` §6.4 is the normative selection contract.
  §5 lists the package responsibilities, Table 1 names `theoremc-runner`, and
  §12.1 holds the runner sketch.
- `docs/cargo-theorem-cli-users-guide.md`, lines 65 to 79, describes the
  selectors in the present tense while also saying project commands are not
  accepted yet. That inconsistency needs fixing.
- `docs/developers-guide.md`:
  - §3.8 is the application-boundary section and serves as the `cargo-theorem`
    component architecture.
  - §3.3 holds the test conventions.
- ADRs live in `docs/adrs/`, and the next number is 0014.
- `docs/contents.md` and `docs/repository-layout.md` index the documents and
  crates.
- `docs/users-guide.md` covers the library and the macro only. It needs no
  change, because no library or command behaviour changes.
- `docs/CHANGELOG.md` is OrthoConfig's imported changelog, not this project's.
  It is not touched.

### Skills and references for the implementer

Load `rust-router` first, then these skills:

- `rust-types-and-apis` for the spec, selection, and diagnostic types;
- `rust-errors` for the error enums;
- `rust-unit-testing` for rstest, googletest, pretty_assertions, and insta;
- `proptest`;
- `hexagonal-architecture` for the port and adapter split;
- `domain-cli-and-daemons` and `rust-async-and-concurrency` for the runner's
  threads and deadline;
- `arch-crate-design` for `theoremc-runner`;
- `arch-decision-records` for ADR-0014;
- `execplans`.

Repository references:

- `docs/rust-testing-with-rstest-fixtures.md`;
- `docs/rstest-bdd-users-guide.md`;
- `docs/reliable-testing-in-rust-via-dependency-injection.md`;
- `docs/rust-doctest-dry-guide.md`;
- `docs/complexity-antipatterns-and-refactoring-strategies.md`;
- `docs/ortho-config-users-guide.md`;
- `docs/developers-guide.md`;
- `docs/theoremc-design.md` (DES);
- `docs/theorem-file-specification.md` (TFS), for context only, because theorem
  files are not read in this task.

### Budget survey

Production code is estimated at about 1,350 lines:

| Area                                     | Lines |
| ---------------------------------------- | ----- |
| `spec.rs`                                | 180   |
| `model.rs` and `metadata.rs` translation | 220   |
| `selection.rs`                           | 160   |
| `suggest.rs` (edit distance and ranking) | 70    |
| Workspace errors                         | 120   |
| `diagnostic.rs`                          | 140   |
| Cargo adapter                            | 150   |
| Selector argument structs                | 90    |
| Resolution service and span              | 60    |
| `theoremc-runner`                        | 320   |

Test code is estimated at about 1,650 lines:

| Area                                  | Lines |
| ------------------------------------- | ----- |
| Spec tables                           | 200   |
| Kernel tables                         | 150   |
| Exhaustive oracle                     | 220   |
| Properties                            | 200   |
| Translation tables and JSON documents | 160   |
| Fake-runner adapter tests             | 180   |
| Runner tests                          | 240   |
| Span tests                            | 80    |
| Fixture builder                       | 120   |
| Integration tests                     | 220   |
| BDD bindings                          | 80    |

Comparable plans provide the history:

- `docs/execplans/3-1-1-build-rs-scanning-of-theorems.md` set "roughly 10
  changed files or 600 net lines" and never revised it.
- `docs/execplans/3-1-2-generate-out-dir-theorem-suite-rs.md` set "roughly 12
  changed files or 700 net lines" and never revised it.
- `docs/execplans/4-1-2-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli.md`
  set no line budget. Its one size issue was a 400-line breach in `cli.rs`,
  fixed by splitting out the tests.

No plan shows repeated increases. This task is about four times larger than the
3.1.x tasks, because it adds a crate, a process boundary, and an exhaustive
oracle. The limits therefore carry 13% to 35% contingency over the estimates.

## Conformance basis

These upstream artefacts were inspected at `4c05966`:

- `docs/roadmap.md`, task 4.2.1 and step 4.2;
- `docs/cargo-theorem-cli-design.md`, §§4.1, 5 (Table 1), 6.3 to 6.6, 8.3 to
  8.6, 12.1, 15, 16, 17, and 18;
- `docs/developers-guide.md`, §§3.3, 3.8, and 3.9;
- ADR-0008 (build-support boundary), ADR-0009 (OrthoConfig CLI
  configuration), and ADR-0010 (MSRV);
- AGENTS.md, the governing standard.

No Terms of Reference document exists for this work.

Requirements:

- `REQ-selectors`: the service accepts `--manifest-path`, repeated `--package`,
  `--workspace`, and repeated `--exclude`. Source: design §6.4.
- `REQ-protocol`: workspace data comes from `cargo metadata` format version 1,
  through `cargo_metadata`. Source: design §6.4.
- `REQ-virtual`: virtual workspaces resolve correctly, including their default
  members. Source: the roadmap's success criterion.
- `REQ-ambiguity`: ambiguity and unknown-package diagnostics enumerate valid
  package identifiers, within a bound. Sources: the roadmap, and design §§8.3
  and 15.
- `REQ-exclude`: excluded members never appear in a workspace selection.
  Source: the roadmap.
- `REQ-manifest`: an explicit manifest anchors resolution independently of the
  current directory. Sources: the roadmap, and design §6.3.
- `REQ-single`: mutation commands can demand exactly one unambiguous target.
  Source: design §6.4.
- `REQ-readonly`: resolution writes nothing and uses no network. Source: design
  §§10.3 and 16.
- `REQ-bounded`: the Cargo child has a deadline and bounded output. Sources:
  design §8.6, and AGENTS.md.
- `REQ-runner`: processes are created only through a shared, injectable
  command runner owned by a library. Sources: design §§5 and 12.1, and
  developers' guide §3.8.
- `REQ-noadvertise`: no unimplemented command or flag is advertised. Source:
  design §6.6.
- `REQ-telemetry`: project resolution and child-process execution emit spans.
  Source: design §17.

Gaps:

- `GAP-root`: the application uses `"."` as the project root, not the Cargo
  workspace root (design §6.3). This task supplies `workspace_root`.
  - Wiring it into configuration discovery happens with the first project
    command, in 4.2.3 or 4.2.4.
  - That work resolves the workspace before calling `effective_settings`, as
    `INV-config-free` allows.
- `GAP-envelope`: `arguments` and `valid_values` are not rendered yet. Task
  4.1.4 owns them.

Trace links:

```plaintext
design §6.4 -> REQ-selectors -> EP-M3 -> selectors tests, workspace_resolution BDD
design §6.4 -> REQ-protocol -> EP-M2 -> adapter argv test, real-cargo tests
roadmap 4.2.1 -> REQ-virtual -> EP-M1, EP-M3 -> kernel table, BDD "virtual workspace defaults"
roadmap 4.2.1 -> REQ-ambiguity -> EP-M1, EP-M3 -> INV-diagnostic tests, insta snapshots, BDD
roadmap 4.2.1 -> REQ-exclude -> EP-M1, EP-M3 -> INV-exclusion exhaustive test, BDD "excluded members"
roadmap 4.2.1 -> REQ-manifest -> EP-M3 -> BDD "explicit manifest"
design §6.4 -> REQ-single -> EP-M1, EP-M3 -> require_single tests, BDD "single-target"
design §16 -> REQ-readonly -> EP-M3 -> INV-readonly fixture snapshot test
design §8.6 -> REQ-bounded -> EP-M2 -> runner cases 1-8
design §12.1 -> REQ-runner -> EP-M2 -> fake-runner adapter tests, single Command::new site check
design §6.6 -> REQ-noadvertise -> EP-M3 -> agent-context regression test
design §17 -> REQ-telemetry -> EP-M2, EP-M3 -> scoped-subscriber span tests
```

## Verification plan

### Abstract model

Let `M` be a non-empty finite set of workspace members. Each member `m` has
these properties:

- a name `n(m)`;
- a semantic version `v(m)`;
- an opaque package ID `i(m)`;
- a manifest path.

Cargo guarantees that `n` and `i` are injective on `M` (`AXIOM-unique-names`).
Let `D ⊆ M` be Cargo's default members for the chosen anchor
(`AXIOM-default-members`).

A spec `s` denotes a match set `μ(s) ⊆ M`:

- A bare `name` matches `{m | n(m) = name}`.
- `name@p` and `name:p` match `{m | n(m) = name ∧ p ⊑ v(m)}`. Here `p ⊑ v`
  means every component present in the partial version `p` equals the
  corresponding component of `v`. A pre-release, when present, must match
  exactly.
- An ID spec matches `{m | i(m) = id}`.

By injectivity, `|μ(s)| ≤ 1` for every spec.

A request `R` takes one of three forms:

- `Default`;
- `Packages(S)`, where `S` is a non-empty list of specs;
- `Workspace(E)`, where `E` is a possibly empty list of specs.

`U(R)` is the list of unmatched specs: the specs of `S` or `E`, in their order,
deduplicated, for which `μ(s) = ∅`. The selection function is:

```plaintext
sel(Default)       = D
sel(Packages(S))   = ⋃_{s ∈ S} μ(s)
sel(Workspace(E))  = M \ ⋃_{e ∈ E} μ(e)
```

Precedence is as follows:

1. Malformed specs and globs are rejected when they are parsed, before Cargo
   runs.
2. If `U(R) ≠ ∅`, the outcome is `Unknown(U(R), names(M))`.
3. Otherwise, if `sel(R) = ∅`, the outcome is `Empty`.
4. Otherwise, the outcome is the selection, as a list sorted by name with no
   duplicates.

`single(Sel)` is the unique member when `|Sel| = 1`, and otherwise
`Ambiguous(sorted names(Sel))`.

Every enumerated list is truncated to its first 50 entries in its stated order,
and the outcome reports the total. Suggestions are a deterministic function of
the unmatched spec and `names(M)`.

Lemma `L-compose`: `sel` depends on the specs only through their match sets.
Matching (`μ`) and selection (`sel`) can therefore be verified separately. The
kernel is checked over bare names, and the spec forms are checked against `μ`.

The translation `τ` maps a `cargo_metadata::Metadata` value to a snapshot
`(root, M, D)`:

- `M` is the set of packages whose `id` is listed in `workspace_members`.
  Other packages are ignored.
- `D` comes from `workspace_default_members`, after `is_available()` is
  checked.

`τ` fails with `cargo.metadata_invalid` in these cases:

- a member ID has no package;
- a default ID is not a member;
- two members share a name;
- a version cannot be parsed;
- stdout is not UTF-8.

`τ` fails with `cargo.unsupported_version` when the default-member data is
unavailable.

### Invariants and lemmas

- `INV-subset` (`REQ-exclude`, `REQ-ambiguity`): every selected member is in
  `M`. A non-member package never appears, even when it shares a name with a
  member.
- `INV-default` (`REQ-virtual`, `REQ-manifest`): `sel(Default) = D` exactly.
- `INV-package`: `sel(Packages(S)) = ⋃μ(S)`.
- `INV-exclusion` (`REQ-exclude`): the selection meets these conditions:
  - `sel(Workspace(E)) ∩ μ(e) = ∅` for every `e`;
  - `M \ ⋃μ(E) ⊆ sel(Workspace(E))`.
- `INV-order`: an `Ok` selection is strictly increasing by name. It is
  invariant under permutation of the member insertion order, under permutation
  of `S` and `E`, and under duplication of specs.
- `INV-nonempty`: an `Ok` selection is never empty.
- `INV-diagnostic` (`REQ-ambiguity`):
  - `Unknown` lists exactly `U(R)`, bounded.
  - Its `valid_values` are the first `min(50, |M|)` member names in ascending
    order, with total `|M|`.
  - Its suggestions are a subset of `names(M)`, ranked as decision D6 states.
  - `Ambiguous` lists the sorted names of the selection under the same bound.
  - Every listed value names a member.
- `INV-exact`:
  - Names and IDs match only by exact string equality.
  - Versions match by componentwise equality.
  - A spec that differs from a member name by case, by `-` versus `_`, or by
    prefix does not match, unless it equals another member's name.
- `INV-readonly` (`REQ-readonly`): resolution leaves the fixture file tree
  byte-identical, and creates no `Cargo.lock` and no `target/`.
- `INV-bounded` (`REQ-bounded`):
  - Retained stdout is at most its limit.
  - Retained stderr is at most its limit.
  - The runner returns within deadline plus pipe grace plus 1 s.
  - Every child is reaped, or killed and then reaped, before return.
- `INV-config-free`: resolution reads no configuration value. It takes only
  `ResolutionInput`, the `CargoProgram` taken from `EnvSource`, and a runner.
- `L-roundtrip`: for every member `m` of any valid snapshot, the specs `n(m)`,
  `n(m)@v(m)`, `n(m):v(m)`, and `i(m)` each parse, and each has match set `{m}`.
- `L-partial`: for every version `v`, every prefix of `v` (`major`,
  `major.minor`, and the full version with its pre-release) satisfies `⊑ v`. A
  partial version with any component changed does not.
- `L-translation`: `τ` succeeds on every metadata document that satisfies the
  consistency conditions. When it succeeds, it preserves the names, versions,
  and IDs, and the membership of `D`. Non-member packages are dropped.

### Axioms (trusted, not verified here)

- `AXIOM-unique-names`: Cargo rejects a workspace whose members share a package
  name. Source: probe 1, Cargo 1.100.0.
- `AXIOM-default-members`: Cargo 1.71 and later emits
  `workspace_default_members`. It computes the field with its own
  package-selection rules, relative to the working directory or
  `--manifest-path`. Sources: the `cargo metadata` documentation, the
  `cargo_metadata` 0.23.1 documentation, and probe 3.
- `AXIOM-no-deps-readonly`: `cargo metadata --no-deps` resolves no
  dependencies, writes no `Cargo.lock`, and needs no network. Sources: the
  `cargo metadata` documentation and probe 5. The `INV-readonly` test re-checks
  this axiom against the real tool on every run.
- `AXIOM-format-v1`: within format version 1, Cargo only adds fields and enum
  values, and package IDs are opaque. Source: the "Compatibility" section of the
  `cargo metadata` documentation.
- `AXIOM-cargo-env`: Cargo sets `CARGO` for external subcommands. Source: the
  Cargo reference for environment variables.
- `AXIOM-rustup-auto-install`: rustup 1.28.1 and later honours
  `RUSTUP_AUTO_INSTALL=0`. Sources: the rustup 1.28.1 announcement and the
  rustup book.
- `AXIOM-cargo-metadata-parse`: `MetadataCommand::parse` deserializes format
  version 1 faithfully. `WorkspaceDefaultMembers::is_available()` reports
  whether the field was present.
- `AXIOM-pipes`: on Unix, a pipe's read end reaches end-of-file once every
  holder of the write end exits. Killing the direct child does not close pipes
  that a grandchild holds.

### Methods per obligation

#### Kernel invariants

The obligations are `INV-subset`, `INV-default`, `INV-package`, `INV-exclusion`,
`INV-order`, `INV-nonempty`, and `INV-diagnostic`.

- Method: a bounded exhaustive test against an independent oracle, plus named
  `rstest` tables. The tables use `pretty_assertions::assert_eq` for selection
  vectors and `googletest` `elements_are!` matchers for `valid_values`.
- Rationale: the kernel is a finite set computation. Exhaustive enumeration up
  to four members is cheap and stronger than sampling within that bound. The
  only size-dependent behaviour, truncation at 50 entries, has its own named
  test with 120 members.
- Domain:
  - The members are the first `k` names (`1 ≤ k ≤ 4`) of
    `["foo_bar", "foo-bar", "app", "foo"]`, inserted in every one of the `k!`
    orders.
  - Defaults range over every subset `D ⊆ M`.
  - Requests are:
    - `Default`;
    - `Packages` over every non-empty subset of `names(M) ∪ {"fop"}`, given in
      descending-name order so that the input order differs from the output
      order;
    - `Workspace` over every subset of `names(M) ∪ {"fop"}`.
  - For each `k` there are `k!·2^k·2^(k+2) = k!·2^(2k+2)` cases. The total is
    `16 + 128 + 1,536 + 24,576 = 26,256`, which the test asserts exactly.
  - Every `Ok` outcome is also checked with `require_single`.
- Artefact: `crates/cargo-theorem/src/workspace/selection_exhaustive_tests.rs`.
  The oracle uses `u8` bitmasks and set algebra, and shares no code with the
  kernel.
- Evidence:
  `cargo nextest run -p cargo-theorem -E 'test(/selection_exhaustive/)'`
  passes. The test asserts the per-class counts.
- Non-vacuity:
  - Each outcome class (`Ok`, `Unknown`, `Empty`, and single `Ambiguous`)
    occurs at least 100 times. The test asserts these counts.
  - Every request form is reached.
  - Four seeded faults are applied temporarily and reverted:
    1. Ignore exclusions in `select`. `INV-exclusion` must catch it.
    2. Build `PackageSelection` in spec order instead of name order. Inject
       this in `PackageSelection::from_matches`. `INV-order` must catch it,
       because specs arrive in descending order.
    3. Normalize `-` to `_` in `PackageSpec::matches`. `INV-exact` must catch
       it through the `foo-bar`/`foo_bar` pair.
    4. Silently drop unmatched `--exclude` specs, which mirrors Cargo's
       warning. `INV-diagnostic` must catch it through `"fop"`.

    Record each red transcript in `Artefacts and notes`.

#### Spec grammar

The obligations are `L-roundtrip`, `L-partial`, and `INV-exact`.

- Method: `proptest`, alongside `rstest` grammar tables.
- Domain:
  - names matching `[a-z][a-z0-9_-]{0,15}`;
  - versions as `(u64, u64, u64)` with an optional pre-release `[a-z]{1,5}`;
  - IDs in both shapes that Cargo prints:
    - `path+file:///…/name#version`;
    - `path+file:///…#name@version`.
- Artefact: `crates/cargo-theorem/src/workspace/spec_property_tests.rs`.
- Evidence: passes at the default 256 cases. A regression file is committed
  under `proptest-regressions/` only when it records a real, fixed failure.
- Non-vacuity:
  - Use `prop_assert!` classification counters to show that both names with
    `-` and names with `_` occur, and that both versions with and without a
    pre-release occur.
  - The mutation property derives a near-miss spec: a case flip, a `-`/`_`
    swap, a dropped character, or a perturbed version component. It asserts
    no match, unless the near-miss equals another member's name.
  - A seeded case-insensitive matcher must fail this property.

#### Translation

The obligations are `L-translation` and the failure cases of `τ`.

- Method: named `rstest` cases over JSON documents.
- Artefacts:
  - documents in `crates/cargo-theorem/tests/data/metadata/*.json`, derived
    from probe output;
  - the table in `src/workspace/metadata_tests.rs`.
- Non-vacuity:
  - One valid document for each ID shape.
  - One document that also lists non-member packages, as Cargo does without
    `--no-deps`. `τ` must drop them, which exercises `INV-subset` filtering.
  - One document for each failure case:
    - default members absent, which must not panic;
    - a default that is not a member;
    - a member with no package;
    - a duplicate name;
    - an invalid version;
    - non-UTF-8 bytes.

#### Runner bounds

The obligation is `INV-bounded`.

- Method: runner tests with real `/bin/sh` children, under `#[cfg(unix)]`.
- Rationale: pipe and kill semantics belong to the platform, and only real
  processes exercise them.
- Cases:
  1. A normal exit returns the captured output and `elapsed`.
  2. Stdout one byte over the limit, under policy `Fail`, returns
     `StdoutLimitExceeded` and the child is reaped. On Linux, `/proc/<pid>` is
     absent after return. A zombie would still have that entry, so its
     absence shows the child was reaped.
  3. A 1 MiB stderr flood under a 1 KiB `Truncate` limit returns `Ok` with
     `truncated = true`, `bytes_seen = 1048576`, and `bytes_retained = 1024`.
  4. `sleep 30` with a 200 ms deadline returns `TimedOut` in under 5 s.
  5. A missing program returns `Spawn`.
  6. A non-zero exit is returned as data.
  7. `exec >&- 2>&-; sleep 30` closes its pipes and keeps running. It must
     return `TimedOut` within the bound.
  8. `sleep 30 & echo ok` exits while a grandchild holds the pipes. It must
     return `StreamsHeldOpen` within the deadline plus pipe grace.
- A ninth case, a child killed by `kill -9 $$`, reports `signal = Some(9)`.
- Artefacts: `crates/theoremc-runner/src/system_tests.rs`, and the span test
  in `crates/theoremc-runner/src/span_tests.rs`.
- Non-vacuity: two seeded faults are applied temporarily and reverted.
  - Removing the kill call must make case 4 exceed its bound.
  - Replacing the sliced wait with a plain `wait()` must make case 7 exceed
    its bound.

#### Adapter mapping

The obligations are the adapter's error mapping and its argument-vector
contract (decisions D6 and D8).

- Method: named tests with a fake `CommandRunner` that records the
  `CommandSpec` and returns scripted outcomes.
- Artefact: `crates/cargo-theorem/src/workspace/cargo_adapter_tests.rs`.
- Non-vacuity:
  - The recorded `argv` contains `--no-deps`.
  - The recorded environment contains `RUSTUP_AUTO_INSTALL=0` and
    `CARGO_TERM_COLOR=never`.
  - Each runner error and each exit outcome maps to its own code, and each
    mapping is asserted separately:
    - signal maps to `cargo.terminated`;
    - non-zero exit maps to `workspace.metadata_failed`, with
      `cargo_exit_code` set and a 4 KiB `stderr_excerpt`;
    - `StreamsHeldOpen` maps to `cargo.timed_out`.

#### Telemetry

The obligation is `REQ-telemetry`.

- Method: tests that use a scoped `tracing-subscriber` registry with a
  recording layer, installed through `tracing::subscriber::with_default`.
- Artefacts:
  - `crates/theoremc-runner/src/span_tests.rs`;
  - `crates/cargo-theorem/src/workspace/span_tests.rs`.
- Non-vacuity:
  - The `process.run` and `project.resolve` spans are recorded with non-empty
    `outcome` and `elapsed_ms` fields.
  - No recorded field value contains the fixture path, a spec, or stderr
    text. The test asserts this against the known values.

#### Real-Cargo behaviour

The obligations are `INV-readonly`, `INV-default` against real Cargo,
`REQ-manifest`, `REQ-virtual`, and the ambiguous-name fixtures.

- Method: integration and BDD tests that run real `cargo metadata`, using
  `env!("CARGO")`, on fixture workspaces in temporary directories.
- Artefacts:
  - `crates/cargo-theorem/tests/workspace_resolution.rs`;
  - `crates/cargo-theorem/tests/common/workspace_fixture.rs`;
  - `crates/cargo-theorem/tests/workspace_resolution_bdd.rs`;
  - `crates/cargo-theorem/tests/features/workspace_selection.feature`.
- Non-vacuity:
  - The read-only test uses the fixture with a path dependency. Removing the
    `--no-deps` flag makes Cargo write `Cargo.lock`, so the test fails. This
    has been exercised once as a seeded fault and recorded.
  - The ambiguous-names fixture asserts that `vendor/foo/Cargo.toml` exists
    and declares `foo 0.2.0`, so its precondition is real.
  - The real-proxy test checks two preconditions and skips with a logged
    reason when either fails:
    - `rustup --version` runs through the runner;
    - the version is at least 1.28.1.

    The fixture pins
    `channel = "theoremc-test-nonexistent-toolchain"`. The test asserts
    `workspace.metadata_failed` within 30 s.

### Formal verification decision

Verus is not warranted. The kernel's obligations are set identities over finite
member sets, with exact string equality. A Verus proof would:

- model sets and strings;
- show the implementation computes `D`, `⋃μ(S)`, and `M \ ⋃μ(E)`;
- nearly restate the specification;
- add a toolchain, a CI lane, and a proof directory that the repository does
  not have.

It would add little evidence beyond the independent oracle.

Kani is not warranted either. The kernel manipulates heap strings and ordered
collections, which Kani explores poorly. The bounded exhaustive test gives the
same guarantee for every case within the bound, over an abstract domain, with
an independent oracle.

The genuinely risky properties sit outside the reach of any prover:

- Cargo's default-member semantics (`AXIOM-default-members`);
- the read-only behaviour of `cargo metadata`, which is checked against the
  real tool;
- pipe and kill behaviour, which belongs to the platform and is exercised
  with real processes.

A stronger method would add an unbounded statement for more than four members.
Two facts narrow that residual gap:

- The kernel's per-member logic is uniform. It has no counter, index, or
  threshold that changes behaviour above four members.
- The only size-dependent behaviour, truncation at 50 entries, has its own
  named test.

`L-compose` is the argument for checking spec forms separately from the kernel.
This decision is recorded as D10.

### Suggested BDD features

The feature file is
`crates/cargo-theorem/tests/features/workspace_selection.feature`, bound in
`crates/cargo-theorem/tests/workspace_resolution_bdd.rs` with rstest-bdd 0.6.0.
The steps do the following:

- build fixtures with the shared builder;
- call `cargo_theorem::workspace::resolve_selection` with a
  `CargoMetadataSource` over `theoremc_runner::SystemCommandRunner`;
- assert on member names, or on diagnostic codes and values.

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
    And the suggestions are "foo"

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

No new probe is needed. The planning-time probes answered the feasibility
questions:

- duplicate member names;
- default-member semantics;
- read-only behaviour;
- child processes;
- names that differ only by `-` versus `_`.

Their transcripts are in `Artefacts and notes`. The one remaining uncertainty
is the `OrthoConfig` flatten interaction, which EP-M3 resolves first.

### `EP-M1` decision records, diagnostics, and the pure selection kernel

This milestone is assigned:

- `REQ-virtual`, `REQ-exclude`, `REQ-ambiguity`, and `REQ-single`, at kernel
  level;
- decisions D1 to D6 and D10 to D14;
- `INV-subset` to `INV-exact`, `L-roundtrip`, `L-partial`, `L-compose`, and
  `L-translation`.

Record the decisions before writing any code that depends on them:

1. Write `docs/adrs/adr-0014-workspace-package-selection.md`. Its status is
   Accepted, dated when this plan is approved. Its sections are:
   - Context.
   - Decision. Summarize D1 to D9 and D11 to D14:
     - list the differences from Cargo explicitly;
     - include the per-command selector decision;
     - include the minimum rustup version for the no-download guarantee.
   - Consequences:
     - `serde_json/unbounded_depth`;
     - opaque package IDs;
     - the amendments to developers' guide §3.8 and design §5.
2. Update `docs/cargo-theorem-cli-design.md`:
   - §6.4: the spec grammar, the selection and eligibility rules, the two
     argument structs, the codes and classes, the 50-choice bound, the pinned
     argument keys, and the `cargo metadata` invocation contract.
   - §5: workspace selection stays in `cargo-theorem`, and `theoremc-runner`
     exists as a crate.
   - §12.1: the runner already exists, and 4.4.1 extends it.
   - Reference ADR-0014 from each of these sections.
3. Index ADR-0014 in `docs/contents.md`.

Then build the kernel test-first.

Red:

1. Create `crates/cargo-theorem/src/workspace/mod.rs` and
   `src/diagnostic.rs`, with the public types stubbed by `todo!()` bodies.
2. Add `spec_tests.rs`, a set of `rstest` tables:
   - accepted forms: `name`, `name@1`, `name@1.4`, `name@1.4.3`,
     `name@1.4.3-alpha.1`, `name:1.4`, and both ID shapes;
   - `package.invalid_spec`: the empty string, `@1`, `foo@`, `foo@x`, and
     `foo@1.2.3.4`;
   - `package.glob_unsupported`: `foo*`, `f?o`, and `[f]oo`;
   - an ID spec whose path contains `[`, which is accepted.
3. Add `selection_tests.rs`, with named cases over the virtual fixture
   snapshot:
   - the default selection;
   - an explicit package;
   - repeated and duplicate packages;
   - the whole workspace;
   - the workspace with excludes;
   - an unknown package, plus several unknown packages reported in order;
   - an unknown exclude;
   - an empty selection after exclusion;
   - `require_single` with one member and with two;
   - truncation and suggestions with 120 members;
   - 60 unknown specs, bounded to 50 with `specs_total` set.
4. Add `diagnostic_tests.rs`:
   - every `DiagnosticCode` maps to the class in D6;
   - `ExitClass::as_str()` agrees with its serde form;
   - `ExitClass::code()` matches design §8.4.
5. Run
   `cargo nextest run -p cargo-theorem -E 'test(/workspace::|diagnostic::/)'`.
   Expect assertion failures from the `todo!()` panics.

Green:

1. Implement `workspace/spec.rs`: `PackageSpec`, `PartialVersion`,
   `PackageSpec::parse`, and `PackageSpec::matches`.
2. Implement `workspace/model.rs`:
   - the `PackageName` and `PackageId` newtypes;
   - `WorkspaceMember`;
   - `WorkspaceSnapshot`, whose constructor validates name uniqueness and that
     every default is a member.
3. Implement `workspace/selection.rs`: `SelectionRequest`, `select`,
   `PackageSelection::from_matches`, `PackageSelection::members`, and
   `PackageSelection::require_single`.
4. Implement `workspace/suggest.rs`.
5. Implement `workspace/error.rs`: `SelectionError`, with `diagnostic()`.
6. Implement `diagnostic.rs`: `ExitClass`, `DiagnosticCode`, and
   `Diagnostic` (`Serialize`).
7. Re-express `ApplicationError::exit_code` through `ExitClass`. Its existing
   tests must pass unchanged.
8. Declare `pub mod workspace;` and `pub mod diagnostic;` in `src/lib.rs`, with
   module documentation that states D3.

Rerun the focused tests and expect them to pass.

Then add the exhaustive oracle and the properties. Apply each seeded fault in
turn: observe the failure, record the transcript, and revert. Confirm that
`git diff` shows no fault code.

Add `workspace/metadata.rs`, containing `τ` from `cargo_metadata::Metadata` to
`WorkspaceSnapshot`, with its JSON-document tests. This milestone introduces the
`cargo_metadata` dependency.

Refactor so that every file stays under 400 lines, splitting test modules with
`#[path]` as `cli_tests.rs` does. Then run the gates.

### `EP-M2` `theoremc-runner` and the `cargo metadata` adapter

This milestone is assigned:

- `REQ-bounded`, `REQ-runner`, `REQ-protocol`, and `REQ-telemetry` for
  processes;
- decisions D7, D8, and D9;
- `INV-bounded`.

Red:

1. Create `crates/theoremc-runner/`:
   - `Cargo.toml` with `rust-version = "1.89"`, the workspace lints, and the
     MSRV declaration;
   - stubs in `src/lib.rs` and `src/system.rs`.
2. Add the crate to the workspace members.
3. Add the runner tests (cases 1 to 9) and the runner span test.
4. Add `workspace/cargo_adapter_tests.rs` with a recording fake runner. It
   asserts:
   - the exact `argv` with and without a manifest;
   - `current_dir`;
   - the environment overrides;
   - the limits and their policies;
   - that the program comes from an injected `CARGO`, falling back to `cargo`
     when `CARGO` is absent or empty (`ortho_config::MapEnv`);
   - each outcome mapping in D6.

   It also covers the explicit-manifest checks:
   - a missing path gives `manifest.not_found`;
   - a file not named `Cargo.toml` gives `manifest.invalid_name`;
   - a relative path is resolved against the start directory.
5. Expect failures from the stubs.

Green:

1. Implement `theoremc-runner`:
   - `CommandSpec`, `StreamLimit`, `OverflowPolicy`, `CapturedStream`,
     `ProcessExit`, `CommandOutput`, `CommandError`, and the `CommandRunner`
     trait, in `src/lib.rs`;
   - `SystemCommandRunner`, in `src/system.rs`. It is the only `Command::new`
     site in the workspace's production code.
2. `SystemCommandRunner` runs one command as follows:
   1. Spawn with null stdin and piped stdout and stderr.
   2. Start one reader thread per pipe. A `Fail` reader sets an `AtomicBool`
      on overflow and stops. A `Truncate` reader keeps reading and counting,
      and retains bytes only up to its limit.
   3. Loop on `wait_timeout(min(25 ms, remaining))`. Check the overflow flag
      on every iteration.
   4. On overflow or when the deadline passes, kill the child and wait for
      it.
   5. After the child exits, join the readers within a 2 s pipe grace. If a
      reader is still blocked, detach it and return `StreamsHeldOpen`.
   6. Record `elapsed` and emit the `process.run` span.
3. Implement `workspace/cargo_adapter.rs`:
   - `CargoProgram::from_env(&dyn EnvSource)`;
   - `ManifestAnchor`, which is `Discover { start_dir }` or
     `Explicit { start_dir, manifest_path }`;
   - `CargoMetadataSource<R: CommandRunner>`.
4. The adapter validates an explicit manifest through
   `cap_std::fs_utf8::Dir::open_ambient_dir` on the parent directory. It does
   not walk ancestors, because Cargo's own discovery failure maps to
   `workspace.metadata_failed`.
5. Add `theoremc-runner` and `wait-timeout` as dependencies of
   `cargo-theorem`.

Apply each runner seeded fault once, record it, and revert. Then run the gates,
including `make msrv` for the new crate.

### `EP-M3` resolution service, selector arguments, spans, and real-Cargo fixtures

This milestone is assigned:

- `REQ-selectors`, `REQ-manifest`, `REQ-readonly`, `REQ-noadvertise`, and
  `REQ-telemetry` for resolution;
- end-to-end evidence for `REQ-virtual`, `REQ-exclude`, `REQ-ambiguity`, and
  `REQ-single`;
- `INV-readonly` and `INV-config-free`.

Red, flatten compatibility: add a test-only command struct to
`workspace/selectors_tests.rs`. It derives `clap::Args`, `Deserialize`,
`Serialize`, and `OrthoConfig`, and flattens `WorkspaceSelectorArgs` with
`#[serde(skip)]`. Parse it through a test `Parser`. If the derive rejects the
struct, apply the mitigation from `Risks`, record the discovery, and continue.

Red, selector tables: add `rstest` cases for both argument structs.

- These parse into the expected `ResolutionInput`:
  - no flags;
  - `--package a --package b`;
  - `--workspace --exclude a`;
  - `--manifest-path x/Cargo.toml`.
- These are rejected:
  - `--exclude a` without `--workspace`;
  - `--package a --workspace`;
  - `SinglePackageArgs` with two `--package` values.

  Each of these is a Clap error kind, which renders as `cli.usage`.
- `--package 'f*'` yields `package.glob_unsupported` from `into_input`.

Red, integration:

1. Add the fixture builder `tests/common/workspace_fixture.rs`.
   - It writes manifests and empty `src/lib.rs` files into a
     `tempfile::TempDir` through `cap_std::fs_utf8`.
   - Every fixture declares `[workspace]`, so Cargo never walks up into the
     theoremc workspace.
   - It provides `snapshot_tree() -> Result<Vec<(Utf8PathBuf, Vec<u8>)>>`.
   - Every helper returns `Result`.
2. Add `tests/workspace_resolution.rs` with:
   - the cases from the BDD scenarios, as plain tests;
   - a non-virtual root whose default is the root package only;
   - resolution from inside a member directory;
   - a malformed member manifest, which gives `workspace.metadata_failed` with
     a `stderr_excerpt`;
   - a directory with no `Cargo.toml`, which gives
     `workspace.metadata_failed`;
   - the real-proxy toolchain test;
   - `INV-readonly`.
3. Add the `.feature` file and its bindings.
4. Add an `insta` snapshot test, `tests/workspace_resolution/diagnostics.rs`.
   It covers the JSON serialization and the `Display` text of `package.unknown`,
   `selection.ambiguous`, `selection.empty`, and the truncated 120-member
   diagnostic, with temporary paths redacted by insta filters.
5. Add the resolution span test.
6. Add an agent-context regression test to
   `tests/entry_points/agent_context.rs`. It asserts that the emitted context
   contains none of `manifest-path`, `package`, `workspace`, or `exclude`.
7. Run:

   ```plaintext
   cargo nextest run -p cargo-theorem -E 'binary(=workspace_resolution) | binary(=workspace_resolution_bdd)'
   ```

   Expect a compile failure on `resolve_selection`.

Green:

1. Implement `workspace/selectors.rs`. `WorkspaceSelectorArgs` and
   `SinglePackageArgs` are Clap `Args` structs with long options only. Each has
   `into_input(self, start_dir) -> Result<ResolutionInput, SelectionError>`.
2. In `workspace/mod.rs`, implement:

   ```rust
   resolve_selection(source: &CargoMetadataSource<impl CommandRunner>, input: &ResolutionInput)
       -> Result<ResolvedSelection, ResolutionError>
   ```

   It emits the `project.resolve` span. `ResolvedSelection` keeps the
   `WorkspaceSnapshot`, the `PackageSelection`, `workspace_root`, and the
   stderr truncation facts, so that each command runs `cargo metadata` once.
3. Update `.config/nextest.toml` with a new override:
   - filter:
     `binary(=workspace_resolution) | binary(=workspace_resolution_bdd)`;
   - `test-group = "nested-cargo-builds"`;
   - `slow-timeout = { period = "10s", terminate-after = 3 }`.
4. Add the dev-dependencies.

Refactor, then run the gates.

### `EP-M4` guides, roadmap closure, and delivery

1. Update `docs/cargo-theorem-cli-users-guide.md`. Rewrite the selectors
   section to cover:
   - selectors apply to project commands as those commands ship, starting
     with `list`, `get`, and `check`, and they are given after the subcommand;
   - the spec forms, with `name@version` as the stable qualified form;
   - repeated `--package`;
   - the `--exclude` rules, including the deliberate difference from Cargo
     that an unknown exclude is an error;
   - ordering;
   - the unknown and ambiguous diagnostics, with an example;
   - exit classes 2, 4, and 8;
   - the minimum rustup version for the no-download guarantee.
2. Update `docs/developers-guide.md`:
   - §3.8:
     - rewrite the "Reusable … services remain in library packages" sentence
       per ADR-0014;
     - describe `theoremc-runner` and its reuse policy: it is the only
       process-creation path, and 4.4.1 extends it;
     - the detach policy is acceptable only for one-shot commands;
     - describe the `workspace` module, with one `cargo metadata` call per
       command and no reparsing of `Cargo.toml`;
     - state the resolution-before-configuration rule;
     - state the extraction trigger.
   - §3.3: correct the rstest-bdd version to 0.6.0, and document the
     crate-level `crates/cargo-theorem/tests/{features,common}/` convention,
     the fixture rule that every fixture declares `[workspace]`, and the
     nextest override.
3. Update `docs/repository-layout.md` with a new `crates/theoremc-runner/` row
   and the `crates/cargo-theorem/` description.
4. Index this ExecPlan in `docs/contents.md`.
5. Leave `docs/users-guide.md` unchanged and record why under
   `Outcomes & retrospective`.
6. Mark roadmap task 4.2.1 done (`- [x] 4.2.1.`) only after every acceptance
   criterion passes.
7. Run every gate, complete `Outcomes & retrospective`, and set the status to
   COMPLETE.

## Milestones and plateaus

### `EP-M1`

- Outcome: the decisions are recorded. The kernel, spec parser, translation,
  suggestions, and diagnostic module are public and tested.
- Requirements: `REQ-virtual`, `REQ-exclude`, `REQ-ambiguity`, and
  `REQ-single`, at kernel level.
- Acceptance:
  - the exhaustive, table, and property tests pass;
  - the four kernel faults are recorded;
  - the gates pass.
- Conformance:
  - This command prints nothing:

    ```plaintext
    rg -n 'clap|ortho_config|std::process|cargo_metadata' \
      crates/cargo-theorem/src/workspace/{spec,model,selection,suggest}.rs
    ```

  - ADR-0014 is accepted.
  - Design §§5, 6.4, and 12.1 are updated.
- Recovery: revert the milestone commits. Only `cargo-theorem` and the docs
  have changed.
- Remaining gaps: Cargo is not invoked yet.
- Compatibility: none needed.

### `EP-M2`

- Outcome: the runner crate and the adapter are complete.
- Requirements: `REQ-bounded`, `REQ-runner`, and `REQ-protocol`.
- Acceptance:
  - runner cases 1 to 9 pass;
  - the fake-runner tests and the span tests pass;
  - two runner faults are recorded;
  - the gates and `make msrv` pass.
- Conformance:
  - `rg -n 'Command::new' crates/*/src src` matches only
    `crates/theoremc-runner/src/system.rs`.
  - `theoremc-runner` has no dependency on Clap, OrthoConfig, or
    `cargo_metadata`.
- Recovery: revert the milestone commits. EP-M1 still stands on its own.
- Remaining gaps: there is no real-Cargo evidence yet.
- Compatibility: none needed.

### `EP-M3`

- Outcome: the service and both argument structs are complete, with
  real-Cargo evidence.
- Requirements: every `REQ-*`.
- Acceptance:
  - the integration, BDD, snapshot, span, and agent-context tests pass;
  - the read-only fault is recorded;
  - the gates pass.
- Conformance:
  - The agent context is unchanged.
  - No selector is registered on `Commands`.
  - Cargo is invoked only with `--no-deps --offline`.
- Recovery: revert the milestone commits.
- Remaining gaps: `GAP-root` and `GAP-envelope`, owned by tasks 4.2.3, 4.2.4,
  and 4.1.4.
- Compatibility: none needed.

### `EP-M4`

- Outcome: the guides are accurate, the roadmap item is marked done, and this
  plan is COMPLETE.
- Acceptance: every gate passes.
- Conformance: every upstream artefact is reconciled.
- Recovery: documentation commits revert independently.
- Compatibility: none needed.

## Concrete steps

Run every command from the worktree root. Use `set -o pipefail` whenever output
is piped through `tee`.

```plaintext
EP-M1 red:   cargo nextest run -p cargo-theorem -E 'test(/workspace::|diagnostic::/)' -> todo!() panics
EP-M1 green: same command -> all passed
EP-M2 red:   cargo nextest run -p theoremc-runner -p cargo-theorem -E 'test(/system|span|cargo_adapter/)' -> failures
EP-M2 green: same command -> all passed (real-process cases are Unix-only)
EP-M3 red:   cargo nextest run -p cargo-theorem \
               -E 'binary(=workspace_resolution) | binary(=workspace_resolution_bdd)' -> compile failure
EP-M3 green: same command -> all passed
Snapshots:   cargo insta test -p cargo-theorem --review   (accept only after reading each snapshot)
```

Run the milestone gates in order, preferably through the `scrutineer` agent:

```plaintext
set -o pipefail
make fmt
make check-fmt          2>&1 | tee /tmp/check-fmt-theoremc-4-2-1-workspace-and-package-resolution.out
make lint               2>&1 | tee /tmp/lint-theoremc-4-2-1-workspace-and-package-resolution.out
make test               2>&1 | tee /tmp/test-theoremc-4-2-1-workspace-and-package-resolution.out
make msrv               2>&1 | tee /tmp/msrv-theoremc-4-2-1-workspace-and-package-resolution.out
make check-agent-native 2>&1 | tee /tmp/agent-native-theoremc-4-2-1-workspace-and-package-resolution.out
make markdownlint       2>&1 | tee /tmp/markdownlint-theoremc-4-2-1-workspace-and-package-resolution.out
make nixie              2>&1 | tee /tmp/nixie-theoremc-4-2-1-workspace-and-package-resolution.out
make spelling           2>&1 | tee /tmp/spelling-theoremc-4-2-1-workspace-and-package-resolution.out
```

One local baseline failure is known: the Kani BDD test fails on clean `main`
because of a rustup library-path interaction. CI skips that test because Kani
is not installed there. If it appears, record it as a baseline failure, not a
regression.

`make check-fmt` checks only uncommitted Markdown (`mdtablefix --git`). Before
pushing documentation commits, run `mdtablefix --check` on the changed files
directly.

## Validation and acceptance

1. All of these pass: `make check-fmt`, `make lint`, `make test`, `make msrv`,
   `make check-agent-native`, `make markdownlint`, `make nixie`, and
   `make spelling`.
2. The BDD feature passes all eight scenarios against real Cargo. Its binding
   failed to compile before the implementation existed; that is the recorded
   red evidence.
3. The exhaustive kernel test reports exactly 26,256 cases, with at least 100
   for each outcome class. It fails under each of its four seeded faults, and
   the transcripts are recorded.
4. Runner cases 1 to 9 pass. Each of the two runner faults makes its case fail
   by exceeding the bound.
5. The real-Cargo read-only test passes. With the `--no-deps` flag removed, it
   fails because `Cargo.lock` appears; this is recorded.
6. The insta snapshots show the following:
   - `package.unknown` has class `usage`, lists
     `["app","foo","foo-bar","foo_bar"]`, and suggests `["foo"]`;
   - the 120-member diagnostic lists 50 values, with
     `valid_values_total: 120` and `valid_values_truncated: true`.
7. `cargo theorem context --json` still lists the single command path
   `context`, and none of the selector flags.
8. The span tests record `process.run` and `project.resolve` with no paths and
   no spec text in their fields.

Quality criteria:

- Tests: every test above passes under `make test`.
- Verification: each `INV-*` and `L-*` obligation is discharged as
  `Methods per obligation` describes. The axioms are recorded in ADR-0014.
- Lint and typecheck: `make lint` and `make msrv` are clean.
- Performance: each real-Cargo fixture call completes in under 2 s locally.
- Security:
  - Cargo runs with `--no-deps --offline`, `RUSTUP_AUTO_INSTALL=0`, and
    `CARGO_TERM_COLOR=never`.
  - Processes are created only through argument vectors.
  - Spans carry no paths or raw stderr.

## Idempotence and recovery

- Every step can be re-run.
- Fixtures live in fresh temporary directories, deleted when their `TempDir`
  drops.
- Seeded faults are applied by hand, observed, and reverted. Run
  `git diff --stat` before committing to confirm no fault remains.
- When a milestone fails its gates, either fix forward within the iteration
  tolerance or revert that milestone's commits. Earlier milestones remain valid
  plateaus.
- Commit proptest regression files only when they record a real failure that
  has been fixed.

## Artefacts and notes

The planning-time probes ran with Cargo 1.100.0 (`3acc79c0c 2026-11-05`) and
rustup 1.29.3, in throwaway `/tmp` directories.

Probe 1, duplicate member names:

```plaintext
$ cargo metadata --no-deps --format-version 1
error: two packages named `foo` in this workspace:
- /tmp/ws-probe.jElz/a/foo/Cargo.toml
- /tmp/ws-probe.jElz/b/foo/Cargo.toml
```

Probe 2, a member and an excluded path package that share the name `foo`:

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

Probe 3, default-member semantics with `default-members = ["a"]`:

```plaintext
root:                          default: ['path+file:///tmp/ws-probe3.FqAP/a#0.1.0']
--manifest-path b/Cargo.toml:  workspace_root=/tmp/ws-probe3.FqAP default: [...b#0.1.0]
cwd=b:                         default: [...b#0.1.0]
cwd=a/sub:                     default: [...a#0.1.0]
no default-members (virtual):  [...a#0.1.0, ...b#0.1.0]
non-virtual root:              default: ['path+file:///tmp/ws-probe4.OaBH#root@0.1.0']
--manifest-path b (directory): error: manifest path `b` is a directory but expected a file
no Cargo.toml anywhere:        error: could not find `Cargo.toml` in `/tmp/tmp.l6d7ZyACR3` or any parent directory
--no-deps without Cargo.lock:  exit 0, no Cargo.lock created; --locked also exits 0
```

Probe 5, read-only behaviour and child processes:

```plaintext
find snapshot before/after `cargo metadata --no-deps --offline`: tree unchanged
strace execve: /home/leynos/.cargo/bin/cargo (rustup proxy), .../toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo
```

Probe 6, members whose names differ only by `-` versus `_`:

```plaintext
members = ["foo-bar", "foo_bar"]: metadata ids ['path+file:///tmp/ws-probe6.Zsx0/foo-bar#0.1.0',
'path+file:///tmp/ws-probe6.Zsx0/foo_bar#0.1.0'], no warnings
```

Measurement from the review: `cargo metadata --no-deps` on this repository
emits 33,363 bytes for six members, at 2.2 to 14.4 KB per package, and runs in
0.15 s.

External references, consulted with Firecrawl:

- Cargo book, `cargo metadata`: output format, compatibility, and `--no-deps`;
- Cargo book, Package ID Specifications;
- Cargo book, `cargo check`: package-selection rules, globs, and `--exclude`;
- docs.rs, `cargo_metadata` 0.23.1: `MetadataCommand`, `Metadata`,
  `WorkspaceDefaultMembers`, and its rust-version of 1.86;
- docs.rs, `cargo-util-schemas` 0.14.3: rust-version 1.97;
- the rustup book and the rustup 1.28.1 announcement: `RUSTUP_AUTO_INSTALL`.

### Design review summary

The expert review ran on 2026-11-16 as three panels. All three returned
"proceed with conditions"; this revision applies every condition.

- Structure and alternatives (Pandalump and Wafflecat). Findings:
  - D1 contradicted developers' guide §3.8 and design §5:176;
  - a `cargo-theorem`-owned runner could not be extended by library-owned
    backends;
  - `exit_class` was missing from the interface tolerance;
  - `Diagnostic` was placed in the wrong module;
  - the resolution-before-configuration rule was not stated;
  - the `WorkspaceDefaultMembers` panic was not guarded;
  - the ancestor walk was unnecessary.

  Strongest alternative adopted: create `theoremc-runner` now and keep
  `workspace` in the CLI package.
- Contracts and scaling (Telefono and Buzzy Bee). Findings:
  - the exit class for a missing toolchain contradicted itself;
  - `selection.conflict` was unreachable;
  - "global selectors" was inconsistent with per-command flattening;
  - the eligibility rules were undefined;
  - single-target commands need their own argument struct;
  - a typed code-to-class table was needed;
  - pinned argument keys were needed;
  - globs should be checked only in non-ID specs;
  - the §8.6 field names were missing;
  - suggestions are needed for large workspaces;
  - the exhaustive case count needed fixing;
  - the nextest timeout needed its own override;
  - the snapshot should be kept for single invocation.
- Failure modes and rigour (Doggylump and Dinolump). Findings:
  - the wait after the readers finish was unbounded;
  - wrapper and grandchild runner cases were missing;
  - the minimum rustup version needed stating;
  - the ancestor-walk test was fragile;
  - seeded fault 2 could not be injected as written;
  - a fault for unknown excludes was missing;
  - `τ` lacked a fixture containing non-members;
  - tracing spans were owned by no task;
  - the test conventions had drifted;
  - googletest and pretty_assertions were not assigned to any tests;
  - the deadline should be 30 s.

  Decision D10 (no Verus, no Kani) was confirmed.

## Interfaces and dependencies

### Authorized dependencies

These dependencies are authorized, and every one is already present in
`Cargo.lock`.

`crates/theoremc-runner/Cargo.toml`:

- runtime: `thiserror` (workspace), `wait-timeout = "0.2.1"`, and
  `tracing = "0.1.44"`;
- development: `rstest = "0.28.0"`, `pretty_assertions = "1.4.1"`, and
  `tracing-subscriber = "0.3.23"` with `default-features = false` and the
  `registry` and `std` features.

`crates/cargo-theorem/Cargo.toml`:

- runtime:
  - `theoremc-runner = { path = "../theoremc-runner", version = "0.1.0" }`;
  - `cargo_metadata = "0.23.1"`, `camino = "1.2.2"`, `semver = "1.0.26"`, and
    `tracing = "0.1.44"`;
  - `cap-std = { version = "4.0.2", features = ["fs_utf8"] }`, moved from
    dev-dependencies to dependencies.
- development:
  - `googletest = "0.14.3"` and `pretty_assertions = "1.4.1"`;
  - `insta = { version = "1", features = ["json", "filters"] }`, matching the
    version locked for `theoremc-macros`;
  - `proptest = { version = "1.11.0", default-features = false, features =
    ["std"] }`;
  - `rstest-bdd = "0.6.0"` and `rstest-bdd-macros = "0.6.0"`;
  - `tracing-subscriber`, as for `theoremc-runner`.

The root `Cargo.toml` lists `crates/theoremc-runner` in `[workspace] members`.

### Public surface at the end of EP-M3

The type names below are binding. Field details may be refined, provided each
refinement is recorded in `Decision log`. Every struct and enum is
`#[non_exhaustive]` unless it is a pure newtype.

```rust
// crates/theoremc-runner/src/lib.rs
pub enum OverflowPolicy { Fail, Truncate }
pub struct StreamLimit { pub bytes: usize, pub policy: OverflowPolicy }
pub struct CommandSpec { pub program: std::path::PathBuf, pub argv: Vec<std::ffi::OsString>,
    pub current_dir: std::path::PathBuf, pub env: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    pub stdout: StreamLimit, pub stderr: StreamLimit, pub deadline: std::time::Duration }
pub struct CapturedStream { pub bytes: Vec<u8>, pub bytes_seen: u64, pub bytes_retained: u64,
    pub limit_bytes: u64, pub truncated: bool }
pub enum ProcessExit { Code(i32), Signal(i32) }
pub struct CommandOutput { pub exit: ProcessExit, pub stdout: CapturedStream,
    pub stderr: CapturedStream, pub elapsed: std::time::Duration }
#[derive(Debug, thiserror::Error)]
pub enum CommandError { Spawn { .. }, TimedOut { .. }, StdoutLimitExceeded { .. },
    StreamsHeldOpen { .. }, Io { .. } }
pub trait CommandRunner { fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, CommandError>; }
pub struct SystemCommandRunner;            // src/system.rs, the only production `Command::new` site

// crates/cargo-theorem/src/diagnostic.rs
pub enum ExitClass { Success, Usage, Configuration, ProjectState, BackendUnavailable,
    VerificationMismatch, VerificationIncomplete, ExternalToolFailure, DeliveryFailure, Internal }
impl ExitClass { pub const fn code(self) -> u8; pub const fn as_str(self) -> &'static str; }
pub enum DiagnosticCode { PackageUnknown, PackageGlobUnsupported, PackageInvalidSpec, SelectionEmpty,
    SelectionAmbiguous, ManifestNotFound, ManifestInvalidName, WorkspaceMetadataFailed,
    CargoUnavailable, CargoTerminated, CargoTimedOut, CargoOutputLimit, CargoMetadataInvalid,
    CargoUnsupportedVersion }
impl DiagnosticCode { pub const fn as_str(self) -> &'static str; pub const fn class(self) -> ExitClass; }
pub struct Diagnostic { pub code: DiagnosticCode, pub message_fallback: String,
    pub arguments: serde_json::Map<String, serde_json::Value>, pub valid_values: Vec<String> }
// Diagnostic serializes `code` via as_str() and adds `class` derived from code.class().

// crates/cargo-theorem/src/workspace/
pub struct PackageName(String); pub struct PackageId(String);
pub struct WorkspaceMember { /* name, version: semver::Version, id, manifest_path: Utf8PathBuf */ }
pub struct WorkspaceSnapshot { /* root: Utf8PathBuf, members by name, default_members */ }
pub enum PackageSpec { Name(PackageName), NameVersion(PackageName, PartialVersion), Id(PackageId) }
pub enum SelectionRequest { Default, Packages(Vec<PackageSpec>) /* non-empty */,
    Workspace { exclude: Vec<PackageSpec> } }
pub enum ManifestAnchor { Discover { start_dir: Utf8PathBuf },
    Explicit { start_dir: Utf8PathBuf, manifest_path: Utf8PathBuf } }
pub struct ResolutionInput { pub anchor: ManifestAnchor, pub request: SelectionRequest }
pub struct PackageSelection { /* non-empty, strictly increasing by name */ }
impl PackageSelection { pub fn members(&self) -> &[WorkspaceMember];
    pub fn require_single(&self) -> Result<&WorkspaceMember, SelectionError>; }
pub fn select(snapshot: &WorkspaceSnapshot, request: &SelectionRequest)
    -> Result<PackageSelection, SelectionError>;
pub struct CargoProgram(std::ffi::OsString);
impl CargoProgram { pub fn from_env(env: &dyn ortho_config::EnvSource) -> Self; }
pub struct CargoMetadataSource<R: theoremc_runner::CommandRunner> { /* runner, program */ }
impl<R: theoremc_runner::CommandRunner> CargoMetadataSource<R> {
    pub fn load(&self, anchor: &ManifestAnchor) -> Result<WorkspaceSnapshot, MetadataError>; }
#[derive(clap::Args)]
pub struct WorkspaceSelectorArgs { /* --manifest-path, --package..., --workspace, --exclude... */ }
#[derive(clap::Args)] pub struct SinglePackageArgs { /* --manifest-path, --package (at most one) */ }
// Both: pub fn into_input(self, start_dir: Utf8PathBuf) -> Result<ResolutionInput, SelectionError>;
pub struct ResolvedSelection { /* workspace_root, snapshot, selection, stderr_truncated */ }
pub fn resolve_selection<R: theoremc_runner::CommandRunner>(source: &CargoMetadataSource<R>,
    input: &ResolutionInput) -> Result<ResolvedSelection, ResolutionError>;
// SelectionError, MetadataError, and ResolutionError each expose `fn diagnostic(&self) -> Diagnostic`.
```

### Ownership and reuse policy

This policy is also recorded in developers' guide §3.8.

- `spec.rs`, `model.rs`, `selection.rs`, `suggest.rs`, and `error.rs` are
  pure. They depend only on `semver`, `camino`, `serde`, `thiserror`, and
  `crate::diagnostic`.
- `metadata.rs` owns the anti-corruption translation from `cargo_metadata`. No
  other module touches `cargo_metadata` types.
- `cargo_adapter.rs` is the only code that knows Cargo's command line.
- `theoremc-runner` is the only process-creation path in the workspace's
  production code. Backend providers (roadmap 4.4) extend it rather than adding
  a second one.
- Later commands reuse this work only through three entry points:
  `resolve_selection`, `PackageSelection::require_single`, and the two argument
  structs. Each command calls `resolve_selection` exactly once.
- A parallel implementation, or reparsing `Cargo.toml` for membership,
  violates the design.

## Revision note

- 2026-11-16: Initial draft from the Wyvern reconnaissance, Cargo probes, and
  Firecrawl research.
- 2026-11-16: Expert design review applied; see "Design review summary".
  - The runner moves to a new `theoremc-runner` crate, with names aligned to
    design §§8.6 and 12.1.
  - `diagnostic` becomes an application-level module, with a typed
    code-to-class table and pinned argument keys.
  - A `SinglePackageArgs` struct is added.
  - The eligibility rules, the per-command selector decision, and the
    resolution-before-configuration invariant are now explicit.
  - The deadline is 30 s.
  - Spans are added now; metrics stay deferred.
  - The ancestor walk is removed.
  - The exhaustive domain gains insertion orders and an exact case count.
  - New seeded faults are added for unknown excludes and for unbounded waits.
  - Budgets are raised to cover the new crate.
  - Milestone order is unchanged. The remaining scope grows by the runner crate
    and the span tests.
