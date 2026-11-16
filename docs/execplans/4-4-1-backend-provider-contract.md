# 4.4.1 Define a backend provider contract and injected command runner

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (expert design review applied; awaiting user approval)

Branch: `4-4-1-backend-provider-contract`

Roadmap item: `4.4.1` in `docs/roadmap.md`.

## Purpose / big picture

Theoremc will eventually install, check, and run proof backends (Kani today,
Verus next) through `cargo theorem backend ...` and `cargo theorem run`. Today
the repository has no code that talks to a proof backend: Kani is installed by
the Python tool `rust-prover-tools`, and the only subprocess code in the
workspace is a test-only deadline helper, `src/arg_lowering_rustc_deadline.rs`.

This task creates the seam every later backend task plugs into. After it lands,
a new library crate, `crates/theoremc-runner`, provides:

1. A *backend provider contract*: Rust traits through which a backend
   describes itself, discovers an installation, resolves a release, plans an
   installation and its post-install preparation, checks its health, plans a
   run as an argument vector, parses a run's output into per-target proof
   outcomes, and plans and parses a counterexample replay. A *provider* is the
   backend-specific implementation of those traits (for example, the Kani
   provider that roadmap task 4.4.3 will write).
2. A *command runner port*: one injected `CommandRunner` trait that is the only
   way provider-driven code starts a process. It owns timeouts, cancellation,
   bounded capture of standard output and standard error, and delivery of
   protected (secret) inputs through a channel that is never part of the
   argument vector. A shell command string cannot be expressed in its types.
3. A production adapter, `ProcessCommandRunner`, that implements the port with
   `std::process`, a dedicated process group, a single-threaded supervision
   loop over non-blocking pipes, a bounded grace period, and bounded "bookend"
   capture (the start and the end of each stream).
4. Test doubles behind a `test-support` feature: a `ScriptedCommandRunner`
   that records every command and replays scripted outcomes, a `FakeBackend`
   provider that exercises every capability through it, and a reusable fake
   executable whose behaviour is selected by its arguments.

A caller in a later task will write code like this:

```rust,ignore
let probe = ProbeContext::new(&planning, &runner).with_cancellation(&cancel);
let installation = provider.lifecycle().discover(&probe, &discovery_request)?;
let health = provider.lifecycle().check(&probe, &installation)?;
let execution = provider.execution().ok_or(BackendError::unsupported(...))?;
let plan = execution.plan_run(&planning, &installation, &run_request)?;
let output = runner.run(&CommandRequest::new(&plan.command).with_inputs(&channel))?;
let result = execution.parse_run(&run_request, &output)?;
```

Observable success: running
`cargo nextest run -p theoremc-runner --all-features` passes, including the
behavioural scenario "a fake provider exercises every capability without
process-global state", a runner-contract suite that both runners satisfy,
process-adapter tests that stop timed-out or cancelled process trees within a
documented bound, and tests showing that argument vectors arrive byte for byte
and secrets never appear in rendered or serialized output. `make verus`
verifies the two contractual kernels, and a test in the required gate fails if
either production kernel drifts from its verified mirror.

No user-visible command changes in this task. `cargo theorem` gains no new
subcommand (`backend` commands arrive in 4.4.2), so the users' guides need no
behavioural update; `EP-M6` records this explicitly.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- The new crate `crates/theoremc-runner` must not depend on `cargo-theorem`,
  Clap, `ortho_config`, `cargo-orthohelp`, terminal rendering, or any
  process-global configuration (cargo-theorem-cli-design.md §5; the
  `cargo theorem` application-boundary section, §3.8, of
  `docs/developers-guide.md`).
- No production code in `theoremc-runner` may read the process environment
  (already denied by `clippy.toml`), change or read the current directory, rely
  on `PATH` lookup by the operating system, or use `static` mutable state,
  `thread_local!`, global registries, or process-wide signal handlers.
  Environment values reach providers only as an injected `EnvironmentSnapshot`
  value.
- `std::process` may appear only in `crates/theoremc-runner/src/process/` and
  in the fake-tool sources; providers never start processes themselves.
- Providers construct UTF-8 argument vectors only. No API may accept a whole
  command line as one string, and production code never invokes `sh -c` or
  another shell. Tests may launch `/bin/sh` as an ordinary argv program only to
  set a signal disposition the fake tool cannot set without `unsafe` (see
  `D18`).
- Protected inputs never enter argv, the child environment, `Debug` output,
  `Display` output, serde serialization, error messages, or test snapshots
  (cargo-theorem-cli-design.md §§11.3, 16).
- Captured stdout and stderr are each bounded by at most 16 MiB retained bytes,
  configurable only downwards (cargo-theorem-cli-design.md §8.6).
- `unsafe_code` is forbidden workspace-wide, including in binary targets.
  Process-group signalling, non-reaping waits, polling, and non-blocking pipes
  use safe `rustix` APIs.
- The process adapter is Unix-only (`#[cfg(unix)]`), matching the Linux-only
  CI. If Windows support becomes a requirement, stop and escalate.
- Every file stays below 400 lines (AGENTS.md); Clippy thresholds from
  `clippy.toml` apply (`too-many-arguments-threshold = 4`,
  `too-many-lines-threshold = 70`, `cognitive-complexity-threshold = 9`).
- Every module starts with `//!`; every public item has `///` docs with an
  example where meaningful; lints are silenced only with item-scoped
  `#[expect(..., reason = ...)]`.
- Out of scope: `theoremc.toml` backend intent, `theoremc.lock`, the backend
  cache and its locks, `BackendRegistry` and `backend list`/`get` (all 4.4.2),
  executing install plans (4.4.2), Kani and Verus providers (4.4.3, 4.4.4), run
  records and the ledger (6.x), file-backed output sinks (6.x), tracing spans
  and metrics (4.5.4).
- Do not modify completed ExecPlans under `docs/execplans/`.
- Marking the 4.4.1 checkbox is a non-structural roadmap edit; structural
  roadmap edits would need `mapsplice` (`docs/developers-guide.md` §3.9).
- Verus files contain no `assume`, `admit`, `#[verifier::external_body]`, or
  other trusted bodies on proved items.

## Tolerances (exception triggers)

Comparison base: `origin/main` at `4c059669c0503ff0cb95764283449e27128e7781`.
Count from the repository root with:

```bash
git diff --numstat 4c059669c0503ff0cb95764283449e27128e7781 -- . \
  ':(exclude)docs/execplans/4-4-1-backend-provider-contract.md'
```

Whole-file classification; each file counts in exactly one category:

- Code: `*.rs` files (production, tests, fake tool, Verus proofs) and
  `Makefile`.
- Configuration: `Cargo.toml` files, `Cargo.lock`,
  `.github/workflows/*.yml`, `.config/nextest.toml`, `.gitignore`, and
  `tools/verus/VERSION` and `tools/verus/SHA256SUMS`.
- Documentation: `*.md` and `*.feature` files other than this ExecPlan.

Budgets are net lines (additions minus deletions), enforced per category
independently; spare capacity in one category never offsets another.

- Files: stop and escalate above 90 changed non-ExecPlan files.
- Code: stop and escalate above 5,600 net lines (estimate: 2,300 production
  including the fake tool, 2,400 tests, 450 Verus, 40 Makefile).
- Configuration: stop and escalate above 300 net lines (estimate: crate
  manifest 60, root manifest 5, lockfile 30, `verus.yml` 70, nextest 15, pins
  and ignore 5).
- Documentation: stop and escalate above 1,050 net lines (estimate: ADR-0014
  240, design §12.1 200, developers' guide 200, feature file 90,
  `tools/verus/README.md` 40, layout and contents 20, roadmap 1).
- Interface: the only new public API is the `theoremc-runner` crate surface in
  `Interfaces and dependencies`, plus `Makefile` targets `install-verus` and
  `verus`. Any change to another crate's public API stops work.
- Dependencies: only those listed in `Interfaces and dependencies`. Anything
  else (for example `process-wrap`, `wait-timeout`, `tokio`, `mockall`,
  `mockable`, `nix`, `signal-hook`) stops work.
- Verus: if a kernel proof still fails after five focused proof attempts, or a
  proof appears to need an `assume`, stop and record the open verification
  condition. Before counting attempts, apply the shape gate in `EP-M2`.
- Iterations: if a focused test still fails after five fix attempts, stop.
- Time: if a milestone exceeds one working day of elapsed effort, stop.
- Ambiguity: if the delivered trait surface must diverge from this plan's
  `Interfaces and dependencies` in a way that changes a method's inputs or
  outputs, stop and present options.

Budget survey. The crate is greenfield, so the estimate comes from the feature
list rather than from callers. Comparable plans:
`docs/execplans/2-3-3-struct-literal-synthesis-from-yaml-maps.md` set "14
changed files or roughly 800 net lines" and never raised it;
`docs/execplans/3-3-2-referenced-type-probes-for-generated-struct-literal-synthesis.md`
set "eight changed code files or 500 net Rust lines" and split files rather
than raising budgets;
`docs/execplans/4-1-2-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli.md`
used qualitative triggers only. None raised a numeric budget, so there is no
evidence of repeated underestimation. The first draft of this plan budgeted
4,600 code lines; the design review added a provider-preparation step, richer
outcome types, a runner-contract suite, Verus on-ramp contract tests, and the
mirror tripwire, so the budget rose to 5,600 before approval. The contingency
is about 15 % per category.

## Risks

- Risk: the pinned `rust-prover-tools` commit (`tools/rust-prover-tools/REF`,
  `592558f7…`) or the shared `install-prover-tools` action may not support
  Verus installation from `tools/verus/SHA256SUMS`. Severity: medium.
  Likelihood: medium. Mitigation: `EP-M0` probes both; fallback is bumping the
  REF in a separate commit (authorized), otherwise `make install-verus` in CI;
  if neither works, stop.
- Risk: a backend leaves a descendant holding stdout or stderr open, or a
  descendant escapes the group with `setsid()`. Severity: high. Likelihood:
  medium (sccache and job-server children are common under Cargo). Mitigation:
  the supervision loop never blocks on a pipe; after the run bound it closes
  its pipe ends and returns what it captured (`D13`).
- Risk: process-group identifier reuse makes a late group signal hit an
  unrelated process group. Severity: high. Likelihood: low but catastrophic on
  a shared host. Mitigation: the leader is observed with a non-reaping `waitid`
  and reaped only after the last group signal (`D14`).
- Risk: the Verus mirror drifts from production. Severity: high. Likelihood:
  medium. Mitigation: production and mirror use the same data representation
  and textually identical marked bodies, checked by a test in `make test`
  (`D16`); the property and enumeration tests tie production to oracles
  transcribed from the Verus specifications.
- Risk: timing-based process tests flake on a loaded shared host, or leak
  fake-tool processes when a test times out. Severity: medium. Likelihood:
  medium. Mitigation: readiness files instead of sleeps, generous upper bounds
  only, start-time-aware liveness checks, a dedicated nextest group, and a hard
  lifetime plus parent-death check in every fake-tool sleeping mode.
- Risk: the trait surface guessed now does not fit Kani or Verus (ADR-0006
  §10 warns against freezing guessed traits). Severity: medium. Likelihood:
  medium. Mitigation: the fake provider models the documented Kani and Verus
  behaviours from `rust-prover-tools`, including Verus's probe-dependent
  toolchain preparation; the crate is pre-1.0 with no external consumers, so
  4.4.3 and 4.4.4 may change the traits together with every caller.
- Risk: the cargo-theorem process loses `SIGINT` delivery to the backend,
  because the child leads its own process group, and a crashed parent leaves
  the backend running. Severity: medium. Likelihood: medium. Mitigation: the
  contract states that the composition root (4.4.2) maps `SIGINT` and `SIGTERM`
  to `CancellationSignal`; parent-death signalling needs `unsafe` `pre_exec`
  and is a recorded residual risk for 6.x reconciliation.

## Progress

- [x] (2026-11-16) Branch renamed, Lody session titled, reconnaissance by four
  Wyvern scouts and one prior-art researcher completed.
- [x] (2026-11-16) Initial draft written and committed.
- [x] (2026-11-16) Expert panel review (three reviewers, six lenses) applied;
  verdict "proceed with conditions"; both blocking conditions resolved in this
  revision (`D16`, `D17`).
- [ ] Plan approved by the user.
- [ ] `EP-M0` feasibility probes.
- [ ] `EP-M1` crate skeleton, value types, limits, request validation, and
  redaction.
- [ ] `EP-M2` binding and capture kernels with Verus proofs (end of PR A).
- [ ] `EP-M3` provider traits, contexts, and provider validation.
- [ ] `EP-M4` scripted runner, fake provider, runner-contract suite, and the
  success scenario (end of PR B; roadmap criterion met).
- [ ] `EP-M5` `ProcessCommandRunner` production adapter.
- [ ] `EP-M6` documentation, ADR, and roadmap closure (end of PR C).

## Surprises & discoveries

- Observation: `AGENTS.md` mandates `mockable::Env`, but no workspace crate
  depends on `mockable`; the CLI uses `ortho_config`'s `EnvSource` and
  `MapEnv`, which `AGENTS.md` also accepts. Evidence: `Cargo.lock`;
  `crates/cargo-theorem/src/cli.rs` `ConfigurationSources`. Impact: the runner
  takes an immutable `EnvironmentSnapshot` value (`D4`).
- Observation: `rust-prover-tools` has no Kani concrete-playback code. Impact:
  replay planning and parsing are shaped from theoremc-design.md §8.5 and
  cargo-theorem-cli-design.md §11.4.
- Observation: two references named in the task brief do not exist under
  those paths: `docs/rstest-bdd-users-guide.md` (absent; rstest-bdd guidance
  lives in `docs/developers-guide.md` test conventions and the crate's own
  documentation) and `docs/ortho-config-users-guide.md` (the guide is
  `docs/ortho-config/users-guide.md`). `docs/developers-guide.md` also cites
  `rstest-bdd` v0.5.0 while the workspace pins 0.6.0. Impact: corrected
  references here; the version is fixed in `EP-M6`.
- Observation (design review): the fake tool cannot ignore `SIGTERM`, because
  setting a signal disposition needs `unsafe` or a forbidden dependency. Impact:
  `D18`.
- Observation (design review): `wait-timeout` 0.2.1, present only
  transitively through `rusty-fork`, reaps the child and installs a process-wide
  `SIGCHLD` handler through a `static mut`. Impact: rejected in `D13`.

## Decision log

- D1. Decision: create a new workspace crate `crates/theoremc-runner` rather
  than a module in `cargo-theorem` or `theoremc-core`. Rationale:
  cargo-theorem-cli-design.md §5 assigns "backend provider interface, command
  execution abstraction" to `theoremc-runner` and forbids the CLI being the
  only home of reusable execution logic; `theoremc-core` is consumed by the
  proc-macro and must stay free of process code. Rejected: a module in
  `cargo-theorem`, which would force extraction when `run` (6.2) needs the same
  types. The crate is published with its siblings (because `cargo-theorem` will
  depend on it) but documented as an unstable pre-1.0 API in ADR-0014.
  Date/Author: 2026-11-16, planning agent.
- D2. Decision: deliver the production `ProcessCommandRunner` in this task, as
  the third of three stacked pull requests: PR A (`EP-M0` to `EP-M2`: crate,
  value types, kernels, Verus), PR B (`EP-M3`, `EP-M4`: traits, doubles,
  success scenario), PR C (`EP-M5`, `EP-M6`: adapter, documentation, roadmap).
  Rationale: the roadmap says the injected runner owns timeouts, cancellation,
  and stream capture; proving those only against a fake would encode guesses
  that 4.4.3 and 4.4.4 then inherit. A single pull request of about 6,000 lines
  is too large to review, so the stack keeps each review focused. The roadmap
  checkbox is ticked only in PR C. Rejected: deferring the adapter to 4.4.3,
  which would mix provider porting with process-supervision debugging.
  Date/Author: 2026-11-16, planning agent; revised after design review.
- D3. Decision: two contexts. `PlanningContext` is pure data (environment
  snapshot, host target, workspace root, release-source policy) passed to every
  planning method. `ProbeContext` adds the injected runner, a cancellation
  signal, and small probe limits, and is passed only to `discover` and `check`.
  Planning and parsing methods are pure functions of their inputs. Rationale:
  under `D4` every command needs explicit environment, working directory, and
  absolute tools, so planners need data the first draft did not give them.
  Keeping the runner only in `ProbeContext` keeps everything but probing
  deterministic, while following §12.1's `discover(context)` shape. Rejected:
  fully sans-I/O providers returning probe steps to an orchestrator
  (`plan_probe() -> ProbeStep::{Run(CommandSpec), Done(T)}` plus
  `interpret(output)`), which would make every method pure and remove the
  scripted runner from the success test, at the cost of a step protocol that
  diverges from §12.1 and about 150 more lines; ADR-0014 records it as the
  strongest alternative and the migration path if probing grows. Date/Author:
  2026-11-16, planning agent; revised after design review.
- D4. Decision: inject the environment as an immutable `EnvironmentSnapshot`
  value and make every child environment explicit: the runner clears the
  inherited environment and applies exactly `CommandSpec::environment`.
  Programs and working directories are absolute UTF-8 paths. Tool lookup is a
  pure helper, `EnvironmentSnapshot::program_candidates(name)`, which yields
  absolute candidates from the snapshot's `PATH`; discovery probes candidates
  in order and treats spawn `NotFound` as "try the next". Discovered tool paths
  are stored in `Installation`, which planners then use. Rationale: the
  strongest reading of "without process-global state": neither providers nor
  children observe ambient environment, current directory, or operating-system
  `PATH` lookup. `rust-prover-tools` already curates Kani's child environment,
  so providers need this control. Rejected: Cuprum-style overlay environments,
  which make child behaviour depend on the caller's shell. Date/Author:
  2026-11-16, planning agent; revised after design review.
- D5. Decision: any process that was started and observed returns
  `Ok(CommandOutput)`, whatever its exit; `CommandError` is reserved for
  failures that prevent a faithful observation (invalid request, binding
  rejection, cancellation before start, spawn failure, or a failure to signal
  or reap). Pipe errors after spawn are recorded on the affected
  `CapturedStream` instead. Rationale: Verus `--version` exits non-zero as
  useful data, and a timed-out proof must still yield partial output for
  `parse_run`. Native exit codes stay in data, never in the application's exit
  taxonomy (§8.4). Date/Author: 2026-11-16, planning agent.
- D6. Decision: bookend capture with lazily grown buffers. Each stream retains
  the first `head_bytes` and last `tail_bytes` it produced, with
  `head_bytes + tail_bytes <= 16 MiB` (run default 4 MiB head and 12 MiB tail;
  probe default 64 KiB total), and records `bytes_seen`, `bytes_retained`,
  `limit_bytes`, and `truncated`. Capture stays in memory in this task; a
  file-backed sink seam is a 6.x addition to `CommandRequest`. Rationale: Kani
  prints its verdict last and compile errors first. §8.6's "bounded sinks
  stream … to files" applies when run directories exist (6.x); ADR-0014 records
  the reconciliation. Worst case is 32 MiB per running job. Date/Author:
  2026-11-16, planning agent; revised after design review.
- D7. Decision: protected inputs support one delivery mode, standard input,
  with at most one stdin-delivered input per command; any other mode is
  rejected as unsupported before process creation. Commands without a protected
  input get `Stdio::null()` as standard input. The channel is a map, so
  duplicate references are rejected when the channel is built. Rationale: §11.3
  allows "a stdin-like channel"; neither Kani nor Verus needs a secret today; a
  null stdin enforces §8.1 non-interactive operation and prevents a
  background-group child stopping on terminal input. Date/Author: 2026-11-16,
  planning agent; revised after design review.
- D8. Decision: Verus for the protected-input binding kernel and the bookend
  capture-accounting kernel only; exhaustive enumeration, property tests,
  parameterized tests, and process-level integration tests elsewhere. See
  `Formal verification decision`. Date/Author: 2026-11-16, planning agent.
- D9. Decision: handwritten test doubles exported behind `test-support`
  rather than `mockall` mocks; both runners call the same
  `CommandRequest::validate`, and one runner-contract suite runs against both.
  Rationale: 4.4.2 to 4.5.2 reuse these doubles from other crates; they must
  script failures and record calls; shared validation stops the fake from
  accepting requests the real runner rejects. Date/Author: 2026-11-16, planning
  agent; revised after design review.
- D10. Decision: no tracing spans or metrics in this task.
  Rationale: the metric registry and its tests are roadmap 4.5.4, and no
  application subscriber exists yet; unverified telemetry would violate
  AGENTS.md. The supervision loop is the single place to add the child-process
  span later. Date/Author: 2026-11-16, planning agent.
- D11. Decision: ADR-0006 §10 does not block these traits.
  Rationale: §10 concerns harness renderers; the provider lifecycle traits are
  prescribed by the accepted design §12.1, and two real backends plus the fake
  demonstrate shared lifecycle behaviour. ADR-0014 records the distinction.
  Date/Author: 2026-11-16, planning agent.
- D12. Decision: capabilities are fine-grained and partly derived. A
  descriptor declares `Install`, `Preparation`, `SuiteExecution`, and
  `ProofFileExecution`; `Replay` is derived from `replay().is_some()`.
  `validate_provider` rejects a provider that declares a run target without
  `execution()`, provides `execution()` without a run target, or provides
  `replay()` without `SuiteExecution`. `descriptor()` lives on
  `BackendProvider`. `BackendRegistry` moves to 4.4.2, where `list` and `get`
  need it. Rationale: §12.1 distinguishes suite execution from raw proof-file
  execution, and §12.6 keeps a Verus provider that can only run proof files out
  of `run --backend auto`; deriving what can be derived removes a class of
  dishonest descriptors. Date/Author: 2026-11-16, planning agent (design
  review).
- D13. Decision: the process adapter supervises on the calling thread with one
  loop: pipes set non-blocking, `rustix::event::poll` with a timeout equal to
  the nearest deadline or a 25 ms cancellation tick, non-blocking reads and the
  protected stdin write inside the loop, and a non-reaping exit check each
  wake. When the run bound expires it closes its pipe ends and returns.
  Rationale: blocking reader or writer threads cannot be interrupted, so a
  descendant holding a pipe open would hang `run` or leak threads, buffers, and
  descriptors. Rejected: reader threads plus a 10 ms `try_wait` poll (the first
  draft); `wait-timeout` (reaps, global `SIGCHLD` handler); Linux `pidfd` (not
  portable to macOS and cannot signal a group in `rustix` 1.1.5, kept as a
  possible later refinement). Date/Author: 2026-11-16, planning agent (design
  review).
- D14. Decision: observe the leader's exit with
  `rustix::process::waitid(WaitId::Pid, EXITED | NOHANG | NOWAIT)`, send every
  group signal while the leader is still an unreaped zombie, reap with
  `Child::wait` last, treat `ESRCH` from a group signal as "already gone", and
  wrap the child in a guard that kills the group and reaps on every early
  return or unwind. Rationale: a reaped leader frees its process and group
  identifiers for reuse, so a later group signal could hit an unrelated process
  group. The adapter requires that nothing else in the process reaps children
  (no `waitpid(-1)` and no ignored `SIGCHLD`); this is `AXIOM-sole-reaper`.
  Date/Author: 2026-11-16, planning agent (design review).
- D15. Decision: `Termination` records how the run ended and the native status
  separately: a `StopCause` (`Completed`, `TimedOut`, `Cancelled`), the native
  `NativeStatus` (`Exited { code }` or `Signalled { signal }`), and an
  `Escalation` (`None`, `Graceful`, `Forced`). The cause is `Completed` exactly
  when the leader's exit was observed before the runner sent any signal;
  otherwise it is the deadline or cancellation that triggered the first signal.
  After `SIGTERM` the runner sends `SIGCONT` to the group so stopped members
  cannot consume the grace period. When the leader exits normally, the runner
  waits up to `drain` for both pipes to close; only if they stay open does it
  terminate the remaining group (`DrainOutcome`). Rationale: preserves native
  status (`INV-native-status`), removes the ambiguity of the first draft's
  `forced` flags, and does not kill daemons after a successful run unless they
  keep the pipes open. Date/Author: 2026-11-16, planning agent (design review).
- D16. Decision: proof-to-production bridge by textual mirror. Each production
  kernel body sits between `// MIRROR-BEGIN <name>` and `// MIRROR-END <name>`
  markers, and the Verus `exec fn` body sits between the same markers. A test
  in the required gate (`crates/theoremc-runner/tests/verus_mirror.rs`) asserts
  the two bodies are equal after normalizing whitespace and comments and
  dropping lines tagged `// ghost` (loop invariants, `proof` blocks, `assert`,
  `decreases`). Oracles in the property and enumeration tests are direct
  transcriptions of the Verus `spec fn`s. Rationale: a name-only check would
  let a kernel change ship while the path-filtered Verus job re-verifies a
  stale mirror. The three links are: mirror against specification (Verus),
  production against specification (oracle tests), and mirror equal to
  production (tripwire). Date/Author: 2026-11-16, planning agent (design review
  condition 1).
- D17. Decision: production kernels use exactly the representations the
  mirror verifies: `Vec<u8>` for the head, and a `Vec<u8>` ring buffer with
  explicit `start` and `len` for the tail; no `VecDeque`. Shape gate: if a
  proof would need a refinement lemma over a different production shape, change
  the production shape instead of adding a bridge. Rationale: removes the
  refinement gap between a `Seq`-level proof and a differently shaped runtime
  structure. Date/Author: 2026-11-16, planning agent (design review condition
  2).
- D18. Decision: the forced-kill path is tested through cancellation of a
  fake tool launched as
  `/bin/sh -c 'trap "" TERM; exec "$0" "$@"' <tool> <mode>`. POSIX keeps an
  ignored signal ignored across `execve`. This is the only shell use, and only
  in tests. Rationale: the fake tool cannot ignore `SIGTERM` without `unsafe`
  or a forbidden dependency. Date/Author: 2026-11-16, planning agent (design
  review).
- D19. Decision: the fake tool's logic lives in
  `theoremc_runner::test_support::fake_tool::run(args) -> ExitCode`; the crate's
  `[[bin]]` target is a three-line shim, and later crates that need a fake
  executable add their own shim. Rationale: `CARGO_BIN_EXE_*` is available only
  within one package, and 4.4.3 and 4.4.4 need fake executables in their own
  tests. Date/Author: 2026-11-16, planning agent (design review).
- D20. Decision: the Verus on-ramp follows the estate standard: pins and a
  README under `tools/verus/`, Make targets delegating through
  `$(PROVER_TOOLS)`, a separate `.github/workflows/verus.yml` (not
  `proofs.yml`, which also triggers Kani on any `crates/**` change), contract
  tests for pins and recipes, and `make verus` kept out of `make all`,
  `make test`, and `make lint`. `PROVER_TOOLS` changes from `?=` to an
  `override` locked to the pinned source, with a command-line-only contract seam
  (`PROVER_TOOLS_CONTRACT_TEST=1 PROVER_TOOLS=<fake>`), because it now drives
  two prover targets. Rationale: the Verus skill's project on-ramp and its
  catalogue of review failures (environment-overridable runner, unpinned
  source, shell wrappers). Date/Author: 2026-11-16, planning agent (design
  review).

## Outcomes & retrospective

Not started. Complete at each milestone and at the end.

## Context and orientation

Inspected base revision: `4c059669c0503ff0cb95764283449e27128e7781`
(`origin/main`). Relocate any symbol with `rg -n '<symbol>'`.

Workspace. The root `Cargo.toml` declares members `crates/test-helpers`,
`crates/theoremc-build-support`, `crates/cargo-theorem`,
`crates/theoremc-core`, and `crates/theoremc-macros`; edition 2024;
`rust-version = "1.89"`; strict `[workspace.lints]` (denies `unwrap_used`,
`expect_used`, `indexing_slicing`, `missing_docs`,
`missing_docs_in_private_items`, `cognitive_complexity`, `print_stdout`,
`print_stderr`, `allow_attributes`, `must_use_candidate`,
`missing_const_for_fn`; `unsafe_code = "forbid"`). `clippy.toml` disallows
`std::env::var`, `var_os`, `vars`, `vars_os`, `set_var`, `remove_var`, and
`set_current_dir`. The toolchain is `nightly-2026-11-16` with `-Zthreads=8` and
`mold`. The root `Cargo.toml` already gates BDD test targets with
`[[test]] ... required-features = ["test-support"]`, the pattern this plan
reuses.

`crates/cargo-theorem` is the only application package; its only command is
`context`, and `CONFIGURABLE_COMMAND_NAMES` in
`crates/cargo-theorem/src/config/mod.rs` reserves `backend` as a configuration
namespace only. This task does not edit `cargo-theorem`.

Existing process code. `src/arg_lowering_rustc_deadline.rs` provides a test-only
`run_within_deadline` that polls `Child::try_wait` every 25 ms and kills the
child on overrun. It has no cancellation, process group, or bounded capture,
and it reaps with `try_wait`. It is a reference, not a reuse candidate. Every
other `std::process::Command` use is in tests.

Dependencies already in `Cargo.lock`: `rustix` 1.1.5, `camino` 1.2.6, `cap-std`
4.0.3, `insta` 1.49.0, `googletest` 0.14.4, `pretty_assertions` 1.4.1,
`proptest` 1.12.0, `rstest` 0.28.0, `rstest-bdd` 0.6.0, `serde` 1.0.229,
`serde_json` 1.0.154, `tempfile` 3.27.0, `thiserror` 2.0.21. Absent: `mockable`,
`mockall`, `nix`, `process-wrap`, `metrics`; `tracing` and `wait-timeout` are
transitive only.

Verification infrastructure. Kani is installed through `make install-kani`
(`$(PROVER_TOOLS) kani install`) and exercised by `make kani` from
`.github/workflows/proofs.yml`; locally the Kani BDD test is known to fail
because of a shadow-loaded `librustc_driver`, so this plan does not rely on
Kani. No Verus proofs, pins, or targets exist. `tools/kani/VERSION` is `0.67.0`;
`tools/rust-prover-tools/REF` pins `592558f7183b554fc749696246984c5d7300d029`.
The newest stable Verus release on 2026-11-16 is `0.2026.11.02.8bf68ee`; its
`x86-linux` asset digest is
`f3a0119292df8855e4435da8130bee1873b0e6a60c306accf81a08ea687bba3c`. Makefile
contract tests already exist in `tests/lint_makefile_contract.rs`, which reads
the Makefile as text; the Verus contract test follows that style.

Prior art from `leynos/rust-prover-tools`, which the contract must host in
4.4.3 and 4.4.4:

- Kani: semantic-version pin; install with
  `cargo install --locked kani-verifier --version V`, then `cargo kani setup`;
  version check runs `<kani-command> --version` and takes the first `N.N.N`
  match from stdout and stderr; the legacy `--kani-command` string is replaced
  by a typed program plus argv (§12.4); proof execution curates
  `LD_LIBRARY_PATH`, `RUSTFLAGS`, `PATH`, and `RUSTUP_TOOLCHAIN`.
- Verus: version and `SHA256SUMS` pins; HTTPS download with a 2 GiB ceiling,
  15 s connection timeout, and 300 s overall deadline; checksum verified before
  extraction; directory normalization; `verus --version` may exit non-zero yet
  report `Toolchain: <name>`; a missing toolchain is installed with
  `rustup toolchain install` only when allowed; one proof file per invocation.
- Cuprum, its command library: typed argv-only programs, explicit execution
  context, its own process group when a timeout applies, `SIGTERM` then grace
  then `SIGKILL`, non-zero exit returned as data.

Terms used in this plan:

- *Port*: a trait owned by the library and implemented by the outside world.
  `CommandRunner` is a driven port.
- *Adapter*: a concrete port implementation, such as `ProcessCommandRunner`.
- *Kernel*: a tiny, pure, dependency-free function whose contract is proved in
  Verus.
- *Mirror*: the Verus copy of a kernel, kept in `verus/`, because Verus
  compiles its own files and cannot import workspace crates.
- *Bookend capture*: retaining the first and last bytes of a stream that
  exceeds its limit.
- *Process group*: a POSIX set of processes signalled together; the runner
  makes each child the leader of a new group.
- *Run bound*: the longest time `run` may take:
  `timeout + grace + 2 × drain + 1 s`.

Signposts. Read before implementing: cargo-theorem-cli-design.md §§5, 8.1, 8.4,
8.6, 11.2 ("Concurrency and task ownership"), 11.3, 11.4, 12, 16, 17, 18;
`docs/developers-guide.md` §3.8 (the backend-integration paragraph) and its
test conventions; `docs/reliable-testing-in-rust-via-dependency-injection.md`;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rust-doctest-dry-guide.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md`; ADR-0006 §10,
ADR-0007 invariants 8 and 10, ADR-0011. `docs/theoremc-design.md` §8 (Kani) and
`docs/theorem-file-specification.md` give backend context but impose no direct
requirement; `docs/ortho-config/users-guide.md` matters only to the 4.4.2
composition root. Skills: `rust-router`, `hexagonal-architecture`,
`rust-types-and-apis`, `rust-errors`, `rust-async-and-concurrency`,
`rust-unit-testing`, `proptest`, `verus`, `rust-verification`, `nextest`,
`arch-decision-records`, `github-stacks`, `execplans`.

## Conformance basis

Upstream artefacts, all at revision `4c059669`:

- `docs/roadmap.md` task 4.4.1 and step 4.4 (requires 4.1.2, complete).
- `docs/cargo-theorem-cli-design.md` (accepted) §§5, 8.1, 8.4, 8.6, 11.2,
  11.3, 11.4, 12, 16, 17, 18.
- `docs/developers-guide.md` §3.8.
- ADR-0006 §10; ADR-0007 invariants 8 and 10; ADR-0009; ADR-0010 (all
  accepted).
- `AGENTS.md` Rust, testing, environment-injection, architecture, and
  observability rules.

There is no separate Terms of Reference; step 4.4's idea statement plays that
role.

Requirements:

- `REQ-capabilities`: providers expose discovery, release resolution, install
  planning, health checks, argument-vector run planning, run-result parsing,
  and replay planning (roadmap 4.4.1; design §12.1).
- `REQ-separation`: lifecycle concerns stay separate from execution and replay
  (developers-guide §3.8; design §12.1).
- `REQ-argv-only`: commands are argument vectors; shell strings are
  inexpressible (roadmap 4.4.1; design §§12.1, 16).
- `REQ-one-runner`: one injected runner owns process creation, timeouts,
  cancellation, bounded capture, and test doubles (roadmap 4.4.1).
- `REQ-protected`: protected inputs bind exactly once; missing, extra,
  duplicate, and unsupported references are rejected before process creation;
  secrets never render or persist (design §§11.3, 12.1, 16, 18).
- `REQ-bounded`: each stream retains at most 16 MiB, lowerable only, and
  records the four accounting fields (design §8.6).
- `REQ-native-status`: native exit codes and signals are preserved as data
  (design §8.4).
- `REQ-owned-termination`: timeouts and cancellation stop the owned process
  group after a bounded grace period, within a stated overall budget (design
  §11.2; AGENTS.md).
- `REQ-no-global`: a fake provider exercises every capability without
  process-global state (roadmap 4.4.1 success criterion).
- `REQ-library-boundary`: no application dependencies in the runner crate
  (design §5; ADR-0009).
- `REQ-capability-declared`: descriptors declare capabilities, including the
  suite-versus-proof-file distinction (design §§12.1, 12.6; ADR-0007 inv. 8).
- `REQ-evidence-status`: parsed outcomes use ADR-0007's six terminal statuses
  with reasons, never a flattened verdict (ADR-0007 inv. 10).
- `REQ-supply-chain`: release sources are HTTPS unless a test-only source is
  injected; downloads carry size and time bounds (design §16).

Gaps: `GAP-no-crate`, `GAP-no-port`, `GAP-no-verus`, and `GAP-illustrative-api`
(design §12.1 is explicitly illustrative).

Trace:

```plaintext
roadmap 4.4.1 -> REQ-capabilities, REQ-separation -> EP-M3, EP-M4 -> backend/*_tests.rs, backend_provider_contract_bdd
roadmap 4.4.1 -> REQ-argv-only -> EP-M1, EP-M5 -> spec_tests, process_runner::argv_round_trip
roadmap 4.4.1 -> REQ-one-runner -> EP-M1, EP-M4, EP-M5 -> runner_contract (both runners)
design 11.3,16,18 -> REQ-protected -> EP-M2, EP-M4, EP-M5 -> verus/protected_binding.rs, runner_contract
design 8.6 -> REQ-bounded -> EP-M1, EP-M2, EP-M5 -> verus/capture_accounting.rs, process_runner::flood
design 8.4 -> REQ-native-status -> EP-M1, EP-M5 -> process_runner::native_status
design 11.2 -> REQ-owned-termination -> EP-M5 -> process_runner::{timeout,cancel,drain}
roadmap 4.4.1 success -> REQ-no-global -> EP-M4 -> backend_provider_contract_bdd, no_global_state
design 5, ADR-0009 -> REQ-library-boundary -> EP-M1 -> dependency_boundary
design 12.1, 12.6 -> REQ-capability-declared -> EP-M3 -> provider_validation_tests
ADR-0007 inv. 10 -> REQ-evidence-status -> EP-M3, EP-M4 -> fake parse_run scenarios
design 16 -> REQ-supply-chain -> EP-M3 -> install_plan_tests
GAP-no-verus -> EP-M0, EP-M2 -> make verus, verus_makefile_contract, verus_mirror
GAP-illustrative-api -> EP-M6 -> design §12.1 rewrite, ADR-0014
```

## Verification plan

### Abstract model

The contract has three independently reasoned parts.

Protected-input binding. A command declares a finite sequence
`D = [d_0 … d_{n-1}]` of protected references, each with a public name and a
delivery mode. The channel supplies a finite sequence of distinct keys
`S = [s_0 … s_{m-1}]` (distinct by construction, because the channel is a map).
Keys are opaque and compared only for equality. `bind(D, S)` returns either a
mapping `p : 0..n -> 0..m` with `S[p(i)] = key(D[i])`, or a fault naming one
witness. Precondition: `S` has no duplicates (a type invariant of the channel).
Binding is otherwise total. Delivery-mode validation (only `Stdin`, at most
one) is a separate, finite check.

Capture accounting. A stream is a finite sequence of chunks `c_0 … c_{k-1}`
whose concatenation is `C`. Limits are natural numbers `h` (head) and `t`
(tail) with `h + t <= L_max = 16 MiB`. After consuming all chunks the account
holds `seen`, `head`, and `tail`:

```plaintext
seen      = min(|C|, u64::MAX)
truncated = |C| > h + t
retained  = C                                   if not truncated
          = C[0 .. h] ++ C[|C| - t .. |C|]        if truncated
bytes_retained = |retained|,  limit_bytes = h + t
```

Process supervision. The adapter is a state machine
`Running -> Terminating -> Killing -> Draining -> Reaping`, plus
`Running -> Draining` on a normal leader exit. Events are leader exit (observed
without reaping), deadline expiry, cancellation, grace expiry, drain expiry,
and pipe end-of-file. The pure decision function
`next_action(state, observations) -> Action` owns every transition; the
operating-system effects are tested end to end.

### Invariants and lemmas

- `INV-bind-exact` (REQ-protected): given distinct `S`, `bind` returns
  `Ok(p)` if and only if keys in `D` are pairwise distinct and
  `set(D) = set(S)`; when it returns `Ok(p)`, `p` is a bijection between `0..n`
  and `0..m` with `S[p(i)] = key(D[i])`.
- `INV-bind-witness` (REQ-protected): each fault carries a genuine witness:
  `DuplicateDeclared { first, second }` has `first < second` and equal keys in
  `D`; `Missing { index }` has `key(D[index])` absent from `S`;
  `Extra { index }` has `S[index]` absent from `D`. Faults are checked in that
  order, so the reported fault is deterministic.
- `INV-bind-before-spawn` (REQ-protected): when request validation fails
  (binding, delivery mode, limits, or cancellation already raised), no process
  is created.
- `INV-capture-accounting` (REQ-bounded): the capture specification holds
  after every prefix of chunks.
- `INV-limits-lower-only` (REQ-bounded): no `CaptureLimits` value exceeds
  16 MiB in total; `ExecutionLimits` requires a finite timeout and caps `grace`
  at 30 s and `drain` at 10 s.
- `INV-argv-preserved` (REQ-argv-only): the child observes exactly
  `[program, argv…]` byte for byte for any UTF-8 argument content, including
  whitespace, quotes, `$`, `;`, `*`, backslashes, and newlines.
- `INV-env-exact` (REQ-no-global): the child observes exactly
  `CommandSpec::environment` and the requested working directory, and a null
  standard input when no protected input is declared.
- `INV-secret-silent` (REQ-protected): for every protected value `v`, the
  bytes of `v` appear in none of the `Debug`, `Display`, or serde renderings or
  error text of any contract type, nor in the child's argv or environment;
  `CommandProvenance` records environment variable names only.
- `INV-native-status` (REQ-native-status): the native exit code or signal is
  reported verbatim in `Termination::status` for every stop cause.
- `INV-cause-precedence` (REQ-native-status): the stop cause is `Completed`
  exactly when the leader's exit was observed before the runner sent a signal.
- `INV-bounded-return` (REQ-owned-termination): `run` returns within the run
  bound for any child behaviour, including descendants that hold pipes open or
  leave the group, and within one cancellation tick plus `grace + drain + 1 s`
  after cancellation.
- `INV-group-reaped` (REQ-owned-termination): after `run` returns from a
  timeout, cancellation, or terminated drain, no live process remains in the
  owned group, and the leader has been reaped.
- `INV-no-stray-signal` (REQ-owned-termination): the runner never signals a
  process group whose leader it has already reaped.
- `INV-capability-honesty` (REQ-capability-declared): `validate_provider`
  accepts a provider only when declared run targets and `execution()` agree and
  replay implies suite execution; every planning method returns
  `BackendError::Unsupported` for an undeclared capability or run target.
- `INV-runner-parity` (REQ-one-runner): `ScriptedCommandRunner` and
  `ProcessCommandRunner` reject exactly the same invalid requests with the same
  errors.
- `INV-isolation` (REQ-no-global): two independent contexts used concurrently
  produce the same results as when used sequentially.
- `INV-https-only` (REQ-supply-chain): an install plan containing a
  `LocalArchive` source fails validation against the production
  `ReleaseSourcePolicy`.

Lemmas (Verus; treat the decomposition as provisional):

- `L-bind-sound`: `bind(D, S) = Ok(p)` implies the bijection property.
- `L-bind-complete`: distinct `D` with `set(D) = set(S)` implies `Ok`.
- `L-bind-faults`: every `Err` satisfies `INV-bind-witness`.
- `L-capture-init`: the empty account meets the specification for the empty
  stream.
- `L-capture-step`: if the account meets the specification for
  `c_0 … c_{k-1}`, it meets it for `c_0 … c_k` after `push(c_k)`. With
  `L-capture-init`, induction yields `INV-capture-accounting`.

### Axioms (trusted, not verified here)

- `AXIOM-execve-argv`: on Unix, `std::process::Command` passes program and
  arguments to `execve` without shell interpretation (Rust standard library
  documentation). Exercised by `INV-argv-preserved` tests.
- `AXIOM-env-clear`: `Command::env_clear` then `envs` yields exactly the
  listed variables (std documentation). Exercised by `INV-env-exact` tests.
- `AXIOM-process-group`: `CommandExt::process_group(0)` makes the child a new
  group leader before `exec`; `rustix::process::kill_process_group` signals
  every member that has not left the group (POSIX `setpgid`, `kill(2)`).
- `AXIOM-pgid-pinned`: a process or group identifier is not reused while an
  unreaped zombie holds it (POSIX). Basis for `INV-no-stray-signal`.
- `AXIOM-sole-reaper`: nothing else in the process calls `waitpid(-1)` or sets
  `SIGCHLD` to `SIG_IGN`. Documented precondition of `ProcessCommandRunner`.
- `AXIOM-group-containment`: Kani, Verus, Cargo, and Rustup descendants
  normally stay in the group; escape is tolerated by `D13` but not contained.
- `AXIOM-sigignore-exec`: an ignored signal stays ignored across `execve`
  (POSIX). Basis for `D18`.
- `AXIOM-sigpipe`: Rust binaries ignore `SIGPIPE`, so writing to a closed pipe
  returns `EPIPE` (Rust runtime documentation).
- `AXIOM-monotonic`: `std::time::Instant` is monotonic.
- `AXIOM-verus`: Verus and Z3 are sound for the proved fragment.

### Methods per obligation

- Obligation: `INV-bind-exact`, `INV-bind-witness` (`L-bind-*`).
  Method: Verus proof of the mirrored kernel; exhaustive enumeration of the
  production kernel against an oracle transcribed from the Verus `spec fn`s;
  named `rstest` cases for readability. Rationale: callers can build
  declaration lists and channels of any length, and the kernel is the security
  gate before spawning; Verus gives the unbounded guarantee, enumeration gives
  exhaustive coverage of small shapes. Domain: all finite `D` and distinct `S`
  (Verus); every `D` with `|D| <= 3` and every distinct `S` with `|S| <= 3`
  over a three-symbol alphabet (enumeration, about 1,600 pairs). Artefact:
  `verus/protected_binding.rs`;
  `crates/theoremc-runner/src/command/binding_kernel.rs` and
  `binding_kernel_tests.rs`. Evidence: `make verus` reports zero errors;
  `cargo nextest run -p theoremc-runner --all-features binding_kernel` passes.
  Non-vacuity: the enumeration counts outcome classes in local variables and
  asserts each class (`Ok`, `DuplicateDeclared`, `Missing`, `Extra`) occurs;
  Verus witness lemmas show each antecedent is inhabited (a two-element
  permutation and one input per fault); a seeded fault (dropping the
  duplicate-declared scan) must fail both `make verus` and the enumeration,
  then be reverted.
- Obligation: `INV-bind-before-spawn`, `INV-runner-parity`.
  Method: one `rstest` runner-contract suite parameterized over both runners.
  Domain: missing, extra, duplicate, unsupported-mode, two-stdin, limits above
  cap, and already-cancelled requests. Artefact:
  `crates/theoremc-runner/tests/runner_contract.rs`. Evidence: identical errors
  from both runners; for the process runner the fake tool's `touch-marker` mode
  leaves no marker. Non-vacuity: a valid request creates the marker.
- Obligation: `INV-capture-accounting` (`L-capture-*`).
  Method: Verus proof of the mirrored account; property test of the production
  account against an oracle transcribed from the specification; process-level
  flood tests. Domain: all chunk sequences and limits (Verus); generated chunk
  sequences with totals below, equal to, one above, and far above `h + t`, with
  small limits including `h = 0` and `t = 0` (proptest, 512 cases). Artefact:
  `verus/capture_accounting.rs`;
  `crates/theoremc-runner/src/command/capture_kernel.rs` and tests.
  Non-vacuity: a manual `TestRunner` loop classifies cases into local counters
  and asserts the equality and one-over boundaries both occur; a seeded
  off-by-one in tail eviction must fail both proof and property test.
- Obligation: `INV-limits-lower-only`.
  Method: `rstest` table at, below, and above each cap.
- Obligation: `INV-argv-preserved`, `INV-env-exact`.
  Method: property test spawning the fake tool's `echo-context` mode, which
  writes argv, environment, working directory, and whether stdin is null as
  NUL-separated records. Domain: up to eight UTF-8 arguments of up to 32 bytes
  from an alphabet weighted towards shell metacharacters and whitespace; 64
  cases with `max_shrink_iters = 64`. Non-vacuity: a named case with `"a b"`,
  `"$(touch x)"`, and `"; exit 1"` arrives intact and creates no `x`.
- Obligation: `INV-secret-silent`.
  Method: property test over random 16-byte secrets placed in declarations,
  channels, outputs, and errors, checking `Debug`, `Display`, and `serde_json`;
  `insta` snapshots of redacted renderings (see test strategy); process test
  that the secret reaches the child's stdin and nowhere else. Non-vacuity: a
  negative control renders an unredacted copy in the test module and shows the
  check fails.
- Obligation: `INV-native-status`, `INV-cause-precedence`.
  Method: `rstest` table running the fake tool with exit codes 0, 1, 2, and 101
  and self-signalling with `SIGTERM` and `SIGKILL`; timeout and cancellation
  cases assert the native status is carried.
- Obligation: supervision decisions.
  Method: exhaustive `rstest` table over the Cartesian product of state ×
  {leader exited, not} × {cancelled, not} × {before, at, after} for each
  relevant deadline × {pipes open, closed}; proptest that a later clock never
  un-expires a deadline. Rationale: the function is finite, so enumeration is
  exhaustive; Kani is not used (see the formal verification decision).
- Obligation: `INV-bounded-return`, `INV-group-reaped`, `INV-no-stray-signal`.
  Method: process integration tests: a cooperative sleeper (graceful timeout);
  the `D18` `SIGTERM`-ignoring sleeper cancelled after its readiness file
  appears (forced path); a leader that exits while a grandchild holds stdout
  open (drain path); a grandchild that leaves the group with `setsid` via the
  test-only shell wrapper (bounded return without containment). Evidence: `run`
  returns within its documented bound plus 5 s slack; a process counts as gone
  when `/proc/<pid>` is absent, is in state `Z`, or has a different start time
  than recorded; no non-zombie process in `/proc` carries the owned group
  identifier. Non-vacuity: each test first asserts the grandchild is alive.
- Obligation: `INV-capability-honesty`.
  Method: `rstest` cases with honest and dishonest providers defined in the
  test module, and a case per undeclared run target.
- Obligation: `INV-isolation`, `REQ-no-global`.
  Method: the BDD scenario running two fake providers with different
  environments on two threads, compared with sequential runs; a fitness test
  scanning `crates/theoremc-runner/src` (excluding `*_tests.rs` and comments)
  for `std::env`, `static mut`, `thread_local!`, `OnceLock`, `LazyLock`,
  `set_current_dir`, and `std::process` outside `src/process/` and the fake
  tool.
- Obligation: `INV-https-only`.
  Method: `rstest` table over plans with HTTPS, HTTP, and local sources under
  the production and test policies.

### Test strategy notes

Assertions use `googletest` matchers for structured values (`matches_pattern!`
for `Termination`, `BindingFault`, and `BackendError`; `elements_are!` for
recorded command lists) and `pretty_assertions::assert_eq` for whole
`CommandSpec` and `InstallPlan` comparisons. `insta` snapshots cover outputs
whose shape is a contract consumed later: `CommandProvenance` JSON, redacted
`Debug` of a spec with a protected declaration, `Display` of every
`CommandError` and `BackendError` variant, the fake provider's `InstallPlan`,
the success scenario's recorded command list, and `BackendDescriptor` JSON
(which `backend get` consumes in 4.4.2). Snapshots contain no paths from the
host: tests build specs under fixed absolute fixture paths such as `/opt/fake`.
Every assertion carries a diagnostic message. No end-to-end CLI test is added,
because no command-line behaviour changes; the process-level tests are the
end-to-end tests for the runner boundary.

### Formal verification decision

Verus is warranted for two kernels:

1. Protected-input binding is the security gate before a process starts: a
   wrong answer runs a command without its credential or binds a secret to the
   wrong declaration. Callers may build declaration lists and channels of any
   length, so the property is unbounded even though `D7` limits stdin delivery
   to one input.
2. Bookend capture accounting is the contract behind four persisted fields in
   design §8.6 and behind every parser's view of truncated output; it is an
   inductive property over unbounded chunk sequences.

Verus is not warranted for the supervision state machine (its risk lies in
operating-system behaviour, which a model proof would not discharge; the finite
decision function is enumerated exhaustively and the effects are tested end to
end), value-type validation (finite partitions), argv preservation (a platform
axiom tested end to end), or provider validation (finite). Kani is not used:
the repository runs Kani only for generated harnesses, the local Kani test
environment is known to be broken, and the interesting kernel cases need
unbounded lengths.

The bridge between proof and production is `D16` and `D17`. Tests continue to
own every observable behaviour; the proofs own only the two semantic
properties, and proof obligations that shaped the API (binding faults in
declaration order, the channel-as-map precondition, the bookend representation)
are recorded in design §12.1 and ADR-0014, not only in `verus/`.

### Suggested BDD features

Path:
`crates/theoremc-runner/tests/features/backend_provider_contract.feature`, run
by `crates/theoremc-runner/tests/backend_provider_contract_bdd.rs`
(`rstest-bdd` 0.6.0, `required-features = ["test-support"]`).

```gherkin
Feature: Backend provider contract
  Providers expose every backend capability through injected ports only.

  Background:
    Given a fake backend registered as "fake"
    And a scripted command runner
    And an injected environment with "FAKE_HOME" set to "/opt/fake"

  Scenario: A fake provider exercises every capability without process-global state
    When the caller exercises each capability in order
    Then each capability produces the expected outcome:
      | capability   | outcome                        | recorded command      |
      | discover     | installation at /opt/fake      | /opt/fake/bin/fake    |
      | resolve      | release 1.2.3                  |                       |
      | plan_install | download, extract, normalize   |                       |
      | check        | healthy                        | /opt/fake/bin/fake    |
      | prepare      | toolchain step                 |                       |
      | plan_run     | proof-file command             |                       |
      | parse_run    | one target proved              |                       |
      | plan_replay  | playback command               |                       |
      | parse_replay | playback artefact              |                       |
    And every recorded command uses an absolute program and the injected environment

  Scenario: Independent providers do not share state
    Given a second fake backend with "FAKE_HOME" set to "/srv/other"
    When both providers discover their installations concurrently
    Then each installation reports its own home directory

  Scenario: Discovery reports a missing installation
    Given the scripted runner cannot start the discovery probe
    When the caller discovers the installation
    Then discovery fails with error code "backend.not_installed"

  Scenario: A timed-out run is parsed as undetermined
    Given the scripted runner stops the run on timeout after partial output
    When the caller parses the run output
    Then the target outcome is "undetermined" with reason "timed_out"

  Scenario: A command with a missing protected input is rejected before any process is created
    Given a run plan that declares the protected input "token"
    And a protected input channel without "token"
    When the caller submits the command
    Then the runner rejects it with fault "missing" naming "token"
    And the scripted runner recorded no execution

  Scenario: Replay is unavailable for a provider without replay capability
    Given a fake backend registered as "no-replay" without replay capability
    When the caller asks for the replay capability
    Then no replay capability is available
    And the descriptor does not declare replay
```

Links: `REQ-no-global`, `REQ-capabilities`, `REQ-evidence-status`,
`REQ-protected`, `REQ-capability-declared`; milestone `EP-M4`.

## Plan of work

### `EP-M0` feasibility probes (prototype, scratch deleted afterwards)

Questions: can the pinned `rust-prover-tools` install and run Verus; does the
shared `install-prover-tools` action accept `tools: verus`; and do the needed
`rustix` APIs (`process::kill_process_group`, `process::waitid` with
`WaitIdOptions::NOWAIT`, `event::poll`, non-blocking pipe configuration) exist
in 1.1.5 with the features chosen, compile at Rust 1.89, and need no `unsafe`?

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

2. Add `.verus/` to `.gitignore`, add the Make targets from `Concrete steps`,
   then install Verus and verify a trivial lemma kept under `/tmp`:

   ```bash
   make install-verus
   make verus VERUS_PROOF_FILE=/tmp/verus-probe/probe.rs
   ```

3. Read the `install-prover-tools` action at the commit pinned in
   `.github/workflows/proofs.yml` and record whether it supports Verus.
4. In a scratch crate under `/tmp`, exercise the four `rustix` APIs and run
   `cargo +1.89 check`.

Go: all succeed; keep the pins, ignore entry, and targets for `EP-M2`. No-go on
Verus: bump `tools/rust-prover-tools/REF` to a commit that supports
`verus install` and retry once, else stop. No-go on the action: the workflow
uses `make install-verus`. No-go on `rustix`: stop and escalate. Record
transcripts in `Artefacts and notes`.

### `EP-M1` crate skeleton, value types, limits, validation, and redaction

Assigned: `REQ-library-boundary`, `REQ-argv-only`, `REQ-bounded`,
`REQ-native-status`, `INV-limits-lower-only`, `INV-secret-silent`,
`GAP-no-crate`.

Create `crates/theoremc-runner/` (see `Interfaces and dependencies`) and add it
to the root `[workspace] members`. Implement the `command` module's value types
with private fields and validating constructors, a handwritten redacting
`Debug` for every type that can hold a `ProtectedValue`, and
`CommandRequest::validate`.

Red: `src/command/limits_tests.rs`, `spec_tests.rs`, `request_tests.rs`,
`redaction_tests.rs` (property test, negative control, snapshots), and
`tests/dependency_boundary.rs` (reads the crate manifest through `cap_std` and
fails if it names `ortho_config`, `cargo-orthohelp`, `clap`, or
`cargo-theorem`). `cargo nextest run -p theoremc-runner --all-features` fails
to compile. Green: implement. Refactor; run the gates.

### `EP-M2` binding and capture kernels with Verus proofs

Assigned: `INV-bind-exact`, `INV-bind-witness`, `INV-capture-accounting`,
`L-bind-*`, `L-capture-*`, `GAP-no-verus`.

Proof sketch first: write `verus/runner_contract.rs` (shared definitions and
`mod` declarations), `verus/protected_binding.rs`, and
`verus/capture_accounting.rs` with the `spec fn`s and lemma statements only; run
`make verus` and record the open goals (the red state for proofs). Then:

1. Red: `binding_kernel_tests.rs` (enumeration and named cases) and
   `capture_kernel_tests.rs` (property test and named cases), with oracles
   transcribed from the `spec fn`s; they fail to compile.
2. Green: implement `binding_kernel::bind_keys` (nested scans in fault order)
   and `capture_kernel::CaptureAccount::{new, push, finish}` with the `D17`
   representation and a saturating `u64` counter, between `MIRROR` markers.
3. Copy each marked body into the Verus `exec fn` and add only `// ghost`
   lines; discharge the lemmas. Discover proof plumbing (triggers, sequence
   broadcasts, auxiliary lemmas) as verification conditions demand. Shape gate
   (`D17`): if a lemma would need a refinement over a different production
   shape, change the production shape.
4. Run the seeded-fault controls; revert them.
5. Add `tests/verus_mirror.rs` (crate), `tests/verus_makefile_contract.rs`
   (root: non-empty pins; checksum row names the configured archive;
   `install-verus` and `verus` delegate through `$(PROVER_TOOLS)` with no
   `curl`, `unzip`, `sha256sum`, or `rustup toolchain install`; a dry run with
   `PROVER_TOOLS_CONTRACT_TEST=1 PROVER_TOOLS=<recording fake>` on the make
   command line records the exact
   `verus run --repo-root . --proof-file verus/runner_contract.rs` invocation),
   `tools/verus/README.md`, and `.github/workflows/verus.yml` (pull requests
   touching `verus/**`, `tools/verus/**`, `tools/rust-prover-tools/**`,
   `crates/theoremc-runner/src/command/*_kernel.rs`, or the workflow; weekly;
   `workflow_dispatch`).
6. Change `PROVER_TOOLS` to the `override` form with the command-line contract
   seam (`D20`), and confirm `make -n install-kani` still expands to the pinned
   source.
7. Wire `ProtectedInputChannel` binding and `CapturedStream` construction onto
   the kernels in the same milestone, so no kernel is dead code.

Open the stacked PR A at the end of this milestone.

### `EP-M3` provider traits, contexts, and provider validation

Assigned: `REQ-capabilities`, `REQ-separation`, `REQ-capability-declared`,
`REQ-evidence-status`, `REQ-supply-chain`, `INV-capability-honesty`,
`INV-https-only`.

Create the `backend` module with the types and traits in
`Interfaces and dependencies`. Red: `src/backend/provider_validation_tests.rs`,
`install_plan_tests.rs`, and `outcome_tests.rs` with in-test providers. Green:
implement `validate_provider`, `InstallPlan::validate`, and the outcome types.

### `EP-M4` scripted runner, fake provider, contract suite, success scenario

Assigned: `REQ-no-global`, `REQ-one-runner` (doubles), `INV-isolation`,
`INV-runner-parity` (scripted half).

Create `src/test_support/` (`#[cfg(feature = "test-support")]`):
`ScriptedCommandRunner` (expected-command matchers with scripted
`Result<CommandOutput, CommandError>` replies; calls `CommandRequest::validate`
first; records each executed `CommandSpec`; `verify()` returns `Result` listing
unmatched or leftover expectations), `FakeBackend` (all capabilities; discovery
probes `<FAKE_HOME>/bin/fake --version`; preparation emits a toolchain step
when the health report says the toolchain is missing; `parse_run` maps
termination and a simple line protocol on stdout to `TargetOutcome`s; replay
plans and parses a playback command whose artefact is a file under the run
directory), and `FakeBackend::without_replay`.

Red: the feature file and `tests/backend_provider_contract_bdd.rs`,
`tests/runner_contract.rs` (scripted runner only for now), and
`tests/no_global_state.rs`. Green: implement the doubles. The roadmap success
criterion is met when the first scenario passes. Open stacked PR B.

### `EP-M5` `ProcessCommandRunner` production adapter

Assigned: `REQ-one-runner`, `REQ-owned-termination`, `REQ-protected` (delivery),
`REQ-bounded` (pumping), `INV-argv-preserved`, `INV-env-exact`,
`INV-native-status`, `INV-cause-precedence`, `INV-bounded-return`,
`INV-group-reaped`, `INV-no-stray-signal`, `INV-runner-parity` (process half).

Create `src/process/` (`#[cfg(unix)]`): `mod.rs` (`ProcessCommandRunner`,
`impl CommandRunner`), `spawn.rs` (`env_clear`, `envs`, `current_dir`,
`process_group(0)`, piped stdout and stderr, piped or null stdin; non-blocking
pipe ends), `guard.rs` (owns `Child` and group identifier; kills and reaps on
drop), `supervise.rs` (the `D13` poll loop; reads 64 KiB chunks into each
`CaptureAccount`; writes the protected input; records pipe errors on the
stream), `decide.rs` (pure `next_action`), and `signal.rs`
(`kill_process_group` with `TERM`, `CONT`, and `KILL`, mapping `ESRCH` to
"gone"; `waitid` with `NOWAIT`). Add `src/test_support/fake_tool.rs` and the
`src/bin/fake_tool.rs` shim (`[[bin]] name = "theoremc-runner-fake-tool"`,
`required-features = ["test-support"]`) with modes `echo-context`,
`exit <code>`, `signal <name>`, `sleep <ms>`, `ready-then-sleep <file> <ms>`,
`flood <stream> <bytes>`, `spawn-lingering-grandchild <pid-file>`,
`read-stdin`, and `touch-marker <path>`. Every sleeping mode exits after at
most 30 s and when its parent process changes.

Red: `tests/process_runner.rs` with submodules under `tests/process_runner/`,
and the process half of `tests/runner_contract.rs`; declare both test targets
with `required-features = ["test-support"]`; add the `process-runner` nextest
group (`max-threads = 2`, filter
`binary(=process_runner) | binary(=runner_contract)`). Green: implement the
adapter. Refactor until every function is within the Clippy thresholds. Open
stacked PR C when `EP-M6` is also complete.

### `EP-M6` documentation, ADR, and roadmap closure

Assigned: `GAP-illustrative-api`.

1. Write
   `docs/adrs/adr-0014-backend-provider-contract-and-command-runner-port.md`
   recording `D1` to `D20`, the Verus decision, the sans-I/O alternative, the
   in-memory capture to file-sink path, the `SIGINT` responsibility of the
   composition root, the parent-death residual risk, and the crate's unstable
   pre-1.0 status.
2. Rewrite cargo-theorem-cli-design.md §12.1 to describe the delivered
   traits, contexts, `CommandRequest`, `Termination`, bookend capture, run
   bound, environment policy, protected-input binding semantics, and the proof
   obligations that shaped them, citing ADR-0014.
3. Update `docs/developers-guide.md` §3.8 with the `theoremc-runner`
   component architecture and reuse policy, the Verus proof inventory and
   mirror rule, `make install-verus` and `make verus`, the `process-runner`
   nextest group, and the fake-tool shim pattern; correct the `rstest-bdd`
   version.
4. Update `docs/repository-layout.md` (new crate, `verus/`, `tools/verus/`)
   and `docs/contents.md` (ADR-0014).
5. Leave `docs/users-guide.md` and `docs/cargo-theorem-cli-users-guide.md`
   unchanged, because no user-facing behaviour changes; record that in
   `Outcomes & retrospective`.
6. Tick roadmap 4.4.1 only after every acceptance item passes.

Stale-claim search, excluding completed ExecPlans:

```bash
rg -n 'illustrative rather than a frozen|CommandRunner|theoremc-runner' docs \
  --glob '!docs/execplans/*'
```

## Milestones and plateaus

- `EP-M0`: probe results recorded; pins, ignore entry, and targets kept, or
  the plan blocked. Recovery: delete scratch under `/tmp`. Compatibility: none.
- `EP-M1`: the crate builds; value types validate; secrets cannot render.
  Acceptance: the three gates and the new tests. Conformance: no application
  dependency. Recovery: revert the milestone commit. Compatibility: none.
- `EP-M2`: kernels proved and tested; tripwire and on-ramp contract tests
  green. Acceptance: the three gates plus `make verus`; the forbidden-construct
  search below prints nothing. Recovery: revert. Compatibility: none. End of PR
  A.
- `EP-M3`: traits, contexts, and validation tested with in-test providers.
  Acceptance: the three gates. Compatibility: none.
- `EP-M4`: roadmap success criterion met. Acceptance: the three gates and the
  BDD scenarios. Compatibility: none. End of PR B.
- `EP-M5`: production adapter meets every process obligation. Acceptance: the
  three gates. Recovery: revert; the port and fakes remain valid.
  Compatibility: none.
- `EP-M6`: documents reconciled; roadmap ticked. Acceptance: `make fmt`,
  `make markdownlint`, `make nixie`, `make spelling`, the three gates, and
  `make verus`. Compatibility: none. End of PR C.

At each boundary record: requirements satisfied, design still followed,
upstream assumptions valid, no unapproved interface or dependency, no trust
boundary or persisted-format change, and trace links current.

## Concrete steps

Working directory: the repository root. Use `set -o pipefail` and log each
gate; run gates sequentially, delegating full runs to the `scrutineer` agent:

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

`make verus` applies from `EP-M2`. Forbidden-construct search for proofs:

```bash
rg -n '\b(assume|admit)\s*\(|external_body' verus/
```

Focused commands:

```plaintext
Red:   cargo nextest run -p theoremc-runner --all-features <filter>  -> compile error or failing assertion
Green: cargo nextest run -p theoremc-runner --all-features <filter>  -> selected tests pass
Proof: make verus                                                    -> "verification results:: N verified, 0 errors"
```

Planned Makefile changes (recipes use tabs):

```makefile
override PROVER_TOOLS_SOURCE := git+https://github.com/leynos/rust-prover-tools@$(PROVER_TOOLS_REF)
ifeq ($(origin PROVER_TOOLS_CONTRACT_TEST),command line)
else
override PROVER_TOOLS := $(UV) tool run --python 3.14 --from $(PROVER_TOOLS_SOURCE) prover-tools
endif
VERUS_PROOF_FILE ?= verus/runner_contract.rs
VERUS_FLAGS ?=

install-verus: ## Install the pinned Verus release
	$(PROVER_TOOLS) verus install --repo-root .

verus: ## Verify the runner kernels with Verus
	$(PROVER_TOOLS) verus run --repo-root . --proof-file "$(VERUS_PROOF_FILE)" $(VERUS_FLAGS)
```

`make verus` stays out of `make all`, `make test`, and `make lint`, like
`make kani`. Stacked pull requests use the `github-stacks` skill. Commit after
each milestone, gated as above, using the repository's commit-message format
and attribution line.

## Validation and acceptance

- `cargo nextest run -p theoremc-runner --all-features` passes; the scenario
  "A fake provider exercises every capability without process-global state"
  fails before `EP-M4` and passes after.
- `make verus` reports every lemma verified with zero errors; the
  forbidden-construct search prints nothing; `verus_mirror` fails if a marked
  production body is edited without its mirror (checked once by a temporary
  edit, then reverted).
- Forced cancellation: the `SIGTERM`-ignoring fake tool, cancelled after its
  readiness file appears, with 2 s grace, yields `StopCause::Cancelled`,
  `Escalation::Forced`, and `NativeStatus::Signalled { signal: 9 }` within 10
  s; its grandchild is gone.
- Drain: a leader that exits 0 while a grandchild holds stdout open, with
  1 s drain, yields `StopCause::Completed`, `NativeStatus::Exited { code: 0 }`,
  and `DrainOutcome::DescendantsTerminated` within 6 s.
- Flood: `flood stdout 20000000` with run defaults yields
  `bytes_seen = 20000000`, `bytes_retained = 16777216`,
  `limit_bytes = 16777216`, `truncated = true`, and retained bytes equal to the
  first 4 MiB and last 12 MiB of the generated stream.
- Binding: a missing protected input yields
  `CommandError::ProtectedInput(BindingFault::Missing { name: "token" })` from
  both runners, and the marker file is absent.
- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, and `make spelling` pass.

Quality criteria: tests as above; verification obligations discharged as in
`Methods per obligation`; no Clippy, rustdoc, or Whitaker findings; no
performance target beyond the run bound; security review through the
`INV-secret-silent`, `INV-https-only`, and `INV-no-stray-signal` evidence.

## Idempotence and recovery

Each milestone is one or more commits that can be reverted alone; the stack can
be rebased with `gh stack`. `make install-verus` reuses an existing
installation. Process tests create files only under `tempfile` directories, and
fake tools exit on their own within 30 s. Seeded faults and the tripwire check
are temporary edits: revert them with `git restore <file>` and delete any
`proptest-regressions` file they create.

## Artefacts and notes

Reconnaissance (2026-11-16, revision `4c059669`): the Verus release query,
`Cargo.lock` survey, `rust-prover-tools` behaviour summary, and the expert
panel's findings are reflected above. Probe transcripts and red, green, and
seeded-fault evidence are added here during implementation.

## Interfaces and dependencies

New package `crates/theoremc-runner`: edition 2024, `rust-version = "1.89"`,
`[lints] workspace = true`, `features = { test-support = [] }`, published with
its siblings and documented as unstable.

Dependencies (caret requirements):

- `thiserror` (workspace), `camino = "1.2.2"`, `serde` (workspace) for
  `CommandProvenance` and descriptor serialization.
- Unix-only `rustix` under `[target.'cfg(unix)'.dependencies]` as
  `rustix = { version = "1.1.5", features = ["process", "event", "pipe"] }`,
  adjusted to the exact feature set `EP-M0` confirms.
- Dev: `rstest = "0.28.0"`, `rstest-bdd = "0.6.0"`,
  `rstest-bdd-macros = "0.6.0"`, `googletest = "0.14.3"`,
  `pretty_assertions = "1.4.1"`, `proptest = "1.11.0"`, `insta = "1.47.2"`,
  `serde_json` (workspace), `tempfile = "3.27.0"`,
  `cap-std = { version = "4.0.2", features = ["fs_utf8"] }`.

Module layout (each file under 400 lines):

```plaintext
crates/theoremc-runner/src/
  lib.rs
  command/{mod, spec, environment, protected, binding_kernel, capture_kernel,
           limits, request, output, cancel, runner, error, provenance}.rs
  backend/{mod, identity, descriptor, context, request, release, install,
           health, outcome, lifecycle, execution, replay, provider, error}.rs
  process/{mod, spawn, guard, supervise, decide, signal}.rs
  test_support/{mod, scripted_runner, fake_backend, fake_tool}.rs
  bin/fake_tool.rs
```

Signatures that must exist at the end. Names and inputs are normative; private
fields may grow if recorded in `Decision log`.

```rust
pub trait CommandRunner: Send + Sync {
    fn run(&self, request: &CommandRequest<'_>) -> Result<CommandOutput, CommandError>;
}

/// Built with `CommandRequest::new(&spec)` and `with_inputs`, `with_limits`,
/// `with_cancellation`; later tasks add sink and spawn-observer setters.
pub struct CommandRequest<'a> { /* spec, inputs, limits, cancellation */ }
impl CommandRequest<'_> {
    pub fn validate(&self) -> Result<BoundRequest<'_>, CommandError>;
}

pub struct CommandSpec {
    program: ProgramPath,                       // absolute, UTF-8
    argv: Vec<PublicArgument>,                  // UTF-8, public only
    working_directory: Utf8PathBuf,             // absolute
    environment: CommandEnvironment,            // the child's whole environment
    protected_inputs: Vec<ProtectedInputDeclaration>, // name + DeliveryMode
}

pub struct Termination {
    pub cause: StopCause,         // Completed | TimedOut | Cancelled
    pub status: NativeStatus,     // Exited { code } | Signalled { signal }
    pub escalation: Escalation,   // None | Graceful | Forced
}

pub struct CommandOutput {
    pub termination: Termination,
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
    pub elapsed: Duration,
    pub delivery: DeliveryOutcome, // NotRequested | Written | Incomplete { kind } | Abandoned
    pub drain: DrainOutcome,       // Clean | DescendantsTerminated | PipesAbandoned
}

impl CapturedStream {
    pub fn bytes_seen(&self) -> u64;
    pub fn bytes_retained(&self) -> u64;
    pub fn limit_bytes(&self) -> u64;
    pub fn is_truncated(&self) -> bool;
    pub fn head(&self) -> &[u8];
    pub fn tail(&self) -> Vec<u8>;
    pub fn read_error(&self) -> Option<io::ErrorKind>;
}

pub trait BackendProvider: Send + Sync {
    fn descriptor(&self) -> &BackendDescriptor;
    fn lifecycle(&self) -> &dyn BackendLifecycle;
    fn execution(&self) -> Option<&dyn BackendExecution>;
    fn replay(&self) -> Option<&dyn BackendReplay>;
    fn capabilities(&self) -> CapabilitySet; // provided: declared + derived
}

pub trait BackendLifecycle: Send + Sync {
    fn discover(&self, probe: &ProbeContext<'_>, request: &DiscoveryRequest)
        -> Result<Installation, BackendError>;
    fn resolve(&self, request: &VersionRequest) -> Result<ResolvedRelease, BackendError>;
    fn plan_install(&self, request: &InstallRequest<'_>) -> Result<InstallPlan, BackendError>;
    fn check(&self, probe: &ProbeContext<'_>, installation: &Installation)
        -> Result<HealthReport, BackendError>;
    fn plan_preparation(&self, request: &PreparationRequest<'_>)
        -> Result<InstallPlan, BackendError>;
}

pub trait BackendExecution: Send + Sync {
    fn plan_run(&self, planning: &PlanningContext<'_>, installation: &Installation,
        request: &BackendRunRequest) -> Result<RunPlan, BackendError>;
    fn parse_run(&self, request: &BackendRunRequest, output: &CommandOutput)
        -> Result<BackendRunResult, BackendError>;
}

pub trait BackendReplay: Send + Sync {
    fn plan_replay(&self, planning: &PlanningContext<'_>, installation: &Installation,
        request: &ReplayRequest) -> Result<RunPlan, BackendError>;
    fn parse_replay(&self, request: &ReplayRequest, output: &CommandOutput)
        -> Result<ReplayResult, BackendError>;
}

pub fn validate_provider(provider: &dyn BackendProvider) -> Result<(), ProviderContractError>;
```

Supporting shapes (sketches; field names are normative, visibility is private
unless shown):

```rust
pub struct PlanningContext<'a> {
    environment: &'a EnvironmentSnapshot,
    host: HostTarget,
    workspace_root: &'a Utf8Path,
    sources: &'a ReleaseSourcePolicy,
}
// Holds &PlanningContext, &dyn CommandRunner, &CancellationSignal, and probe
// ExecutionLimits (default 30 s timeout, 64 KiB capture).
pub struct ProbeContext<'a> { /* ... */ }

// The typed Kani command override and Verus configured binary (§§12.4-12.5).
pub struct DiscoveryRequest {
    configured_program: Option<ProgramPath>,
    managed_root: Option<Utf8PathBuf>,
}

// Exact versions only; "latest" is a future ReleaseIndex port (4.4.2).
pub struct VersionRequest { version: BackendVersion, checksums: ChecksumPins }

pub struct ResolvedRelease {
    backend: BackendId,
    version: BackendVersion,
    artefacts: BTreeMap<HostTarget, ReleaseArtefact>,
    required_toolchain: Option<ToolchainName>,
}
pub struct ReleaseArtefact { source: ReleaseSource, sha256: Sha256Digest }
pub enum ReleaseSource {
    Https(HttpsUrl),
    LocalArchive(Utf8PathBuf),
    CargoCrate { name: CrateName, version: BackendVersion },
}

pub struct InstallRequest<'a> {
    release: &'a ResolvedRelease,
    planning: &'a PlanningContext<'a>,
    staging_root: &'a Utf8Path,
}
pub struct PreparationRequest<'a> {
    installation: &'a Installation,
    health: &'a HealthReport,
    planning: &'a PlanningContext<'a>,
    policy: PreparationPolicy,
}

// Step paths are relative to the staging root.
pub struct InstallPlan(Vec<InstallStep>);
impl InstallPlan {
    pub fn validate(&self, sources: &ReleaseSourcePolicy) -> Result<(), BackendError>;
}
pub enum InstallStep {
    Download { artefact: ReleaseArtefact, limits: DownloadLimits },
    Extract { archive: Utf8PathBuf, destination: Utf8PathBuf },
    Normalize { expected_root: Utf8PathBuf },
    Command(CommandSpec),
}
pub struct DownloadLimits { max_bytes: u64, connect_timeout: Duration, total_timeout: Duration }

pub struct HealthReport {
    probes: Vec<HealthProbe>,
    required_toolchain: Option<RequiredToolchain>, // { name, present }
    status: HealthStatus,
}

pub struct BackendRunRequest {
    target: RunTarget,
    arguments: Vec<BackendArgument>,
    limits: ExecutionLimits,
}
pub enum RunTarget { Suite { harnesses: Vec<HarnessName> }, ProofFile(Utf8PathBuf) }
pub enum BackendArgument {
    Public { name: ArgumentName, value: PublicArgument },
    ProtectedSecret { name: ProtectedInputName, input_ref: ProtectedInputRef },
}
pub struct RunPlan { command: CommandSpec, artefacts: Vec<ExpectedArtefact> }

pub struct BackendRunResult { outcomes: Vec<TargetOutcome>, partial: bool }
pub struct TargetOutcome {
    native_target: NativeTarget,
    status: ProofStatus,
    reason: Option<StatusReason>,
}
// ADR-0007 terminal statuses.
pub enum ProofStatus { Proved, Falsified, Unreachable, Undetermined, Unsupported, ExecutionError }
pub enum StatusReason { ResourceExhaustion, TimedOut, Cancelled, OutputTruncated }

// thiserror; large payloads boxed; closed reason enums; truncation is a
// partial result, never an error.
pub enum BackendError {
    Unsupported { capability: Capability },
    NotInstalled { reason: NotInstalledReason },
    InvalidRequest { reason: InvalidRequestReason },
    UnsupportedHost { host: HostTarget },
    Interrupted { cause: StopCause },
    UnexpectedOutput { reason: OutputReason },
    Command(Box<CommandError>),
}
impl BackendError {
    pub fn code(&self) -> BackendErrorCode; // stable dotted strings, e.g. backend.not_installed
}

// Faults carry the public ProtectedInputName, never the reference. Error text
// names programs by file name only, never argument or environment values.
pub enum CommandError {
    InvalidRequest { reason: InvalidRequestReason },
    ProtectedInput(BindingFault),
    UnsupportedDelivery { name: ProtectedInputName },
    CancelledBeforeStart,
    Spawn { kind: io::ErrorKind, raw_os_error: Option<i32> },
    Signal { kind: io::ErrorKind },
    Reap { kind: io::ErrorKind },
}
```

Reuse policy (copied into the developers' guide): `theoremc-runner` owns every
backend process start. Providers depend on the `backend` and `command` modules
and never import `process` or `std::process`. The composition root (the
`cargo-theorem` binary, from 4.4.2) builds `ProcessCommandRunner`,
`EnvironmentSnapshot`, `ReleaseSourcePolicy::https_only()`, and the provider
set once, maps `SIGINT` and `SIGTERM` to the shared `CancellationSignal`, and
passes references down. Other crates use `test_support` doubles only from tests
and obtain fake executables through their own `fake_tool` shim. A second
process-spawning abstraction in production code anywhere in the workspace
violates this design.

## Revision note

- 2026-11-16: initial draft from reconnaissance.
- 2026-11-16: applied the expert panel review (structure and contracts,
  failure modes and cost, alternatives and viability). Main changes: planning
  and probe contexts; a preparation step and richer install steps; finer,
  partly derived capabilities with the registry deferred to 4.4.2; per-target
  outcomes using ADR-0007 statuses and replay parsing; a `Termination` that
  preserves native status; a single-thread, non-reaping supervision loop (`D13`,
  `D14`); the textual mirror tripwire and shared kernel representations (`D16`,
  `D17`, the review's two blocking conditions); the shared request validation
  and runner-contract suite; the Verus on-ramp (`D20`); UTF-8 arguments and
  name-only environment provenance; injected release-source policy; three
  stacked pull requests. Budgets rose before approval (code 4,600 to 5,600;
  configuration 250 to 300; documentation 900 to 1,050; files 70 to 90).
  Milestone order is unchanged except that the registry left `EP-M3`.
