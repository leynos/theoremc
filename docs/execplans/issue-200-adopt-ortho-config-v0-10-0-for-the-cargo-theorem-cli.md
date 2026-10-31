# Adopt OrthoConfig v0.10.0 in the cargo theorem CLI foundation

This ExecPlan is a living record. Update it at each milestone and before
stopping work.

Status: IN PROGRESS

## Purpose / big picture

Build the planned `cargo theorem` configuration foundation on the published
`ortho_config` 0.10.0 release. Users will have a dedicated Cargo subcommand
package with the established configuration contract, while reusable theoremc
libraries remain independent of application configuration and share the
workspace's Rust 1.89 floor. The issue is complete when the exact candidate
passes the local quality gates, receives a clear CodeRabbit review, and is
published as a draft pull request.

## Constraints

- Keep the imported OrthoConfig guides byte-for-byte identical to commit
  `38812c563e3953a490e3efc202609f37e21a0517`.
- Keep OrthoConfig and process configuration in `crates/cargo-theorem`; do not
  add them to `theoremc-core`, `theoremc-macros`, or the `theoremc` facade.
- Declare Rust 1.89 for every workspace package. `cargo-theorem` requires it
  for OrthoConfig, and the reusable libraries require it for `serde-saphyr`
  2.0.0 (ADR-010).
- Keep OrthoConfig default features enabled. Do not use unreleased APIs.
- Follow roadmap Phase 3.1; do not implement backend, proof-run, project-edit,
  or other later command handlers in this change.
- Do not mutate process environment in tests. Use OrthoConfig source injection
  and isolated child processes for binary contracts.
- Do not merge the pull request or publish a release.

## Tolerances (exception triggers)

- Scope: if the implementation requires changes outside the CLI foundation,
  its governing docs, workspace manifests, CI, and tests, stop and reassess.
- Interface: if a reusable library API or its MSRV must change, stop and record
  the conflict before editing it. The user-approved MSRV increase to Rust 1.89
  is recorded in ADR-010.
- Dependencies: only `ortho_config` 0.10.0 and dependencies directly needed by
  the application foundation are in scope; keep default features enabled.
- Compatibility: do not add aliases or fallbacks for configuration names not
  specified by the approved CLI design.
- Review: resolve every actionable CodeRabbit concern before the next
  milestone; use the requested cooldown if the managed review is rate-limited.

## Risks

- Risk: v0.10.0's derive and selected-subcommand APIs may constrain the
  application type layout. Severity: medium. Likelihood: medium. Mitigation:
  use the imported v0.10.0 users' guide and exercise the real derive/API
  boundary in tests.
- Risk: the shared lockfile may select transitive dependencies newer than the
  workspace MSRV. Severity: high. Likelihood: medium. Mitigation: run the
  workspace-wide Rust 1.89 check; do not hide failures with flags that bypass
  MSRV validation.
- Risk: imported upstream Markdown may conflict with repository formatting.
  Severity: medium. Likelihood: medium. Mitigation: preserve its bytes and
  restore imported files after any formatter attempt; report a deterministic
  gate conflict rather than rewriting upstream text.
- Risk: generated command metadata may omit the Cargo wrapper and advertise an
  invocation rejected by the real parser. Severity: medium. Likelihood: medium.
  Mitigation: derive wrapper metadata from `CargoCommand` and execute the
  emitted context path in an isolated subprocess regression.
- Risk: the designed CLI surface is much larger than this foundation milestone.
  Severity: medium. Likelihood: high. Mitigation: keep operational commands
  outside scope and implement only configuration, metadata, and the entry shape
  assigned to Phase 3.1.

## Progress

- [x] (2026-10-26) Renamed the branch to
  `issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli`.
- [x] (2026-10-26) Read repository instructions and confirmed a clean baseline
  at `af7608108fee9e86a90cb683f8293bde56d227e4`.
- [x] (2026-10-26) Fetched the `v0.10.0` tag; tag object
  `996e5a1c9f22ace4f55bf5dcc196d2c0524262bf` peels to the expected commit.
- [x] (2026-10-26) Imported both upstream guides from that immutable commit.
- [x] (2026-10-26) Recorded the CLI baseline inventory below; no production
  CLI exists to execute at baseline.
- [x] (2026-10-26) Recorded the configuration decision in the design, ADR,
  roadmap, and user/developer documentation.
- [x] (2026-10-26) Reran documentation gates after preserving the imported
  guides' upstream sibling paths; the final tree content was committed as
  `07eee8bdb37f52106e32f370a760f868adddf0bd`.
- [x] (2026-10-26) Reviewed exact M1 head
  `261d92b0ac2d038170d482811fbfb500366fa2ca` with CodeRabbit 0.9.1 against
  `origin/main` at `af7608108fee9e86a90cb683f8293bde56d227e4`; it reviewed all
  12 changed Markdown files and reported zero findings.
- [x] Implement the application package and injected-source tests.
- [x] Run the complete sequential deterministic gate set for the pre-rebase
  M2 candidate.
- [x] Rerun the full sequential deterministic gate set on the rebased candidate.
- [x] Review committed M2 candidate
  `363642074d1f2b2cc94ef53a14b26b5ed36de8e5` with `coderabbit review --agent`.
- [x] Resolve the verified review findings and rerun deterministic gates on
  the working-tree candidate.
- [x] Complete the focused Rust 1.89 MSRV retry and remaining build checks.
- [x] Review commit `8fd5082` with CodeRabbit and verify its findings.
- [x] Resolve the findings and rerun the full sequential gate set.
- [x] Review commit `18500dd` with CodeRabbit and verify its findings.
- [x] Move inline CLI tests and fix the copied derive example.
- [x] Rerun the full sequential gate set after those fixes.
- [x] Review commit `864cc8f` with CodeRabbit and verify its findings.
- [x] Correct the nested Clap trait in the design example; confirm the root
  placeholder is already absent from the candidate.
- [x] Run docs-only gates after correcting the derive example.
- [x] Review commit `29620d7` with CodeRabbit and fix verified findings.
- [x] Rerun the full sequential gate set after the final review fixes.
- [x] Review commit `08a2441` and test Cargo/direct help parity.
- [x] Pass the complete sequential gate set after the help test.
- [x] Review commit `5f54145` and reconcile profile documentation.
- [x] Run docs gates after correcting profile guidance.
- [x] Review commit `8a67ece` after the profile correction; CodeRabbit returned
  zero findings.
- [x] Address CodeRabbit review10's actionable findings and run the complete
  sequential gate set on the resulting candidate.
- [x] Rerun formatting, Markdown, Mermaid, and pinned-guide checksum checks
  after recording the final docs-gate results and read-only dispositions in
  this plan.
- [x] Publish the review10 follow-up to existing draft PR #204 for hosted
  validation.
- [x] Split CodeScene's large test method and pass the full local gate set.
- [x] Pass hosted CI, coverage, CodeScene, and Gecko checks on implementation
  head `40a8b47545ad9c61cdaa1e86956155f14bbe797e`.
- [x] Complete the first requested cooldown and attempt an exact-head review.
- [x] Complete the final requested cooldown and obtain a CodeRabbit review
  result for exact candidate `aea054739617eb8b3b542ede27ad907457b7b79d`.
- [x] Record the final review and hosted results, refresh the draft PR body,
  and confirm the PR remains open and draft.
- [x] (2026-10-27) Verify the hosted assessment's invocation-path and workspace
  acceptance findings against the current code, roadmap, ADR, and guide.
- [x] Include the declared `theorem` wrapper in generated agent context and add
  an end-to-end regression that executes the emitted context path.
- [x] Reconcile CONFIG-WORKSPACE with step 3.1.1's standalone context contract
  and defer nested-workspace resolution and its root/member fixture to 3.1.2.
- [x] (2026-10-31) Adopt `serde-saphyr` 2.0.0, raise every workspace package
  to Rust 1.89, simplify `make msrv` and CI to one workspace check, and record
  ADR-010.
- [x] (2026-10-31) Run the sequential gates for the assessment repair and
  serde-saphyr/MSRV candidate; the gate set passed on commit `5fc2a5f`.
- [ ] Publish the candidate and obtain candidate-bound assessment confirmation
  before readiness.

M2 implementation is now present in the working tree: the dedicated package,
Cargo entry wrapper, OrthoConfig configuration model, selected-command adapter,
agent context, JSON diagnostics, injected-source tests, binary-level entry
tests, mixed-MSRV CI checks, and Make targets. The user's guide now
distinguishes the registered `context --json` command from later planned
handlers. Formatting was applied with `make fmt`; the two imported upstream
guides still match their recorded SHA-256 checksums. The first `make test`
compile exposed import and `ExitCode` constness errors, which were corrected.
Expanding `make lint` to the whole workspace exposed an existing
`clippy::map_unwrap_or` finding in the macro trybuild fixture helper; that
expression now uses `map_or_else`. Both pre-rebase and rebased M2 candidates
passed the full deterministic gate set; exact-head CodeRabbit review and
delivery remain pending.

CodeRabbit reviewed `29620d7` after the nested Clap example fix and found two
valid CLI edge cases: a failed help-output write could return success when its
fallback reached stderr, and a missing global option value could hide a later
`--json` flag. The CLI now returns failure for the first case and only skips a
global value token when it is not another option. A writer-injection unit test
and isolated binary test cover both cases. The first two gate attempts stopped
at Clippy on the intentional fallback result and a shadowed test binding; both
were corrected. M2-38 then passed the full sequential set: formatting, lint,
typecheck, 846 tests and doctests, workflow contracts, Markdown, Mermaid,
agent-native, Rust 1.88 and 1.89 checks, build, release, and Makefile
validation. Logs are under `/tmp/` with the `m2-38` suffix. A clean exact-head
review is the remaining gate before delivery.

CodeRabbit reviewed `08a2441` and identified a roadmap acceptance gap: the
end-to-end suite had not compared help output through Cargo with direct
invocation. A focused test now installs the external-command symlink in an
isolated `PATH` and asserts the two help outputs and streams match. M2-40
identified and fixed an overlong test function; M2-41 exposed a missing
temporary symlink in the new test setup. M2-42 passed the full sequential
gates, including 847 tests and doctests, both MSRVs, and the release build.
Logs are under `/tmp/` with the `m2-42` suffix. A clean review of the committed
candidate remains before push.

CodeRabbit reviewed `5f54145` and found one contradiction in the users' guide:
it correctly said profiles cannot contain `cmds`, then later suggested that a
`run` profile might alter command-specific settings. The later sentence now
states that profiles apply only to global settings. Documentation checks and a
clean review of the resulting commit remain.

CodeRabbit reviewed commit `8a67ece` after the profile correction and returned
zero findings. Later follow-up reviews found additional maintainability and
typed-error issues, which have since been addressed. The latest complete gate
run is green; a fresh review of the current candidate and delivery remain.

Live `origin/main` advanced to `70926a18a32031d4c16c76b7c070a513f54a1bef` while
the implementation was in progress. The branch was rebased onto that commit
after preserving `3151a112074dd22ee97d0a53b58cfd5fce8f844f` as
`recovery/issue-200-pre-rebase-3151a11`. `git range-diff` matched all eleven M1
commits unchanged and showed the M2 commit inheriting the base's Rust setup
action and `rstest` 0.28 update. The CLI test dependency now uses `rstest`
0.28.0 as well. Scrutineer run M2-22 passed the full sequential gate set on the
rebased candidate after that alignment.

The first exact-head `coderabbit review --agent` completed on
`363642074d1f2b2cc94ef53a14b26b5ed36de8e5` with four findings. Read-only
verification grouped the two environment-name comments as one documentation
defect and confirmed three fixes: use the tested nested `THEOREMC_CMDS__...__`
environment form, skip implicit configuration loading for `context --json`
while retaining explicit-file validation, and select `cargo-theorem` in the
Makefile build recipe. The change adds unit and child-process coverage for the
context rule. Gate results are recorded below; the exact-head review remains
pending.

The fixes are now in the working tree. The sequential M2-24 run passed format
checking, Clippy, type checking, workspace tests (844 tests and doctests),
workflow-contract tests, Markdown lint, Mermaid validation, and agent-native
checking. `make msrv` passed the Rust 1.88 library check but failed during the
Rust 1.89 application check when Cargo could not write two files under
`target/debug`. The full log is
`/tmp/msrv-4589bf31-6003-42c5-b78f-652d6e7b79c1-issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli-m2-24.out`.
Investigation found adequate free disk space and inodes, a healthy build-limit
supervisor, no active Cargo or cleanup process in this worktree, and missing
individual hashed artifact directories under otherwise populated targets. The
cause is not established. The focused M2-25 retry passed both Rust-version
checks (717 seconds), followed by `make build`, `make release` (671 seconds),
and `mbake validate Makefile`. The release build compiled successfully. Its
format, Markdown, and Mermaid checks also passed after updating this plan; logs
are under `/tmp/` with the `m2-25` suffix. The source gates from M2-24 remain
green. A clean exact-head review is the next gate.

CodeRabbit reviewed commit `8fd5082` and returned four findings, grouped into
three defects. Read-only verification confirmed that parse failures after
`context --json` must retain the JSON diagnostic contract; typoed command
namespaces must fail while planned top-level namespaces such as `run` remain
available; and eight unique relative targets in the byte-preserved guides need
local redirects to the pinned upstream commit. The working tree now contains
those fixes, regression tests, design notes, and redirect pages. The imported
guide hashes remain unchanged. M2-28 stopped at Clippy on four diagnostics; the
parser detector was split into smaller helpers and the namespace check now uses
`contains`. M2-29 then passed all sequential gates, including 845 tests and
doctests, Rust 1.88 and 1.89 checks, release build, Markdown, Mermaid,
agent-native, and Makefile validation. Logs are under `/tmp/` with the `m2-29`
suffix. A local relative-link scan also passed for both imported guides, and
their recorded hashes still match. Only the clean exact-head review remains
before delivery.

CodeRabbit reviewed commit `18500dd` and returned three findings: two duplicate
comments that `cli.rs` exceeded the 400-line limit, and one missing comma in
the copied OrthoConfig derive example. The tests now live in `cli_tests.rs`
under the same `tests` module, and the example separates `discovery(...)` from
`profiles`. The M2-31 full sequence passed formatting, lint, typecheck, all 845
tests and doctests, workflow contracts, Markdown, Mermaid, agent-native, both
MSRV checks, build and release targets, and Makefile validation. Logs are under
`/tmp/` with the `m2-31` suffix. A clean exact-head review remains.

CodeRabbit reviewed commit `864cc8f` and returned two findings. The copied
`CommandLine` tuple payload incorrectly derived `Parser`; it now derives
`Args`, matching the implemented `TheoremArgs` wrapper. The roadmap comment
about a remaining root `src/main.rs` was stale:
`git ls-tree 864cc8f src/main.rs` returns no entry, and the working tree has no
such file. No source change was needed for that finding. Final docs gates and a
clean exact-head review remain. The M2-34 docs-only sequence passed formatting,
both imported-guide checksum checks, Markdown lint, and Mermaid validation.

The first workspace-wide Clippy run found 27 pre-existing diagnostics in
`theoremc-macros` test helpers that the old root-only `make lint` command did
not visit. The helper code was corrected without lint suppressions: result
tests now return diagnostics instead of asserting, generated string assembly
uses safe UTF-8 handling, test defaults name `IndexMap`, and shadowed bindings
were renamed. The `make lint` target now matches the workspace-wide command
specified in the repository instructions. A fresh full gate sequence is still
required. A subsequent lint attempt found a missing fixture import, two
OrthoConfig documentation-trait imports, and a discarded formatting result;
these were corrected. Agent-context output now uses compact JSON serialization.

The next sequential gate run passed `make check-fmt` but `make lint` found 32
Clippy diagnostics across the new CLI package. The captured lint log is
`/tmp/lint-4589bf31-6003-42c5-b78f-652d6e7b79c1-issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli-m2-7.out`.
The implementation now uses non-shadowing names, borrowed errors, fallible
diagnostic writes, and structured test assertions. The two renderer/config
records retain independent boolean CLI switches with narrowly justified
`clippy::struct_excessive_bools` expectations. Formatting has been reapplied;
the full sequence must be rerun from the first gate.

The following run passed `make check-fmt` but `make lint` found eight
`anyhow::Error`/boxed-error mismatches in result-returning source tests and one
collapsible write-error conditional. Those are corrected. Reviewing the
end-to-end contract also showed its expected agent-context command list did not
match OrthoConfig's documented bridge shape, which includes the binary root as
well as the registered `context` command. The child-process tests now assert
that two-entry shape and check JSON streams with `anyhow::ensure!`. A fresh
gate run is pending.

The next lint attempt found five remaining `&str` to `anyhow::Error`
conversions in `ok_or` expressions. Those now create explicit `anyhow!` errors,
including the child-process test helpers, and the isolated Cargo test child
preserves `RUSTUP_HOME` while keeping its configuration home temporary.
`make fmt` passed; all deterministic gates remain pending.

The next workspace lint pass found two potentially panicking indexes in the
end-to-end command metadata assertion. They now use checked accessors. The
latest formatting command passed; restart the complete gate sequence.

## Baseline inventory

- Baseline commit: `af7608108fee9e86a90cb683f8293bde56d227e4`.
- Compiler: `rustc 1.95.0-nightly (842bd5be2 2026-01-29)`, LLVM 22.1.0,
  `x86_64-unknown-linux-gnu`. `rust-toolchain.toml` pins `nightly-2026-01-30`;
  Rust 1.88 and 1.89 toolchains are installed.
- Executables: no `cargo-theorem` package or binary. The root `theoremc`
  binary is a placeholder that prints `Hello from Theorem Compiler!`.
- Planned command tree: `init`, `list`, `get`, `create`,
  `action list/get/create`, `check`, `build-script install/check/run/delete`,
  `generate`, `run`, `replay`, `report create`,
  `backend list/get/install/check/sync/update/delete/run`,
  `jobs list/get/cancel/prune`, `profile list/get/save/delete`, `context`, and
  `feedback`; see CLI design §7. None is currently implemented.
- Planned global flags and defaults: renderer modes `--color auto`,
  `--emoji auto`, `--progress auto`, and `--accessibility auto`, plus `--plain`,
  `--no-pager`, `--width`, `--locale`, `--quiet`, `--verbose`, `--no-input`,
  and `--json` where supported. Command-specific flags and their defaults are
  specified in the CLI design and users' guide, but no parser currently
  enforces them.
- Planned configuration: `theoremc.toml`, explicit `--config`, the
  `THEOREMC_*` environment prefix, per-command settings, and named profiles.
  Legacy `KANI`, `VERUS_*`, and `INPUT_*` values are scoped to a future backend
  migration adapter. No configuration file or environment key is currently read
  by the repository.
- Planned process contract: exit 0 success, 2 usage, 3 configuration, and 4–10
  for the stable error classes in CLI design §8.4. Human output uses stdout for
  primary output and stderr for diagnostics/progress. JSON success writes one
  envelope to stdout only; JSON failure writes one diagnostic envelope to
  stderr only. The envelope is versioned and keeps `result` and `error`
  mutually exclusive. Report formats are separate artefacts.
- CLI test inventory and result: zero CLI unit, integration, snapshot, or
  behavioural tests; there is no `crates/cargo-theorem` test target. The
  baseline CLI-specific test result is not applicable because the CLI does not
  exist. Existing workspace tests do not exercise `cargo theorem`.
- Callers: local search found no workflow or Makefile invocation. Authenticated
  GitHub code search across `leynos` returned only the CLI design document in
  this repository; no other repository caller was found.
- Adoption duplication check: issue #200 is open; issue #117 is open and
  superseded; no adoption PR or PR on the requested branch exists.

## Surprises & discoveries

- Observation: the repository has no `cargo-theorem` package or CLI tests, but
  the root `theoremc` package has a greeting stub binary. Evidence: baseline
  tree has no `crates/cargo-theorem`; `src/main.rs` prints
  `Hello from Theorem Compiler!`; there are no CLI test files. Impact: Phase
  3.1's dedicated package will replace the placeholder binary as prescribed by
  the accepted design. This does not claim compatibility with a previously
  shipped `cargo theorem` command.
- Observation: no other `leynos` repository code-search result invokes the
  command; the only match is this repository's design document. Evidence:
  authenticated `gh search code` for `"cargo theorem"` returned one file in
  `leynos/theoremc`; local search found no workflow or Makefile caller. Impact:
  no external caller needs a migration in this change.
- Observation: CodeGraph MCP tools are not available in this session.
  Evidence: the requested reindex and symbol-search tools are absent from the
  exposed tool catalogue. Impact: structural reconnaissance uses the skill's
  targeted file/search fallback and is recorded as such.
- Observation: the supplied OrthoConfig tag resolves exactly as expected, and
  the imported guides remain byte-identical after local documentation edits.
  Evidence: tag object/peeled commit verification and byte comparisons with
  SHA-256 checksums in `docs/ortho-config-v0-10-0-provenance.md`. Impact: use
  the v0.10.0 contract; no tag-resolution deviation is needed.
- Observation: Firecrawl search returned no results for a release-scoped query,
  but scraping the immutable upstream manifest confirmed package version
  `0.10.0` and Rust `1.89.0` at the expected commit. Evidence: Firecrawl
  scraped the `Cargo.toml` page at commit
  `38812c563e3953a490e3efc202609f37e21a0517`; the local fetched manifest
  confirms default features are `serde_json` and `toml`. Impact: the dependency
  keeps its default feature set, and the CLI accepts TOML configuration only.
- Observation: Cargo's external-tools documentation says custom subcommands
  use the caller's working directory as their invocation context and describes
  the injected command token and arguments, but does not describe a workspace
  root argument. Evidence: Firecrawl scrape of the official Cargo Book's
  [External Tools](https://doc.rust-lang.org/cargo/reference/external-tools.html)
  page on 2026-10-26. Inference: project commands must resolve the workspace
  root themselves and pass it to OrthoConfig. Impact: step 3.1.1 tests that
  `context --json` ignores automatic project configuration and skips workspace
  discovery to meet its standalone contract. Nested-member workspace-root
  resolution and the competing root/member configuration fixture belong to
  roadmap step 3.1.2 and are not claimed as verified in this foundation.

## Decision log

- Decision: implement the configuration spine as the existing roadmap's
  Phase 3.1 CLI foundation, not as a standalone package or early backend CLI.
  Rationale: Phase 3.1 explicitly owns package structure, configuration,
  command metadata, MSRV, and Cargo invocation. Date/Author: 2026-10-26, Codex
  under issue #200.
- Decision: depend on `ortho_config` 0.10.0 only in the application crate,
  with its default features. Keep reusable libraries at Rust 1.88 and do not
  add a direct `ortho_l10n` dependency while no library implementation needs a
  separate Fluent runtime. Rationale: the released facade already re-exports
  the v0.10.0 localization runtime; the library code has no current Fluent
  implementation to replace. Date/Author: 2026-10-26, Codex under issue #200.
- Decision: adopt `serde-saphyr` 2.0.0 and raise every workspace package to
  Rust 1.89, superseding the Rust 1.88 library floor recorded above. Rationale:
  2.0.0 is the current parser release and declares Rust 1.89; keeping 1.1.0
  pinned would preserve a split MSRV that the application package already
  exceeds, solely to hold back the parser. OrthoConfig remains
  application-only. Recorded in ADR-010. Date/Author: 2026-10-31, user-directed
  under issue #200.
- Decision: use the accepted CLI policy: `THEOREMC_*`, `theoremc.toml` at the
  Cargo workspace root, explicit `--config` selection, TOML, defaults < files <
  named profile < environment < explicit CLI values for global settings, and
  `cmds.<subcommand>` namespaces with defaults < files < environment < CLI for
  command-specific settings. OrthoConfig v0.10.0 forbids `cmds` in profile
  bodies, so profiles are global-only. Keep operation selection, delivery, and
  mutation-safety switches CLI-only. Rationale: this preserves the merged CLI
  design within the published release contract. Date/Author: 2026-10-26, Codex
  under issue #200.
- Decision: generated agent context describes only command paths registered by
  the current foundation. Later roadmap slices extend it from the same command
  tree when operational handlers ship; planned commands are not advertised as
  available. Rationale: this resolves the design's context-completeness wording
  without starting deferred CLI handlers. Date/Author: 2026-10-26, Codex under
  issue #200.
- Decision: resolve global and selected-command settings from one injected
  OrthoConfig file chain. The application-owned `SelectedSubcommandMerge`
  adapter handles the canonical filename and explicit `--config` path because
  the v0.10 selected-subcommand file helper only discovers prefix-derived
  dotfiles. Rationale: one selected file and discovery order must apply to both
  global and command-specific values. Date/Author: 2026-10-26, Codex under
  issue #200.

- Decision: include the declared Cargo wrapper in agent-context invocation
  paths by deriving its metadata from `CargoCommand` and nesting application
  metadata beneath it. Rationale: an agent must be able to execute the emitted
  `cargo-theorem theorem context` path with the required `--json` flag. Keep
  CONFIG-WORKSPACE limited to the foundation's no-discovery contract; roadmap
  step 3.1.2 owns `cargo metadata`, nested-member resolution, and the competing
  root/member configuration fixture. This corrects an unfulfilled plan claim
  without starting deferred project commands. Date/Author: 2026-10-27, Codex
  following hosted assessment comment `6441857666`.

## Outcomes & retrospective

The pre-readiness assessment of `7fc73fe` found a real agent-context invocation
bug and an ExecPlan acceptance mismatch. The current repair derives the
`theorem` metadata level from the declared Cargo wrapper, adds a subprocess
regression that follows the emitted invocation path, and makes the workspace
boundary explicit. Validation and a renewed assessment remain pending.

Work is in progress. Update this section at each completed milestone and record
any remaining validation or delivery blocker before completion.

The pre-rebase M2 candidate passed all nine deterministic gates sequentially on
2026-10-26 (scrutineer run M2-20): `make check-fmt`, `make lint`,
`make typecheck`, `make test` (842 tests and doctests; 631 seconds),
`make test-workflow-contracts`, `make markdownlint`, `make nixie`,
`make check-agent-native`, and `make msrv` (Rust 1.88 libraries and Rust 1.89
application). Those logs are under `/tmp/` with the `m2-20` suffix. The branch
has since been rebased onto live main and its CLI test dependency aligned to
`rstest` 0.28. The rebased candidate passed the full sequential gate set in
scrutineer run M2-22; logs are under `/tmp/` with the `m2-22` suffix. The first
CodeRabbit review reported four findings, grouped into three underlying
defects. All three are fixed. M2-24 passed all source gates other than the Rust
1.89 application half of `make msrv`, which failed while writing target
artifacts; M2-25 then passed both MSRV checks and the build, release,
formatting, docs, Mermaid, and Makefile checks. The subsequent review of
`8fd5082` found parse-error JSON, command-namespace validation, and imported-
guide relative-link issues. M2-29 passed the full gate sequence after those
fixes. Review of `18500dd` then found the oversized CLI source file and a
missing comma in the copied derive example. Those are fixed in the working tree
and M2-31 passed all gates after them. Review of `864cc8f` then found the
nested Clap derive typo; this is fixed in the working tree. Its root-binary
concern was invalid for the candidate, which already deletes `src/main.rs`. The
derive correction passed the M2-33 docs-only gates, and M2-34 passed after the
ExecPlan update. A clean exact-head review and delivery remain.

CodeRabbit's review of `29620d7` found two valid diagnostic edge cases, both
covered by regression tests and fixed in the working tree. The full M2-38 gate
sequence passed after the fixes. Request one clean review on the committed
candidate before pushing.

The review of `08a2441` also found that end-to-end help parity was missing. The
test now installs `cargo-theorem` in its temporary `PATH` before invoking
Cargo, then compares both outputs. M2-42 passed all sequential gates on this
candidate, including 847 tests, both MSRVs, and release. Review the committed
test fix before delivery.

CodeRabbit reviewed `5f54145` and found one contradictory profile example. The
users' guide now says profiles affect global settings only. Run the
documentation gates and obtain a clean review before pushing.

The profile correction was reviewed on commit `8a67ece` with zero findings.
M2-44 docs-only gates passed. Implementation, documentation, and review are
complete; push and draft PR creation remain.

The initial documentation candidate was committed as `cdb1eb9`.
`make check-fmt`, `make markdownlint`, and `make nixie` passed before the first
CodeRabbit 0.9.1 review (`/tmp/coderabbit-theoremc-issue-200-docs.out`). That
review covered all 12 changed documentation paths and identified four small
prose or consistency fixes plus broken relative links in the immutable upstream
guide. Commit `6b39971` corrected the prose and mapped imported links to their
local copy or pinned upstream target; the same gates passed again. The review of
`6b39971` found one further valid concern: the project scope did not say how
the Cargo workspace root was resolved. Commit `3c43681` now requires project
commands to resolve the root through `cargo metadata`, states that
`context --json` skips workspace inspection, and adds CONFIG-WORKSPACE to the
verification plan. Its docs gates passed:

- `make check-fmt`:
  `/tmp/check-fmt-theoremc-issue-200-workspace-root-review-fix.out`;
- `make markdownlint`:
  `/tmp/markdownlint-theoremc-issue-200-workspace-root-review-fix.out`; and
- `make nixie`:
  `/tmp/nixie-theoremc-issue-200-workspace-root-review-fix.out`.

M1 remains open until CodeRabbit reassesses the current committed candidate.

The final review of `6fe127e` completed successfully after the requested
rate-limit cooldown and reported two related link findings:

- The migration guide's `users-guide.md` links resolved to theoremc's
  unrelated root users' guide after the import rename.
- The imported guides can instead be co-located in `docs/ortho-config/` under
  their upstream filenames, preserving both bytes and sibling relative links.

The second finding supplies the compliant fix. Both imported files now live in
`docs/ortho-config/`; the index, repository layout, provenance record, and this
plan point to the new paths. Their SHA-256 checksums remain unchanged. The full
review result is
`/tmp/coderabbit-theoremc-issue-200-m1-final-6fe127e-retry.out`. The relocated
candidate passed its documentation gates:

- `make check-fmt`:
  `/tmp/check-fmt-theoremc-issue-200-final-plan-evidence.out`;
- `make markdownlint`:
  `/tmp/markdownlint-theoremc-issue-200-final-plan-evidence.out`; and
- `make nixie`:
  `/tmp/nixie-theoremc-issue-200-final-plan-evidence.out`.

A follow-up CodeRabbit review of the exact committed candidate is still needed
before M1 closes.

The later M1 review preparation made three release-boundary corrections before
implementation:

- OrthoConfig v0.10.0 rejects `cmds` and `inherits` in profile bodies, so named
  profiles apply to global settings only. The selected command retains its
  defaults < files < environment < CLI precedence.
- Compact agent context describes only handlers registered in the foundation
  command tree; later roadmap slices extend it as their handlers ship.
- The v0.10 selected-subcommand file helper searches prefix-derived dotfiles,
  not `theoremc.toml` or the explicit `--config` path. The design therefore
  resolves a single injected file chain and uses an application-owned
  `SelectedSubcommandMerge` adapter for selected `cmds.<name>` values.

The final documentation candidate passed `make check-fmt`, `make markdownlint`,
and `make nixie`. The runner executed these against the working-tree contents
later committed as `07eee8b`; its recorded `HEAD` was the parent
`173f9488f7af0e8eef639c02a890f2aa10f21ce3`. Logs:

- `make check-fmt`:
  `/tmp/check-fmt-4589bf31-6003-42c5-b78f-652d6e7b79c1-issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli-3.out`;
- `make markdownlint`:
  `/tmp/markdownlint-4589bf31-6003-42c5-b78f-652d6e7b79c1-issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli-3.out`;
  and
- `make nixie`:
  `/tmp/nixie-4589bf31-6003-42c5-b78f-652d6e7b79c1-issue-200-adopt-ortho-config-v0-10-0-for-the-cargo-theorem-cli-3.out`.

The review request at `d17eab5` was rate-limited before these final
corrections. A fresh review of the exact final head then completed M1.

CodeRabbit completed the exact-head M1 review on
`261d92b0ac2d038170d482811fbfb500366fa2ca` against `origin/main` at
`af7608108fee9e86a90cb683f8293bde56d227e4`. It reviewed 12 Markdown files and
reported zero findings; the review log is
`/tmp/coderabbit-theoremc-issue-200-m1-final-261d92b.out`. M1 is complete.

## Context and orientation

The CLI is planned in `docs/cargo-theorem-cli-design.md`; its foundation is
Phase 3.1 in `docs/roadmap.md`. The current workspace has the `theoremc` facade,
`theoremc-core`, `theoremc-macros`, build support, and test helpers, all now
sharing the workspace's Rust 1.89 MSRV. `src/main.rs` is only a greeting
placeholder. The new application package belongs at `crates/cargo-theorem/` and
owns command parsing, configuration types, source loading, application error
handling, and the composition root. Domain crates remain inward-facing and
configuration-free.

The imported guides are `docs/ortho-config/users-guide.md` and
`docs/ortho-config/v0-10-0-migration-guide.md`. Their source is the
`leynos/ortho-config` repository at tag `v0.10.0`, commit
`38812c563e3953a490e3efc202609f37e21a0517`.

## Conformance basis

- Issue #200, baseline commit `af7608108fee9e86a90cb683f8293bde56d227e4`:
  import provenance (REQ-1), inventory and preserve boundaries (REQ-2), decide
  configuration policy (REQ-3), integrate (REQ-4), verify (REQ-5), document and
  deliver a draft PR (REQ-6).
- `docs/cargo-theorem-cli-design.md` §4–§6 and §19: application boundary,
  external-subcommand shape, workspace MSRV, config discovery and precedence.
- `docs/roadmap.md` step 3.1.1: package, command metadata, configuration,
  workspace MSRV, direct/Cargo invocation, and base output contracts. Step
  3.1.2 owns Cargo workspace/package discovery and its nested-workspace
  fixtures.
- `docs/adr-002-library-first-internationalization-and-localization-with-fluent.md`:
  keep localization ownership at the application boundary.
- Repository instructions in `AGENTS.md`: Rust, documentation, test, commit,
  and PR gates.

Trace links:

- REQ-1 -> EP-M1 -> provenance hashes and byte comparison.
- REQ-2 -> EP-M1 -> baseline inventory, test count, caller search, and
  compiler identity.
- REQ-3 -> EP-M1 -> accepted design and ADR decision.
- REQ-4 -> EP-M2 -> application package and OrthoConfig integration tests.
- REQ-5 -> EP-M2/EP-M3 -> injected-source, binary, Rust-version, and canonical
  gate evidence.
- REQ-6 -> EP-M3 -> documentation, commits, pushed head, and draft PR.

## Verification plan

- Obligation CLI-ENTRY: Cargo dispatch and direct execution with the injected
  `theorem` token reach the same command parser. Method: isolated binary tests
  that invoke both forms and compare status, streams, and normalized help. Also
  execute the context invocation path read from emitted agent JSON with its
  required `--json` flag. Artefact: `crates/cargo-theorem/tests/`. Negative
  control: invoke the binary without the required external-subcommand token and
  require a usage failure.
- Obligation CONFIG-ORDER: defaults, explicit file, selected profile,
  environment, and explicit CLI values merge in that order. Method: finite
  table tests using `_with_sources`, `MapEnv`, and explicit file sources.
  Artefact: application configuration unit tests. Negative control: reverse two
  source values and require the expected assertion to fail.
- Obligation CONFIG-SCOPE: a selected subcommand reads only its `cmds.<name>`
  layer. Method: test two commands with conflicting values and prove a sibling
  setting does not leak. Artefact: application configuration tests. Negative
  control: select the sibling command with the wrong merge result and require
  the test to fail.
- Obligation CONFIG-WORKSPACE (foundation): `context --json` skips automatic
  configuration and workspace discovery. Method: run the isolated binary in a
  directory with malformed `theoremc.toml` and no Cargo workspace, and require
  valid context JSON with successful status and empty stderr. Artefact:
  `context_ignores_malformed_automatic_project_configuration`. Negative
  control: automatic file loading would parse the malformed TOML and fail.
  Deferred to roadmap step 3.1.2: resolve a nested member with
  `cargo metadata`, pass the workspace root to OrthoConfig, and prove that the
  root project file wins over a competing member-local file. This foundation
  does not claim that deferred implementation or fixture as verified.
- Obligation CONFIG-ERROR: malformed, unreadable, explicit-missing, invalid,
  and rejected unknown-key files produce an actionable configuration error;
  absence of an optional discovered file remains non-fatal. Method: table tests
  plus child-process stream/status checks. Artefact: application and binary
  tests. Negative control: treat a required explicit file as optional and
  require the failure assertion to catch it.
- Obligation CLI-ERROR: help/version remain successful display requests;
  parse failures retain Clap's exit status and stream. Method: child-process
  tests with an isolated environment. Artefact: binary integration tests.
  Negative control: force a parse error through the config-error branch and
  require the status/stream assertion to fail.
- Obligation MSRV: every workspace package compiles on Rust 1.89 with
  `serde-saphyr` 2.0.0 and OrthoConfig 0.10.0 resolved. Method:
  `RUSTFLAGS="" cargo +1.89 check --workspace`. Non-vacuity: the command must
  compile the facade, `theoremc-core`, `theoremc-macros`, build support, test
  helpers, and `cargo-theorem`. Negative control:
  `RUSTFLAGS="" cargo +1.85 check -p theoremc-core` must fail because the
  package declares Rust 1.89, showing the gate rejects a toolchain below the
  floor. The retired 1.88 channel is not required for the control.
- Third-party axioms: treat the v0.10.0 OrthoConfig APIs as documented; exercise
  the repository-owned derive, source-injection, profile, and Clap integration
  against the real dependency. No third-party internals are re-proved.
- No new business invariant requires Kani or Verus; configuration ordering and
  command boundaries are directly observable and proportionately covered by
  table and process tests.

## Plan of work

### EP-M1: import, inventory, and design

Import the pinned documentation, record provenance separately, inventory the
CLI and callers, and update the CLI design, ADR, roadmap, repository layout,
developer guide, and users' guide. No application code changes occur in this
milestone. Run documentation formatting and lint gates, preserve imported
bytes, commit, then request the first CodeRabbit review.

### EP-M2: application configuration foundation

Add `crates/cargo-theorem` with Rust 1.89 metadata, `ortho_config = "0.10.0"`
using default features, application-owned config and command types, the Cargo
external-subcommand wrapper, and injected-source tests. Remove the root
placeholder binary as prescribed by the CLI design. Do not add operational
backend or project-mutation commands. Run formatting, lint, typecheck, tests,
and the workspace MSRV check sequentially; commit and obtain CodeRabbit review
before the final docs/delivery milestone.

### EP-M3: reconcile and deliver

Update the users' and developers' guides from actual implementation evidence,
refresh the living plan, run all canonical gates sequentially, resolve review
concerns, commit, verify SSH identity, push the issue branch, set its upstream,
and create a draft PR with `Closes #200`, `Closes #117`, and the Lody session
link in the terminal `## References` section. Do not merge.

## Milestones and plateaus

- EP-M1 outcome: authoritative source, baseline, and approved configuration
  contract are recorded. Acceptance: matching tag/commit, matching hashes,
  current caller/test inventory, docs gates pass. Recovery: imported docs can
  be removed without touching source code. Remaining gaps: application code.
  Compatibility decision: none; no `cargo theorem` CLI shipped at baseline.
- EP-M2 outcome: dedicated application package consumes v0.10.0 and validates
  configuration without contaminating reusable crates. Acceptance: focused and
  binary tests plus the workspace MSRV check pass. Recovery: revert the
  application package and restore the placeholder binary as one commit.
  Remaining gaps: future CLI handlers. Compatibility decision: none; the
  package is new.
- EP-M3 outcome: documented, reviewed, pushed draft PR. Acceptance: all local
  gates pass, CodeRabbit concerns are clear, remote head equals local `HEAD`,
  PR is draft, and references are present. Recovery: amend and re-run gates on
  the same candidate. Remaining gaps: merge/release intentionally excluded.
  Compatibility decision: none.

## Concrete steps

Run commands from the repository root. Use Makefile gates for repository-wide
validation, and capture their output under `/tmp` as required by `AGENTS.md`.
The split stable checks clear the repository's nightly-only `RUSTFLAGS` with an
empty assignment; they do not bypass Rust version checks.

Expected source provenance:

```text
tag object: 996e5a1c9f22ace4f55bf5dcc196d2c0524262bf
peeled commit: 38812c563e3953a490e3efc202609f37e21a0517
```

## Validation and acceptance

- Imported files match `git show <commit>:<source-path>` byte-for-byte and the
  recorded SHA-256 values.
- No `ortho_config` or `ortho_l10n` dependency appears in a reusable package.
- Both Cargo and direct invocation accept the external-subcommand shape.
- With no config file or environment values, the application uses the same
  built-in defaults as its parser declarations.
- File, profile, environment, and explicit CLI precedence is covered for each
  implemented configuration type.
- Optional-file absence succeeds; explicit-file absence and malformed/unreadable
  files fail with a configuration status and actionable stderr.
- Help/version exit successfully. Invalid arguments preserve Clap status and
  stream. Machine-readable output contains no localized prose.
- Run `make check-fmt`, `make markdownlint`, `make nixie`, `make lint`,
  `make typecheck`, and `make test` sequentially, with output logs under
  `/tmp`. Also run the workspace MSRV check. Record actual outcomes; do not
  infer a pass.

## Idempotence and recovery

The documentation imports are deterministic copies from an immutable commit.
The branch is local until the final push. Keep each validated milestone in its
own commit. If a gate fails, read its captured log, fix the cause, and rerun
that gate before proceeding. Never discard unrelated changes.

## Artefacts and notes

- Imported documentation: `docs/ortho-config/users-guide.md` and
  `docs/ortho-config/v0-10-0-migration-guide.md`.
- Provenance record: `docs/ortho-config-v0-10-0-provenance.md`.
- Implementation and verification evidence will be appended as milestones
  complete.

## Interfaces and dependencies

- `crates/cargo-theorem` is the only application package that depends on
  OrthoConfig, Clap, localization, or process configuration.
- Configuration structs derive `ortho_config::OrthoConfig`; parser DTOs and
  validated application settings remain distinct where parsing needs differ.
- Injected tests use v0.10.0 `_with_sources` entry points, `MapEnv`, explicit
  `ConfigDiscovery` file sources, and the application-owned selected-command
  adapter that consumes the same resolved `cmds.<name>` layer.
- The Cargo entry point uses the single-variant wrapper shape documented by
  OrthoConfig v0.10.0 so both external-subcommand invocation forms work.

## Revision notes

- 2026-10-26: Created after importing the pinned upstream guides and completing
  the initial repository, branch, compiler, and remote issue/PR inventory.
- 2026-10-26: M2 first gate sequence passed formatting and workspace Clippy,
  then stopped at tests. The failure identified OrthoConfig's generated
  `config_path` selector missing from `GlobalOptions`' Serde shape, while
  omitted boolean values also needed struct-level Serde defaults. The app now
  owns the explicit config path field and keeps it out of merged settings; the
  full sequence must be rerun before the milestone can be reviewed or committed.
- 2026-10-26: The second M2 sequence passed formatting, Clippy, and typecheck,
  then showed that `serde(skip)` removes `config_path` from the destination
  shape as well. It also exposed that Clap's `Error::print` bypasses injected
  output writers. The config selector now remains recognized while being
  skipped during deserialization, and parse diagnostics are rendered into the
  caller-selected stream. Rerun the full sequence, including `make typecheck`.
- 2026-10-26: The third M2 sequence passed formatting, Clippy, and typecheck;
  tests confirmed the parser-writer fix but showed Serde also excludes fields
  marked `skip_deserializing` from the accepted field set. The selector now
  uses a custom deserializer that consumes its internal CLI value and returns
  `None`, preserving `deny_unknown_fields` for other keys. Rerun the full gate
  sequence before advancing.
- 2026-10-26: M2-14 stopped at Clippy's `doc_markdown` check for the new helper
  comment; the finding is corrected with the required code formatting. Restart
  the full gate sequence before moving to review.
- 2026-10-26: M2-15 passed formatting, Clippy, and typecheck. All config
  precedence tests passed; the remaining test failures were stale assertions
  for OrthoConfig's single-quoted profile error and Clap help's leading command
  description. The assertions now check the actual message and require the
  usage line in stdout. Restart the complete gate sequence.
- 2026-10-26: M2-16 passed check-fmt, lint, and typecheck; the help assertion
  now passes, as do all configuration tests. The final failure expected
  `cargo theorem` as the version label, while the executable correctly reports
  `cargo-theorem`. The test now checks the executable's version label. Rerun
  the full sequence before reviewing the milestone.
- 2026-10-26: M2-17 passed every gate except the Rust 1.88 library check. The
  pre-existing `serde-saphyr = "1.0.1"` caret requirement resolved to 1.3.0,
  whose declared floor is 1.89. Keep library MSRVs unchanged and resolve to a
  release that compiles on 1.88; rerun the full gate sequence afterward.
- 2026-10-26: Updated the core crate's caret minimum to
  `serde-saphyr = "1.1.0"` and locked the workspace to 1.1.0, preserving the
  Rust 1.88 library policy. The MSRV gate must prove this release compiles
  before the dependency change is considered complete.
- 2026-10-26: Updated repository layout and the documentation index to remove
  the deleted root placeholder binary and describe the new application crate.
  The imported docs remain checksum-identical. Formatting has passed; rerun all
  candidate gates, including the Rust 1.88 check, before CodeRabbit review.
- 2026-10-26: Focused `make msrv` passed with `serde-saphyr` 1.1.0: Rust 1.88
  checked the workspace without `cargo-theorem`, and Rust 1.89 checked the
  application package. The roadmap now marks the package, OrthoConfig spine,
  mixed-MSRV policy, and context foundation as implemented; the broader output
  envelope remains planned. Rerun the complete gate sequence on this candidate.
- 2026-10-27: PR #204's hosted run measured 86.62% coverage against the 91.47%
  baseline (±1.00 percentage-point tolerance) and CodeScene flagged duplicated
  policy validation plus three oversized test methods. The application now
  shares its policy validator, splits those methods, and exercises the CLI's
  parser, configuration, output, and diagnostic boundary in-process with
  injected `MapEnv` sources. An intermediate full run passed check-fmt, lint,
  typecheck, workflow contracts, Markdown, Nixie, agent-native, both MSRV
  checks, build/release, and Makefile validation; `make test` stopped on two
  overly strict help-prefix assertions. The focused replacement test now
  passes. A separate lint run required moving diagnostics out of `cli.rs`'s
  same-named module directory. Repeat the full gates and application coverage
  before committing this review follow-up. The local whole-workspace coverage
  attempt remains unavailable because its Kani harness lacks
  `profiler_builtins`; hosted coverage is the deciding ratchet result.
- 2026-10-27: The final local source-candidate sequence passed check-fmt, lint,
  typecheck, all 864 workspace tests and doctests, workflow contracts,
  Markdown, Nixie, agent-native, both MSRV checks, build/release, and Makefile
  validation. Package-only `cargo-theorem` coverage passed 46 tests and
  measured 678/716 lines (94.69%) and 61/70 functions (87.14%). CodeRabbit
  returned one documentation finding: `cargo-orthohelp` is a runtime bridge in
  `agent_context.rs` and shares the Rust 1.89 floor. ADR-009, the CLI design,
  and developers' guide now record that boundary. Run the documentation gates
  and another CodeRabbit review before committing; hosted CI and the coverage
  ratchet remain pending the follow-up push.
- 2026-10-27: CodeRabbit review6 reported a delimiter-handling bug in JSON
  intent detection, repeated Cargo subprocess setup, and two overlapping ADR
  structure findings. The pre-command scan now stops at `--`, with a regression
  test proving later operands do not select JSON diagnostics. The two parity
  tests share an `rstest` fixture. ADR-009 now uses separate Status and Date
  sections, an accepted-decision summary, and the repository's required Context
  and problem statement section. Rerun the full candidate gates, then review
  this final diff before committing and pushing.
- 2026-10-27: Review7 passed the full local sequence: check-fmt, lint,
  typecheck, 865 workspace tests and doctests, workflow contracts, Markdown,
  Nixie, agent-native, both MSRV checks, build/release, and Makefile
  validation. The application-only coverage run passed 47 tests and measured
  703/740 lines (95.00%) and 62/71 functions (87.32%). CodeRabbit review6's
  four findings have been addressed; obtain a clean review of the resulting
  diff before committing. Hosted CI and the global coverage ratchet remain
  pending the follow-up push.
- 2026-10-27: CodeRabbit review10 returned five findings, including two
  duplicate typed-merge-error reports, two conflicting suggestions about
  extracting the shared JSON diagnostic writer, and a CI cache suggestion. The
  selected-command conversion now preserves the inner `Arc<OrthoError>`, with a
  regression test for its message; both JSON failure paths call one writer. CI
  caches the pinned `cargo-orthohelp` binary keyed by its version, target, and
  toolchain-file hash. Floating Rust 1.88 and 1.89 channel aliases remain
  uncached so the cache cannot freeze a stale patch release.
- 2026-10-27: The review10 follow-up passed the complete sequential gate run
  (M13): `make fmt`, `make check-fmt`, `make lint`, `make typecheck`,
  `make test` (866 tests and doctests), package coverage (48 tests), workflow
  contracts, Markdown, Mermaid, agent-native, Rust 1.88/1.89 MSRV checks,
  build, release, and Makefile validation. CLI coverage measured 711/755 lines
  (94.17%) and 64/73 functions (87.67%); no branch counters were emitted. Logs
  use the `pr204-review13` suffix under `/tmp`. The full-workspace coverage
  attempt remains unavailable because the Kani harness lacks
  `profiler_builtins`; the hosted coverage ratchet remains pending the push.
  The exact candidate is ready for the requested fresh CodeRabbit review.
- 2026-10-27: After recording the M13 result, the docs-only rerun passed
  `make fmt`, `make check-fmt`, `make markdownlint`, and `make nixie`; both
  imported-guide checksums still match the provenance record. Logs use the
  `pr204-finaldocs` suffix under `/tmp`. The formatter introduced no candidate
  path changes.
- 2026-10-27: The final docs-only run after this entry also passed `make fmt`,
  `make check-fmt`, `make markdownlint`, and `make nixie`; both imported-guide
  checksums still match. Logs use the `pr204-finaldocs2` suffix under `/tmp`.
  The candidate diff hash was unchanged by formatting
  (`6577496077da99f6f2f84ce9dfb261cf53a1b5fee66a7c92290b5acfc3f91404`), as was
  its 15-path set.
- 2026-10-27: A read-only verification confirmed the review10 repairs in
  `cli_diagnostics.rs`, `error.rs`, their regression tests, and the CI
  workflow. The third docs-only run passed the same four formatting, Markdown,
  and Mermaid gates and reverified both guide checksums. Logs use the
  `pr204-finaldocs3` suffix under `/tmp`. Formatting changed the aggregate
  candidate hash from
  `f857cdfb07fcf3aea87e3957f1be6d249ea6ba40f6d5c2ad97852ddfa3466dcb` to
  `6e6a40433667aa0c24491f39c98f0cea029a8f516bdbba416880b4e7e819d7f0`; the
  15-path set stayed constant. The per-file pre-format contents were not
  captured, so the exact source of that formatting change is unverified. The
  next hosted review identified a test-method size issue, recorded below.
- 2026-10-27: Pushed commit `ab787ed` to the existing draft PR after the
  review10 fixes. The hosted CodeScene gate failed because
  `parse_error_json_detection_skips_global_values_and_respects_end_of_options`
  in `runner.rs` was classified as a large method. The prior Actions run
  remained in progress on that head and became stale after the fix. The test is
  now expressed as focused `rstest` cases for command shapes and global-option
  values. Its first gate attempt failed because the parameters lacked
  `#[case]`; that was corrected before the accepted run.
- 2026-10-27: The corrected test split passed the full sequential local gates:
  format, check-fmt, lint, typecheck, workspace tests (882/882 and doctests),
  package coverage (64 tests), workflow contracts, Markdown, Mermaid,
  agent-native, both MSRVs, build/release, and Makefile validation. Package
  coverage measured 651/695 lines (93.67%) and 64/73 functions (87.67%); no
  branch counters were emitted. Logs use the `pr204-review13` suffix under
  `/tmp`; the LCOV report is
  `/tmp/theoremc-issue-200-cargo-theorem-pr204-review13.lcov`. Full-workspace
  local coverage remains unavailable because the Kani harness lacks
  `profiler_builtins`.
- 2026-10-27: `coderabbit review --agent` was attempted on the published head
  after local gates, but returned a rate limit: all 10 included reviews were
  used and the service reported an 8-minute wait. Following the requested
  policy, `vsleep` is observing a randomized 84-minute cooldown. This produced
  no review result; do not retry before the cooldown ends. Hosted CodeScene and
  CI checks must also pass on the corrected published head.
- 2026-10-27: Hosted run `40441587594` passed on exact implementation head
  `40a8b47545ad9c61cdaa1e86956155f14bbe797e`, including coverage and tests;
  CodeScene and Gecko Security also passed. The workflow emitted cache-glob
  warnings for absent Python dependency files and an informational sccache
  statistics notice, but no job or step failed. The PR remained open and draft.
- 2026-10-27: After the 84-minute wait, a second exact-head
  `coderabbit review --agent` attempt again hit the included-review quota and
  produced no findings or review result. The service reported a 7-minute wait.
  The requested randomized 53-minute `vsleep` was interrupted with 39 minutes
  remaining (exit 143), so no cooldown completion was claimed. A fresh
  randomized 67-minute `vsleep` is now running; do not retry before it ends.
- 2026-10-27: Hosted run `40447436629` completed successfully on exact head
  `bb6caa0eaa69564beefdf1b075022ea73892d1da`. The test, coverage, lint,
  typecheck, MSRV, formatting, Markdown lint, and agent-native steps passed.
  CodeScene and Gecko Security also passed. CodeRabbit, Kody, and automerge
  checks skipped because this PR remains a draft. GitHub did not expose the
  live test log while it was running; the final run snapshot confirms success.
  CodeRabbit's CLI review and the final PR-body reconciliation remain pending.
- 2026-10-27: The plan-only update in `42038e9` passed the documentation gates
  (`make fmt`, `make check-fmt`, `make markdownlint`, and `make nixie`) and was
  pushed to PR #204. Hosted run `40449365196` then passed all CI steps on exact
  head `42038e9e05cbeccc379147ae63bab1553691d39d`; CodeScene and Gecko Security
  passed as well. The PR remains open and draft. The final CodeRabbit CLI
  review is still pending the full randomized 67-minute cooldown.
- 2026-10-27: Hosted run `40450935062` passed all CI steps on exact head
  `5f58cec9f2ffbce862d4b1b529b99f148b70cf73`; CodeScene result `8143203` and
  Gecko Security passed. The PR remains open and draft, with no unresolved
  hosted failures. The final CodeRabbit CLI review still awaits completion of
  the full randomized cooldown.
- 2026-10-27: After the full randomized 67-minute cooldown, the final exact-head
  `coderabbit review --agent` attempt on `0d7603d` again hit the
  included-review rate limit and returned no review result. The service
  reported 8 minutes and 41 seconds. Following the requested policy, a new
  randomized 71-minute `vsleep` is running; do not retry before it ends. All
  deterministic local and hosted checks passed on `0d7603d` before this attempt.
- 2026-10-27: Hosted run `40455819761` passed all CI steps on exact head
  `d5c54786594ef44c6126385859208150d9ba9028`; CodeScene result `8143544` and
  Gecko Security passed. The PR remains open and draft. The latest CodeRabbit
  cooldown is still running, with no review result available.
- 2026-10-27: Hosted run `40458253698` passed all CI steps on exact head
  `49124775751bf22189a41d3d1c29630453b9596b`; CodeScene result `8143699` and
  Gecko Security passed. The PR remains open and draft. The full randomized
  71-minute CodeRabbit cooldown is still running.
- 2026-10-27: Hosted run `40460611761` passed all CI steps on exact head
  `68c9c38250895143d71eb66c90994ac5181f6522`; CodeScene result `8143899` and
  Gecko Security passed. After the full randomized 71-minute cooldown, the
  exact-head CodeRabbit attempt again hit the included-review rate limit; the
  service reported 10 minutes and 5 seconds and returned no review result. A
  fresh randomized 80-minute `vsleep` is running; do not retry before it ends.
- 2026-10-27: Hosted run `40463463197` passed all CI steps on exact head
  `11d4ac3882d84a7cb2fa0d116da53526acc10cf6`; CodeScene result `8144100` and
  Gecko Security passed. The PR remains open and draft. The full randomized
  80-minute CodeRabbit cooldown is still running.
- 2026-10-27: Hosted run `40465966227` passed all CI steps on exact head
  `88680de13b207319abd3ed53e1f86815efb168fb`; CodeScene result `8144427` and
  Gecko Security passed. The PR remains open and draft, and the full randomized
  80-minute CodeRabbit cooldown is still running.
- 2026-10-27: Hosted run `40468279136` passed all CI steps on exact head
  `8e65a7ca216724baf1754915c3c7d9c191544771`; CodeScene result `8144716` and
  Gecko Security passed. The run emitted informational cache/runner annotations
  only. The PR remains open and draft; the full randomized CodeRabbit cooldown
  is still running.
- 2026-10-27: Hosted run `40470960699` passed all CI steps on exact head
  `7a2c5099ca393362dc025893d3e21308182382a5`; CodeScene result `8145132` and
  Gecko Security passed. After the full randomized 80-minute cooldown, the
  exact-head CodeRabbit attempt again hit the included-review rate limit. The
  service reported 2 minutes and 45 seconds and returned no review result. A
  fresh randomized 72-minute `vsleep` is running; do not retry before it ends.
- 2026-10-27: Hosted run `40473275765` passed all CI steps on exact head
  `f9f04e8b8ab3fba7135bd01f258b0a9ec2ef802b`; CodeScene result `8145455` and
  Gecko Security passed. The PR remains open and draft, and the full randomized
  72-minute CodeRabbit cooldown is still running.
- 2026-10-27: Hosted run `40476232178` passed all CI steps on exact head
  `6ec814ffdf518501f03ca8387e8cf116d7456c1e`; CodeScene result `8145848` and
  Gecko Security passed. The PR remains open and draft. The fresh randomized
  72-minute CodeRabbit cooldown is still running; its exact-head review remains
  pending.
- 2026-10-27: Hosted run `40480470936` passed all CI steps on exact head
  `aea054739617eb8b3b542ede27ad907457b7b79d`; CodeScene result `8146424` and
  Gecko Security passed. After the full randomized 72-minute cooldown,
  `coderabbit review --agent --base origin/main --include-untracked` completed
  on that exact head with zero findings. The PR body records the result, and PR
  #204 remains open and draft.
- 2026-10-27: Hosted assessment request `6441826875` received substantive
  response `6441857666` for base `70926a1` and head `7fc73fe`. Both findings
  were verified: emitted context paths omitted `theorem`, and CONFIG-WORKSPACE
  overstated the foundation's workspace discovery evidence. The working-tree
  repair adds the declared wrapper metadata and an executable emitted-path
  regression, and assigns nested-workspace discovery evidence to roadmap step
  3.1.2. The sequential gates and candidate-bound confirmation remain pending;
  no new formal proof is introduced by this repair.
- 2026-10-31: User-directed dependency decision: adopt `serde-saphyr` 2.0.0
  instead of holding 1.1.0 to keep the Rust 1.88 library floor. All workspace
  packages now declare Rust 1.89; `make msrv` and CI run one
  `cargo +1.89 check --workspace`. ADR-010 records the decision and supersedes
  ADR-009's mixed-MSRV clauses. The full sequential gate set passed on
  committed tree `5fc2a5f`: formatting, Markdown, Mermaid, lint, typecheck,
  workflow contracts, agent-native policy, MSRV, 884 tests, build, release, and
  Makefile validation. The 1.85 negative control failed with `serde-saphyr`
  2.0.0 and `theoremc-core` requiring Rust 1.89. Logs use the `issue-200-msrv`
  suffix under `/tmp`. Publishing the candidate and its candidate-bound
  assessment remain pending.
