# 4.4.1 Define a backend provider contract and injected command runner

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting design review and user approval)

Branch: `4-4-1-backend-provider-contract`

Roadmap item: `4.4.1` in `docs/roadmap.md`.

## Purpose / big picture

Theoremc will eventually install, check, and run proof backends (Kani today,
Verus next) through `cargo theorem backend ...` and `cargo theorem run`. Today
the repository has no code that talks to a proof backend at all: Kani is
installed by the Python tool `rust-prover-tools`, and the only subprocess code
in the workspace is a test-only deadline helper in
`src/arg_lowering_rustc_deadline.rs`.

This task creates the seam every later backend task plugs into. After it lands,
a new library crate, `crates/theoremc-runner`, provides:

1. A *backend provider contract*: three narrow Rust traits through which a
   backend describes itself, discovers an installation, resolves a release,
   plans an installation, checks its health, plans a run as an argument vector,
   parses a run's output, and plans a counterexample replay. A *provider* is
   the backend-specific implementation of those traits (for example, the Kani
   provider that roadmap task 4.4.3 will write).
2. A *command runner port*: one injected `CommandRunner` trait that is the only
   way provider-driven code starts a process. It owns timeouts, cancellation,
   bounded capture of standard output and standard error, and delivery of
   protected (secret) inputs through a channel that is never part of the
   argument vector. Shell command strings cannot be expressed.
3. A production adapter, `ProcessCommandRunner`, that implements the port with
   `std::process`, a dedicated process group, a bounded grace period, and
   bounded "bookend" capture (the start and the end of each stream).
4. Test doubles behind a `test-support` feature: a `ScriptedCommandRunner` that
   records every command and replays scripted outcomes, and a `FakeBackend`
   provider that exercises every capability through it.

A caller in a later task will write code like this:

```rust,ignore
let context = BackendContext::new(&runner, &environment, &cancellation, host);
let installation = provider.lifecycle().discover(&context)?;
let health = provider.lifecycle().check(&context, &installation)?;
let execution = provider.execution().ok_or(BackendError::unsupported(...))?;
let spec = execution.plan_run(&run_request)?;
let output = runner.run(CommandRequest::new(&spec, &channel, &limits, &cancel))?;
let result = execution.parse_run(&run_request, &output)?;
```

Observable success for this task: running
`cargo nextest run -p theoremc-runner --all-features` passes, including the
behavioural scenario "a fake provider exercises every capability without
process-global state", process-adapter tests that kill a timed-out or cancelled
process tree within a bounded time, and property tests showing that argument
vectors arrive byte-for-byte and secrets never appear in rendered or serialized
output. `make verus` verifies the two contractual kernels (protected-input
binding and bounded capture accounting).

No user-visible command changes in this task. `cargo theorem` gains no new
subcommand; `backend` commands arrive in 4.4.2. The users' guides therefore
need no behavioural update, and this plan records that explicitly in `EP-M6`.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- The new crate `crates/theoremc-runner` must not depend on `cargo-theorem`,
  Clap, `ortho_config`, `cargo-orthohelp`, terminal rendering, or any
  process-global configuration (cargo-theorem-cli-design.md §5;
  docs/developers-guide.md "Configuration, CLI parsing, and process error
  handling belong to `crates/cargo-theorem`").
- No production code in `theoremc-runner` may read the process environment
  (`std::env::*` is already denied by `clippy.toml`), change the current
  directory, use `static` mutable state, `thread_local!`, `OnceLock`/`LazyLock`
  holding mutable state, or global registries. Environment values reach
  providers only as an injected `EnvironmentSnapshot` value.
- Providers construct argument vectors only. No API may accept a whole command
  line as one string, and nothing may invoke `sh -c`, `cmd /C`, or another
  shell in production code (cargo-theorem-cli-design.md §§12.1, 16).
- Protected inputs never enter argv, `Debug` output, `Display` output, serde
  serialization, tracing fields, error messages, or test snapshots
  (cargo-theorem-cli-design.md §§11.3, 16).
- Captured stdout and stderr are each bounded by at most 16 MiB retained bytes.
  Configuration may lower but never raise that limit
  (cargo-theorem-cli-design.md §8.6).
- `unsafe_code` is forbidden workspace-wide; signalling a process group must
  use a safe API (`rustix::process::kill_process_group`).
- Every file stays below 400 lines (AGENTS.md); Clippy thresholds in
  `clippy.toml` apply (`too-many-arguments-threshold = 4`,
  `too-many-lines-threshold = 70`, `cognitive-complexity-threshold = 9`).
- Every module starts with `//!`; every public item has `///` docs with an
  example where meaningful; `#[expect(..., reason = ...)]` only, never
  `#[allow]`.
- The process adapter is Unix-only (`#[cfg(unix)]`), matching the Linux-only
  CI (`.github/workflows/*.yml` all run on Ubuntu). If Windows support is
  required, stop and escalate.
- Do not implement `theoremc.toml` backend intent, `theoremc.lock`, the backend
  cache, `backend` CLI commands, Kani or Verus providers, run records, the run
  ledger, or the metric registry. Those belong to 4.4.2 to 4.4.5, 5.x, 6.x, and
  4.5.4 respectively.
- Do not modify completed ExecPlans under `docs/execplans/`.
- Use `mapsplice` semantics only for structural roadmap edits; marking the
  4.4.1 checkbox done is a non-structural edit (docs/developers-guide.md §3.9).
- Verus proofs must contain no `assume`, `admit`, `#[verifier::external_body]`
  on proved items, or trusted bodies introduced to make a proof pass.

## Tolerances (exception triggers)

Comparison base: `origin/main` at `4c059669c0503ff0cb95764283449e27128e7781`.
Counting command, run from the repository root:

```bash
git diff --numstat 4c059669c0503ff0cb95764283449e27128e7781 -- . \
  ':(exclude)docs/execplans/4-4-1-backend-provider-contract.md'
```

Each line counts in exactly one category by whole-file rule:

- Code: `*.rs` files (production, tests, fake tool, Verus proofs) and the
  `Makefile`.
- Configuration: `Cargo.toml` files, `Cargo.lock`, `.github/workflows/*.yml`,
  `.config/nextest.toml`, `tools/verus/*` pin files.
- Documentation: `*.md` and `*.feature` files other than this ExecPlan.

Budgets measure net lines (additions minus deletions), per category,
independently; spare capacity in one category never offsets another.

- Files: stop and escalate if more than 70 non-ExecPlan files change.
- Code: stop and escalate above 4,600 net lines (planned: about 2,000
  production including the fake tool, 2,000 tests, 450 Verus, 30 Makefile).
- Configuration: stop and escalate above 250 net lines.
- Documentation: stop and escalate above 900 net lines (ADR about 220, design
  §12.1 about 180, developers' guide about 160, feature file about 80,
  repository layout and contents about 20).
- Interface: the only new public API is the `theoremc-runner` crate surface in
  `Interfaces and dependencies`. Any change to another crate's public API stops
  work.
- Dependencies: only those listed in `Interfaces and dependencies`. Anything
  else (for example `process-wrap`, `tokio`, `mockall`, `mockable`, `nix`)
  stops work.
- Verus: if either kernel proof still fails after five focused proof attempts,
  or a proof needs an `assume`, stop and record the open verification condition.
- Iterations: if a focused test still fails after five fix attempts, stop.
- Time: if a milestone exceeds one working day of elapsed effort, stop.
- Ambiguity: if the design's §12.1 shape and this plan conflict in a way that
  changes the trait surface, stop and present options.

Budget survey. The new crate is greenfield, so the estimate comes from the
feature list rather than from callers. Comparable plans:
`docs/execplans/2-3-3-struct-literal-synthesis-from-yaml-maps.md` set "14
changed files or roughly 800 net lines" and never raised it;
`docs/execplans/3-3-2-referenced-type-probes-for-generated-struct-literal-synthesis.md`
set "eight changed code files or 500 net Rust lines" and split files rather
than raise budgets;
`docs/execplans/4-1-2-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli.md`
used qualitative triggers only. None raised a numeric budget, so there is no
evidence of repeated underestimation; the larger figures here reflect a new
crate with a production adapter, test doubles, and two proofs. The contingency
is about 15 % per category.

## Risks

- Risk: the pinned `rust-prover-tools` commit
  (`tools/rust-prover-tools/REF`, `592558f7…`) may predate `verus install`
  reading `tools/verus/SHA256SUMS`, or Verus may fail to install on the host.
  Severity: medium. Likelihood: medium. Mitigation: `EP-M0` probes installation
  first; fallback is bumping the REF in a separate commit, which is an
  authorized configuration change; if neither works, stop.
- Risk: a backend leaves a descendant holding stdout or stderr open after the
  group leader exits, so reader threads never see end-of-file and `run` hangs.
  Severity: high. Likelihood: medium. Mitigation: after the leader exits the
  runner waits for end-of-file only until a drain deadline, then terminates the
  owned group and records `descendants_terminated`; `EP-M5` tests this with a
  fake tool that spawns a lingering grandchild.
- Risk: a descendant calls `setsid()` and escapes the process group, so group
  signals cannot reach it. Severity: medium. Likelihood: low for Kani and
  Verus. Mitigation: recorded as `AXIOM-group-containment`; the drain deadline
  still bounds `run`; full containment (cgroups) is out of scope.
- Risk: the Verus mirror drifts from the production kernel, so the proof
  stops describing shipped code. Severity: high. Likelihood: medium.
  Mitigation: kernels are tiny, dependency-free modules; a property test checks
  production against an independent specification oracle; a contract test pins
  the mirror's file and lemma names; the developers' guide makes updating the
  mirror part of any kernel change.
- Risk: timing-based process tests are flaky on a loaded shared host.
  Severity: medium. Likelihood: medium. Mitigation: assert generous upper
  bounds only (never exact durations), drive cancellation through a
  deterministic signal, and run process tests in a dedicated nextest group with
  limited concurrency.
- Risk: the trait surface guessed now does not fit Kani or Verus in 4.4.3 and
  4.4.4 (ADR-0006 §10 warns against freezing guessed traits). Severity: medium.
  Likelihood: medium. Mitigation: the fake provider is modelled on the
  documented Kani and Verus behaviours from `rust-prover-tools` (see
  `Context and orientation`), and the crate is pre-1.0 and internal, so 4.4.3
  may change the traits together with every caller without compatibility shims.
- Risk: protected-input delivery through standard input deadlocks when the
  child never reads it and the secret exceeds the pipe buffer. Severity:
  medium. Likelihood: low. Mitigation: a dedicated writer thread, closed on
  termination, whose outcome is recorded as `ProtectedDelivery::NotConsumed`
  rather than blocking `run`.

## Progress

- [x] (2026-11-16) Branch renamed, Lody session titled, reconnaissance by four
  Wyvern scouts and one prior-art researcher completed.
- [x] (2026-11-16) Initial draft written.
- [ ] Design review by expert panel applied.
- [ ] Plan approved by the user.
- [ ] `EP-M0` Verus and process-group feasibility probe.
- [ ] `EP-M1` crate skeleton, value types, limits, and redaction.
- [ ] `EP-M2` protected-input binding and capture kernels with Verus proofs.
- [ ] `EP-M3` provider traits, registry, and capability honesty.
- [ ] `EP-M4` scripted runner, fake provider, and the behavioural success
  scenario.
- [ ] `EP-M5` `ProcessCommandRunner` production adapter.
- [ ] `EP-M6` documentation, ADR, and roadmap closure.

## Surprises & discoveries

- Observation: `AGENTS.md` mandates `mockable::Env`, but no crate in the
  workspace depends on `mockable`; the CLI uses `ortho_config`'s `EnvSource` and
  `MapEnv`, which `AGENTS.md` also accepts. Evidence: `Cargo.lock` has no
  `mockable`; `crates/cargo-theorem/src/cli.rs` `ConfigurationSources`. Impact:
  the runner takes an injected immutable `EnvironmentSnapshot` value instead
  (decision `D4`).
- Observation: `rust-prover-tools` has no Kani concrete-playback code to port.
  Evidence: repository search at HEAD found only a `playback/` bundle
  directory. Impact: replay planning is shaped from the Kani documentation and
  theoremc-design.md §8.5, not from existing behaviour.
- Observation: `docs/developers-guide.md` still cites `rstest-bdd` v0.5.0
  while the workspace pins 0.6.0. Impact: corrected in `EP-M6`.

## Decision log

- D1. Decision: create a new workspace crate `crates/theoremc-runner` rather
  than a module in `cargo-theorem` or `theoremc-core`. Rationale:
  cargo-theorem-cli-design.md §5 assigns "backend provider interface, command
  execution abstraction" to `theoremc-runner` and forbids the CLI being the
  only home of reusable execution logic; `theoremc-core` is consumed by the
  proc-macro and must stay free of process code. The rejected alternative, a
  feature-grouped module in `cargo-theorem`, would force a later extraction when
  `run` (6.2) and report code need the same types. Date/Author: 2026-11-16,
  planning agent.
- D2. Decision: deliver the production `ProcessCommandRunner` in this task, not
  only the port and fakes. Rationale: the roadmap says the injected runner owns
  "timeouts, cancellation, stream capture, and test doubles". A port whose
  timeout, cancellation, capture, and protected-input semantics are proven only
  against a fake would encode guesses; the real adapter falsifies them now,
  before 4.4.3 and 4.4.4 depend on them. Rejected: defer the adapter to 4.4.3,
  which would mix provider porting with process-supervision debugging.
  Date/Author: 2026-11-16, planning agent.
- D3. Decision: provider methods that must observe the outside world
  (`discover`, `check`) receive a `BackendContext` that carries the injected
  runner; planning and parsing methods (`resolve`, `plan_install`, `plan_run`,
  `parse_run`, `plan_replay`) are pure functions of their inputs. Rationale: it
  follows §12.1's signatures, keeps the run path orchestrated by the caller
  (which later owns ledgers and artefacts), and keeps everything but probing
  deterministic. Rejected: a fully sans-I/O probe state machine for discovery,
  which is heavier than two backends justify. Date/Author: 2026-11-16, planning
  agent.
- D4. Decision: inject the environment as an immutable `EnvironmentSnapshot`
  value, and make every child environment explicit: the runner clears the
  inherited environment and applies exactly `CommandSpec::environment`. Working
  directories and program paths must be absolute. Rationale: this is the
  strongest reading of "without process-global state": neither providers nor
  children observe ambient environment, current directory, or `PATH` lookup.
  `rust-prover-tools` already curates Kani's child environment, so providers
  need this control anyway. Rejected: an overlay mode inheriting the parent
  environment (Cuprum's default), which makes child behaviour depend on the
  caller's shell. Date/Author: 2026-11-16, planning agent.
- D5. Decision: a terminated, timed-out, cancelled, or non-zero-exit process is
  a successful `run` returning `CommandOutput` with a `Termination` value;
  `CommandError` is reserved for failures that prevent a faithful observation
  (invalid spec, binding rejection, spawn failure, pipe or signal I/O failure).
  Rationale: Verus `--version` exits non-zero as useful data, and a timed-out
  proof must still yield partial output for `parse_run` to classify as
  `verification_incomplete` (design §8.4). Native exit codes stay in data,
  never in the CLI taxonomy. Date/Author: 2026-11-16, planning agent.
- D6. Decision: bookend capture. Each stream retains the first `head_bytes` and
  the last `tail_bytes` it produced, with `head_bytes + tail_bytes <= 16 MiB`
  (default 4 MiB head, 12 MiB tail), and records `bytes_seen`, `bytes_retained`,
  `limit_bytes`, and `truncated`. Rationale: Kani prints its verdict at the
  end of its output and compile errors at the start; head-only capture would
  lose verdicts, tail-only would lose first errors. Streaming full logs to run
  artefacts is a later sink concern (6.x). Rejected: head-only (simplest proof,
  worst parsing outcome). Date/Author: 2026-11-16, planning agent.
- D7. Decision: protected inputs support one delivery mode in this task,
  standard input, with at most one protected input per command; any other
  requested mode is rejected as `Unsupported` before process creation.
  Rationale: §11.3 allows "an in-memory value, an OS-protected handle, or a
  stdin-like channel"; neither Kani nor Verus needs a secret today, so one
  honest mode plus a closed rejection path meets §18's binding tests without
  speculative file-descriptor plumbing. Date/Author: 2026-11-16, planning agent.
- D8. Decision: use Verus for the protected-input binding kernel and the
  bookend capture-accounting kernel only; use property tests, parameterized
  tests, and process-level integration tests for everything else. Rationale: see
  `Formal verification decision`. Date/Author: 2026-11-16, planning agent.
- D9. Decision: hand-written test doubles (`ScriptedCommandRunner`,
  `FakeBackend`) exported behind the crate's `test-support` feature rather than
  `mockall` mocks. Rationale: 4.4.2 to 4.5.2 reuse these doubles from other
  crates; they must script failures (spawn error, timeout, truncation,
  cancellation) and record calls, which a hand-written double expresses as data
  and `mockall` would regenerate per test. No new dependency is needed.
  Date/Author: 2026-11-16, planning agent.
- D10. Decision: no tracing spans or metrics in this task.
  Rationale: the metric registry and its tests are roadmap 4.5.4, and the
  application subscriber does not exist yet; adding unverified telemetry would
  violate AGENTS.md's "verify a relevant event produces a non-zero signal". The
  process adapter keeps a single obvious place (`supervise`) to add the
  child-process span in 4.5.4. Date/Author: 2026-11-16, planning agent.
- D11. Decision: ADR-0006 §10 ("avoid a premature backend trait hierarchy")
  does not block these traits. Rationale: ADR-0006 §10 concerns harness
  *renderers*; the provider lifecycle traits are prescribed by the accepted
  cargo-theorem-cli-design.md §12.1, and two real backends plus the fake
  already demonstrate shared lifecycle behaviour. The new ADR-0014 records the
  distinction. Date/Author: 2026-11-16, planning agent.

## Outcomes & retrospective

Not started. Complete at each milestone and at the end.

## Context and orientation

Inspected base revision: `4c059669c0503ff0cb95764283449e27128e7781`
(`origin/main`). Search with `rg -n '<symbol>'` to relocate any symbol.

Workspace. The root `Cargo.toml` declares members `crates/test-helpers`,
`crates/theoremc-build-support`, `crates/cargo-theorem`,
`crates/theoremc-core`, and `crates/theoremc-macros`, edition 2024,
`rust-version = "1.89"`, and strict `[workspace.lints]` (denies `unwrap_used`,
`expect_used`, `indexing_slicing`, `missing_docs`,
`missing_docs_in_private_items`, `cognitive_complexity`, `print_stdout`,
`print_stderr`, `allow_attributes`, `must_use_candidate`,
`missing_const_for_fn`; `unsafe_code = "forbid"`). `clippy.toml` disallows
`std::env::var`, `var_os`, `vars`, `vars_os`, `set_var`, `remove_var`, and
`set_current_dir`; it does not disallow `std::process::Command`. The toolchain
is `nightly-2026-11-16` (`rust-toolchain.toml`) with `-Zthreads=8` and `mold`
from `.cargo/config.toml`.

`crates/cargo-theorem` is the only application package. Its only command is
`context`; `crates/cargo-theorem/src/config/mod.rs`
`CONFIGURABLE_COMMAND_NAMES` reserves `backend` as a configuration namespace
only. Its environment seam is `ConfigurationSources` in
`crates/cargo-theorem/src/cli.rs`, tested with `ortho_config::MapEnv`. Nothing
in this task edits `cargo-theorem`.

Existing process code. `src/arg_lowering_rustc_deadline.rs` provides a test-only
`run_within_deadline` that polls `Child::try_wait` every 25 ms and kills the
child on overrun. It has no cancellation, process group, or bounded capture. It
is a useful reference, not a reuse candidate, because it is `#[cfg(test)]` in
the root crate. All other `std::process::Command` uses are in tests.

Dependencies already in `Cargo.lock`: `rustix` 1.1.5, `libc` 0.2.190, `camino`
1.2.6, `cap-std` 4.0.3, `insta` 1.49.0, `googletest` 0.14.4,
`pretty_assertions` 1.4.1, `proptest` 1.12.0, `rstest` 0.28.0, `rstest-bdd`
0.6.0, `serde` 1.0.229, `serde_json` 1.0.154, `tempfile` 3.27.0, `thiserror`
2.0.21. Absent: `mockable`, `mockall`, `nix`, `process-wrap`, `tracing` (only
transitive), `metrics`.

Verification infrastructure. Kani is installed through `make install-kani`
(`$(PROVER_TOOLS) kani install`) and exercised by `make kani`, run only from
`.github/workflows/proofs.yml`. No Verus proofs, pins, or Make targets exist;
`tools/kani/VERSION` is `0.67.0`, `tools/rust-prover-tools/REF` pins
`592558f7183b554fc749696246984c5d7300d029`. The newest stable Verus release on
2026-11-16 is `0.2026.11.02.8bf68ee`, whose `x86-linux` asset digest is
`sha256:f3a0119292df8855e4435da8130bee1873b0e6a60c306accf81a08ea687bba3c`
(GitHub releases API). Proptest is used in about thirteen test modules; there
are no regression-seed directories.

Prior art from `leynos/rust-prover-tools` (Python), which the provider contract
must be able to host in 4.4.3 and 4.4.4:

- Kani: semantic-version pin in `tools/kani/VERSION`; install via
  `cargo install --locked kani-verifier --version V` then `cargo kani setup`;
  version check runs `<kani-command> --version` and takes the first `N.N.N`
  match from stdout plus stderr; the legacy `--kani-command` is split with
  `shlex` (theoremc replaces it with a typed program plus argv); proof
  execution curates `LD_LIBRARY_PATH`, `RUSTFLAGS`, `PATH`, and
  `RUSTUP_TOOLCHAIN` for the child.
- Verus: version and `SHA256SUMS` pins; HTTPS download with a 2 GiB ceiling,
  15 s socket timeout and 300 s overall deadline; checksum verified before
  extraction; directory normalization; `verus --version` may exit non-zero and
  still report `Toolchain: <name>`, which is data, not failure; missing
  toolchains are installed with `rustup toolchain install` only when allowed;
  one proof file per invocation.
- Cuprum (its command library): argv-only typed programs, explicit execution
  context (environment mode, working directory, timeout, cancel grace), own
  process group when a timeout applies, SIGTERM then grace then SIGKILL,
  non-zero exit returned as data, tail capture of 64 KiB for unbounded output.

Terms used in this plan:

- *Port*: a trait owned by the library that the outside world implements
  (hexagonal architecture). `CommandRunner` is a driven port.
- *Adapter*: a concrete implementation of a port, such as
  `ProcessCommandRunner`.
- *Kernel*: a tiny, pure, dependency-free function whose contract is proved in
  Verus and mirrored in production.
- *Mirror*: the Verus copy of a kernel's algorithm and data model, kept in
  `verus/`, because Verus compiles its own files and cannot import workspace
  crates.
- *Bookend capture*: retaining the first and last bytes of a stream when it
  exceeds its limit.
- *Process group*: a POSIX set of processes that can be signalled together;
  the runner makes each child the leader of a new group.

Signposts. Read before implementing: cargo-theorem-cli-design.md §§5, 8.4, 8.6,
11.2 ("Concurrency and task ownership"), 11.3, 12, 16, 17, 18;
`docs/developers-guide.md` §3.8 "Backend integration" and the test conventions;
`docs/reliable-testing-in-rust-via-dependency-injection.md`;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rstest-bdd-users-guide.md`;
`docs/rust-doctest-dry-guide.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md`; ADR-0006 §10,
ADR-0007 invariants 8 and 10, ADR-0011. `docs/theoremc-design.md` §8 (Kani
backend) and `docs/theorem-file-specification.md` provide backend semantics
context but impose no direct requirement here;
`docs/ortho-config-users-guide.md` is relevant only to the 4.4.2 composition
root. Skills: `rust-router`, `hexagonal-architecture`, `rust-types-and-apis`,
`rust-errors`, `rust-async-and-concurrency` (threads, cancellation),
`rust-unit-testing`, `proptest`, `verus`, `rust-verification`, `nextest`,
`arch-decision-records`, `execplans`.

## Conformance basis

Upstream artefacts, all at revision `4c059669`:

- `docs/roadmap.md` task 4.4.1 and step 4.4 (requires 4.1.2, complete).
- `docs/cargo-theorem-cli-design.md` (Status: accepted) §§5, 8.4, 8.6, 11.2,
  11.3, 12.1, 16, 17, 18.
- `docs/developers-guide.md` §3.8 "Backend integration".
- ADR-0006 (accepted) §10; ADR-0007 (accepted) invariants 8 and 10; ADR-0009
  (accepted) application-only OrthoConfig; ADR-0010 (accepted) Rust 1.89 MSRV.
- `AGENTS.md` Rust, testing, environment-injection, and observability rules.

There is no separate Terms of Reference document; the roadmap's step 4.4 idea
statement plays that role.

Requirements:

- `REQ-capabilities`: providers expose discovery, release resolution, install
  planning, health checks, argument-vector run planning, run-result parsing,
  and replay planning (roadmap 4.4.1; design §12.1).
- `REQ-separation`: lifecycle concerns stay separate from execution and replay
  concerns (developers-guide §3.8; design §12.1).
- `REQ-argv-only`: commands are argument vectors; shell strings are
  inexpressible (roadmap 4.4.1; design §§12.1, 16).
- `REQ-one-runner`: one injected runner owns process creation, timeouts,
  cancellation, bounded capture, and test doubles (roadmap 4.4.1; design §12.1).
- `REQ-protected`: protected inputs bind exactly once through the channel;
  missing, extra, duplicate, and unsupported references are rejected before
  process creation; secrets never render or persist (design §§11.3, 12.1, 16,
  18).
- `REQ-bounded`: each stream retains at most 16 MiB, configurable only
  downwards, and records `bytes_seen`, `bytes_retained`, `limit_bytes`,
  `truncated` (design §8.6).
- `REQ-native-status`: native exit codes and signals are preserved as data and
  never become the application's exit taxonomy (design §8.4).
- `REQ-owned-termination`: cancellation and timeouts stop the owned process
  group after a bounded grace period (design §11.2).
- `REQ-no-global`: a fake provider exercises every capability without
  process-global state (roadmap 4.4.1 success criterion).
- `REQ-library-boundary`: the runner crate has no application dependencies
  (design §5; ADR-0009).
- `REQ-capability-declared`: descriptors declare capabilities so callers do not
  infer them from a backend name (design §12.1; ADR-0007 invariant 8).

Gaps against the baseline: `GAP-no-crate` (no runner crate), `GAP-no-port` (no
command-runner abstraction), `GAP-no-verus` (no Verus pins, targets, or proofs),
`GAP-illustrative-api` (design §12.1 is explicitly illustrative and must be
replaced by the delivered contract).

Trace:

```plaintext
roadmap 4.4.1 -> REQ-capabilities, REQ-separation -> EP-M3 -> tests/provider_registry.rs, src/backend/*_tests.rs
roadmap 4.4.1 -> REQ-argv-only -> EP-M1, EP-M5 -> proptest argv_round_trip (tests/process_runner.rs)
roadmap 4.4.1 -> REQ-one-runner -> EP-M4, EP-M5 -> tests/process_runner.rs, src/test_support/*
design 11.3,16,18 -> REQ-protected -> EP-M2 (kernel, Verus), EP-M5 (adapter) -> verus/protected_binding.rs, tests/process_runner.rs
design 8.6 -> REQ-bounded -> EP-M1 (limits), EP-M2 (kernel, Verus), EP-M5 -> verus/capture_accounting.rs, proptest capture_matches_oracle
design 8.4 -> REQ-native-status -> EP-M1, EP-M5 -> termination tests
design 11.2 -> REQ-owned-termination -> EP-M5 -> timeout and cancellation process-tree tests
roadmap 4.4.1 success -> REQ-no-global -> EP-M4 -> tests/backend_provider_contract_bdd.rs
design 5, ADR-0009 -> REQ-library-boundary -> EP-M1 -> dependency fitness test
ADR-0007 inv. 8 -> REQ-capability-declared -> EP-M3 -> registry capability-honesty tests
GAP-illustrative-api -> EP-M6 -> design §12.1 rewrite and ADR-0014
GAP-no-verus -> EP-M0, EP-M2 -> make verus
```

## Verification plan

### Abstract model

The contract has three independently reasoned parts.

Protected-input binding. A command declares a finite sequence
`D = [d_0 … d_{n-1}]` of protected references, each with a delivery mode, and
the caller supplies a channel whose entries carry a finite sequence of keys
`S = [s_0 … s_{m-1}]`. Keys are opaque and comparable for equality only.
`bind(D, S)` returns either a permutation `p` of `0..n` with
`S[p(i)] = key(D[i])` for every `i`, or a fault naming one witness. Binding is
total: there is no precondition.

Capture accounting. A stream is a finite sequence of chunks `c_0 … c_{k-1}`,
each a byte sequence; `C` is their concatenation and `|C|` its length. Limits
are `h` (head bytes) and `t` (tail bytes), natural numbers with
`h + t <= L_max = 16 MiB`. After consuming all chunks the account holds `seen`,
`head`, and `tail`. The specification is:

```plaintext
seen      = min(|C|, u64::MAX)
truncated = |C| > h + t
retained  = C                                  if not truncated
          = C[0 .. h] ++ C[|C| - t .. |C|]       if truncated
bytes_retained = |retained|,  limit_bytes = h + t
```

Process supervision. The adapter is a state machine
`Spawned -> Running -> (Exited | Terminating) -> Draining -> Reaped`. Events
are child exit, deadline expiry, cancellation, grace expiry, and pipe
end-of-file. Its observable contract is the `Termination` value and the
wall-clock bound on `run`; it is tested, not proved (see the formal
verification decision).

### Invariants and lemmas

- `INV-bind-exact` (REQ-protected): `bind` returns `Ok(p)` if and only if keys
  in `D` are pairwise distinct, keys in `S` are pairwise distinct, and
  `set(D) = set(S)`; when it returns `Ok(p)`, `p` is a bijection with
  `S[p(i)] = key(D[i])`.
- `INV-bind-witness` (REQ-protected): each fault carries a genuine witness:
  `DuplicateDeclared(i, j)` has `i < j` and equal keys in `D`;
  `DuplicateSupplied(i, j)` likewise in `S`; `Missing(i)` has `key(D[i])`
  absent from `S`; `Extra(j)` has `S[j]` absent from `D`.
- `INV-bind-before-spawn` (REQ-protected): when binding or delivery-mode
  validation fails, no process is created and nothing is written.
- `INV-capture-accounting` (REQ-bounded): the capture specification above
  holds after every prefix of chunks, not only at the end.
- `INV-limits-lower-only` (REQ-bounded): no `ExecutionLimits` value can hold a
  per-stream retention above 16 MiB; construction above the cap fails.
- `INV-argv-preserved` (REQ-argv-only): the child observes exactly
  `[program, argv…]`, byte for byte, for any argument content including spaces,
  quotes, `$`, `;`, newlines, and non-UTF-8 bytes on Unix.
- `INV-env-exact` (REQ-no-global): the child observes exactly
  `CommandSpec::environment` and the requested working directory.
- `INV-secret-silent` (REQ-protected): for every protected value `v`, the
  bytes of `v` appear in none of `format!("{:?}")`, `Display`, serde output, or
  error text of any contract type, and never in the child's argv or environment.
- `INV-native-status` (REQ-native-status): an exit code or terminating signal
  is reported verbatim in `Termination`.
- `INV-bounded-return` (REQ-owned-termination): `run` returns within
  `timeout + grace + drain + ε` for any child behaviour within
  `AXIOM-group-containment`, and returns promptly after cancellation.
- `INV-group-reaped` (REQ-owned-termination): after `run` returns from a
  timeout or cancellation, no process remains in the owned process group.
- `INV-capability-honesty` (REQ-capability-declared): a registered provider's
  descriptor declares `Execution` if and only if `execution()` is `Some`,
  `Replay` if and only if `replay()` is `Some`, and `Replay` implies
  `Execution`; unsupported optional lifecycle operations return
  `BackendError::Unsupported`.
- `INV-registry-order` (REQ-capabilities): registry identifiers are unique and
  listing order is deterministic (sorted by `BackendId`).
- `INV-isolation` (REQ-no-global): two independent contexts used concurrently
  produce the same results as when used sequentially.

Lemmas (Verus, provisional decomposition):

- `L-bind-sound`: `bind(D, S) = Ok(p)` implies `INV-bind-exact`'s bijection
  property.
- `L-bind-complete`: the distinctness and set-equality premise implies
  `bind(D, S)` is `Ok`.
- `L-bind-faults`: every `Err` satisfies `INV-bind-witness`.
- `L-capture-step`: if the account satisfies the specification for chunks
  `c_0 … c_{k-1}`, then after `push(c_k)` it satisfies it for `c_0 … c_k`.
- `L-capture-init`: the empty account satisfies the specification for the
  empty stream. Together with `L-capture-step` this yields
  `INV-capture-accounting` by induction.

### Axioms (trusted, not verified here)

- `AXIOM-execve-argv`: on Unix, `std::process::Command` passes the program and
  arguments to `execve` without shell interpretation (Rust standard library
  documentation). Exercised by `INV-argv-preserved` tests against a real child.
- `AXIOM-env-clear`: `Command::env_clear` followed by `envs` yields exactly the
  listed variables in the child. Exercised by `INV-env-exact` tests.
- `AXIOM-process-group`: `std::os::unix::process::CommandExt::process_group(0)`
  makes the child a new group leader before `exec`, and
  `rustix::process::kill_process_group` delivers a signal to every member that
  has not left the group. Exercised by grandchild tests.
- `AXIOM-group-containment`: Kani, Verus, Cargo, and Rustup descendants do not
  call `setsid`/`setpgid` to leave the group. Documented, not tested.
- `AXIOM-pipe-eof`: a pipe reader sees end-of-file only when every write end
  is closed (POSIX). Basis for the drain deadline.
- `AXIOM-monotonic`: `std::time::Instant` is monotonic.
- `AXIOM-verus`: Verus and Z3 are sound for the proved fragment.

### Methods per obligation

- Obligation: `INV-bind-exact`, `INV-bind-witness` (`L-bind-*`).
  Method: Verus proof of the mirrored kernel, plus a property test of the
  production kernel against an independent oracle, plus named `rstest` cases.
  Rationale: a security contract over arbitrary-length sequences; Verus gives
  the unbounded guarantee, the property test bridges mirror and production.
  Domain: all finite sequences of keys (Verus); generated sequences of up to
  eight keys drawn from a four-symbol alphabet so duplicates, gaps, and extras
  are common (proptest, 512 cases). Artefact: `verus/protected_binding.rs`;
  `crates/theoremc-runner/src/command/binding_kernel.rs` and
  `binding_kernel_tests.rs`. Evidence: `make verus` reports zero errors for
  every lemma; `cargo nextest run -p theoremc-runner binding_kernel` passes.
  Non-vacuity: the oracle is written declaratively (sort-and-compare of
  multisets, not the kernel's loop); the property test records the class counts
  (`ok`, each fault kind) through `prop_assert!` plus a coverage assertion that
  every class occurs at least once in a fixed-seed run; a seeded fault
  (swapping the duplicate checks for a length check) must fail both Verus
  (`L-bind-faults` open goal) and the property test, and is reverted. Verus
  antecedents are inhabited by explicit witness lemmas (`lemma_bind_ok_witness`
  with a two-element permutation, and one witness per fault).
- Obligation: `INV-bind-before-spawn`.
  Method: process-level integration test. Domain: missing, extra, duplicate,
  and unsupported cases. Artefact:
  `crates/theoremc-runner/tests/process_runner.rs`. Evidence: the fake tool
  writes a marker file on start; after each rejected run the marker is absent
  and `CommandError::ProtectedInput` names the fault. Non-vacuity: a positive
  control with a valid binding creates the marker.
- Obligation: `INV-capture-accounting` (`L-capture-*`).
  Method: Verus proof of the mirrored account, property test of the production
  account against an oracle, and process-level flooding tests. Domain: all
  chunk sequences and limits (Verus); generated chunk sequences with total
  sizes around `h + t` (below, equal, one above, far above), with small limits
  (proptest, 512 cases). Artefact: `verus/capture_accounting.rs`;
  `crates/theoremc-runner/src/command/capture_kernel.rs` and tests. Evidence:
  as above. Non-vacuity: the oracle concatenates and slices (no ring buffer);
  generators are classified so the equality boundary `|C| = h + t` and the
  one-over boundary each appear; a seeded off-by-one in tail eviction must be
  rejected by both proof and property test.
- Obligation: `INV-limits-lower-only`.
  Method: `rstest` table over constructor inputs at, below, and above the cap.
  Artefact: `crates/theoremc-runner/src/command/limits_tests.rs`.
- Obligation: `INV-argv-preserved`, `INV-env-exact`.
  Method: property test spawning the fake tool, which writes its argv,
  environment, and working directory as NUL-separated records to stdout.
  Domain: up to eight arguments of up to 32 bytes from an alphabet weighted
  towards shell metacharacters, whitespace, quotes, and invalid UTF-8; 64 cases
  (each case spawns a process). Artefact:
  `crates/theoremc-runner/tests/process_runner.rs`. Non-vacuity: a named case
  with `"a b"`, `"$(touch x)"`, and `"; exit 1"` must arrive intact and must
  not create `x`.
- Obligation: `INV-secret-silent`.
  Method: property test over random 16-byte secrets embedded in specs,
  channels, outputs, and errors, checking `Debug`, `Display`, and `serde_json`
  renderings; `insta` snapshot of `CommandProvenance` JSON for a representative
  spec; process test that the fake tool's reported argv and environment do not
  contain the secret but its stdin does. Non-vacuity: a negative control derives
  `Debug` on a copy of `ProtectedValue` in the test module and shows the check
  fails.
- Obligation: `INV-native-status`.
  Method: `rstest` table running the fake tool with exit codes 0, 1, 2, 101,
  and self-signalling with SIGTERM and SIGKILL.
- Obligation: `INV-bounded-return`, `INV-group-reaped`.
  Method: process-level integration tests with generous bounds: a sleeper that
  ignores SIGTERM (forces the SIGKILL path), a sleeper that exits on SIGTERM, a
  leader that exits while a grandchild holds stdout open, and cancellation
  raised from another thread after the child reports readiness on stdout.
  Evidence: `run` returns within the documented bound plus 5 s slack; the
  grandchild's pid (written to a file) no longer exists. Non-vacuity: a
  positive control shows the grandchild is alive before cancellation.
- Obligation: supervision decisions (the pure `next_action` function).
  Method: `rstest` table over (state, elapsed, cancelled, exited) and a
  proptest that deadlines are monotone (a later clock never un-expires).
- Obligation: `INV-capability-honesty`, `INV-registry-order`.
  Method: `rstest` cases registering honest and dishonest fake providers;
  proptest that listing order is independent of registration order.
- Obligation: `INV-isolation`, `REQ-no-global`.
  Method: the behavioural scenario runs two fake providers with different
  injected environments on two threads and compares with sequential results; a
  fitness test scans `crates/theoremc-runner/src` (excluding `src/bin/` and
  `src/process/`) for `std::env`, `static mut`, `thread_local!`, `OnceLock`,
  `LazyLock`, and `set_current_dir`.

### Formal verification decision

Verus is warranted for two kernels:

1. Protected-input binding is contractual security logic: a wrong answer leaks
   a secret into the wrong channel or runs a command without its credential.
   Its property quantifies over arbitrary-length sequences, so bounded checking
   would leave the general case unproved.
2. Bookend capture accounting is the contract behind four persisted fields in
   design §8.6 and behind every parser's view of truncated output. It is an
   inductive property over unbounded chunk sequences.

Verus is not warranted for the supervision state machine (its correctness
depends on the operating system, so a proof over a model would not discharge
the real risk; integration tests do), for value-type validation (finite
partitions, `rstest`), for argv preservation (an axiom about the platform,
tested end to end), or for the registry (simple, fully covered by tests). Kani
is not used: the repository runs Kani only for generated harnesses, the
kernels' interesting cases need unbounded lengths, and process code is outside
Kani's model.

The bridge between proof and production: each kernel lives in its own
dependency-free production module whose algorithm is mirrored line for line in
`verus/`; the Verus file's module documentation names the production module and
function; a contract test (`crates/theoremc-runner/tests/verus_mirror.rs`)
asserts that the named files and lemma names exist; and the property tests
check production against oracles independent of both. Tests continue to own
every observable behaviour; the proofs own only the two semantic properties.

### Suggested BDD features

Proposed path:
`crates/theoremc-runner/tests/features/backend_provider_contract.feature`, run
by `crates/theoremc-runner/tests/backend_provider_contract_bdd.rs`
(`rstest-bdd` 0.6.0).

```gherkin
Feature: Backend provider contract
  Providers expose every backend capability through injected ports only.

  Background:
    Given a fake backend registered as "fake"
    And a scripted command runner
    And an injected environment containing "FAKE_HOME" set to "/opt/fake"

  Scenario: A fake provider exercises every capability without process-global state
    When the caller discovers, resolves "1.2.3", plans installation, checks health,
    plans a run for "proofs/example.rs", parses its output, and plans a replay
    Then every capability returns a successful result
    And the scripted runner recorded exactly the discovery and health probes
    And every recorded command is an absolute program with an argument vector
    And every recorded command environment equals the injected environment subset

  Scenario: Independent providers do not share state
    Given a second fake backend with "FAKE_HOME" set to "/srv/other"
    When both providers discover their installations concurrently
    Then each installation reports its own home directory

  Scenario: Discovery reports a missing installation
    Given the scripted runner fails to spawn the discovery probe
    When the caller discovers the installation
    Then discovery fails with a "not installed" backend error

  Scenario: Parsing a timed-out run reports an incomplete result
    Given the scripted runner times out the run after partial output
    When the caller parses the run output
    Then the result is "incomplete" with the native termination "timed out"

  Scenario: Replay is refused for a provider without replay capability
    Given a fake backend registered as "no-replay" without replay capability
    When the caller asks for the replay capability
    Then no replay capability is available
    And the descriptor does not declare replay
```

Links: `REQ-no-global`, `REQ-capabilities`, `REQ-native-status`,
`REQ-capability-declared`; milestone `EP-M4`.

## Plan of work

### `EP-M0` feasibility probe (prototype, deleted afterwards)

Questions: can the pinned `rust-prover-tools` install and run Verus here, and
does `rustix` 1.1.5's process API compile at the workspace MSRV with
`unsafe_code = "forbid"`?

1. Create `tools/verus/VERSION` and `tools/verus/SHA256SUMS`, re-reading the
   digest from the GitHub releases API at implementation time and using the
   newest non-rolling release if a newer one exists. The values on 2026-11-16
   are:

   ```plaintext
   tools/verus/VERSION:
   0.2026.11.02.8bf68ee

   tools/verus/SHA256SUMS:
   f3a0119292df8855e4435da8130bee1873b0e6a60c306accf81a08ea687bba3c  verus-0.2026.11.02.8bf68ee-x86-linux.zip
   ```

2. Install Verus and verify a two-line trivial lemma kept under `/tmp`:

   ```bash
   make install-verus
   make verus VERUS_PROOF_FILE=/tmp/verus-probe/probe.rs
   ```

   This probe adds the two Make targets from `Concrete steps` early. They
   delegate to the existing pinned `$(PROVER_TOOLS)` variable, which expands to:

   ```plaintext
   uv tool run --python 3.14 --from git+https://github.com/leynos/rust-prover-tools@<REF> prover-tools
   ```

3. In a scratch crate under `/tmp`, call
   `rustix::process::kill_process_group` and
   `std::os::unix::process::CommandExt::process_group(0)` and check
   `cargo +1.89 check`.

Go: both succeed; keep the pin files for `EP-M2`. No-go on Verus: bump
`tools/rust-prover-tools/REF` to a commit that supports `verus install` and
retry once; then stop and escalate. No-go on `rustix`: stop and escalate
(alternatives need `unsafe` or a new dependency). Record transcripts in
`Artefacts and notes`. The `.verus/` install directory must be ignored by Git;
add it to `.gitignore` if absent.

### `EP-M1` crate skeleton, value types, limits, and redaction

Assigned: `REQ-library-boundary`, `REQ-argv-only`, `REQ-bounded` (limits),
`REQ-native-status` (types), `INV-limits-lower-only`, `INV-secret-silent`
(types), `GAP-no-crate`.

Create `crates/theoremc-runner/` with `Cargo.toml` (see
`Interfaces and dependencies`), `src/lib.rs`, and the `command` module's value
types: `ProgramPath`, `PublicArgument`, `CommandEnvironment`, `CommandSpec`,
`ProtectedInputRef`, `ProtectedValue`, `ProtectedDelivery`,
`ProtectedInputDeclaration`, `ProtectedInputChannel`, `ExecutionLimits`,
`CaptureLimits`, `CancellationSignal`, `Termination`, `CapturedStream`,
`CommandOutput`, `CommandProvenance`, `CommandError`. Add the crate to the root
`[workspace] members`.

Red: write `src/command/limits_tests.rs` (cap table), `spec_tests.rs` (relative
program or working directory rejected; argv order preserved),
`redaction_tests.rs` (secret property test and the provenance snapshot), and
`tests/dependency_boundary.rs` (reads `crates/theoremc-runner/Cargo.toml`
through `cap_std` and fails if it names `ortho_config`, `cargo-orthohelp`,
`clap`, or `cargo-theorem`). Run
`cargo nextest run -p theoremc-runner --all-features` and expect compile
failures for the missing types. Green: implement the types with private fields,
validating constructors, and hand-written `Debug` for every type that can
contain a `ProtectedValue`. Refactor and run the three gates.

### `EP-M2` binding and capture kernels with Verus proofs

Assigned: `INV-bind-exact`, `INV-bind-witness`, `INV-capture-accounting`,
`L-bind-*`, `L-capture-*`, `GAP-no-verus`.

Proof sketch first. Write `verus/runner_contract.rs` (root: shared spec
definitions and `mod` declarations), `verus/protected_binding.rs`, and
`verus/capture_accounting.rs` with the spec functions and the lemma
*statements* from `Invariants and lemmas`; run `make verus` and record the open
goals (the red state for proofs). Then:

1. Red: `src/command/binding_kernel_tests.rs` and `capture_kernel_tests.rs`
   with oracle property tests and named cases; they fail to compile.
2. Green: implement `binding_kernel::bind_keys` (nested scans, faults checked
   in the order duplicate-declared, duplicate-supplied, missing, extra) and
   `capture_kernel::CaptureAccount::{new, push, finish}` (head vector, tail
   `VecDeque`, saturating `u64` counter).
3. Mirror both algorithms as `exec fn`s in the Verus files and discharge the
   lemmas. Keep proof engineering (trigger annotations, sequence-axiom
   broadcasts, auxiliary induction lemmas) local and discovered rather than
   planned.
4. Run the seeded-fault controls described in the verification plan; revert.
5. Add `make install-verus` and `make verus` (see `Concrete steps`), the
   `verus` job in `.github/workflows/proofs.yml`, and
   `crates/theoremc-runner/tests/verus_mirror.rs`.

Wire `ProtectedInputChannel::bind` and `CapturedStream` construction onto the
kernels in the same milestone so no kernel is dead code.

### `EP-M3` provider traits, registry, and capability honesty

Assigned: `REQ-capabilities`, `REQ-separation`, `REQ-capability-declared`,
`INV-capability-honesty`, `INV-registry-order`.

Create the `backend` module: `BackendId`, `BackendVersion`, `HostTarget`,
`Capability`, `CapabilitySet`, `BackendDescriptor`, `EnvironmentSnapshot`,
`BackendContext`, `VersionRequest`, `ResolvedRelease`, `ReleaseSource`,
`Installation`, `InstallPlan`, `InstallStep`, `HealthReport`, `HealthProbe`,
`BackendRunRequest`, `RunTarget`, `BackendRunResult`, `RunVerdict`,
`ReplayRequest`, `BackendError`, the three traits `BackendLifecycle`,
`BackendExecution`, `BackendReplay`, the umbrella `BackendProvider`, and
`BackendRegistry`.

Red: `src/backend/registry_tests.rs` with honest, dishonest (declares replay
without providing it; provides replay without execution), and duplicate
providers defined inside the test module, plus the ordering property test.
Green: implement
`BackendRegistry::new(providers) -> Result<Self, RegistryError>` validating
honesty and uniqueness, `list`, and `get`.

### `EP-M4` scripted runner, fake provider, and the success scenario

Assigned: `REQ-no-global`, `REQ-one-runner` (test doubles), `INV-isolation`.

Create `src/test_support/` behind `#[cfg(feature = "test-support")]`:
`ScriptedCommandRunner` (a queue of expected-command matchers and scripted
`Result<CommandOutput, CommandError>` replies; records every `CommandSpec`;
reports unmatched or leftover expectations as test failures through a
`verify()` returning `Result`) and `FakeBackend` (implements all three traits;
discovery probes `<home>/bin/fake --version` through the context's runner;
resolution accepts exact versions; installation plans a download step plus a
command step; health runs a probe; run planning builds argv from `RunTarget`;
parsing maps termination and stdout to `RunVerdict`; replay plans a playback
command). `FakeBackend::without_replay` builds the variant used by the refusal
scenario.

Red: the feature file and `tests/backend_provider_contract_bdd.rs` from
`Suggested BDD features`, plus the `tests/no_global_state.rs` fitness test.
Green: implement the doubles. The success criterion is met when the first
scenario passes.

### `EP-M5` `ProcessCommandRunner` production adapter

Assigned: `REQ-one-runner`, `REQ-owned-termination`, `REQ-protected` (delivery),
`REQ-bounded` (pumping), `INV-argv-preserved`, `INV-env-exact`,
`INV-native-status`, `INV-bounded-return`, `INV-group-reaped`,
`INV-bind-before-spawn`.

Create `src/process/` (`#[cfg(unix)]`): `mod.rs` (`ProcessCommandRunner`,
`impl CommandRunner`), `spawn.rs` (builds `std::process::Command`: `env_clear`,
`envs`, `current_dir`, `process_group(0)`, piped stdio), `pump.rs` (one reader
thread per stream feeding a `CaptureAccount`, reading in 64 KiB chunks and
continuing to drain after truncation), `stdin.rs` (protected writer thread),
`supervise.rs` (poll loop every 10 ms over `try_wait`, `CancellationSignal`,
and deadlines, delegating each decision to the pure `next_action`), and
`signal.rs` (`kill_process_group` with `Signal::TERM` and `Signal::KILL`). Add
`src/bin/fake_tool.rs` (`[[bin]] name = "theoremc-runner-fake-tool"`,
`required-features = ["test-support"]`) with modes `echo-context`,
`exit <code>`, `signal <name>`, `sleep <ms>`, `ignore-term-and-sleep <ms>`,
`flood <stream> <bytes>`, `spawn-lingering-grandchild <pid-file>`, `read-stdin`,
`ready-then-sleep`, and `touch-marker <path>`.

Red: `tests/process_runner.rs` and `tests/process_runner/*.rs` covering every
obligation listed for this milestone, placed in a new nextest group
`process-runner` (`max-threads = 2`) in `.config/nextest.toml`. Green:
implement the adapter. Refactor to keep each function under the Clippy
thresholds.

### `EP-M6` documentation, ADR, and roadmap closure

Assigned: `GAP-illustrative-api`.

1. Write
   `docs/adrs/adr-0014-backend-provider-contract-and-command-runner-port.md`
   recording `D1` to `D11`, the Verus decision, and the rejected alternatives.
2. Rewrite cargo-theorem-cli-design.md §12.1 to describe the delivered traits,
   `CommandRequest`, `Termination`, bookend capture, environment policy,
   protected-input binding semantics, and the proof obligations, citing
   ADR-0014; state the remaining illustrative parts (none expected).
3. Update `docs/developers-guide.md` §3.8 with the component architecture of
   `theoremc-runner`, its reuse policy (only `theoremc-runner` starts backend
   processes; providers never call `std::process`; test doubles come from the
   `test-support` feature), the Verus proof inventory and mirror-maintenance
   rule, `make verus`, and the `process-runner` nextest group; correct the
   `rstest-bdd` version.
4. Update `docs/repository-layout.md` (new crate, `verus/`, `tools/verus/`)
   and `docs/contents.md` (ADR-0014).
5. Record in `docs/users-guide.md`? No: this task changes no user-facing
   behaviour. State that in `Outcomes & retrospective` instead and leave the
   users' guides untouched.
6. Mark roadmap 4.4.1 done only after every acceptance item passes.

Stale-claim search, excluding completed ExecPlans:

```bash
rg -n 'illustrative rather than a frozen|CommandRunner|theoremc-runner' docs \
  --glob '!docs/execplans/*'
```

## Milestones and plateaus

- `EP-M0`: probe results recorded; pin files kept or the plan blocked.
  Recovery: delete scratch under `/tmp`. Compatibility: none.
- `EP-M1`: the crate builds, value types validate, secrets cannot render.
  Acceptance: `make check-fmt`, `make lint`, `make test` pass; new tests pass.
  Conformance: no dependency on application crates; no new public API outside
  the crate. Recovery: revert the milestone commit. Remaining: kernels, traits,
  adapter. Compatibility: none.
- `EP-M2`: kernels proved and property-tested; `make verus` green.
  Acceptance: the three gates plus `make verus`. Conformance: proofs contain no
  `assume` or `admit` (`rg -n 'assume|admit|external_body' verus/` returns
  nothing). Recovery: revert; pins remain harmless. Compatibility: none.
- `EP-M3`: traits and registry compile and are tested with in-test providers.
  Acceptance: the three gates. Compatibility: none.
- `EP-M4`: roadmap success criterion met. Acceptance: the three gates and the
  BDD scenario. Compatibility: none.
- `EP-M5`: production adapter meets every process obligation. Acceptance: the
  three gates. Recovery: revert; the port and fakes remain valid.
  Compatibility: none.
- `EP-M6`: documents reconciled; roadmap ticked. Acceptance: `make fmt`,
  `make markdownlint`, `make nixie`, `make spelling`, the three gates.
  Compatibility: none.

At each boundary record: requirements satisfied, design still followed,
upstream assumptions valid, no unapproved interface or dependency, no trust
boundary or persisted-format change, trace links current.

## Concrete steps

Working directory: the repository root. Use `set -o pipefail` and log every
gate:

```bash
set -o pipefail
BR=$(git branch --show-current)
make fmt 2>&1 | tee /tmp/fmt-theoremc-$BR.out
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-$BR.out
make lint 2>&1 | tee /tmp/lint-theoremc-$BR.out
make test 2>&1 | tee /tmp/test-theoremc-$BR.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-$BR.out
make nixie 2>&1 | tee /tmp/nixie-theoremc-$BR.out
make spelling 2>&1 | tee /tmp/spelling-theoremc-$BR.out
make verus 2>&1 | tee /tmp/verus-theoremc-$BR.out
```

Run gates sequentially; delegate full gate runs to the `scrutineer` agent.
`make verus` applies from `EP-M2` onwards.

Focused commands:

```plaintext
Red:   cargo nextest run -p theoremc-runner --all-features <filter>  -> compile error or failing assertion
Green: cargo nextest run -p theoremc-runner --all-features <filter>  -> all selected tests pass
Proof: make verus                                                    -> "verification results:: N verified, 0 errors"
```

Planned Makefile additions (recipes use tabs):

```makefile
VERUS_PROOF_FILE ?= verus/runner_contract.rs
VERUS_FLAGS ?=

install-verus: ## Install the pinned Verus release
	$(PROVER_TOOLS) verus install --repo-root .

verus: ## Verify the runner kernels with Verus
	$(PROVER_TOOLS) verus run --repo-root . --proof-file "$(VERUS_PROOF_FILE)" $(VERUS_FLAGS)
```

`make verus` stays out of `make all`, `make test`, and `make lint`, matching
`make kani`; CI runs it from `.github/workflows/proofs.yml` on pull requests
touching `verus/**`, `tools/verus/**`, or
`crates/theoremc-runner/src/command/*_kernel.rs`, and weekly.

Commit after each milestone, gated by the three gates (plus `make verus` from
`EP-M2`). Use the `commit-message` skill's format and the repository's
attribution line.

## Validation and acceptance

- `cargo nextest run -p theoremc-runner --all-features` passes; the scenario
  "A fake provider exercises every capability without process-global state"
  fails before `EP-M4` and passes after.
- `make verus` reports every lemma in `verus/` verified with zero errors, and
  `rg -n 'assume|admit|external_body' verus/` prints nothing.
- Timeout test: a fake tool ignoring SIGTERM with a 300 ms timeout and 200 ms
  grace returns `Termination::TimedOut { forced: true }` within 6 s, and its
  grandchild pid is gone.
- Flood test: `flood stdout 20000000` with default limits yields
  `bytes_seen = 20000000`, `bytes_retained = 16777216`,
  `limit_bytes = 16777216`, `truncated = true`, and the retained bytes equal
  the first 4 MiB and last 12 MiB of the expected stream.
- Binding test: a missing protected input yields
  `CommandError::ProtectedInput(Missing { name })` and the marker file is
  absent.
- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, and `make spelling` pass.

Quality criteria: tests as above; verification obligations discharged as in
`Methods per obligation`; no Clippy, rustdoc, or Whitaker findings; no
performance target beyond the bounded-return contract; security review of
redaction via the `INV-secret-silent` tests.

## Idempotence and recovery

Every milestone is a separate commit and can be reverted alone.
`make install-verus` is idempotent (it reuses an existing installation).
Process tests create files only under `tempfile` directories. The seeded-fault
controls are temporary edits; revert them with `git checkout -- <file>` and
delete any `proptest-regressions` file they create.

## Artefacts and notes

Reconnaissance (2026-11-16, revision `4c059669`): Verus release query output,
`Cargo.lock` dependency survey, and the `rust-prover-tools` behaviour summary
are reflected in `Context and orientation`. Probe and red/green transcripts are
added here during implementation.

## Interfaces and dependencies

New package `crates/theoremc-runner` (`publish = false` is not set on sibling
crates, so follow them and leave it unset), edition 2024,
`rust-version = "1.89"`, `[lints] workspace = true`.

Dependencies (caret requirements):

- `thiserror` (workspace), `camino = "1.2.2"`, `serde` (workspace, derive) for
  `CommandProvenance` and descriptor serialization.
- Unix-only `rustix` with the `process` feature, declared under
  `[target.'cfg(unix)'.dependencies]` as
  `rustix = { version = "1.1.5", features = ["process"] }`.
- Dev: `rstest = "0.28.0"`, `rstest-bdd = "0.6.0"`,
  `rstest-bdd-macros = "0.6.0"`, `googletest = "0.14.3"`,
  `pretty_assertions = "1.4.1"`, `proptest = "1.11.0"`, `insta = "1.47.2"`,
  `serde_json` (workspace), `tempfile = "3.27.0"`,
  `cap-std = { version = "4.0.2", features = ["fs_utf8"] }`.
- Features: `test-support = []`.

Module layout (each file under 400 lines):

```plaintext
crates/theoremc-runner/src/
  lib.rs
  command/{mod.rs, spec.rs, protected.rs, binding_kernel.rs, capture_kernel.rs,
           limits.rs, output.rs, cancel.rs, runner.rs, error.rs, provenance.rs}
  backend/{mod.rs, identity.rs, descriptor.rs, context.rs, lifecycle.rs,
           execution.rs, replay.rs, registry.rs, error.rs}
  process/{mod.rs, spawn.rs, pump.rs, stdin.rs, supervise.rs, signal.rs}
  test_support/{mod.rs, scripted_runner.rs, fake_backend.rs}
  bin/fake_tool.rs
```

Signatures that must exist at the end (names are normative; field lists may
grow during implementation if recorded in `Decision log`):

```rust
pub trait CommandRunner: Send + Sync {
    fn run(&self, request: CommandRequest<'_>) -> Result<CommandOutput, CommandError>;
}

pub struct CommandRequest<'a> {
    spec: &'a CommandSpec,
    inputs: &'a ProtectedInputChannel,
    limits: &'a ExecutionLimits,
    cancellation: &'a CancellationSignal,
}

pub struct CommandSpec {
    program: ProgramPath,                 // absolute, UTF-8
    argv: Vec<PublicArgument>,            // public values only
    working_directory: Utf8PathBuf,       // absolute
    environment: CommandEnvironment,      // the child's entire environment
    protected_inputs: Vec<ProtectedInputDeclaration>,
}

pub enum Termination {
    Exited { code: i32 },
    Signalled { signal: i32 },
    TimedOut { forced: bool },
    Cancelled { forced: bool },
}

pub struct CommandOutput {
    pub termination: Termination,
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
    pub elapsed: Duration,
    pub protected_delivery: ProtectedDelivery,
    pub descendants_terminated: bool,
}

pub struct CapturedStream { /* head, tail, bytes_seen: u64, limit_bytes: u64 */ }
impl CapturedStream {
    pub fn bytes_seen(&self) -> u64;
    pub fn bytes_retained(&self) -> u64;
    pub fn limit_bytes(&self) -> u64;
    pub fn is_truncated(&self) -> bool;
    pub fn head(&self) -> &[u8];
    pub fn tail(&self) -> &[u8];
}

pub trait BackendLifecycle: Send + Sync {
    fn descriptor(&self) -> &BackendDescriptor;
    fn discover(&self, context: &BackendContext<'_>) -> Result<Installation, BackendError>;
    fn resolve(&self, request: &VersionRequest) -> Result<ResolvedRelease, BackendError>;
    fn plan_install(&self, release: &ResolvedRelease, host: &HostTarget)
        -> Result<InstallPlan, BackendError>;
    fn check(&self, context: &BackendContext<'_>, installation: &Installation)
        -> Result<HealthReport, BackendError>;
}

pub trait BackendExecution: Send + Sync {
    fn plan_run(&self, installation: &Installation, request: &BackendRunRequest)
        -> Result<CommandSpec, BackendError>;
    fn parse_run(&self, request: &BackendRunRequest, output: &CommandOutput)
        -> Result<BackendRunResult, BackendError>;
}

pub trait BackendReplay: Send + Sync {
    fn plan_replay(&self, installation: &Installation, request: &ReplayRequest)
        -> Result<CommandSpec, BackendError>;
}

pub trait BackendProvider: Send + Sync {
    fn lifecycle(&self) -> &dyn BackendLifecycle;
    fn execution(&self) -> Option<&dyn BackendExecution>;
    fn replay(&self) -> Option<&dyn BackendReplay>;
}

pub struct BackendContext<'a> {
    runner: &'a dyn CommandRunner,
    environment: &'a EnvironmentSnapshot,
    cancellation: &'a CancellationSignal,
    limits: &'a ExecutionLimits,
    host: HostTarget,
}
```

`InstallPlan` is data only: an ordered `Vec<InstallStep>` with
`InstallStep::Download { url, sha256, max_bytes }` (HTTPS enforced at
construction unless the `test-support` feature builds a local-file variant),
`InstallStep::Extract { archive, destination }`, and
`InstallStep::Command(CommandSpec)`. Executing a plan is 4.4.2.

`BackendError` (thiserror, boxed large payloads): `Unsupported { capability }`,
`NotInstalled { reason }`, `InvalidRequest { reason }`,
`UnsupportedHost { host }`, `Command(Box<CommandError>)`,
`UnexpectedOutput { code }`, `OutputLimit { stream }`. `CommandError`:
`InvalidSpec { reason }`, `ProtectedInput(BindingFault)`,
`UnsupportedDelivery { name }`, `Spawn { kind: io::ErrorKind }`,
`Io { stage, kind }`, `Signal { kind }`. Error text names programs by file name
only and never includes argument or environment values.

Reuse policy (to be copied into the developers' guide): `theoremc-runner` owns
every backend process start. Providers depend only on the `backend` and
`command` modules and never import `process`. The composition root (the
`cargo-theorem` binary, from 4.4.2) constructs `ProcessCommandRunner`,
`EnvironmentSnapshot`, and `BackendRegistry` once and passes references down.
Other crates use `test_support` doubles only from tests. A second
process-spawning abstraction anywhere in the workspace's production code
violates this design.

## Revision note

- 2026-11-16: initial draft from reconnaissance; awaiting expert design review
  and user approval.
