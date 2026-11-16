# Task 4.1.4: versioned JSON application contracts for `cargo theorem`

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting design review and user approval)

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
   `schema_version`, `kind`, `command`, `status`, and exactly one of `result` or
   `error`.
2. Which bytes go to standard output (stdout) and which to standard error
   (stderr)? These are the *stream invariants*. In JSON mode a success writes
   exactly one JSON document to stdout and nothing to stderr; a failure writes
   nothing to stdout and exactly one JSON diagnostic to stderr. Output from any
   child process the application runs must never appear beside that document.
3. Which process exit codes can a caller rely on? These are the *stable exit
   classes*: `0` success, `2` usage, `3` configuration, `4` project state, `5`
   backend unavailable, `6` verification mismatch, `7` verification incomplete,
   `8` external tool failure, `9` delivery failure, and `10` internal. No other
   code is ever returned.
4. How do the global human-output options (`--color`, `--emoji`, `--progress`,
   `--accessibility`, `--plain`, `--no-pager`, `--width`, `--locale`, `--quiet`,
   `--verbose`) behave? These are the *global renderer options*. Task 4.1.2
   already parses and merges them as strings; nothing yet validates them fully
   or makes them affect output.

Today, the observable behaviour is partial. `cargo theorem context --json`
writes the agent context to stdout. A configuration failure in JSON mode writes
a hand-built diagnostic whose `command` field is hard-coded to `["context"]`. A
failure to write a diagnostic exits with code `1`, which is outside the
documented taxonomy. Clap's help and parse errors ignore `--color`, `NO_COLOR`,
and `--width`. Nothing prevents a future command from inheriting the terminal's
stdout into a child process.

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
an explicit choice overrides the `NO_COLOR` convention. The exact diagnostic
codes and messages above are illustrative; the implementation fixes them
through snapshots in milestone `EP-M2`.

The behaviour of `context --json` on success is preserved: it still prints
OrthoConfig's bare agent-context document (see decision `D1`). Deliberate
changes are the diagnostic shapes and codes, exit code `1` becoming `10`, the
rejection of invalid `--width` and `--locale` values, and colour and width now
applying to human output.

This work closes roadmap task 4.1.4 and fixes the contracts that tasks 4.2.3,
4.2.4, 4.5.1, and 8.3.2 build on.

## Constraints

- Do not begin implementation until the user explicitly approves this plan.
- Keep every change inside `crates/cargo-theorem/`, the new `verus/` and
  `tools/verus/` directories, `Makefile`, `.github/workflows/proofs.yml`, and
  documentation. The reusable libraries (`theoremc-core`, `theoremc-macros`,
  `theoremc-build-support`, and the root `theoremc` facade) must not change and
  must not gain application dependencies (ADR-0009, design §5).
- Do not implement any later command handler (`list`, `get`, `check`, `run`,
  `backend`, and so on). The registered command tree remains `context` only,
  and agent context must not advertise anything new (design §6.6).
- Do not fork or extend OrthoConfig's agent-context or documentation schemas
  (design §6.1).
- Machine-readable identifiers (schema versions, kinds, diagnostic codes, exit
  class names, status values, argument keys, valid values) are never localized
  (ADR-0002 decision 4; ADR-0009).
- Production code must not read the process environment except through the
  existing injected `ortho_config::EnvSource` seam created at the composition
  root (`process_sources()` in `crates/cargo-theorem/src/config/mod.rs`).
  Terminal detection is likewise probed once at the composition root and
  injected. Tests must not mutate process environment; child processes may be
  configured with `Command::env_clear` and `Command::env` (AGENTS.md
  "Subprocess isolation is the sole exemption").
- No source file may exceed 400 lines. `src/config_tests.rs` (350 lines),
  `tests/entry_points.rs` (330), and `src/config/mod.rs` (310) are close to the
  limit; new tests go in new files.
- Every module begins with a `//!` comment. Every `assert!`-family call carries
  a diagnostic message. No `unwrap`, `expect` outside tests, `print!`, or
  `eprint!` (workspace lints).
- Lints are not silenced. Item-scoped `#[expect(..., reason = "...")]` is the
  only permitted suppression.
- Authorized new dev-dependencies for `crates/cargo-theorem` only, all already
  present in `Cargo.lock`: `insta` 1.49 (with the `filters` feature if needed),
  `googletest` 0.14, `pretty_assertions` 1.4, `proptest` 1.12 (no default
  features, `std`), `rstest-bdd` 0.6, and `rstest-bdd-macros` 0.6. No new
  runtime dependency is authorized. `ortho_config::LanguageIdentifier` (a
  re-export of `unic-langid`) provides locale validation, and
  `std::io::IsTerminal` provides terminal detection.
- Verus is not a Cargo dependency. It is installed and run only through the
  pinned `rust-prover-tools` (`tools/rust-prover-tools/REF`), and `make verus`
  stays out of `make test`, `make lint`, and `make all`.
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
# Code: Rust sources, Verus proofs, Makefile, feature files, test fixtures.
git diff --numstat "$base" -- 'crates/cargo-theorem/**/*.rs' \
  'crates/cargo-theorem/tests/features/*.feature' 'verus/**' Makefile
# Configuration: manifests and CI workflows (Cargo.lock excluded as generated).
git diff --numstat "$base" -- 'crates/cargo-theorem/Cargo.toml' \
  'tools/verus/**' '.github/workflows/*.yml'
# Documentation, excluding ExecPlans.
git diff --numstat "$base" -- 'docs/**' ':!docs/execplans/**' '*.md'
# Snapshot files are counted by file number only.
git diff --name-only "$base" -- '**/snapshots/*.snap' | wc -l
```

Limits are net lines (additions minus deletions) and are enforced per category;
spare capacity in one category never offsets another.

- Files: if more than 45 non-ExecPlan files change (excluding `Cargo.lock` and
  `.snap` files), stop and escalate. Snapshot files: at most 24.
- Code: if Rust, Verus, Makefile, and feature-file changes exceed 3,400 net
  lines, stop and escalate. Within that, production Rust under
  `crates/cargo-theorem/src/` (excluding `*_tests.rs` and `tests` modules) must
  not exceed 1,400 net lines.
- Configuration: if manifest, pin, and workflow changes exceed 120 net lines,
  stop and escalate.
- Documentation: if documentation changes (excluding ExecPlans) exceed 900 net
  lines, stop and escalate.
- Interface: if any reusable library API must change, or the registered command
  tree must grow, stop and escalate.
- Dependencies: if any runtime dependency, or any dev-dependency beyond those
  listed in `Constraints`, is needed (including enabling clap's `wrap_help`
  feature, which pulls in `terminal_size`), stop and escalate.
- Architecture: if the stream invariant cannot be enforced without a
  process-global mechanism other than the panic hook installed at the
  composition root, or if `context --json` would need to change shape, stop and
  escalate.
- Proof tooling: if `prover-tools verus install` cannot install a pinned Verus
  on Linux x86-64 after two attempts, stop and escalate rather than writing a
  shell installer.
- Iterations: if a milestone's gates still fail after five fix attempts, stop
  and escalate.
- Time: if a milestone takes more than one working day, stop and escalate.
- Ambiguity: if the OrthoConfig derive cannot carry typed renderer enums and the
  string fallback (decision `D6`) changes observable behaviour, stop and
  present options.

Budget derivation. The survey found no comparable history of budget increases:
the 4.1.2 plan set no line budget, and plans 3-1-2 (700 net lines), 3-2-2
(600), 3-3-1 (500), and 3-3-2 (500) record no raises; 3-3-2 instead split a
test file after the fact to respect the 400-line rule. This task is larger than
those: an estimate from the planned modules is about 1,250 net lines of
production Rust (protocol types 260, renderer policy and human rendering 320,
emission 180, process capture 330, rewiring of `cli.rs`, `cli_diagnostics.rs`,
`error.rs`, and `config/` about 160), about 1,750 lines of tests (unit 900,
end-to-end 350, behavioural 300, property 200), and about 200 lines of Verus.
The code budget adds roughly 15% contingency to that 3,200-line estimate.
Documentation covers one ADR (about 160 lines), design-document updates (about
180), the users' guide (about 180), the developers' guide (about 200), and
index entries; 900 lines leaves about 20% contingency.

## Risks

- Risk: the OrthoConfig v0.10.0 derive may not accept enum-typed fields with
  `clap::ValueEnum` on `GlobalOptions`. Severity: medium. Likelihood: medium.
  Mitigation: milestone `EP-M3` starts with a prototype step that converts
  `color` only and runs the existing precedence tests; fallback decision `D6`
  keeps `Option<String>` fields and converts in `resolve()`.
- Risk: without clap's `wrap_help` feature, `Command::term_width` may not wrap
  help text. Severity: low. Likelihood: high. Mitigation: `--width` is defined
  to bound cargo-theorem's own human prose; help wrapping is applied only if
  the prototype shows clap honours it without new dependencies (decision `D9`).
- Risk: the re-execution test that proves file-descriptor-level non-leakage may
  be flaky under nextest or slow. Severity: medium. Likelihood: low.
  Mitigation: the child is the unit-test binary itself, run with
  `--ignored --exact`, sentinel strings make the assertion independent of
  libtest's banner, and the test has a bounded deadline.
- Risk: the shared `install-prover-tools` action or `prover-tools` may not
  support Verus on the pinned reference. Severity: medium. Likelihood: medium.
  Mitigation: `EP-M5` begins by running `prover-tools verus install` locally;
  the proof tolerance escalates instead of writing an installer.
- Risk: the Verus model may drift from the executable kernel. Severity: high.
  Likelihood: medium. Mitigation: the kernel is tiny, the Verus file documents
  which production function each item mirrors, and a proptest refinement test
  checks the production kernel against a plain-Rust reference model built from
  the same specification functions.
- Risk: a grandchild process that inherits a pipe can keep a reader thread
  blocked after the child is killed. Severity: medium. Likelihood: low in this
  slice (no production caller spawns processes yet). Mitigation: reader threads
  are joined with a bounded grace period; on expiry the capture is reported
  incomplete and the threads are detached. Process-group ownership is deferred
  to the run lifecycle (design §11.2).
- Risk: insta snapshot churn from the agent-context document, which contains
  the package version. Severity: low. Likelihood: medium. Mitigation: filter
  the version string and keep the context snapshot to the stream/exit
  transcript plus a parsed summary of command paths.

## Progress

- [x] (2026-11-16 18:40Z) Renamed the branch, set its upstream to
  `origin/4-1-4-versioned-json-application-contracts`, and titled the Lody
  session.
- [x] (2026-11-16 19:10Z) Reconnaissance: Wyvern team surveyed tests, docs,
  plan history, and the OrthoConfig 0.10.0 sources; external prior art checked.
- [x] (2026-11-16 19:40Z) Initial draft written.
- [ ] Community-of-experts design review applied.
- [ ] Plan approved by the user.
- [ ] `EP-M1` protocol types and exit classes.
- [ ] `EP-M2` emission and stream discipline.
- [ ] `EP-M3` global renderer options.
- [ ] `EP-M4` captured subprocess boundary.
- [ ] `EP-M5` Verus proof of bounded capture.
- [ ] `EP-M6` behavioural and end-to-end contract suite.
- [ ] `EP-M7` documentation, roadmap, and delivery.

## Surprises & discoveries

- Observation: OrthoConfig 0.10.0 and cargo-orthohelp 0.10.0 provide no
  envelope, exit-class, renderer-option, or terminal-detection types. Evidence:
  registry source search of both crates; the cargo-orthohelp README lists
  `--json` summaries and stable exit classes as planned. Impact: these
  contracts are application-owned in this slice.
- Observation: OrthoConfig's upstream agent-native design (§6.2 "Structured
  output") already defines the same JSON-mode contract and names a planned
  `JsonModeContract` with `subprocess_output_policy` choices
  `capture_preview_and_log`, `inherit_only_in_human_mode`, and `forbidden`. Its
  global-option glossary lives in `cargo_orthohelp::policy::vocabulary`, but
  not in the 0.10.0 release. Evidence: `docs/agent-native-cli-design.md` on
  OrthoConfig's `main` branch; `cargo-orthohelp-0.10.0/src/policy/vocabulary/`
  has only verbs and flags. Impact: the local glossary is a shadowing adapter
  under design §6.1 and must name the OrthoConfig item it shadows.
- Observation: OrthoConfig's `AgentCommand.output_modes` exists but
  `bridge_ir_to_agent_context` always leaves it empty, and the agent-context
  schema has no exit-code field. Evidence:
  `cargo-orthohelp-0.10.0/src/agent_context/mod.rs:219`. Impact: populating
  output metadata is deferred to 4.5.1, which owns command output metadata.
- Observation: `cargo_metadata::MetadataCommand::exec` inherits stderr when
  `verbose(true)` is set. Evidence: `cargo_metadata-0.23.1/src/lib.rs:1231`.
  Impact: task 4.2.1 must build the command with `cargo_command()`, run it
  through the captured-process boundary from `EP-M4`, and parse the captured
  stdout with `MetadataCommand::parse`.
- Observation: no Verus wiring exists in the repository; Kani is wired through
  `make kani` and `.github/workflows/proofs.yml`. Impact: `EP-M5` is the
  repository's first Verus on-ramp.

## Decision log

- D1. Decision: `cargo theorem context --json` keeps printing OrthoConfig's
  bare agent-context document on success; it is not wrapped in the application
  envelope. Its failures use the application diagnostic envelope like every
  other command. Rationale: design §14 states that the command emits
  OrthoConfig's versioned agent-context document, OrthoConfig's ADR-0007
  reserves `context --json` as the canonical downstream introspection command,
  and the document already carries its own `schema_version` and `kind`.
  Wrapping it would make cargo-theorem the only OrthoConfig application whose
  context needs unwrapping. The strongest alternative, wrapping it under
  `result` to satisfy design §8.3's "All application JSON documents use a
  versioned envelope", gives uniform parsing but breaks generic OrthoConfig
  tooling. Design §8.3 will name this as the single exemption. This decision
  changes a public contract and must be confirmed at plan approval.
- D2. Decision: the envelope status is derived from a Rust enum body
  (`Outcome::Result(T)` or `Outcome::Error(Diagnostic)`), not stored
  separately, so `result`/`error` exclusivity and status agreement hold by
  construction. Rationale: makes the invalid states unrepresentable instead of
  validated.
- D3. Decision: in JSON mode, the complete document is serialized to an
  in-memory buffer before any byte is written, then written with one
  `write_all` and `flush`. If writing the primary stream fails after zero bytes
  reached it, one internal diagnostic is attempted on stderr; if any byte
  reached it, nothing further is written anywhere. Either way the exit class is
  `internal` (10). Rationale: guarantees that at most one stream ever carries
  bytes, even under partial writes and `EPIPE`.
- D4. Decision: no process exit code outside `{0, 2, 3, 4, 5, 6, 7, 8, 9, 10}`
  is ever returned. Clap's codes are normalized (help and version to `0`, every
  parse failure to `2`); diagnostic write failures map to `10` instead of `1`;
  a panic is caught at the application boundary, reported as
  `application.panic`, and mapped to `10`. The composition root installs a
  panic hook that suppresses Rust's default panic message so that JSON-mode
  stderr remains one document. Rationale: design §8.4 promises a stable
  taxonomy; codes `1` and `101` are outside it.
- D5. Decision: the captured-subprocess boundary (a driven port plus a std
  adapter) lands in this slice as `crates/cargo-theorem/src/process/`, and it
  is the only sanctioned way for cargo-theorem to start a child process. Its
  adapter always pipes stdout and stderr into bounded captures and connects
  stdin to null; there is no inheriting mode. Rationale: the roadmap success
  criterion requires that subprocess output cannot leak beside a JSON document;
  tasks 4.2.1 (`cargo metadata`), 4.2.4, and 4.4.x are the concrete next
  callers. Design §5 assigns the command-execution abstraction to the future
  `theoremc-runner`; until that package exists, this module is the
  feature-grouped precursor and moves there when it is created. Human-mode
  callers that want live output forward captured chunks through the
  application's stderr writer; that forwarding is deferred to the first caller
  that needs it.
- D6. Decision: renderer policies become enums (`ColorPolicy`, `EmojiPolicy`,
  `ProgressPolicy` with `auto|always|never`; `AccessibilityPolicy` with
  `auto|on|off`) at the `GlobalOptions` field level if the OrthoConfig derive
  supports it, so Clap rejects invalid values with an enumerated usage error
  and file values are rejected during deserialization. Fallback: keep
  `Option<String>` fields and convert in `GlobalOptions::resolve()`, which
  already validates them. Rationale: typed values remove string matching from
  the renderer and let agent context enumerate choices.
- D7. Decision: effective colour is resolved in this order: JSON mode disables
  all decoration; an explicit `always` or `never` from any source (flag,
  environment variable `THEOREMC_COLOR`, or configuration file) wins; else
  `--plain` disables colour; else a non-empty `NO_COLOR` disables colour; else
  a non-empty `CLICOLOR_FORCE` other than `0` enables colour; else `TERM=dumb`
  disables colour; else colour follows whether the target stream is a terminal.
  Emoji and progress follow the same shape without the
  `NO_COLOR`/`CLICOLOR_FORCE` steps; `--accessibility on` and `--plain` disable
  emoji and animated progress unless explicitly `always`; `auto` accessibility
  is treated as `off` because no reliable detection exists. Rationale: the
  no-color.org convention says user configuration and flags override
  `NO_COLOR`; the CLICOLOR convention says `NO_COLOR` overrides
  `CLICOLOR_FORCE`.
- D8. Decision: `--quiet` and `--verbose` are mutually exclusive. On the
  command line Clap reports a usage error; when the conflict arises from merged
  files or environment, it is a configuration error. Quiet suppresses
  non-essential human output but never diagnostics or JSON documents. Verbose
  adds the error source chain to human diagnostics. Neither changes JSON
  documents. Rationale: an explicit conflict is clearer than a hidden
  precedence rule.
- D9. Decision: `--width` must lie in `20..=1000` columns. It bounds
  cargo-theorem's own human prose and is passed to Clap's help rendering only if
  `EP-M3`'s prototype shows Clap honours it without the `wrap_help` feature.
  Rationale: the design gives no bounds; out-of-range widths produce unreadable
  or absurd output, and an error that states the range teaches the caller.
- D10. Decision: `--locale` must parse as a Unicode language identifier through
  `ortho_config::LanguageIdentifier`. The value is recorded in the render
  policy and passed to the application localizer, which remains `NoOpLocalizer`
  until translated catalogues land. JSON output is byte-for-byte independent of
  locale. Rationale: ADR-0002 decisions 1 and 4.
- D11. Decision: no pager is launched in this slice. `--no-pager` is resolved
  and recorded, and the policy always reports paging disabled. Rationale: a
  pager is a subprocess with an inherited terminal; it belongs with the first
  command that produces long human output.
- D12. Decision: `--json` stays a per-command flag on data-returning commands,
  not a global flag. JSON intent for parse failures is detected by a token scan
  for `--json` before any `--` separator, ignoring tokens consumed as values of
  value-taking global options. Help and version output are always human.
  Rationale: per-command placement keeps agent context truthful about which
  commands support JSON; the scan generalizes today's `context`-only heuristic.
- D13. Decision: an optional `warnings` array is reserved in the envelope
  contract but not implemented; its first producer (the legacy-variable
  migration adapter in task 4.4.5) adds it. Adding an optional field is a
  compatible change. Rationale: avoids an unused field now.
- D14. Decision: diagnostic codes are lowercase dotted identifiers whose first
  segment names an area (`cli`, `configuration`, `renderer`, `output`,
  `application`, `process`). Codes are declared once in a closed Rust enum with
  a `const fn as_str`, so the full code list is enumerable in tests and
  documentation. Rationale: stable, greppable, non-localized identifiers
  (ADR-0002 decision 2).
- D15. Decision: Verus is applied to exactly one obligation, the bounded
  capture accumulator (`BC-1` to `BC-3`). Everything else is finite and is
  established by exhaustive enumeration, property tests, snapshots, and
  end-to-end tests. Rationale: see `Verification plan`.

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
  `&mut dyn Write` for stdout and stderr), `execute`, `run_selected`, and the
  helpers that rebuild a configuration-only argument list.
- `cli_diagnostics.rs` (178 lines) renders Clap errors and application errors,
  scans argv for JSON intent (`parse_error_requests_json`), and hand-builds the
  JSON diagnostic with `serde_json::json!` (`write_json_diagnostic`, which
  hard-codes `"command": ["context"]`).
- `error.rs` (101 lines) defines `ApplicationError` (`Configuration`,
  `Serialization`, `Output`) and `exit_code()` mapping to `3` and `10`.
- `config/mod.rs` (310 lines) defines `ResolvedSettings`, `GlobalOptions`
  loading (`load_with_sources`), `resolve()` with string policy validation
  (`POLICY_VALUES`, `ACCESSIBILITY_POLICY_VALUES`), the `cmds` namespace
  validation, `process_sources()` (the environment composition root), and the
  selected-subcommand adapter `ResolvedCommand`.
- `config/global_options.rs` (93 lines) holds the `GlobalOptions` struct with
  the OrthoConfig derive; it is the only code that reads the process
  environment, under a documented `#[expect(clippy::disallowed_methods)]`.
- `agent_context.rs` (86 lines) builds the agent context from the Clap tree via
  `cargo_orthohelp::agent_context::bridge_ir_to_agent_context`.
- Tests: `cli_tests.rs` and `cli_tests/runner.rs` drive
  `run_with_sources_and_io` in-process with `ortho_config::MapEnv` sources and a
  `FailingWriter`; `config_tests.rs` covers precedence;
  `tests/entry_points.rs` with
  `tests/entry_points/{process,support,agent_context}.rs` spawns the real binary
  (`env!("CARGO_BIN_EXE_cargo-theorem")`) with `env_clear()` and a temporary
  working directory, checking exit codes and streams with `anyhow::ensure!`.

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
`make spelling` cover documentation.

Terms used in this plan:

- *Envelope*: the outer JSON object of every command result or diagnostic.
- *Kind*: a dotted, non-localized identifier naming the document type, such as
  `cargo-theorem.diagnostic`.
- *Exit class*: the named category of a process exit code, such as `usage`.
- *Render policy*: the effective, fully resolved set of human-output decisions
  (colour on or off, emoji on or off, and so on) computed from options,
  environment, and terminal facts.
- *Captured child*: a child process whose stdout and stderr are pipes read by
  the application into bounded buffers, never the terminal.
- *Bounded capture*: an accumulator that keeps at most `limit` bytes of a
  stream while counting every byte seen, recording `bytes_seen`,
  `bytes_retained`, `limit_bytes`, and `truncated` (design §8.6).
- *Verus*: a deductive verifier for Rust that proves specifications with the Z3
  solver; proofs live in files compiled by the `verus` tool, not by Cargo.

Signposted documentation, all repository-relative:

- `docs/cargo-theorem-cli-design.md` §§5, 6.5, 6.6, 8.1-8.6, 14, 15, 17, 18
  (the governing design).
- `docs/cargo-theorem-cli-users-guide.md` "JSON output and exit codes" (to be
  updated).
- `docs/adrs/adr-0002-library-first-internationalization-and-localization-with-fluent.md`
  decisions 2 and 4; `docs/adrs/adr-0009-orthoconfig-cli-configuration.md`.
- `docs/developers-guide.md` §3.8 (the CLI package) and the environment
  injection section.
- `docs/ortho-config/users-guide.md` (agent-native conventions,
  `context --json`), `docs/ortho-config/cargo-orthohelp-design.md`.
- `docs/rust-testing-with-rstest-fixtures.md`, `docs/rstest-bdd-users-guide.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-doctest-dry-guide.md`,
  `docs/complexity-antipatterns-and-refactoring-strategies.md`,
  `docs/ortho-config-users-guide.md` if present, else
  `docs/ortho-config/users-guide.md`, `docs/documentation-style-guide.md`.
- `docs/theoremc-design.md` (DES) and `docs/theorem-file-specification.md`
  (TFS) are background only; this task changes neither theorem semantics nor
  file format.

Signposted skills: `execplans`, `rust-router`, `rust-errors`,
`rust-types-and-apis`, `domain-cli-and-daemons`, `rust-unit-testing`,
`proptest`, `verus`, `rust-verification`, `hexagonal-architecture`,
`codegraph-mcp` (the CodeGraph server was unavailable during planning; use
`leta` or `rg` as fallbacks), `en-gb-oxendict-style`, and `commit-message`.

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
  cannot leak beside a JSON document.
- `docs/cargo-theorem-cli-design.md`:
  - `DES-CLI-8.2-STREAMS`: JSON-mode stream rules and the renderer option list.
  - `DES-CLI-8.3-ENVELOPE`: envelope fields, versioning, error shape.
  - `DES-CLI-8.4-EXIT`: the exit-class table.
  - `DES-CLI-8.6-BOUNDS`: truncation metadata for retained streams.
  - `DES-CLI-6.6-SCOPE`: no unimplemented handler is advertised.
  - `DES-CLI-5-BOUNDARY`: application dependencies stay out of libraries; the
    command-execution abstraction belongs to the runner layer.
  - `DES-CLI-18-TESTS`: snapshot, fake command-runner, and end-to-end layers.
- ADR-0002 decision 4 (`ADR2-D4`): machine-facing output uses stable codes and
  structured fields. Decision 2 (`ADR2-D2`): code, arguments, English fallback.
- ADR-0009 (`ADR9`): non-localized identifiers; injected sources.
- `AGENTS.md`: environment injection, 400-line files, lint and test policy.
- No Terms of Reference document exists for this project; the roadmap and the
  design document serve as the requirement baseline.

Trace links:

- `RM-4.1.4` + `DES-CLI-8.3-ENVELOPE` + `ADR2-D2` -> `EP-M1` -> envelope
  property tests and serialized-form snapshots.
- `DES-CLI-8.4-EXIT` -> `EP-M1`, `EP-M2` -> exhaustive exit-class test,
  end-to-end exit assertions.
- `RM-4.1.4` + `DES-CLI-8.2-STREAMS` -> `EP-M2` -> emission enumeration test,
  failing-writer property test, transcript snapshots.
- `DES-CLI-8.2-STREAMS` (renderer options) + `ADR2-D4` -> `EP-M3` -> render
  policy enumeration test, human snapshots, locale-independence test.
- `RM-4.1.4` (subprocess) + `DES-CLI-8.6-BOUNDS` + `DES-CLI-5-BOUNDARY` ->
  `EP-M4`, `EP-M5` -> bounded-capture property test, Verus proof, re-execution
  non-leak test, architecture fitness test.
- `RM-4.1.4` (snapshots) + `DES-CLI-18-TESTS` -> `EP-M6` -> end-to-end
  snapshot matrix and `rstest-bdd` scenarios.
- All of the above -> `EP-M7` -> ADR-0014, design, guides, roadmap.

Deviation to be recorded upstream: decision `D1` narrows design §8.3's "All
application JSON documents use a versioned envelope" by exempting the `context`
success document. `EP-M7` amends §8.3 and records the decision in ADR-0014;
approval of this plan is approval of that amendment.

## Verification plan

This section lists each obligation, its method, and why the method can fail
when the implementation is wrong.

### Invariants and lemmas

`ENV-1` envelope well-formedness. Every serialized envelope is a JSON object
whose keys, in order, are `schema_version`, `kind`, `command`, `status`, then
exactly one of `result` or `error`; `status` is `"success"` exactly when
`result` is present and `"error"` exactly when `error` is present;
`schema_version` is `"1"`; a diagnostic `kind` is `cargo-theorem.diagnostic`; a
success kind is `cargo-theorem.<command path joined by ".">.result`.

- Method: construction by type (decision `D2`), plus a proptest that generates
  envelopes (arbitrary command paths from a small alphabet, arbitrary JSON
  result values, arbitrary diagnostics including every `DiagnosticCode`),
  serializes them, parses the bytes with `serde_json::Value`, and checks the
  rule with an independent validator function written against the JSON, not the
  Rust types. Snapshots fix one success and one error serialization.
- Artefact: `crates/cargo-theorem/src/protocol/envelope_tests.rs`.
- Non-vacuity: the generator's `prop_oneof!` covers both outcomes, recorded
  with `prop_assert!` counts through `proptest`'s classification (or explicit
  counters reported on failure); a negative control feeds the validator a
  hand-written object containing both `result` and `error` and asserts
  rejection, and another with `status: "success"` plus `error`.
- Discharge: `cargo nextest run -p cargo-theorem envelope` passes; the negative
  controls fail validation.

`EXIT-1` exit taxonomy. The map from `ExitClass` to process code is a bijection
onto `{0, 2, 3, 4, 5, 6, 7, 8, 9, 10}`; the class names are the design §8.4
names; `status == success` if and only if the exit code is `0`; no application
path returns another code.

- Method: exhaustive test over `ExitClass::ALL` (a `const` array whose length
  is checked against the enum by a `match` with no wildcard, so a new variant
  fails to compile until added); rstest table of Clap error kinds to `2` and
  help/version to `0`; unit test of the panic guard with a panicking closure;
  end-to-end assertions on every snapshot transcript.
- Non-vacuity: the panic test proves the guard sees a real panic (the closure
  panics, the guard returns `internal`); a negative control maps a writer
  failure and asserts `10`, not `1`.
- Discharge: tests in `protocol/exit_class_tests.rs`, `cli_tests/`, and
  `tests/output_contract.rs` pass.

`STREAM-1` JSON-mode emission. For every outcome (success or failure of any
class) and every primary-stream write fault (none; fails before any byte; fails
after `k >= 1` bytes), the bytes written satisfy: success with no fault puts
exactly one document plus one newline on stdout and nothing on stderr; failure
with no fault puts nothing on stdout and exactly one diagnostic plus one
newline on stderr; a fault after `k >= 1` bytes leads to no further bytes on
either stream; a fault before any byte on stdout leads to exactly one
`output.write_failed` diagnostic on stderr; every faulted case exits `10`.
Consequently, stdout and stderr are never both non-empty.

- Lemma `STREAM-L1`: the counting writer reports exactly the number of bytes
  the inner writer accepted (partial writes included).
- Lemma `STREAM-L2`: the emission decision depends only on (mode, outcome
  class, fault position class) where the position class is `none`, `zero`, or
  `partial`.
- Method: a pure decision kernel
  `plan_fallback(bytes_accepted: u64) -> Fallback` plus exhaustive enumeration
  over the finite abstraction (2 modes × 10 outcome classes × 3 fault
  positions, with both streams faulting independently, 240 cases); a proptest
  with a scripted writer that accepts a random number of bytes per call and
  fails at a random offset, run against real envelopes, asserting the invariant
  on captured bytes (this exercises `STREAM-L1` and the real serializer).
- Non-vacuity: the generator classifies cases into `none`/`zero`/`partial`
  and fails if any class has zero hits; a seeded fault (a test-only emitter
  variant that writes a diagnostic after a partial write) must be rejected by
  the invariant checker.
- Discharge: `emit_tests.rs` passes.

`STREAM-2` no inherited child streams. No code path in `cargo-theorem` starts a
child process whose stdout or stderr is the application's own stdout or stderr.

- Method: (a) structural — the only constructor of child processes is
  `process::StdProcessRunner`, which always sets `Stdio::piped()` for stdout
  and stderr and `Stdio::null()` for stdin; (b) an architecture fitness test
  that scans `crates/cargo-theorem/src/**/*.rs` and fails if
  `std::process::Command`, `Command::new`, `Stdio::inherit`, or
  `process::Stdio` appear outside `src/process/`; (c) a re-execution test: the
  unit-test binary re-runs itself
  (`--ignored --exact process::tests::leak_probe_child --nocapture`); the child
  runs the runner against a grandchild (the same test binary in a third mode)
  that writes unique sentinel strings to its stdout and stderr, then emits one
  JSON envelope through the real emitter; the parent asserts that the sentinels
  appear in the child's captured data (reported inside the envelope) but
  nowhere in the child's actual stdout or stderr, and that the child's stdout
  contains exactly one envelope line.
- Non-vacuity: the positive control (sentinels present in captured output)
  shows the grandchild ran and wrote; a negative control runs the grandchild
  with inherited streams through plain `std::process::Command` in the child
  test (test code is exempt from the fitness scan) and asserts the sentinels
  *do* appear, proving the detector can see leaks.
- Discharge: `process/leak_tests.rs` and `architecture_tests.rs` pass.

`REND-1` render policy resolution. For every combination of mode (human, JSON),
each policy value (`auto`, `always`, `never`, unset), `--plain` (unset, true),
accessibility (`auto`, `on`, `off`), `NO_COLOR` (unset, empty, non-empty),
`CLICOLOR_FORCE` (unset, `0`, `1`), `TERM` (unset, `dumb`, `xterm`), and
terminal status (true, false), the resolved policy follows decision `D7`. In
particular, JSON mode yields colour, emoji, progress, and paging all off; an
explicit `never` yields off; an explicit `always` yields on in human mode
regardless of environment.

- Method: exhaustive enumeration with an iterator product over the finite
  domain (about 5,000 cases) comparing `RenderPolicy::resolve` with an
  independently written oracle that encodes `D7` as an ordered rule list; plus
  named rstest cases for each rule, and human-output snapshots with colour on
  and off.
- Non-vacuity: the enumeration asserts that each oracle rule was the deciding
  rule for at least one case; a mutation control swaps the `NO_COLOR` and
  explicit-flag steps in a test-local copy of the rule list and asserts that
  the comparison then fails.
- Discharge: `render/policy_tests.rs` passes.

`CFG-1` renderer value validation. Out-of-range widths, malformed locales,
unknown policy values, and the quiet/verbose conflict are rejected with the
correct exit class (`usage` on the command line, `configuration` from files or
environment) and a diagnostic whose `valid_values` or `arguments` names the
accepted set or range.

- Method: rstest tables over CLI, environment (`MapEnv`), and file sources;
  boundary cases `19`, `20`, `1000`, `1001` for width.
- Discharge: `config/renderer_tests.rs` passes.

`LOC-1` locale independence of JSON. The JSON document produced for a fixed
outcome is byte-identical under `--locale en-US`, `--locale fr-FR`, and no
locale.

- Method: in-process test comparing bytes; end-to-end snapshot reuses one
  snapshot for all three runs.
- Non-vacuity: the human-mode rendering under the same inputs records the
  locale in verbose output, showing the option was applied.

`BC-1` to `BC-3` bounded capture. For a capture with limit `L` that has been
fed chunks `c_1 ... c_n` (each an arbitrary byte sequence), with
`S = c_1 ++ ... ++ c_n`:

- `BC-1`: `bytes_seen == |S|`.
- `BC-2`: `retained == S[0 .. min(|S|, L)]` (the retained bytes are exactly
  the prefix of the stream), so `bytes_retained == min(|S|, L) <= L`.
- `BC-3`: `truncated` holds exactly when `|S| > L`.

These are properties over unbounded sequences of unbounded chunks; sampled
tests cannot cover them exhaustively, and every later retained stream (backend
logs, artefacts) will reuse the accumulator, so they are selected for Verus
(decision `D15`).

Proof sketch (provisional; lemma boundaries may change when the verification
conditions are seen):

- Abstract model: state `(retained: Seq<u8>, seen: nat)` with parameter
  `limit: nat`; `spec fn flatten(chunks: Seq<Seq<u8>>) -> Seq<u8>`; the
  invariant
  `inv(state, history) := seen == flatten(history).len() &&
  retained == flatten(history).take(min(seen, limit))`.
- Precondition of `push(chunk)`: `inv(state, history)` and `seen + |chunk|`
  does not overflow `u64` in the executable mirror.
- Postcondition: `inv(state', history.push(chunk))`.
- Candidate lemma `lemma_take_concat_short`: if `k <= |a|` then
  `(a + b).take(k) == a.take(k)`.
- Candidate lemma `lemma_take_concat_long`: if `|a| <= k <= |a| + |b|` then
  `(a + b).take(k) == a + b.take(k - |a|)`.
- Candidate lemma `lemma_flatten_push`: `flatten(h.push(c)) == flatten(h) + c`.
- Main lemma `lemma_push_preserves_inv`, then `lemma_inv_from_empty` and an
  induction over the chunk history giving `BC-1` to `BC-3` for any sequence.
- Representation relationship: the executable `BoundedCapture` holds a
  `Vec<u8>` (`retained`), a `u64` (`seen`), and a `u64` (`limit`); the Verus
  file mirrors `push` as an `exec` function over `Vec<u8>` whose `ensures`
  clause states the invariant over the ghost history, and documents which
  production function it mirrors.

Bridge to production: a proptest refinement test runs the production
`BoundedCapture::push` and a plain-Rust reference model (computing
`flatten(history).take(min(|S|, L))` directly) over generated chunk sequences
and limits, including `L = 0`, `L` equal to a chunk boundary, and empty chunks,
and asserts equal observable state after every push. Non-vacuity: the generator
classifies runs as untruncated, exactly-at-limit, and truncated and fails if a
class is empty; a seeded mutant (retaining the *last* `L` bytes) is rejected.
For Verus, non-vacuity is checked by confirming that `requires` clauses are
satisfiable (a `proof fn` constructs a concrete two-chunk witness and calls the
main lemma) and by inspecting that the proof file contains no `assume`, `admit`,
`external_body`, or `#[verifier::external]` items (a `grep` in the
`make verus` recipe fails the target if any appear).

Discharge: `make verus` exits `0` with Verus reporting the expected number of
verified items and zero errors; the refinement proptest passes.

### Axioms (trusted external behaviour)

- `serde_json` serializes struct fields in declaration order and produces valid
  JSON for `serde_json::Value` and derived structs.
- `std::process::Command` with `Stdio::piped()` gives the child new pipe file
  descriptors and never the parent's stdout or stderr; `Stdio::null()` gives
  the null device.
- `std::io::Write::write` returns the number of bytes accepted; `write_all`
  stops at the first error.
- Clap's `Error::kind()` and `Error::get(ContextKind::...)` expose the invalid
  argument, value, and valid values for value-validation errors.
- `std::io::IsTerminal` reports whether a handle is a terminal.
- Cargo runs an external subcommand with the caller's stdio and passes the exit
  code through unchanged.
- Verus and Z3 are sound for the fragment used (sequences, `nat`, `u64`
  arithmetic with explicit overflow preconditions).

### Where conventional testing suffices

Exit-class mapping, envelope shape, render policy resolution, and emission
decisions are finite and are enumerated exhaustively in ordinary tests; a
prover would add tooling cost without adding coverage. Human-output formatting,
Clap integration, and the process adapter are integration plumbing covered by
snapshots, end-to-end tests, and behavioural scenarios. Kani is not used: no
`unsafe`, arithmetic overflow, or bounded state machine here warrants it beyond
the enumeration above.

## Plan of work

Implementation proceeds through seven milestones. Each ends with
`make check-fmt`, `make lint`, and `make test` passing (logs captured with
`tee` under `/tmp`, as in `Concrete steps`) and a commit. Each milestone
follows Red-Green-Refactor: write or extend the failing test first, run it to
see the expected failure, make the smallest change, rerun, then refactor.

### EP-M1: protocol types and exit classes

Assigned: `RM-4.1.4`, `DES-CLI-8.3-ENVELOPE`, `DES-CLI-8.4-EXIT`, `ADR2-D2`,
`ADR2-D4`.

Add a public `protocol` module to the `cargo-theorem` library (public because
integration tests and the next command slices deserialize envelopes with it):

- `crates/cargo-theorem/src/protocol/mod.rs`: `SCHEMA_VERSION`,
  `Envelope<T>` (fields `schema_version`, `kind`, `command: Vec<String>`, and a
  flattened `Outcome<T>` enum serialized as `result` or `error`, with a derived
  `status`), `Diagnostic` (`code: DiagnosticCode`, `class: ExitClass`,
  `message_fallback: String`, `arguments: BTreeMap<String, serde_json::Value>`
  omitted when empty, `valid_values: Option<Vec<String>>` omitted when absent),
  and constructors `Envelope::success(command, kind_path, result)` and
  `Envelope::failure(command, diagnostic)`. `Deserialize` is implemented so
  tests and later consumers can parse documents.
- `crates/cargo-theorem/src/protocol/exit_class.rs`: `ExitClass` with the ten
  variants, `ALL`, `const fn code(self) -> u8`, `const fn as_str(self)`,
  `fn exit_code(self) -> std::process::ExitCode`, serde as snake_case strings.
- `crates/cargo-theorem/src/protocol/code.rs`: closed `DiagnosticCode` enum
  with `as_str` and serde as its dotted string (decision `D14`). Initial codes:
  `cli.usage`, `cli.invalid_value`, `cli.unknown_argument`,
  `cli.missing_argument`, `cli.argument_conflict`, `configuration.invalid`,
  `configuration.not_found`, `configuration.invalid_value`,
  `renderer.invalid_width`, `renderer.invalid_locale`,
  `renderer.conflicting_verbosity`, `output.write_failed`,
  `application.serialization`, `application.panic`. The final list is fixed by
  the snapshots; unused codes are not added.

Rewire `error.rs` so `ApplicationError::exit_class()` returns `ExitClass` and
`ApplicationError::diagnostic()` returns a `Diagnostic`, and replace
`write_json_diagnostic` in `cli_diagnostics.rs` with envelope serialization
carrying the real command path (decision `D12`'s scan supplies the path for
parse failures; it is empty when no command was recognized).

Red: add `protocol/envelope_tests.rs` (validator, proptest, negative controls,
two insta snapshots of serialized forms) and `protocol/exit_class_tests.rs`;
they fail to compile, then fail on the missing behaviour. Green: implement.
Acceptance: `ENV-1` and `EXIT-1` (mapping half) discharged; the existing
end-to-end tests still pass with updated codes. End state: the protocol types
exist and every existing JSON diagnostic uses them. Recovery: revert the
milestone commit. Remaining gaps: emission discipline, renderer, processes.

### EP-M2: emission and stream discipline

Assigned: `RM-4.1.4`, `DES-CLI-8.2-STREAMS`, `DES-CLI-8.4-EXIT`.

Add `crates/cargo-theorem/src/emit.rs` (or `emit/mod.rs` if it grows past about
250 lines) owning the application's two output streams:

- `OutputStreams<'a> { stdout: &'a mut dyn Write, stderr: &'a mut dyn Write }`
  replaces the separate `stdout`/`stderr` parameters through `cli.rs`.
- `CountingWriter` wraps a writer and counts accepted bytes (`STREAM-L1`).
- `emit_json_success(streams, document: &impl Serialize)` and
  `emit_json_failure(streams, envelope)` serialize to a `Vec<u8>` first, then
  write once (decision `D3`), and return an `ExitClass`.
- `plan_fallback(bytes_accepted) -> Fallback` is the pure kernel for the
  fault policy.
- Human emission functions write primary output to stdout and diagnostics to
  stderr through the render policy introduced in `EP-M3` (initially a
  plain-text policy).
- Clap error normalization: `report_clap_error` maps `DisplayHelp`,
  `DisplayVersion`, and `DisplayHelpOnMissingArgumentOrSubcommand` to `0`
  (stdout) and everything else to `2`, extracting `InvalidArg`, `InvalidValue`,
  and `ValidValue` context into `arguments` and `valid_values`.
- Panic guard: `run` wraps dispatch in `std::panic::catch_unwind` and maps a
  panic to `application.panic`/`internal`. `main.rs` installs, once, a panic
  hook that writes nothing (decision `D4`); the guard owns reporting.
- `run_with_io` gains no new process-global reads; `run` remains the only
  function that locks real stdio.

Red: `emit_tests.rs` (enumeration, scripted-writer proptest, seeded-fault
control), new rstest rows in `cli_tests/runner.rs` for help/version/usage
codes, and a panic-guard unit test. Acceptance: `STREAM-1` and `EXIT-1`
discharged; existing tests updated where they asserted `ExitCode::FAILURE`. End
state: every byte the application writes flows through `emit`. Recovery:
revert. Remaining gaps: renderer options, child processes.

### EP-M3: global renderer options

Assigned: `DES-CLI-8.2-STREAMS` (renderer list), `ADR2-D4`, `ADR9`.

1. Prototype (keep or discard by the end of this step): convert
   `GlobalOptions::color` to `Option<ColorPolicy>` with `clap::ValueEnum` and
   serde `rename_all = "lowercase"`, and run `config_tests.rs`. If precedence
   tests pass and agent context still generates, keep typed enums for all four
   policies (decision `D6`); otherwise revert to strings and record the
   evidence in `Surprises & discoveries`. Also check whether Clap wraps help at
   `Command::term_width(n)` without `wrap_help` (decision `D9`).
2. Add `crates/cargo-theorem/src/render/` with `policy.rs` (policy enums if
   not in `config/`,
   `RenderInputs { mode, settings, env facts, terminal facts }`, `RenderPolicy`
   with `color`, `emoji`, `progress`, `paging`, `width`, `verbosity`, `locale`,
   and `RenderPolicy::resolve`), `terminal.rs`
   (`TerminalFacts { stdout_is_terminal, stderr_is_terminal }`, probed in
   `run()` only), and `human.rs` (diagnostic and label rendering with `anstyle`
   styles already in the lockfile via Clap, or plain text when colour is off).
   Environment facts (`NO_COLOR`, `CLICOLOR_FORCE`, `TERM`) are read through
   the injected `EnvSource` passed in `ConfigurationSources`.
3. Validation in `config/`: width range, locale parsing, quiet/verbose
   conflict (decisions `D8` to `D10`), mapped to `usage` when the offending
   value came from the command line (Clap `value_parser` range and
   `conflicts_with`) and to `configuration` otherwise. Move new tests into
   `config/renderer_tests.rs` to keep `config_tests.rs` under 400 lines.
4. Apply the policy: Clap help and errors are rendered with
   `StyledStr::ansi()` when colour is on and plain `to_string()` when off;
   Clap's own colour detection is disabled with `ColorChoice::Never` on the
   command so that only the policy decides. Before configuration files are
   loaded (parse failures and help), the policy uses command-line values,
   injected environment, and terminal facts only; this limitation is documented.

Red: `render/policy_tests.rs` (exhaustive oracle comparison and mutation
control), `config/renderer_tests.rs`, human snapshots (diagnostic with colour
on, off, plain, verbose), and the `LOC-1` test. Acceptance: `REND-1`, `CFG-1`,
`LOC-1` discharged. End state: renderer options are typed, validated, and
applied. Recovery: revert; if the prototype failed, only the fallback path
remains. Remaining gaps: child processes, proof, suites, docs.

### EP-M4: captured subprocess boundary

Assigned: `RM-4.1.4` (subprocess clause), `DES-CLI-8.6-BOUNDS`,
`DES-CLI-5-BOUNDARY`.

Add `crates/cargo-theorem/src/process/`:

- `capture.rs`: `BoundedCapture { retained: Vec<u8>, seen: u64, limit: u64 }`
  with `new(limit)`, `push(&mut self, chunk: &[u8])`, and `finish(self)`, which
  returns a `CapturedStream` holding `bytes`, `bytes_seen`, `bytes_retained`,
  `limit_bytes`, and `truncated`. `push` uses saturating arithmetic on `seen`
  and documents the overflow precondition mirrored by Verus.
- `mod.rs`: the driven port `ProcessRunner`, a trait with one method,
  `run(&self, request: &ProcessRequest)`, returning
  `Result<ProcessOutput, ProcessError>`. `ProcessRequest` holds program,
  arguments (as `OsString`), working directory, an explicit environment
  (`env_clear` plus listed variables), per-stream limits (default and maximum
  16 MiB, design §8.6), and a required deadline. `ProcessOutput` holds the exit
  status (`Option<i32>` code and signal flag), and stdout and stderr
  `CapturedStream`s. `ProcessError` is a `thiserror` enum: `Spawn`, `Deadline`
  (child killed; partial captures attached), `Wait`, and `OutputNotClosed`
  (reader threads did not finish within the grace period).
- `std_runner.rs`: `StdProcessRunner` using `std::process::Command` with
  `stdin(Stdio::null())`, `stdout(Stdio::piped())`, `stderr(Stdio::piped())`;
  two reader threads drain pipes in fixed-size chunks into `BoundedCapture`s
  (continuing to drain after the limit so the child never blocks); the calling
  thread polls `try_wait` until the deadline, kills on expiry, waits, then
  joins readers through a channel with a grace period (default one second).
- No production caller is added in this slice; the module is `pub(crate)` and
  exercised by its tests. To satisfy dead-code lints without artificial
  references, the module is exported from the library as `pub mod process` with
  rustdoc stating that it is the sanctioned process boundary for later command
  slices (tasks 4.2.1 and 4.4.x), or, if reviewers prefer a private module,
  gated with an item-scoped `#[expect(dead_code, reason = "...")]` naming
  roadmap task 4.2.1. The choice is recorded in the decision log during
  implementation.

Red: `capture_tests.rs` (named boundary cases and the refinement proptest with
classification and seeded mutant), `std_runner_tests.rs` (output larger than
the limit is truncated with correct metadata; a sleeping child is killed at the
deadline; spawn failure of a missing program), `leak_tests.rs` (the
re-execution test with positive and negative controls), and
`architecture_tests.rs` (the fitness scan). Acceptance: `STREAM-2` and the test
half of `BC-1` to `BC-3` discharged. End state: a bounded, deadline-aware,
never-inheriting process boundary exists. Recovery: revert. Remaining gaps:
proof, suites, docs.

### EP-M5: Verus proof of bounded capture

Assigned: `DES-CLI-8.6-BOUNDS`, decision `D15`.

1. Add `tools/verus/VERSION`, `tools/verus/SHA256SUMS`, and
   `tools/verus/README.md` pinning a Verus release supported by the pinned
   `rust-prover-tools`; run `prover-tools verus install --repo-root .` locally
   (escalate under the proof-tooling tolerance if it fails).
2. Add Makefile targets `install-verus` and `verus` following the existing
   `install-kani` pattern (`$(PROVER_TOOLS) verus install --repo-root .` and
   `$(PROVER_TOOLS) verus run --repo-root . --proof-file
   verus/bounded_capture.rs`),
   with the recipe failing if the proof file contains `assume(`, `admit(`,
   `external_body`, or `verifier::external`. Keep `verus` out of `make test`,
   `make lint`, and `make all`.
3. Write `verus/bounded_capture.rs` following the proof sketch: spec model,
   candidate lemmas, the exec mirror of `push`, the satisfiability witness.
   Refine the lemma decomposition as the verification conditions require;
   record changes in `Decision log`.
4. Add a `verus` job to `.github/workflows/proofs.yml` using the shared
   `install-prover-tools` action with `tools: verus` if it supports Verus, else
   `make install-verus`, then `make verus`.

Acceptance: `make verus` passes locally with zero errors and the expected
verified-item count, and `make check-fmt`, `make lint`, and `make test` pass.
End state: `BC-1` to `BC-3` are proved for the model and bridged to production
by the refinement test. Recovery: revert; the property test remains. Remaining
gaps: suites, docs.

### EP-M6: behavioural and end-to-end contract suite

Assigned: `RM-4.1.4` (snapshots), `DES-CLI-18-TESTS`.

Add `crates/cargo-theorem/tests/output_contract.rs` (with submodules under
`tests/output_contract/` to stay under 400 lines) reusing
`tests/entry_points/process.rs`'s isolated command builder (move the shared
builder into a `tests/common/` module if needed). Each case records a transcript
`exit: N\n--- stdout ---\n...\n--- stderr ---\n...` as one insta snapshot,
with the package version filtered:

1. `json_success_context`: `theorem context --json` (stdout parsed and reduced
   to schema version, kind, and command paths before snapshotting).
2. `json_failure_usage`: `theorem --color sometimes context --json`.
3. `json_failure_configuration`: `theorem context --json --config missing.toml`.
4. `json_failure_width`: `theorem --width 5 context --json`.
5. `human_success_help`: `theorem --help` with `NO_COLOR=1` absent and
   terminal false (plain).
6. `human_success_help_color`: `theorem --color always --help`.
7. `human_failure_usage`: `theorem --color sometimes context`.
8. `json_locale_independence`: case 3 repeated with `--locale fr-FR`, asserted
   byte-identical to case 3.

A human-mode configuration failure is not reachable end to end in this slice:
`context` requires `--json`, and help never loads configuration. That case is
therefore snapshotted at the human-renderer boundary in `EP-M3` (rendering a
`configuration` diagnostic with colour on and off), and the end-to-end case is
added by the first human-mode data command (task 4.2.3).

Add `crates/cargo-theorem/tests/features/output_contract.feature` and
`crates/cargo-theorem/tests/output_contract_bdd.rs` with these scenarios:

```gherkin
Feature: cargo theorem output contracts
  Agents and humans rely on one versioned JSON document per invocation,
  stable exit classes, and renderer options that never corrupt JSON.

  Scenario: A JSON command writes one document to stdout only
    Given an isolated working directory
    When I run "cargo theorem context --json"
    Then the exit code is 0
    And stdout contains exactly one JSON document
    And stderr is empty

  Scenario: An invalid choice in JSON mode enumerates the valid values
    Given an isolated working directory
    When I run "cargo theorem --color sometimes context --json"
    Then the exit code is 2
    And stdout is empty
    And stderr contains exactly one diagnostic envelope with class "usage"
    And the diagnostic lists the valid values "auto, always, never"

  Scenario: A missing explicit configuration file is a configuration error
    Given an isolated working directory
    When I run "cargo theorem context --json --config missing.toml"
    Then the exit code is 3
    And stdout is empty
    And stderr contains exactly one diagnostic envelope with class "configuration"

  Scenario: NO_COLOR disables colour unless colour is requested explicitly
    Given an isolated working directory
    And the environment variable "NO_COLOR" is "1"
    When I run "cargo theorem --help"
    Then stdout contains no ANSI escape sequences
    When I run "cargo theorem --color always --help"
    Then stdout contains ANSI escape sequences

  Scenario: An out-of-range width is rejected with its bounds
    Given an isolated working directory
    When I run "cargo theorem --width 5 context --json"
    Then the exit code is 2
    And the diagnostic arguments name the range 20 to 1000

  Scenario: Renderer options never alter a JSON document
    Given an isolated working directory
    When I run "cargo theorem --color always --emoji always --locale fr-FR context --json"
    Then stdout is byte-identical to the output of "cargo theorem context --json"
```

The "environment variable" step configures only the child process
(`Command::env`). Acceptance: all snapshots reviewed and accepted, all
scenarios pass, and `make test` passes. End state: the roadmap success
criterion is demonstrated at the executable boundary. Recovery: revert.

### EP-M7: documentation, roadmap, and delivery

Assigned: every trace link.

- Write `docs/adrs/adr-0014-application-output-protocol-and-stream-ownership.md`
  recording decisions `D1` to `D5` and `D15` (envelope shape and the context
  exemption, emission policy, exit-code closure, captured-process boundary, and
  the Verus selection), following the ADR format in
  `docs/documentation-style-guide.md`.
- Update `docs/cargo-theorem-cli-design.md`: §5 (process module as runner
  precursor), §8.2 (decisions `D7` to `D12`, the pre-configuration limitation
  for parse errors and help), §8.3 (context exemption, reserved `warnings`,
  field order, trailing newline, `command` for parse failures, diagnostic code
  grammar), §8.4 (closure of the code set, panic and write-failure mapping),
  and §8.6 (the bounded-capture contract and its proof), each referencing
  ADR-0014.
- Update `docs/cargo-theorem-cli-users-guide.md`: mark the JSON and exit-code
  section as implemented, add a "Renderer options" section (values, `NO_COLOR`
  and `CLICOLOR_FORCE` precedence, width bounds, locale, quiet/verbose), and
  list the diagnostic codes.
- Update `docs/developers-guide.md` §3.8: module map of `cargo-theorem`
  (`protocol`, `emit`, `render`, `process`), the rule that every byte goes
  through `emit`, the process boundary's re-use policy (sole sanctioned child
  launcher; later callers such as task 4.2.1 must use it; it moves to
  `theoremc-runner` when that package exists), how to add a diagnostic code and
  a snapshot, and the Verus workflow (`make install-verus`, `make verus`, which
  properties the proof covers and does not).
- Update `docs/contents.md` (ADR-0014 and this ExecPlan) and
  `docs/repository-layout.md` (`verus/`, `tools/verus/`).
- Mark roadmap task 4.1.4 done in `docs/roadmap.md`.
- Run `make fmt`, `make markdownlint`, `make nixie`, `make spelling`, and the
  three code gates; commit; push; open the draft PR.

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
cargo nextest run -p cargo-theorem --test output_contract 2>&1 | tee /tmp/m6-theoremc.out
```

Snapshot workflow: run the focused test with `INSTA_UPDATE=no` to see the red
failure; rerun with `INSTA_UPDATE=always`, inspect every new `.snap` file in
`git diff`, and commit only reviewed snapshots.

Proof milestone:

```bash
make install-verus 2>&1 | tee "/tmp/install-verus-theoremc-${branch}.out"
make verus 2>&1 | tee "/tmp/verus-theoremc-${branch}.out"
```

Expected tail of a passing proof run (the count is fixed during `EP-M5`):

```plaintext
verification results:: N verified, 0 errors
```

## Validation and acceptance

The work is accepted when all of the following hold on the final commit:

- `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make nixie`, `make spelling`, and `make check-agent-native` pass.
- `make verus` passes with zero errors and no trusted escape hatches.
- The nine end-to-end snapshots exist and match, covering success and failure
  in both modes; the behavioural scenarios pass.
- `cargo theorem context --json` output is unchanged except for any agent
  context changes caused by typed enum values (which must be reviewed and
  recorded).
- Running `cargo theorem --color sometimes context --json` exits `2` with
  empty stdout and one diagnostic on stderr listing the valid values.
- The re-execution test shows grandchild sentinels only inside the captured
  output and never on the child's real streams.
- Roadmap task 4.1.4 is marked done.

## Idempotence and recovery

Every step is a source edit followed by gates; rerunning a gate is safe. Each
milestone ends with a commit, so `git revert <sha>` or a reset to the previous
milestone commit restores a coherent state. `make install-verus` installs into
`.verus/` and may be rerun. Snapshot acceptance is repeatable because snapshots
are reviewed in `git diff` before committing.

## Artefacts and notes

Gate logs live under `/tmp` with the names shown in `Concrete steps`. Record
short transcripts of the key acceptance commands here as milestones complete.

## Interfaces and dependencies

Public library surface added to `cargo-theorem` (pre-release, no compatibility
promise beyond the JSON contract itself):

```rust
pub mod protocol {
    pub const SCHEMA_VERSION: &str = "1";
    pub enum ExitClass { Success, Usage, Configuration, ProjectState,
        BackendUnavailable, VerificationMismatch, VerificationIncomplete,
        ExternalToolFailure, DeliveryFailure, Internal }
    pub enum DiagnosticCode { /* closed set, see EP-M1 */ }
    pub struct Diagnostic { /* code, class, message_fallback, arguments, valid_values */ }
    pub struct Envelope<T> { /* schema_version, kind, command, outcome */ }
}
```

Crate-internal interfaces: `emit::OutputStreams`, `emit::plan_fallback`,
`render::RenderPolicy::resolve`, `render::TerminalFacts`,
`process::ProcessRunner`, `process::StdProcessRunner`,
`process::BoundedCapture`.

Dependencies: no new runtime dependencies. New `[dev-dependencies]` for
`crates/cargo-theorem`: `insta`, `googletest`, `pretty_assertions`, `proptest`,
`rstest-bdd`, `rstest-bdd-macros`, at the versions already in `Cargo.lock`.
Tooling: Verus via `rust-prover-tools`.

## Revision notes

- 2026-11-16: initial draft from Wyvern reconnaissance and external prior art.
