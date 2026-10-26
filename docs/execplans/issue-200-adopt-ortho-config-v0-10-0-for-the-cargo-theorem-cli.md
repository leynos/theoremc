# Adopt OrthoConfig v0.10.0 in the cargo theorem CLI foundation

This ExecPlan is a living record. Update it at each milestone and before
stopping work.

Status: IN PROGRESS

## Purpose / big picture

Build the planned `cargo theorem` configuration foundation on the published
`ortho_config` 0.10.0 release. Users will have a dedicated Cargo subcommand
package with the established configuration contract, while reusable theoremc
libraries keep their Rust 1.88 minimum and remain independent of application
configuration. The issue is complete when the exact candidate passes the local
quality gates, receives a clear CodeRabbit review, and is published as a draft
pull request.

## Constraints

- Keep the imported OrthoConfig guides byte-for-byte identical to commit
  `38812c563e3953a490e3efc202609f37e21a0517`.
- Keep OrthoConfig and process configuration in `crates/cargo-theorem`; do not
  add them to `theoremc-core`, `theoremc-macros`, or the `theoremc` facade.
- Keep reusable packages at Rust 1.88. `cargo-theorem` may require Rust 1.89.
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
  the conflict before editing it.
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
  library MSRV. Severity: high. Likelihood: medium. Mitigation: run the
  separate Rust 1.88 library check and Rust 1.89 CLI check; do not hide
  failures with flags that bypass MSRV validation.
- Risk: imported upstream Markdown may conflict with repository formatting.
  Severity: medium. Likelihood: medium. Mitigation: preserve its bytes and
  restore imported files after any formatter attempt; report a deterministic
  gate conflict rather than rewriting upstream text.
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
- [x] Run the documentation gates for the initial documentation candidate;
  record review remediation and repeat the gates/review before closing M1.
- [ ] Implement the application package and injected-source tests.
- [ ] Run deterministic gates, request CodeRabbit reviews at milestone
  boundaries, commit the validated changes, push, and open a draft PR.

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
- Decision: use the accepted CLI policy: `THEOREMC_*`, `theoremc.toml` at the
  Cargo workspace root, explicit `--config` selection, TOML, defaults < files <
  named profile < environment < explicit CLI values, and `cmds.<subcommand>`
  namespaces. Keep operation selection, delivery, and mutation-safety switches
  CLI-only. Rationale: this preserves the merged CLI design and uses shipped
  v0.10.0 profile and source-injection support. Date/Author: 2026-10-26, Codex
  under issue #200.

## Outcomes & retrospective

Work is in progress. Update this section at each completed milestone and record
any remaining validation or delivery blocker before completion.

The initial documentation candidate was committed as `cdb1eb9`.
`make check-fmt`, `make markdownlint`, and `make nixie` passed before the first
CodeRabbit 0.9.1 review (`/tmp/coderabbit-theoremc-issue-200-docs.out`). The
review covered all 12 changed documentation paths and identified four small
prose or consistency fixes plus broken relative links in the immutable upstream
guide. The imported links are documented in this provenance file rather than
changing the byte-preserved source. M1 remains open until the corrected
candidate passes the same gates and receives a clear reassessment.

## Context and orientation

The CLI is planned in `docs/cargo-theorem-cli-design.md`; its foundation is
Phase 3.1 in `docs/roadmap.md`. The current workspace has the `theoremc` facade,
`theoremc-core`, `theoremc-macros`, build support, and test helpers, all with
Rust 1.88. `src/main.rs` is only a greeting placeholder. The new application
package belongs at `crates/cargo-theorem/` and owns command parsing,
configuration types, source loading, application error handling, and the
composition root. Domain crates remain inward-facing and configuration-free.

The imported guides are `docs/ortho-config-users-guide.md` and
`docs/ortho-config-v0-10-0-migration-guide.md`. Their source is the
`leynos/ortho-config` repository at tag `v0.10.0`, commit
`38812c563e3953a490e3efc202609f37e21a0517`.

## Conformance basis

- Issue #200, baseline commit `af7608108fee9e86a90cb683f8293bde56d227e4`:
  import provenance (REQ-1), inventory and preserve boundaries (REQ-2), decide
  configuration policy (REQ-3), integrate (REQ-4), verify (REQ-5), document and
  deliver a draft PR (REQ-6).
- `docs/cargo-theorem-cli-design.md` §4–§6 and §19: application boundary,
  external-subcommand shape, mixed MSRV, config discovery and precedence.
- `docs/roadmap.md` Phase 3.1: package, command metadata, configuration,
  split MSRV, direct/Cargo invocation, and base output contracts.
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
  that invoke both forms and compare status, streams, and normalized help.
  Artefact: `crates/cargo-theorem/tests/`. Negative control: invoke the binary
  without the required external-subcommand token and require a usage failure.
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
- Obligation MSRV: application-only dependencies do not raise library MSRV.
  Method: `RUSTFLAGS="" cargo +1.88 check --workspace --exclude cargo-theorem`
  and `RUSTFLAGS="" cargo +1.89 check -p cargo-theorem`. Non-vacuity: both
  commands must compile actual package targets.
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
and split MSRV checks sequentially; commit and obtain CodeRabbit review before
the final docs/delivery milestone.

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
  binary tests plus split MSRV checks pass. Recovery: revert the application
  package and restore the placeholder binary as one commit. Remaining gaps:
  future CLI handlers. Compatibility decision: none; the package is new.
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
  `/tmp`. Also run both split MSRV checks. Record actual outcomes; do not infer
  a pass.

## Idempotence and recovery

The documentation imports are deterministic copies from an immutable commit.
The branch is local until the final push. Keep each validated milestone in its
own commit. If a gate fails, read its captured log, fix the cause, and rerun
that gate before proceeding. Never discard unrelated changes.

## Artefacts and notes

- Imported documentation: `docs/ortho-config-users-guide.md` and
  `docs/ortho-config-v0-10-0-migration-guide.md`.
- Provenance record: `docs/ortho-config-v0-10-0-provenance.md`.
- Implementation and verification evidence will be appended as milestones
  complete.

## Interfaces and dependencies

- `crates/cargo-theorem` is the only application package that depends on
  OrthoConfig, Clap, localization, or process configuration.
- Configuration structs derive `ortho_config::OrthoConfig`; parser DTOs and
  validated application settings remain distinct where parsing needs differ.
- Injected tests use v0.10.0 `_with_sources` entry points, `MapEnv`, and
  explicit `SubcommandFileContext`/file sources.
- The Cargo entry point uses the single-variant wrapper shape documented by
  OrthoConfig v0.10.0 so both external-subcommand invocation forms work.

## Revision notes

- 2026-10-26: Created after importing the pinned upstream guides and completing
  the initial repository, branch, compiler, and remote issue/PR inventory.
