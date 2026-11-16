# Task 4.1.4: versioned JSON application contracts for `cargo theorem`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (design review applied; awaiting user approval)

Branch: `4-1-4-versioned-json-application-contracts`

Roadmap item: `4.1.4` in `docs/roadmap.md`.

## Purpose / big picture

`cargo theorem` is theoremc's Cargo subcommand. Roadmap tasks 4.1.1, 4.1.2,
4.1.3, and 4.1.5 built its package, its OrthoConfig configuration spine, and
its only registered command, `cargo theorem context --json`, which prints the
compact agent-context document describing the command tree. Every later command
(list, get, check, run, backend management) will print results for humans and
for agents. Before those commands exist, the application needs one fixed answer
to four questions:

1. What does a machine-readable (`--json`) result or failure look like? This is
   the *JSON application envelope*: a versioned JSON object carrying
   `schema_version`, `kind`, `command`, `status`, and at most one of `result` or
   `error`.
2. Which bytes go to standard output (stdout) and which to standard error
   (stderr)? These are the *stream invariants*. In JSON mode a success writes
   exactly one JSON document to stdout and nothing to stderr; a failure writes
   nothing to stdout and exactly one JSON diagnostic to stderr. Output from any
   child process must never appear beside that document.
3. Which process exit codes can a caller rely on? These are the *stable exit
   classes*: `0` success, `2` usage, `3` configuration, `4` project state, `5`
   backend unavailable, `6` verification mismatch, `7` verification incomplete,
   `8` external tool failure, `9` delivery failure, and `10` internal.
4. How do the global human-output options (`--color`, `--emoji`, `--progress`,
   `--accessibility`, `--plain`, `--no-pager`, `--width`, `--locale`, `--quiet`,
   `--verbose`) behave? These are the *global renderer options*. Task 4.1.2
   already parses and merges them as strings; nothing yet validates them fully
   or makes them affect output.

Today, the observable behaviour is partial. `cargo theorem context --json`
writes the agent context to stdout. A configuration failure in JSON mode writes
a hand-built diagnostic whose `command` field is hard-coded to `["context"]`. A
failure to write a diagnostic exits with code `1`, which is outside the
documented taxonomy. Running `cargo theorem` with no subcommand is reported by
Clap's help-on-missing-subcommand path. Clap's help and parse errors ignore
`--color`, `NO_COLOR`, and `--width`. Nothing prevents a future command from
starting a child process that inherits the terminal's stdout.

After this change a user or agent can observe the following. These examples
assume a built binary and an empty temporary working directory.

```console
$ cargo theorem context --json --config missing.toml; echo "exit=$?"
exit=3
```

Stdout is empty. Stderr holds exactly one line of JSON, shown here
pretty-printed:

```json
{
  "schema_version": "1",
  "kind": "cargo-theorem.diagnostic",
  "command": ["context"],
  "status": "error",
  "error": {
    "code": "configuration.not_found",
    "class": "configuration",
    "message_fallback": "configuration file 'missing.toml' was not found",
    "arguments": { "path": "missing.toml" }
  }
}
```

```console
$ cargo theorem --color sometimes context --json; echo "exit=$?"
exit=2
```

Stderr holds one usage diagnostic whose `valid_values` field is
`["auto","always","never"]`. Running `NO_COLOR=1 cargo theorem --help` prints
help without ANSI escape sequences, while
`NO_COLOR=1 cargo theorem --color always --help` prints coloured help, because
an explicit command-line choice overrides the `NO_COLOR` convention.
`cargo theorem --help` ends with an "Exit status" section listing the ten exit
classes. The exact diagnostic codes and messages above are illustrative; the
implementation fixes them through snapshots.

The behaviour of `context --json` on success is preserved: it still prints
OrthoConfig's bare agent-context document (decision `D1`). Deliberate changes
are the diagnostic shapes and codes, exit code `1` becoming `10`, a bare
`cargo theorem` becoming a usage error (exit `2`), the rejection of invalid
`--width` and `--locale` values, and colour and width now applying to human
output.

This work closes roadmap task 4.1.4 and fixes the contracts that tasks 4.2.3,
4.2.4, 4.4.1, 4.5.1, 6.2.2, and 8.3.2 build on.

## Constraints

- Do not begin implementation until the user explicitly approves this plan,
  including decisions `D1`, `D5`, `D16`, and the roadmap note in `EP-M5`.
- Keep every code change inside `crates/cargo-theorem/`. Documentation changes
  may touch `docs/`. The reusable libraries (`theoremc-core`, `theoremc-macros`,
  `theoremc-build-support`, and the root `theoremc` facade), the workspace
  `clippy.toml`, `Makefile`, and CI workflows must not change, and the
  libraries must not gain application dependencies (ADR-0009, design §5).
- Do not implement any later command handler (`list`, `get`, `check`, `run`,
  `backend`, and so on), and do not build the command runner that roadmap task
  4.4.1 owns. The registered command tree remains `context` only, and agent
  context must not advertise anything new (design §6.6).
- Do not fork or extend OrthoConfig's agent-context or documentation schemas
  (design §6.1).
- Machine-readable identifiers (schema versions, kinds, diagnostic codes, exit
  class names, status values, argument keys, valid values) are never localized
  (ADR-0002 decision 4; ADR-0009).
- Production code must not read the process environment except through the
  existing injected `ortho_config::EnvSource` seam created at the composition
  root (`process_sources()` in `crates/cargo-theorem/src/config/mod.rs`).
  Terminal detection is probed once in `run()` and injected; no in-process test
  calls `IsTerminal`. Tests must not mutate process environment; child
  processes may be configured with `Command::env_clear` and `Command::env`
  (AGENTS.md "Subprocess isolation is the sole exemption").
- No source file may exceed 400 lines. `src/config_tests.rs` (350 lines),
  `tests/entry_points.rs` (330), `src/config/mod.rs` (310), `src/cli.rs` (299),
  and `src/cli_tests/runner.rs` (296) are near the limit; they are split before
  they grow (see `EP-M1` and `EP-M3`). Tests live in `*_tests.rs` files or
  `*_tests/` directories so production lines can be counted mechanically.
- The workspace denies `self_named_module_files`; new module directories use
  `mod.rs`.
- Every module begins with a `//!` comment. Every `assert!`-family call carries
  a diagnostic message. No `unwrap`, `expect` outside tests, `print!`, or
  `eprint!` (workspace lints).
- Lints are not silenced. Item-scoped `#[expect(..., reason = "...")]` is the
  only permitted suppression; each expectation on not-yet-used contract items
  names the roadmap task that will use them (AGENTS.md).
- Authorized new dev-dependencies for `crates/cargo-theorem` only, all already
  in `Cargo.lock`: `insta` 1.49, `googletest` 0.14, `pretty_assertions` 1.4,
  `proptest` 1.12 (no default features, `std`), `rstest-bdd` 0.6, and
  `rstest-bdd-macros` 0.6. No new runtime dependency and no new Cargo feature
  (including clap's `wrap_help`) is authorized.
  `ortho_config::LanguageIdentifier` provides locale validation and
  `std::io::IsTerminal` provides terminal detection.
- Pre-release interfaces: `cargo-theorem` is unreleased and has no external
  consumers, so changing its diagnostic codes, exit code `1`, and crate-private
  APIs needs no compatibility layer.
- Completed ExecPlans (for example the 4.1.2 plan) must not be edited.

## Tolerances (exception triggers)

Comparison base: the merge base with `origin/main` (commit `4c05966` when this
plan was drafted). This ExecPlan file is excluded from every count below.

Counting commands, run from the repository root:

```bash
base=$(git merge-base origin/main HEAD)
# Code: all Rust and feature files in the application package.
git diff --numstat "$base" -- 'crates/cargo-theorem/**/*.rs' \
  'crates/cargo-theorem/**/*.feature'
# Production Rust only (tests live in *_tests.rs files or *_tests/ dirs).
git diff --numstat "$base" -- 'crates/cargo-theorem/src/**/*.rs' \
  ':!**/*_tests.rs' ':!**/*_tests/**'
# Configuration: the package manifest and its Clippy configuration.
git diff --numstat "$base" -- 'crates/cargo-theorem/Cargo.toml' \
  'crates/cargo-theorem/clippy.toml'
# Documentation, excluding ExecPlans.
git diff --numstat "$base" -- 'docs/**/*.md' ':!docs/execplans/**'
# Snapshot files are counted by number only.
git diff --name-only "$base" -- '**/snapshots/*.snap' | wc -l
```

Limits are net lines (additions minus deletions) and are enforced per category;
spare capacity in one category never offsets another.

- Files: if more than 40 non-ExecPlan files change (excluding `Cargo.lock` and
  `.snap` files), stop and escalate. Snapshot files: at most 16.
- Code: if code changes exceed 2,600 net lines, stop and escalate. Within that,
  production Rust must not exceed 1,100 net lines.
- Configuration: if configuration changes exceed 80 net lines, stop and
  escalate. (A crate-local `clippy.toml` that mirrors the root file accounts
  for most of this budget; see decision `D5`.)
- Documentation: if documentation changes (excluding ExecPlans) exceed 800 net
  lines, stop and escalate.
- Interface: if any reusable library API must change, the registered command
  tree must grow, or a process runner becomes necessary, stop and escalate.
- Dependencies: if any runtime dependency, Cargo feature, or dev-dependency
  beyond those listed in `Constraints` is needed, stop and escalate.
- Architecture: if the stream invariant cannot be enforced without a
  process-global mechanism other than the panic hook installed in `main.rs`, or
  if `context --json` would need to change shape, stop and escalate.
- Iterations: if a milestone's gates still fail after five fix attempts, stop
  and escalate.
- Time: if a milestone takes more than one working day, stop and escalate.
- Ambiguity: if a prototype gate (typed policy enums, Clap help width, JSON
  intent reparse, crate-local Clippy configuration) fails and its recorded
  fallback changes observable behaviour, stop and present options.

Budget derivation. The survey found no comparable history of budget increases:
the 4.1.2 plan set no line budget, and plans 3-1-2 (700 net lines), 3-2-2
(600), 3-3-1 (500), and 3-3-2 (500) record no raises; 3-3-2 instead split a
test file after the fact to respect the 400-line rule. The design review
removed the process runner and Verus work (decision `D16`), which cut the first
draft's estimate by about 1,000 lines and 13 files. The remaining estimate is
about 950 net lines of production Rust (protocol types 220, emission and panic
guard 220, renderer policy and human rendering 300, JSON-intent detection 60,
rewiring and file splits in `cli`, `error`, and `config` about 150) and about
1,350 lines of tests (unit 800, end-to-end 300, behavioural 250). Code and
production budgets add roughly 13–15% contingency. Documentation covers one ADR
(about 170 lines), design-document updates (about 200), the users' guide (about
170), the developers' guide (about 150), roadmap notes, and index entries; 800
lines leaves about 15% contingency. The file estimate is about 34.

## Risks

- Risk: the OrthoConfig v0.10.0 derive may not accept enum-typed fields with
  `clap::ValueEnum` on `GlobalOptions`. Severity: medium. Likelihood: medium.
  Mitigation: `EP-M3` starts with a prototype that converts `color` only and
  runs the precedence tests; fallback decision `D6` keeps `Option<String>`
  fields and converts in `resolve()`.
- Risk: without clap's `wrap_help` feature, `Command::term_width` may not wrap
  help text. Severity: low. Likelihood: high. Mitigation: `--width` bounds
  cargo-theorem's own human prose; help wrapping is applied only if the
  prototype shows Clap honours it (decision `D9`). Tests pin the width
  explicitly rather than relying on Clap defaults.
- Risk: Clippy may not read a crate-local `clippy.toml`, or the mirror of the
  root file may drift. Severity: medium. Likelihood: low. Mitigation: `EP-M2`
  proves the fence with a seeded violation before committing it, and a contract
  test asserts the crate file contains every setting line of the root file;
  fallback is the source-scan fitness test (decision `D5`).
- Risk: snapshot churn from Clap or OrthoConfig upgrades (help layout, default
  styles, the agent-context version). Severity: low. Likelihood: medium.
  Mitigation: snapshot plain help only, pin `Command::styles`, assert coloured
  help by stripping escapes and comparing with the plain snapshot, and reduce
  the agent context to its kind, version, and command paths before snapshotting.
- Risk: the panic guard interacts with partially written streams or loses
  diagnostics. Severity: medium. Likelihood: low. Mitigation: counting writers
  live outside the guarded closure and panics flow through the same fallback
  kernel as write faults; the hook records the message and location for the
  guard to report (decision `D4`).
- Risk: the JSON-intent reparse with `ignore_errors(true)` may not recover the
  command path for every malformed input. Severity: low. Likelihood: medium.
  Mitigation: the token scan stays as the fallback, and Clap acts as the oracle
  in a property test (`JSON-INTENT-1`).
- Risk: deferring the captured-process proof to roadmap task 4.4.1 leaves the
  subprocess clause enforced structurally rather than by a running child.
  Severity: medium. Likelihood: certain (by design). Mitigation: the fence
  makes any spawn a reviewed exception; ADR-0014 states the runner's
  obligations; `EP-M5` adds roadmap notes to 4.2.1 and 4.4.1 (decision `D16`).

## Progress

- [x] (2026-11-16 18:40Z) Renamed the branch, set its upstream to
  `origin/4-1-4-versioned-json-application-contracts`, and titled the Lody
  session.
- [x] (2026-11-16 19:10Z) Reconnaissance: a Wyvern team surveyed tests, docs,
  plan history, and the OrthoConfig 0.10.0 sources; external prior art checked.
- [x] (2026-11-16 19:40Z) Initial draft written and committed.
- [x] (2026-11-16 20:30Z) Community-of-experts design review (structure and
  alternatives; contracts; failure modes and cost; verification rigour)
  completed and applied. Verdict: proceed with conditions; the conditions are
  folded into this revision.
- [ ] Plan approved by the user.
- [ ] `EP-M1` protocol types and exit classes.
- [ ] `EP-M2` emission, stream discipline, and the spawn fence.
- [ ] `EP-M3` global renderer options.
- [ ] `EP-M4` behavioural suite and contract hardening.
- [ ] `EP-M5` documentation, roadmap, and delivery.

## Surprises & discoveries

- Observation: OrthoConfig 0.10.0 and cargo-orthohelp 0.10.0 provide no
  envelope, exit-class, renderer-option, or terminal-detection types. Evidence:
  registry source search of both crates; the cargo-orthohelp README lists
  `--json` summaries and stable exit classes as planned. Impact: these
  contracts are application-owned in this slice.
- Observation: OrthoConfig's upstream agent-native design (§6.2 "Structured
  output") already defines the same JSON-mode contract and names a planned
  `JsonModeContract` with `success_stdout` (`one_json_document`),
  `failure_stderr` (`one_json_diagnostic`), and `subprocess_output_policy`
  (`capture_preview_and_log`, `inherit_only_in_human_mode`, `forbidden`)
  choices. Its global-option glossary lives in
  `cargo_orthohelp::policy::vocabulary` on `main`, but not in the 0.10.0
  release. Evidence: `docs/agent-native-cli-design.md` on OrthoConfig's `main`
  branch; `cargo-orthohelp-0.10.0/src/policy/vocabulary/` has only verbs and
  flags. Impact: the local glossary is a shadowing adapter under design §6.1;
  ADR-0014 uses OrthoConfig's vocabulary so task 4.5.1 can map onto it.
- Observation: OrthoConfig's `AgentCommand.output_modes` exists but
  `bridge_ir_to_agent_context` always leaves it empty, and the agent-context
  schema has no exit-code field. Evidence:
  `cargo-orthohelp-0.10.0/src/agent_context/mod.rs:219`. Impact: output
  metadata in agent context is deferred to 4.5.1; the exit-class table is
  published in `--help` instead (decision `D17`).
- Observation: the agent-context document's `kind` is
  `cargo-theorem.agent_context` and its `schema_version` is OrthoConfig's
  `"1"`, a different schema from the application envelope's `"1"`. Evidence:
  `ortho_config-0.10.0/src/agent_context/mod.rs:14-27`. Impact: consumers must
  dispatch on `kind` before `schema_version` (decision `D1`).
- Observation: `cargo_metadata::MetadataCommand::exec` inherits stderr when
  `verbose(true)` is set. Evidence: `cargo_metadata-0.23.1/src/lib.rs:1231`.
  Impact: task 4.2.1 must run `cargo metadata` through captured stdio, never
  through `exec` with inherited streams (roadmap note in `EP-M5`).
- Observation: Clap's own `Error::stream()` sends only `DisplayHelp` and
  `DisplayVersion` to stdout; `DisplayHelpOnMissingArgumentOrSubcommand` goes
  to stderr with code 2. Evidence: `clap_builder` `src/error/mod.rs`, reported
  by the contracts reviewer. Impact: a bare `cargo theorem` is a usage error.
- Observation: `serde_json` in this workspace does not enable
  `preserve_order`, so `serde_json::Value` sorts object keys. Evidence: no
  `preserve_order` feature in any manifest;
  `cargo tree -e features -i serde_json` shows none. Impact: key order is
  checked from raw bytes or snapshots, never from a parsed `Value`.
- Observation: two references named in the task request,
  `docs/rstest-bdd-users-guide.md` and `docs/ortho-config-users-guide.md`, do
  not exist in this repository. Evidence: `ls docs`. Impact: use
  `docs/ortho-config/users-guide.md` and the root `tests/*_bdd.rs` bindings as
  the working references; rstest-bdd-macros 0.6 resolves
  `#[scenario(path = …)]` against `CARGO_MANIFEST_DIR`, so a crate-local
  feature path works.
- Observation: no Verus wiring exists in the repository; Kani is wired through
  `make kani` and `.github/workflows/proofs.yml`. Impact: none in this slice
  after decision `D16`.

## Decision log

- D1. Decision: `cargo theorem context --json` keeps printing OrthoConfig's
  bare agent-context document on success; it is not wrapped in the application
  envelope. Its failures use the application diagnostic envelope. Every JSON
  document cargo-theorem writes carries `schema_version` and `kind`; `status`
  exists only in application envelopes, and consumers dispatch on `kind` before
  reading `schema_version`. OrthoConfig owns the context schema version, so an
  `ortho_config` upgrade may change it without an envelope bump. Rationale:
  design §14 states that the command emits OrthoConfig's document;
  OrthoConfig's ADR-0007 makes `context --json` the canonical introspection
  command across OrthoConfig applications; an agent reads the context before it
  knows this application's envelope. The strongest alternative, wrapping the
  document under `result`, would satisfy design §8.3's "All application JSON
  documents use a versioned envelope" literally and give the success envelope a
  production producer now. No current consumer parses the binary's context
  output (`make check-agent-native` runs `cargo orthohelp` on the package), so
  the argument for bare output is ecosystem convention rather than a known
  caller. Consequence: the success envelope's first end-to-end producer is task
  4.2.3. Design §8.3 is amended to name this single exemption. This changes a
  public contract and must be confirmed at plan approval.
- D2. Decision: the envelope body is a Rust enum (`Outcome::Result(T)` or
  `Outcome::Error(Diagnostic)`) from which `status` is derived, so
  `result`/`error` exclusivity and status agreement hold by construction. The
  `status` vocabulary is open: version 1 defines `success` and `error`, later
  slices may add values (for example an accepted asynchronous job), and
  consumers treat an unknown status as "inspect the exit code". Rationale:
  invalid states are unrepresentable; design §8.2 already anticipates other
  statuses.
- D3. Decision: in JSON mode, the complete document is serialized to an
  in-memory buffer before any byte is written, then written with one
  `write_all` and one `flush`. Emission functions take `OutputStreams` by
  value, so a run emits at most once. If writing stdout fails after zero bytes
  were accepted, one `output.write_failed` diagnostic is attempted on stderr;
  if any byte was accepted, or `flush` fails after `write_all` succeeded,
  nothing further is written anywhere. Either way the exit class is `internal`
  (10). If the failure diagnostic itself cannot be written to stderr, the
  original class is kept, because the exit code is then the only signal. Human
  mode uses the same fallback kernel. A broken pipe (`EPIPE`) is treated like
  any other write fault; whether it should become a distinct, documented
  environmental outcome is open question `Q1`. Rationale: at most one stream
  ever carries bytes, even under partial writes.
- D4. Decision: process exit codes are drawn only from the ten classes for any
  run in which `main` returns normally, except under an explicit passthrough
  mode that a later slice declares in agent context (design §8.4's
  `backend run --passthrough-exit-code`, task 4.4.5); even then a JSON
  diagnostic keeps its stable `class`. Clap's codes are normalized: only
  `DisplayHelp` and `DisplayVersion` exit `0` on stdout; every other Clap
  outcome, including help on a missing subcommand, exits `2` on stderr.
  Diagnostic write failures map as in `D3` instead of `1`. A panic is caught by
  `std::panic::catch_unwind` (with `AssertUnwindSafe`) around dispatch; the
  counting writers live outside the guarded closure, so a panic after `k` bytes
  is routed through the same fallback kernel as a write fault. `main.rs`
  installs once a panic hook, a named and separately unit-tested function, that
  records the panic message and `file:line` in a slot without printing or
  locking stderr; the guard reports it as `application.panic` (human mode prints
  `internal error: <message> at <location>`; JSON mode places the location in
  `arguments.location`). `main.rs` carries
  `#[cfg(panic = "abort")] compile_error!(...)` so an aborting profile cannot
  silently bypass the guard. Abort, stack overflow, and panics in detached
  threads remain outside the closure guarantee and are documented. Rationale:
  design §8.4 promises a stable taxonomy; codes `1` and `101` are outside it; a
  silent hook alone would destroy debugging information.
- D5. Decision: the subprocess clause of the roadmap success criterion is met
  structurally in this slice. A crate-local `crates/cargo-theorem/clippy.toml`
  mirrors the root `clippy.toml` and adds `std::process::Command::spawn`,
  `::output`, and `::status` to `disallowed-methods` with the reason "spawn
  children only through the captured runner (roadmap 4.4.1, ADR-0014)".
  Existing test helpers that spawn the binary carry item-scoped expectations
  under AGENTS.md's subprocess exemption. A contract test asserts that every
  setting line of the root file appears in the crate file. Every byte the
  application writes flows through `emit` (`D3`), and end-to-end tests show
  exactly one document per JSON run. Prototype gate: before committing, a seeded
  `Command::new("true").status()` in production code must make `make lint`
  fail; if Clippy does not read the crate-local file, fall back to a
  source-scan fitness test covering `std::process` imports (including aliases),
  `Command`, `Stdio`, and `MetadataCommand`, with a fixture of an aliased
  import that the scan must flag. Rationale: the design review found that a
  runner in this slice would pre-empt task 4.4.1 ("Inject one command runner
  for timeouts, cancellation, stream capture, and test doubles"), would buffer
  in memory where design §8.6 requires file-backed sinks, and would have no
  caller. A compiler-enforced fence is stronger than a text scan because it
  resolves call paths.
- D6. Decision: the renderer policy vocabulary (`ColorPolicy`, `EmojiPolicy`,
  `ProgressPolicy` with `auto|always|never`; `AccessibilityPolicy` with
  `auto|on|off`) lives in `render::policy`; `config` imports it, and `render`
  never depends on `config`. At the `GlobalOptions` field level these become
  enums with `clap::ValueEnum` if the OrthoConfig derive supports it, so Clap
  rejects invalid values with an enumerated usage error and file values fail
  deserialization. Fallback: keep `Option<String>` fields and convert in
  `GlobalOptions::resolve()`. Rationale: typed values remove string matching
  from rendering and let agent context enumerate choices.
- D7. Decision: effective colour for a stream is resolved in this order. JSON
  mode disables all decoration. An explicit `always` or `never` given on the
  command line or in `THEOREMC_COLOR` wins. A non-empty `NO_COLOR` disables
  colour. A value from any configuration file (system, user, or project)
  applies next. `--plain` disables colour. A non-empty `CLICOLOR_FORCE` other
  than `0` enables colour. `TERM=dumb` disables colour. Otherwise colour
  follows whether that stream (stdout or stderr, separately) is a terminal.
  Emoji and progress follow the same shape without the `NO_COLOR` and
  `CLICOLOR_FORCE` steps; `--accessibility on` and `--plain` disable emoji and
  animated progress unless explicitly `always`. `auto` accessibility is
  documented as "detected by the implementation; no detection is currently
  performed", which today behaves as `off`. Before configuration files are
  loaded (parse failures and help), only command-line values, injected
  environment, and terminal facts apply. Rationale: no-color.org lets user
  configuration and flags override `NO_COLOR`, and the CLICOLOR convention puts
  `NO_COLOR` above `CLICOLOR_FORCE`; ranking all files below `NO_COLOR` is the
  conservative reading, because OrthoConfig 0.10.0 does not report which scope
  supplied a value and a committed project file must not override a
  contributor's `NO_COLOR`.
- D8. Decision: `--quiet` and `--verbose` on the same command line are a Clap
  usage error (`conflicts_with`). A command-line selection of either overrides
  both values from lower sources. If neither is on the command line and the
  merged lower sources set both, the result is a configuration error naming
  both keys. Quiet suppresses non-essential human output but never diagnostics
  or JSON documents. Verbose adds the error source chain and the panic location
  to human diagnostics. Neither changes JSON documents. Rationale: respects
  precedence where it is observable and refuses to guess where it is not.
- D9. Decision: `--width` must lie in `20..=1000` columns; the diagnostic
  carries `arguments.min` and `arguments.max`, the range keys later reused by
  `--limit` (design §8.5). Width bounds cargo-theorem's own human prose and is
  passed to Clap help only if the `EP-M3` prototype shows Clap honours
  `term_width` without `wrap_help`. Rationale: the design gives no bounds;
  starting strict keeps later relaxation additive.
- D10. Decision: `--locale` must parse as a Unicode language identifier through
  `ortho_config::LanguageIdentifier`. The value is recorded in the render
  policy and passed to the application localizer, which remains
  `NoOpLocalizer`. Machine fields of JSON documents are locale-invariant; while
  the localizer is `NoOpLocalizer`, whole documents are too. Task 8.3.2 may add
  a localized projection beside `message_fallback`. Rationale: ADR-0002
  decisions 1 and 4.
- D11. Decision: no pager is launched in this slice. `--no-pager` is resolved
  and recorded, and the policy reports paging disabled. Rationale: a pager is a
  child process with an inherited terminal; it belongs with the first command
  that produces long human output and with the runner from 4.4.1.
- D12. Decision: `--json` stays a per-command flag on data-returning commands
  and is command-line only (design §6.5). For parse failures, JSON intent is
  detected first by a best-effort reparse of the same Clap `Command` with
  `ignore_errors(true)`, which also yields the longest recognized command path
  for the envelope's `command` field (possibly `[]`). If the reparse cannot
  decide, a token scan applies: a literal `--json` before any `--` separator
  that the registered grammar does not consume as an option value, where the
  value-taking options are derived from the Clap tree
  (`get_action().takes_values()`), not from a hand-kept list. A `--json` placed
  before the subcommand counts as intent and yields a usage diagnostic. Help
  and version output are always human. Rationale: per-command placement keeps
  agent context truthful; deriving the grammar prevents drift as commands add
  value options.
- D13. Decision: version 1 of the envelope has a written compatibility policy,
  recorded in ADR-0014 and design §8.3. Consumers ignore unknown fields,
  dispatch on `kind` before `schema_version`, never rely on key order (the
  fixed order exists for snapshot determinism), and treat the exit code as
  authoritative. Additive changes are new optional fields, new codes, kinds,
  and status values, and new optional argument keys. Breaking changes are
  removing, renaming, or retyping a field, changing a code's meaning, and
  changing the class-to-code mapping. A breaking change to a command's `result`
  schema gets a new `kind` (for example a `.v2` suffix), not an envelope bump.
  Reserved, not implemented, names: `warnings` (task 4.4.5), `message` and
  `message_locale` (8.3.2), `location` (8.1.1), `related` and `artefacts` inside
  `error`, and a top-level `job`. Pagination's `next_cursor` lives inside
  `result`. `arguments` is always present (possibly `{}`), and its values are
  strings, integers, or booleans. Rationale: later tasks must be able to add
  data without a version bump.
- D14. Decision: diagnostic codes are a validated newtype over
  `Cow<'static, str>` matching `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$`. The
  application's own codes are `pub(crate) const` items gathered in one
  `APP_CODES` registry slice, so tests and documentation can enumerate them;
  later crates contribute their own codes without a mapping table. A
  diagnostic's `class` belongs to the diagnostic instance, not to the code (for
  example `renderer.invalid_width` is `usage` from the command line and
  `configuration` from a file). Rationale: a closed enum would make every new
  code from `theoremc-core` (task 8.1.1) a breaking change or force a lossy
  mapping.
- D15. Decision: `protocol` is `pub(crate)`. Production code does not derive
  `Deserialize` for envelopes; tests parse documents as `serde_json::Value` (or
  raw bytes for key order) and validate them with test-only code independent of
  the Rust types. Items with no production producer yet carry item-scoped
  `#[expect(dead_code, reason = "...")]` naming their first producer:
  `Envelope::success` (task 4.2.3). Rationale: exporting types only to satisfy
  dead-code lints, or to let tests reuse the serializer's own types, would hide
  unused code and make the tests circular.
- D16. Decision: no Verus proof and no process runner land in 4.1.4. The
  bounded-capture obligations from the first draft (`BC-1` to `BC-3`) and their
  proof sketch are recorded below as deferred obligations for task 4.4.1, which
  owns stream capture, together with the open policy question of whether
  retained output keeps the head or the tail of a stream. Every obligation
  remaining in this slice is over a finite domain and is established
  exhaustively by ordinary tests. Rationale: all four reviewers questioned the
  runner's placement and two rejected Verus for a kernel with no caller whose
  retention policy the design does not specify; proving it now risks proving
  the wrong property in the wrong package.
- D17. Decision: `cargo theorem --help` gains an "Exit status" section
  generated from `ExitClass::ALL`, listing each code, class, and meaning. This
  is the production use of the full taxonomy (so no dead-code expectation is
  needed for unused classes) and the discoverable home for exit contracts until
  OrthoConfig's agent context can carry them. Rationale: design §14 asks for
  exit-code contracts to be discoverable, and the agent-context schema has no
  field for them.

Open questions for approval:

- Q1. Should a broken pipe on stdout (for example `cargo theorem ... | head`)
  exit `10` like other write faults (the plan's default), or be documented as
  an expected environmental outcome that exits `10` silently in human mode?
- Q2. Confirm decision `D1` (bare `context --json`) over wrapping it.
- Q3. Confirm decision `D16` and the roadmap notes in `EP-M5`, which move the
  runner-level evidence for "subprocess output cannot leak beside a JSON
  document" to task 4.4.1.

## Outcomes & retrospective

Not started. Record outcomes, gate evidence, and lessons at each milestone.

## Context and orientation

The repository is a Cargo workspace. The root package `theoremc` is a library
facade; the reusable crates are `crates/theoremc-core`,
`crates/theoremc-macros`, and `crates/theoremc-build-support`. The application
package `crates/cargo-theorem` owns the `cargo-theorem` executable, which Cargo
runs as `cargo theorem`. Cargo passes the token `theorem` as the first
argument, so direct invocation is `cargo-theorem theorem <args>`.

Current files in `crates/cargo-theorem/src/` and their roles:

- `main.rs` (7 lines) calls `cargo_theorem::run(std::env::args_os())`.
- `lib.rs` (11 lines) declares the private modules and re-exports `run`.
- `cli.rs` (299 lines) declares the Clap types (`CargoEntry`, `CargoCommand`,
  `TheoremArgs`, `Commands`, `ContextArgs`), `run`, `run_with_io`,
  `run_with_sources_and_io` (the injected-source test seam taking
  `&mut dyn Write` for stdout and stderr), `execute`, `run_selected` (which
  writes the agent context straight to stdout), and the helpers that rebuild a
  configuration-only argument list.
- `cli_diagnostics.rs` (178 lines) renders Clap errors and application errors,
  scans argv for JSON intent (`parse_error_requests_json`, `takes_global_value`
  with a hand-kept option list), and hand-builds the JSON diagnostic with
  `serde_json::json!` (`write_json_diagnostic`, which hard-codes
  `"command": ["context"]`).
- `error.rs` (101 lines) defines `ApplicationError` (`Configuration`,
  `Serialization`, `Output`) and `exit_code()` mapping to `3` and `10`.
- `config/mod.rs` (310 lines) defines `ResolvedSettings`, `GlobalOptions`
  loading (`load_with_sources`), `resolve()` with string policy validation
  (`POLICY_VALUES`, `ACCESSIBILITY_POLICY_VALUES`), the `cmds` namespace
  validation, `process_sources()` (the environment composition root), and the
  selected-subcommand adapter `ResolvedCommand`. Note that `context` skips the
  configuration merge unless `--config` or `THEOREMC_CONFIG_PATH` is given
  (`should_load_configuration` in `cli.rs`), so environment-sourced renderer
  errors are tested at the configuration layer, not end to end.
- `config/global_options.rs` (93 lines) holds the `GlobalOptions` struct with
  the OrthoConfig derive; it is the only code that reads the process
  environment, under a documented `#[expect(clippy::disallowed_methods)]`.
- `agent_context.rs` (86 lines) builds the agent context from the Clap tree via
  `cargo_orthohelp::agent_context::bridge_ir_to_agent_context`.
- Tests: `cli_tests.rs` and `cli_tests/runner.rs` drive
  `run_with_sources_and_io` in-process with `ortho_config::MapEnv` sources and a
  `FailingWriter`; `config_tests.rs` covers precedence;
  `tests/entry_points.rs` with
  `tests/entry_points/{process,support, agent_context}.rs` spawns the real
  binary (`env!("CARGO_BIN_EXE_cargo-theorem")`) with `env_clear()` and a
  temporary working directory, checking exit codes and streams with
  `anyhow::ensure!`.

Test infrastructure elsewhere: `insta` is used only in `crates/theoremc-macros`
(default `snapshots/` directories, no `insta.yaml`); `rstest-bdd` 0.6 scenarios
live in the root `tests/features/*.feature` with bindings in
`tests/<name>_bdd.rs` using `#[scenario(path = ..., name = ...)]` and
`#[given]`/`#[when]`/`#[then]` steps returning `Result<(), String>`;
`.config/nextest.toml` sets a 60-second slow timeout terminating after five
periods. `make test` runs
`cargo nextest run --workspace --all-targets --all-features` then doctests;
`make lint` runs rustdoc, Clippy, and Whitaker; `make check-fmt` runs
`cargo fmt --check` and `mdtablefix`; `make check-agent-native` runs the
OrthoConfig policy check; `make markdownlint`, `make nixie`, and
`make spelling` cover documentation. CI runs the suite twice (coverage under
`cargo llvm-cov`, then `make test`), so added test time counts twice.

Planned module map after this work (all under `crates/cargo-theorem/src/`):

- `protocol/` (pure contract types, depends only on `serde` and `serde_json`):
  `mod.rs` (`SCHEMA_VERSION`, `Envelope`, `Outcome`, `Status`, `Diagnostic`),
  `exit_class.rs`, `code.rs` (`DiagnosticCode`, `APP_CODES`).
- `render/` (pure policy plus a human formatter, depends on `protocol`):
  `policy.rs` (policy enums, `RenderInputs`, `RenderPolicy::resolve`),
  `terminal.rs` (`TerminalFacts`), `human.rs`.
- `emit/` (the only module that writes to the output streams, depends on
  `protocol` and `render`): `mod.rs` (`OutputStreams`, `CommandOutput`,
  emission functions), `counting.rs` (`CountingWriter`, `plan_fallback`),
  `panic_guard.rs`.
- `error.rs` maps `ApplicationError` to `Diagnostic` and `ExitClass`.
- `cli/` (split from `cli.rs` in `EP-M1`): `mod.rs` (Clap types and `run`),
  `dispatch.rs` (configuration resolution and handlers returning
  `CommandOutput`), `diagnostics.rs` (Clap error normalization),
  `json_intent.rs`.
- `config/` keeps loading; `config/renderer.rs` holds renderer validation.

Dependency direction: `protocol` ← `render` ← `emit` ← `cli`; `error` depends on
`protocol`; `config` depends on `render::policy` for the vocabulary only. No
new dependency edge between `config` and `cli` is added.

Terms used in this plan:

- *Envelope*: the outer JSON object of every command result or diagnostic.
- *Kind*: a dotted, non-localized identifier naming the document type, such as
  `cargo-theorem.diagnostic`.
- *Exit class*: the named category of a process exit code, such as `usage`.
- *Render policy*: the effective, fully resolved set of human-output decisions
  (colour on or off per stream, emoji on or off, and so on) computed from
  options, environment, and terminal facts.
- *Fallback kernel*: the pure function `plan_fallback` that decides, from the
  number of bytes a stream accepted before a fault, whether a fallback
  diagnostic may still be written.
- *Spawn fence*: the Clippy `disallowed-methods` rule that makes starting a
  child process a compile-time lint error unless an item-scoped expectation
  documents a reviewed exception.

Signposted documentation, all repository-relative:

- `docs/cargo-theorem-cli-design.md` §§5, 6.5, 6.6, 8.1-8.6, 12.1, 14, 15, 17,
  18 (the governing design).
- `docs/cargo-theorem-cli-users-guide.md` "JSON output and exit codes" (to be
  updated).
- `docs/adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md`
  decisions 1, 2, and 4; `docs/adrs/adr-0009-orthoconfig-cli-configuration.md`.
- `docs/developers-guide.md` §3.8 (the CLI package) and the environment
  injection section.
- `docs/ortho-config/users-guide.md` (agent-native conventions,
  `context --json`) and `docs/ortho-config/cargo-orthohelp-design.md`.
- `docs/rust-testing-with-rstest-fixtures.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-doctest-dry-guide.md`,
  `docs/complexity-antipatterns-and-refactoring-strategies.md`, and
  `docs/documentation-style-guide.md`.
- `docs/theoremc-design.md` (DES) and `docs/theorem-file-specification.md`
  (TFS) are background only; this task changes neither theorem semantics nor
  file format.

Signposted skills: `execplans`, `rust-router`, `rust-errors`,
`rust-types-and-apis`, `domain-cli-and-daemons`, `rust-unit-testing`,
`proptest`, `rust-verification`, `hexagonal-architecture`, `codegraph-mcp` (the
CodeGraph server was unavailable during planning; use `leta` or `rg` as
fallbacks), `en-gb-oxendict-style`, and `commit-message`.

External prior art consulted: the NO_COLOR convention (<https://no-color.org/>:
a non-empty `NO_COLOR` disables colour; user configuration and flags override
it); the CLICOLOR convention (<https://bixense.com/clicolors/>: `NO_COLOR`
overrides `CLICOLOR_FORCE`); OrthoConfig's agent-native CLI design §6.2 (JSON
mode contract and global option glossary); and Trevin Chow's "10 Principles for
Agent-Native CLIs" (stable exit taxonomy, enumerating errors).

## Conformance basis

Upstream artefacts, at the merge base `4c05966`:

- `docs/roadmap.md` task 4.1.4 (`RM-4.1.4`): success requires snapshots for
  success and failure in human and JSON modes, and that subprocess output
  cannot leak beside a JSON document. Task 4.4.1 (`RM-4.4.1`) owns the command
  runner, including stream capture.
- `docs/cargo-theorem-cli-design.md`:
  - `DES-CLI-8.2-STREAMS`: JSON-mode stream rules and the renderer option list.
  - `DES-CLI-8.3-ENVELOPE`: envelope fields, versioning, error shape.
  - `DES-CLI-8.4-EXIT`: the exit-class table.
  - `DES-CLI-8.6-BOUNDS`: file-backed bounded sinks with truncation metadata.
  - `DES-CLI-6.6-SCOPE`: no unimplemented handler is advertised.
  - `DES-CLI-5-BOUNDARY`: application dependencies stay out of libraries; the
    command-execution abstraction belongs to the runner layer.
  - `DES-CLI-14-CONTEXT`: `context --json` emits OrthoConfig's document and
    should expose exit-code contracts.
  - `DES-CLI-18-TESTS`: snapshot, end-to-end, and behavioural layers.
- ADR-0002 decision 4 (`ADR2-D4`): machine-facing output uses stable codes and
  structured fields. Decision 2 (`ADR2-D2`): code, arguments, English fallback.
- ADR-0009 (`ADR9`): non-localized identifiers; injected sources.
- `AGENTS.md`: environment injection, 400-line files, lint, test, and
  abstraction policy.
- No Terms of Reference document exists for this project; the roadmap and the
  design document serve as the requirement baseline.

Trace links:

- `RM-4.1.4` + `DES-CLI-8.3-ENVELOPE` + `ADR2-D2` -> `EP-M1` -> `ENV-1`
  tests and serialized-form snapshots.
- `DES-CLI-8.4-EXIT` + `DES-CLI-14-CONTEXT` -> `EP-M1`, `EP-M2` -> `EXIT-1`
  tests, the help "Exit status" snapshot, end-to-end exit assertions.
- `RM-4.1.4` + `DES-CLI-8.2-STREAMS` -> `EP-M2` -> `STREAM-1`,
  `JSON-INTENT-1`, `/dev/full` end-to-end test, transcript snapshots.
- `RM-4.1.4` (subprocess) + `DES-CLI-5-BOUNDARY` + `RM-4.4.1` -> `EP-M2` ->
  `STREAM-2` fence evidence; deferred `BC-1` to `BC-3` -> `RM-4.4.1`.
- `DES-CLI-8.2-STREAMS` (renderer options) + `ADR2-D4` -> `EP-M3` ->
  `REND-1`, `CFG-1`, `LOC-1`, human snapshots.
- `RM-4.1.4` (snapshots) + `DES-CLI-18-TESTS` -> `EP-M1` to `EP-M4` ->
  end-to-end snapshot matrix and `rstest-bdd` scenarios.
- All of the above -> `EP-M5` -> ADR-0014, design, guides, roadmap.

Deviations to be recorded upstream, each approved with this plan:

- Decision `D1` narrows design §8.3's "All application JSON documents use a
  versioned envelope" by exempting the `context` success document.
- Decision `D16` satisfies the subprocess clause of `RM-4.1.4` structurally and
  moves runner-level evidence to `RM-4.4.1`; `EP-M5` adds that obligation to
  task 4.4.1 and a captured-stdio note to task 4.2.1.
- Decision `D7` ranks configuration-file colour values below `NO_COLOR`, a
  stricter reading than no-color.org's allowance for user configuration.

## Verification plan

This section lists each obligation, its method, and why the method can fail
when the implementation is wrong. Where Verus is warranted: nowhere in this
slice (decision `D16`). Every remaining obligation ranges over a finite domain
that ordinary tests enumerate exhaustively, or is integration plumbing whose
evidence is the executable boundary. Kani is not used either: there is no
`unsafe` code, no arithmetic beyond byte counting, and no bounded state machine
larger than the enumerations below.

### Invariants and lemmas

`ENV-1` envelope well-formedness. Every serialized envelope is a JSON object
whose keys appear in the order `schema_version`, `kind`, `command`, `status`,
then `result` or `error`; `schema_version` is `"1"`; `status` is `"success"`
when `result` is present and `"error"` when `error` is present; `result` and
`error` never both appear; `arguments` is always an object whose values are
strings, integers, or booleans; a diagnostic `kind` is
`cargo-theorem.diagnostic`; a success kind is
`cargo-theorem.<command path joined by ".">.result`.

- Method: construction by type (decision `D2`). A test-only validator written
  against `serde_json::Value` checks the field rules, and a test-only
  `serde::de::Visitor` over the raw bytes records key order (because `Value`
  sorts keys). A proptest generates envelopes whose outcome variant is chosen
  first (`prop_oneof![success, failure]`), with command paths from a small
  alphabet, arbitrary scalar argument values, and every code in `APP_CODES`,
  then serializes and validates. Named rstest rows pin one case per outcome so
  coverage never depends on generator luck. Two insta snapshots fix one success
  and one error serialization.
- Artefact: `crates/cargo-theorem/src/protocol/envelope_tests.rs`.
- Non-vacuity: the key-order visitor and validator are themselves exercised by
  handwritten negative documents (both `result` and `error`;
  `status: "success"` with `error`; keys out of order; a nested object inside
  `arguments`), each of which must be rejected for the stated reason.
- Discharge: `cargo nextest run -p cargo-theorem envelope` passes.

`CODE-1` diagnostic code grammar. Every code in `APP_CODES` matches the
grammar, codes are unique, and `DiagnosticCode::new` rejects strings outside
the grammar.

- Method: rstest rows for accepted and rejected strings (`cli`, `Cli.usage`,
  `cli..usage`, `cli.usage.`, `cli.usage-x`); an exhaustive loop over
  `APP_CODES` for grammar and uniqueness.
- Non-vacuity: the rejected rows prove the validator can fail.

`EXIT-1` exit taxonomy. The map from `ExitClass` to process code is a bijection
onto `{0, 2, 3, 4, 5, 6, 7, 8, 9, 10}` with the design §8.4 names; an exit code
of `0` implies `status` `success` (or no JSON document, in human mode); `status`
`error` implies a non-zero exit code; for every run in which `main` returns
normally, no other code is returned.

- Method: exhaustive test over `ExitClass::ALL`, whose length is tied to the
  enum by a wildcard-free `match` in a `const fn`, so a new variant fails to
  compile until it is listed; rstest rows mapping each Clap `ErrorKind`
  (including `DisplayHelpOnMissingArgumentOrSubcommand`) to its code and
  stream; the panic-guard tests below; and an exit-code assertion in every
  end-to-end transcript, each checked against the transcript's `status`.
- Non-vacuity: a negative row asserts that a stderr write failure on the
  failure path keeps the original class rather than `1`; the help snapshot
  shows all ten classes, so a dropped variant changes the snapshot.

`STREAM-1` emission. For each outcome (success, failure), each stdout fault
(none; fails before any byte; fails after `k >= 1` bytes; `write_all` succeeds
but `flush` fails), and each stderr fault (none; fails), in both JSON and human
mode, the bytes written satisfy: success without fault puts exactly one
document plus one newline on stdout and nothing on stderr (JSON mode); failure
without fault puts nothing on stdout and exactly one diagnostic plus one
newline on stderr; a fault after any accepted byte, or a flush fault, leads to
no further bytes on either stream and exit `10`; a fault before any byte leads
to exactly one `output.write_failed` diagnostic on stderr and exit `10`; a
stderr fault on the failure path keeps the original class. Hence stdout and
stderr are never both non-empty in JSON mode.

- Lemma `STREAM-L1`: `CountingWriter` reports exactly the bytes the inner
  writer accepted, including partial writes.
- Lemma `STREAM-L2`: the fallback decision depends only on whether the count
  is zero (`plan_fallback`).
- Method: an rstest table driving the real `emit` functions with scripted fake
  writers over the full product (2 outcomes × 4 stdout faults × 2 stderr faults
  × 2 modes = 32 cases), plus two panic rows: a handler that panics before
  writing and one that panics after `k` bytes reached stdout. A proptest feeds
  real envelopes through a writer that accepts a random number of bytes per
  call and faults at a random offset, choosing the fault class first so every
  class is generated. `STREAM-L1` has its own rstest rows (accept 0, 1, and `n`
  bytes per call).
- Non-vacuity: a handwritten bad transcript (bytes on both streams) is fed to
  the transcript checker and must be rejected; the zero-byte and partial rows
  differ in expected output, so swapping the branches of `plan_fallback` fails
  them.
- End-to-end evidence: on Linux, a test runs the binary with stdout redirected
  to `/dev/full` (`#[cfg(target_os = "linux")]`) and expects exit `10`, empty
  stdout, and one `output.write_failed` diagnostic on stderr.
- Discharge: `emit/*_tests.rs` and the `/dev/full` test pass.

`STREAM-2` no inherited child streams. No production code in `cargo-theorem`
starts a child process. Any future spawn site needs a reviewed, item-scoped
lint expectation whose reason names the runner contract.

- Method: the spawn fence (decision `D5`), checked by `make lint` on every
  build; a contract test that the crate-local `clippy.toml` contains every
  setting line of the root file.
- Non-vacuity: the `EP-M2` prototype records, in `Artefacts and notes`, the
  `make lint` failure produced by a seeded spawn in production code before the
  seed is removed. If the fallback source scan is used instead, its fixture
  with an aliased `use std::process as p;` must be flagged.
- Residual gap: runner behaviour (capture, bounds, deadlines) is not exercised
  in this slice; it is deferred to task 4.4.1 with the obligations below.

`JSON-INTENT-1` JSON intent and command path for parse failures. The detector
reports JSON intent exactly when a `--json` token is meant as the flag, and
reports the longest recognized command path.

- Method: rstest rows for `context --json`, `--json context`,
  `--config --json context`, `--locale --json context`, `context -- --json`,
  `--config=--json context`, `context --json --color sometimes`, an unknown
  subcommand with `--json`, and no `--json`. A proptest builds argument vectors
  from a token alphabet (registered options, values, `--`, `--json`, subcommand
  names); whenever Clap parses the vector successfully, the detector's answer
  must equal Clap's parsed `json` flag and command path. Clap is an independent
  oracle for the parseable subset.
- Non-vacuity: the generator chooses whether `--json` is present, and as what,
  before building the vector, and named rows cover each placement.

`REND-1` render policy resolution. For every combination of mode (human, JSON),
command-line policy (`auto`, `always`, `never`, unset), `THEOREMC_COLOR` (unset,
`always`, `never`), file-sourced policy (unset, `always`, `never`), `--plain`
(unset, true), accessibility (`auto`, `on`, `off`), `NO_COLOR` (unset, empty,
non-empty), `CLICOLOR_FORCE` (unset, `0`, `1`), `TERM` (unset, `dumb`, `xterm`),
`stdout_is_terminal` and `stderr_is_terminal` (each true, false), the resolved
policy follows decision `D7`.

- Method: two layers, neither of which re-implements `resolve`. First, named
  rstest rows written as expected outputs, one per rule in `D7` and its
  boundaries (including `NO_COLOR=1` with `--color always`, expecting colour on;
  `NO_COLOR=1` with a file value `always`, expecting colour off; and stdout a
  terminal with stderr not, expecting colour on stdout only). Second, partial
  properties enumerated over the whole product domain: JSON mode means
  everything off; an explicit command-line `never` means colour off; an
  explicit command-line `always` in human mode means colour on; `--plain`
  without explicit values means colour, emoji, and animated progress off;
  paging is always off.
- Non-vacuity: each property's antecedent is counted during enumeration and
  must hold for at least one case; each named row targets a distinct deciding
  rule, so reordering any two rules in `resolve` breaks at least one row.

`CFG-1` renderer value validation. Out-of-range widths, malformed locales,
unknown policy values, and verbosity conflicts are rejected with the right class
(`usage` on the command line, `configuration` from files or environment) and a
diagnostic whose `valid_values` or `arguments` (`min`, `max`, the offending
key) names the accepted set or range.

- Method: rstest tables over command-line, environment (`MapEnv`), and file
  sources, with width boundary rows `19`, `20`, `1000`, `1001`, and a
  mixed-source verbosity row (`--quiet` on the command line with
  `verbose = true` in a file, expecting quiet and no error), plus a same-source
  row (both in one file, expecting a configuration error).
- Discharge: `config/renderer_tests.rs` passes.

`LOC-1` locale invariance of machine fields. For a fixed outcome, the JSON
machine fields are identical under any locale, and with `NoOpLocalizer` the
whole document is.

- Method: an in-process test injects a fake localizer that rewrites every
  message it is given, renders the same failure in JSON and human mode, and
  asserts that the JSON bytes equal those produced with `NoOpLocalizer` while
  the human rendering changed. An end-to-end check compares bytes with and
  without `--locale fr-FR`.
- Non-vacuity: the changed human rendering proves the localizer was invoked.

### Deferred obligations (owned by task 4.4.1)

Recorded so the runner slice starts from a sketched proof rather than from
scratch. Policy question first: design §8.6 requires file-backed bounded sinks
but does not say whether a truncated stream keeps its head or its tail; backend
logs usually need the tail. The obligations below assume a head (prefix) policy
and must be restated if 4.4.1 chooses otherwise.

For a capture with limit `L` fed chunks `c_1 ... c_n`, with
`S = c_1 ++ ... ++ c_n` and `|S|` within `u64`:

- `BC-1`: `bytes_seen == |S|`.
- `BC-2`: the retained bytes are exactly `S[0 .. min(|S|, L)]`, so
  `bytes_retained == min(|S|, L) <= L`.
- `BC-3`: `truncated` holds exactly when `|S| > L`.

Proof sketch for Verus, if 4.4.1 selects it: state
`(retained: Seq<u8>, seen: nat)`; `flatten` defined by `drop_last` so that
`flatten(h.push(c)) == flatten(h) + c` is a lemma, not the definition; invariant
`seen == flatten(h).len() && retained == flatten(h).take(min(seen, L))`;
candidate lemmas on `take` over concatenation for `k <= |a|` and
`|a| <= k <= |a| + |b|`; main lemma that `push` preserves the invariant; a
satisfiability witness whose chunk crosses the limit. Representation relation:
`seen as nat == flatten(h).len()`,
`retained@ == flatten(h).take(min(seen, L) as int)`, `retained.len() <= L`.
Drift control: marked mirror regions compared by a test, plus a refinement
property test against a batch recomputation. The runner slice must also show,
with a real child process in JSON mode, that child output appears only in
captures and never on the application's streams.

### Axioms (trusted external behaviour)

- `serde_json` serializes struct fields in declaration order and produces valid
  JSON; without `preserve_order`, `serde_json::Value` sorts keys.
- `std::io::Write::write` returns the number of bytes accepted and
  `write_all` stops at the first error. A locked stdout is a `LineWriter`:
  bytes it accepts may sit in its buffer, but nothing is buffered unless at
  least one byte reached the descriptor, provided the buffer is empty before
  emission and each document ends in `\n`; the zero/non-zero decision is
  therefore sound.
- Rust ignores `SIGPIPE`, so a closed pipe yields `EPIPE` errors. Rust's
  runtime reopens closed descriptors 0 to 2 on `/dev/null`, and std treats
  `EBADF` on stdio as success, so `>&-` cannot be detected and ends in exit 0.
- Clap's `Error::kind()` and `Error::get(ContextKind::...)` expose the invalid
  argument, value, and valid values for value-validation errors, and
  `ignore_errors(true)` yields best-effort matches.
- `std::io::IsTerminal` reports whether a handle is a terminal.
- Cargo runs an external subcommand with the caller's stdio and passes the exit
  code through, but may itself print warnings on stderr first; stream-exact
  assertions therefore run the binary directly, and "stderr is empty" means
  bytes written by cargo-theorem.
- Clippy reads `disallowed-methods` from the nearest `clippy.toml` above the
  crate's manifest directory (checked by the `EP-M2` prototype).

## Plan of work

Implementation proceeds through five milestones. Each ends with
`make check-fmt`, `make lint`, and `make test` passing (logs captured with
`tee` under `/tmp`, as in `Concrete steps`) and a commit. Each milestone works
outside in: its end-to-end transcript cases are written first as red tests,
then the unit-level red tests, then the green implementation, then a refactor
with the focused tests rerun. Assertions use `googletest`'s `expect_that!` with
`matches_pattern!` and `elements_are!` for diagnostics and `valid_values`,
`verify_that!` inside tests that return `Result`, and
`pretty_assertions::assert_eq!` for transcript and byte comparisons.

End-to-end transcripts are insta snapshots of the form
`exit: N\n--- stdout ---\n...\n--- stderr ---\n...`, produced by running the
binary directly through the isolated command builder in
`tests/entry_points/process.rs` (moved to a shared `tests/common/` module when
a second test target needs it), with the package version filtered.

### EP-M1: protocol types and exit classes

Assigned: `RM-4.1.4`, `DES-CLI-8.3-ENVELOPE`, `DES-CLI-8.4-EXIT`,
`DES-CLI-14-CONTEXT`, `ADR2-D2`, `ADR2-D4`.

1. Refactor first (no behaviour change, existing tests green): split `cli.rs`
   into `cli/mod.rs`, `cli/dispatch.rs`, and `cli/diagnostics.rs` (the current
   `cli_diagnostics.rs`), and move `cli_tests/runner.rs` cases that will grow
   into a second file. Commit.
2. Red: end-to-end transcripts `json_failure_configuration`
   (`theorem context --json --config missing.toml`) and `json_failure_usage`
   (`theorem --color sometimes context --json`), and the help snapshot with the
   "Exit status" section. Unit red tests: `protocol/envelope_tests.rs`,
   `protocol/code_tests.rs`, `protocol/exit_class_tests.rs`.
3. Green: add `protocol/` (`SCHEMA_VERSION`, `Envelope<T>` with a flattened
   `Outcome<T>`, `Status`, `Diagnostic` with `code`, `class`,
   `message_fallback`, `arguments` (always serialized), and optional
   `valid_values`; `DiagnosticCode` and `APP_CODES`; `ExitClass` with `ALL`,
   `const fn code`, `const fn as_str`, and `exit_code()`). Initial codes:
   `cli.usage`, `cli.invalid_value`, `cli.unknown_argument`,
   `cli.missing_argument`, `cli.argument_conflict`, `configuration.invalid`,
   `configuration.not_found`, `configuration.invalid_value`,
   `output.write_failed`, `application.serialization`, and `application.panic`,
   adjusted to what the snapshots need. Rewire `error.rs` to return
   `Diagnostic` and `ExitClass`; replace `write_json_diagnostic` with envelope
   serialization carrying the real command path; normalize Clap codes (decision
   `D4`); add the help "Exit status" section from `ExitClass::ALL` (decision
   `D17`); add `#[expect(dead_code, reason = "first producer: roadmap 4.2.3")]`
   to `Envelope::success`.
4. Refactor and gate.

Acceptance: `ENV-1`, `CODE-1`, and the mapping half of `EXIT-1` discharged; the
two transcripts and the help snapshot accepted; the existing end-to-end tests
pass with updated codes. End state: every existing JSON diagnostic uses the
protocol types. Recovery: revert the milestone commits. Remaining gaps:
emission discipline, panic guard, JSON intent, renderer.

### EP-M2: emission, stream discipline, and the spawn fence

Assigned: `RM-4.1.4`, `DES-CLI-8.2-STREAMS`, `DES-CLI-8.4-EXIT`,
`DES-CLI-5-BOUNDARY`.

1. Red: end-to-end `/dev/full` test (Linux) and `human_failure_usage`
   transcript (`theorem --color sometimes context`); `json_success_context`
   transcript with the stdout document reduced to `kind`, `schema_version`, and
   command paths before snapshotting; unit tests `emit/emit_tests.rs`,
   `emit/counting_tests.rs`, `emit/panic_guard_tests.rs`,
   `cli/json_intent_tests.rs`.
2. Green: add `emit/` with `OutputStreams` (owning the two writers, passed by
   value to emission functions), `CommandOutput` (a JSON document, an envelope,
   or human text), `CountingWriter`, `plan_fallback`, JSON and human emission
   (decision `D3`), and `panic_guard` (decision `D4`). Handlers in
   `cli/dispatch.rs` return `Result<CommandOutput, ApplicationError>`; only
   `emit` touches writers. `main.rs` installs the recording panic hook and
   carries the `cfg(panic = "abort")` guard. Replace
   `parse_error_requests_json` and `takes_global_value` with
   `cli/json_intent.rs` (decision `D12`; prototype the `ignore_errors(true)`
   reparse first and record the outcome).
3. Spawn fence (decision `D5`): prototype by adding
   `crates/cargo-theorem/clippy.toml` and a seeded spawn, confirm `make lint`
   fails, record the evidence, remove the seed; add item-scoped expectations to
   the existing test helpers that spawn the binary; add the mirror contract test
   (`crates/cargo-theorem/tests/clippy_config_mirror.rs`). If the prototype
   fails, implement the fallback source scan instead and record why.
4. Refactor and gate.

Acceptance: `STREAM-1`, `STREAM-2`, `JSON-INTENT-1`, and the panic half of
`EXIT-1` discharged. End state: every byte the application writes flows through
`emit`, panics and write faults map to the taxonomy, and no child can be
spawned without a reviewed exception. Recovery: revert. Remaining gaps:
renderer options, behavioural suite, docs.

### EP-M3: global renderer options

Assigned: `DES-CLI-8.2-STREAMS` (renderer list), `ADR2-D4`, `ADR9`.

1. Prototype (keep or discard by the end of this step): convert
   `GlobalOptions::color` to `Option<ColorPolicy>` with `clap::ValueEnum` and
   serde `rename_all = "lowercase"`, then run `config_tests.rs` and the
   agent-context tests. If precedence holds and agent context still generates,
   keep typed enums for all four policies (decision `D6`); otherwise revert to
   strings and record the evidence. Also check whether Clap wraps help at
   `Command::term_width(n)` without `wrap_help` (decision `D9`).
2. Red: transcripts `json_failure_width` (`theorem --width 5 context --json`),
   `human_success_help` (plain, `TERM` unset, explicit width) and the coloured
   help check (`--color always --help` stripped of escapes equals the plain
   snapshot and contains `ESC[`); the `NO_COLOR` pair; unit tests
   `render/policy_tests.rs`, `render/human_tests.rs` (human-renderer snapshots
   of a `configuration` diagnostic with colour on, off, plain, and verbose —
   the human configuration failure is not reachable end to end, because
   `context` requires `--json` and help never loads configuration),
   `config/renderer_tests.rs`, and the `LOC-1` test.
3. Green: add `render/` (`policy.rs`, `terminal.rs`, `human.rs`); read
   `NO_COLOR`, `CLICOLOR_FORCE`, `TERM`, and `THEOREMC_COLOR` through the
   injected `EnvSource`; probe `TerminalFacts` in `run()` only. Add
   `config/renderer.rs` with width, locale, and verbosity validation (decisions
   `D8` to `D10`), mapping command-line problems to `usage` (Clap
   `value_parser` range and `conflicts_with`) and file or environment problems
   to `configuration`. Render Clap help and errors with `StyledStr::ansi()`
   when colour is on and plain text when off, set `ColorChoice::Never` on the
   command so only the policy decides, and pin `Command::styles`.
4. Refactor and gate.

Acceptance: `REND-1`, `CFG-1`, and `LOC-1` discharged. End state: renderer
options are typed, validated, and applied. Recovery: revert; if a prototype
failed, only its fallback path remains. Remaining gaps: behavioural suite, docs.

### EP-M4: behavioural suite and contract hardening

Assigned: `RM-4.1.4` (snapshots), `DES-CLI-18-TESTS`.

Add `crates/cargo-theorem/tests/features/output_contract.feature` and
`crates/cargo-theorem/tests/output_contract_bdd.rs` (bound with
`#[scenario(path = "tests/features/output_contract.feature", name = ...)]`; the
path resolves against `CARGO_MANIFEST_DIR`). Steps run the binary directly
through the isolated command builder; the environment step configures only the
child (`Command::env`).

```gherkin
Feature: cargo theorem output contracts
  Agents and humans rely on one versioned JSON document per invocation,
  stable exit classes, and renderer options that never corrupt JSON.

  Scenario Outline: JSON mode writes exactly one document to one stream
    Given an isolated working directory
    When I run cargo theorem with "<arguments>"
    Then the exit code is <code>
    And <stream> contains exactly one JSON document of kind "<kind>"
    And <other> is empty

    Examples:
      | arguments                                    | code | stream | other  | kind                         |
      | context --json                               | 0    | stdout | stderr | cargo-theorem.agent_context  |
      | --color sometimes context --json             | 2    | stderr | stdout | cargo-theorem.diagnostic     |
      | context --json --config missing.toml         | 3    | stderr | stdout | cargo-theorem.diagnostic     |
      | --width 5 context --json                     | 2    | stderr | stdout | cargo-theorem.diagnostic     |
      | --locale not_a_locale! context --json        | 2    | stderr | stdout | cargo-theorem.diagnostic     |
      | --quiet --verbose context --json             | 2    | stderr | stdout | cargo-theorem.diagnostic     |

  Scenario: An invalid choice enumerates the valid values
    Given an isolated working directory
    When I run cargo theorem with "--color sometimes context --json"
    Then the diagnostic lists the valid values "auto, always, never"

  Scenario Outline: NO_COLOR yields to an explicit colour choice
    Given an isolated working directory
    And the child environment sets "NO_COLOR" to "1"
    When I run cargo theorem with "<arguments>"
    Then stdout <presence> ANSI escape sequences

    Examples:
      | arguments              | presence         |
      | --help                 | contains no      |
      | --color always --help  | contains         |

  Scenario: Renderer options never alter a JSON document
    Given an isolated working directory
    When I run cargo theorem with "--color always --emoji always --locale fr-FR context --json"
    Then stdout is byte-identical to the output of "context --json"

  Scenario: A closed output stream is an internal error with one diagnostic
    Given an isolated working directory
    And stdout is redirected to a full device
    When I run cargo theorem with "context --json"
    Then the exit code is 10
    And stderr contains exactly one JSON document of kind "cargo-theorem.diagnostic"
```

The last scenario is compiled only on Linux. Also consolidate the end-to-end
snapshot matrix so that, by the end of this milestone, the following
transcripts exist: `json_success_context`, `json_failure_usage`,
`json_failure_configuration`, `json_failure_width`, `human_success_help`,
`human_failure_usage`, plus the help "Exit status" check folded into
`human_success_help`. The locale and coloured-help checks assert byte or
stripped-text equality instead of adding snapshots.

Acceptance: all scenarios pass, all snapshots reviewed and accepted, and the
three gates pass. End state: the roadmap success criterion is demonstrated at
the executable boundary. Recovery: revert.

### EP-M5: documentation, roadmap, and delivery

Assigned: every trace link.

- Write `docs/adrs/adr-0014-application-output-protocol-and-stream-ownership.md`
  recording decisions `D1` to `D5`, `D12` to `D14`, `D16`, and `D17`: envelope
  shape, the compatibility policy, the context exemption and `kind` dispatch
  rule, the emission and panic policy, exit-code closure and its scope, the
  spawn fence and the runner obligations owned by 4.4.1, and the absence of a
  proof in this slice. Use OrthoConfig's `JsonModeContract` vocabulary
  (`one_json_document`, `one_json_diagnostic`, `capture_preview_and_log`) to
  describe the contract, and follow the ADR format in
  `docs/documentation-style-guide.md`.
- Update `docs/cargo-theorem-cli-design.md`: §5 (the spawn fence; runner stays
  with 4.4.1), §8.2 (decisions `D7` to `D12`, the pre-configuration limitation
  for parse errors and help, per-stream colour), §8.3 (context exemption,
  compatibility policy, reserved names, field order, trailing newline,
  `command` for parse failures, code grammar, open `status` vocabulary), §8.4
  (closure scope, Clap normalization, panic and write-failure mapping,
  passthrough exception), §8.6 (the deferred bounded-capture obligations and
  head-versus-tail question), and §14 (exit statuses published in help), each
  referencing ADR-0014.
- Update `docs/cargo-theorem-cli-users-guide.md`: mark the JSON and exit-code
  section as implemented, add a "Renderer options" section (values, `NO_COLOR`
  and `CLICOLOR_FORCE` precedence, width bounds, locale, quiet and verbose),
  document the `kind`-first dispatch rule and the diagnostic codes, and note
  that `--help --json` is human output.
- Update `docs/developers-guide.md` §3.8: the module map and dependency
  direction, the rule that every byte goes through `emit`, how to add a
  diagnostic code and an end-to-end snapshot, the spawn fence and how a later
  runner adds its expectation, the crate-local `clippy.toml` mirror rule, and
  the re-use policy for `protocol` (contract types only; later commands build
  envelopes with it; no production `Deserialize`).
- Update `docs/contents.md` (ADR-0014 and this ExecPlan).
- Roadmap notes in `docs/roadmap.md`: add to task 4.4.1 "Owns the captured
  child-stream obligations deferred by 4.1.4 (ADR-0014): JSON-mode child output
  appears only in bounded captures, with truncation metadata"; add to task
  4.2.1 "Run `cargo metadata` with captured stdio through the runner or an
  adapter satisfying ADR-0014; never inherit stdio". Then mark task 4.1.4 done.
- Run `make fmt`, `make markdownlint`, `make nixie`, `make spelling`, and the
  three code gates; commit; push; mark the PR ready only on instruction.

## Concrete steps

Run every command from the repository root. Gates run sequentially, each with
output captured:

```bash
branch=$(git branch --show-current)
make check-fmt 2>&1 | tee "/tmp/check-fmt-theoremc-${branch}.out"
make lint 2>&1 | tee "/tmp/lint-theoremc-${branch}.out"
make test 2>&1 | tee "/tmp/test-theoremc-${branch}.out"
```

Focused red/green runs during a milestone:

```bash
cargo nextest run -p cargo-theorem envelope 2>&1 | tee /tmp/m1-theoremc.out
cargo nextest run -p cargo-theorem --test output_contract_bdd 2>&1 \
  | tee /tmp/m4-theoremc.out
```

Snapshot workflow: run the focused test with `INSTA_UPDATE=no` to see the red
failure; rerun with `INSTA_UPDATE=always`, inspect every new `.snap` file in
`git diff`, and commit only reviewed snapshots.

Spawn-fence prototype (`EP-M2`): add
`let _ = std::process::Command::new( "true").status();` to a production
function, run `make lint`, and expect a `clippy::disallowed_methods` error
citing the configured reason; remove the seed and rerun `make lint` to see it
pass.

## Validation and acceptance

The work is accepted when all of the following hold on the final commit:

- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, `make spelling`, and `make check-agent-native` pass.
- The end-to-end transcripts cover success and failure in both modes
  (`json_success_context`, `json_failure_usage`, `json_failure_configuration`,
  `json_failure_width`, `human_success_help`, `human_failure_usage`), the human
  configuration failure is covered at the renderer boundary, and all
  behavioural scenarios pass.
- `cargo theorem context --json` output is unchanged on success, apart from
  agent-context changes caused by typed enum values, which must be reviewed and
  recorded.
- `cargo theorem --color sometimes context --json` exits `2` with empty stdout
  and one diagnostic on stderr listing the valid values.
- On Linux, `cargo-theorem theorem context --json > /dev/full` exits `10` with
  one `output.write_failed` diagnostic on stderr.
- A seeded spawn in production code fails `make lint` (evidence recorded).
- Roadmap task 4.1.4 is marked done and the 4.2.1 and 4.4.1 notes are present.

## Idempotence and recovery

Every step is a source edit followed by gates; rerunning a gate is safe. Each
milestone ends with a commit, so `git revert <sha>` or a reset to the previous
milestone commit restores a coherent state. Snapshot acceptance is repeatable
because snapshots are reviewed in `git diff` before committing. The spawn-fence
seed is never committed.

## Artefacts and notes

Gate logs live under `/tmp` with the names shown in `Concrete steps`. Record
short transcripts of the key acceptance commands here as milestones complete,
including the seeded-spawn lint failure.

## Interfaces and dependencies

Crate-internal surface added to `cargo-theorem` (all `pub(crate)`; the JSON
documents are the public contract, not the Rust types):

```rust
pub(crate) mod protocol {
    pub(crate) const SCHEMA_VERSION: &str = "1";
    pub(crate) enum ExitClass { Success, Usage, Configuration, ProjectState,
        BackendUnavailable, VerificationMismatch, VerificationIncomplete,
        ExternalToolFailure, DeliveryFailure, Internal }
    pub(crate) struct DiagnosticCode(/* validated Cow<'static, str> */);
    pub(crate) struct Diagnostic { /* code, class, message_fallback, arguments, valid_values */ }
    pub(crate) struct Envelope<T> { /* schema_version, kind, command, outcome */ }
}
```

Other crate-internal interfaces: `emit::OutputStreams`, `emit::CommandOutput`,
`emit::plan_fallback`, `emit::panic_guard`, `render::RenderPolicy::resolve`,
`render::TerminalFacts`, and `cli::json_intent`.

Dependencies: no new runtime dependencies or features. New `[dev-dependencies]`
for `crates/cargo-theorem`: `insta`, `googletest`, `pretty_assertions`,
`proptest`, `rstest-bdd`, `rstest-bdd-macros`, at the versions already in
`Cargo.lock`. New configuration file: `crates/cargo-theorem/clippy.toml`.

## Revision notes

- 2026-11-16: initial draft from Wyvern reconnaissance and external prior art.
- 2026-11-16: applied the community-of-experts design review (verdict: proceed
  with conditions). Removed the process runner and Verus milestones (decision
  `D16`), replacing the subprocess evidence with a compile-time spawn fence
  (`D5`) and deferred obligations for task 4.4.1; made `protocol` crate-private
  without production `Deserialize` (`D15`); relaxed the status/exit coupling to
  one-way rules with an open status vocabulary (`D2`, `EXIT-1`); added the
  envelope compatibility policy and reserved names (`D13`); replaced the closed
  code enum with a validated newtype (`D14`); corrected Clap normalization for
  help on a missing subcommand (`D4`); made the panic hook record and report
  location and guarded against `panic = "abort"` (`D4`); ranked configuration
  files below `NO_COLOR` and made colour per stream (`D7`); resolved verbosity
  by precedence (`D8`); derived JSON-intent grammar from Clap (`D12`,
  `JSON-INTENT-1`); published exit statuses in help (`D17`); rewrote `STREAM-1`,
  `REND-1`, and `LOC-1` to avoid vacuous oracles and unsupported proptest
  classification; added the `/dev/full` test; folded the end-to-end cases into
  their milestones and reduced the plan to five milestones; and recalibrated
  the tolerances. Remaining work is unchanged in kind: approval, then `EP-M1`.
