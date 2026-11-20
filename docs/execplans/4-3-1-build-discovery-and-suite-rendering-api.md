# 4.3.1 Extract build discovery and suite rendering behind a reusable API

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances (exception triggers)`, `Risks`, `Progress`,
`Surprises & discoveries`, `Decision log`, `Outcomes & retrospective`,
`Conformance basis`, and `Verification plan` must be kept up to date as work
proceeds.

Status: DRAFT (design review applied; awaiting user approval)

Branch: `4-3-1-build-discovery-and-suite-rendering-api`

Roadmap item: `4.3.1` in `docs/roadmap.md`.

## Purpose / big picture

Theoremc turns `.theorem` files under a crate's `theorems/` directory into Rust
code during a normal `cargo build`. A Cargo build script (`build.rs`, a small
program that Cargo compiles and runs before compiling the crate itself)
discovers those files, writes a generated file named `theorem_suite.rs` into
Cargo's per-package output directory (`OUT_DIR`), and prints Cargo directives
(lines such as `cargo::rerun-if-changed=theorems`) that tell Cargo when to run
the script again. The crate's library source then pulls the generated file in
with `include!`.

Today that logic lives in the workspace-internal crate
`crates/theoremc-build-support`. Architectural decision record (ADR) 0008
declares that crate internal: the root `theoremc` library deliberately does not
re-export it, and adopters cannot call it. The repository's own root `build.rs`
is the only real caller, and it hand-writes the Cargo directives itself:

```rust
fn main() -> Result<(), theoremc_build_support::BuildScriptError> {
    let build_output = theoremc_build_support::prepare_build_script()?;
    println!("cargo::rustc-check-cfg=cfg(theoremc_has_theorems)");
    if build_output.has_theorems() {
        println!("cargo::rustc-cfg=theoremc_has_theorems");
    }
    for path in build_output.rerun_paths() {
        println!("cargo::rerun-if-changed={}", path.as_str());
    }
    Ok(())
}
```

An adopter who wants theorems in their own crate therefore has to copy this
adapter, and the planned `cargo theorem build-script run` and
`cargo theorem generate --kind suite` commands (roadmap tasks 4.3.4 and 4.3.5)
have no reusable service to call. The adapter also has a latent defect: a
directory named `x` followed by a line feed and `cargo::rustc-cfg=injected`
produces two directive lines, so a file name can inject an arbitrary Cargo
directive.

After this change:

1. The crate is renamed to `crates/theoremc-build` (package `theoremc-build`)
   and becomes the reusable build-integration library described in
   `docs/cargo-theorem-cli-design.md` §5 Table 1.
2. An adopter's whole build script is one stable call through the public
   facade:

   ```rust
   fn main() {
       theoremc::build::emit_suite_from_env_or_exit();
   }
   ```

   The helper reads Cargo's environment, discovers theorems, writes
   `OUT_DIR/theorem_suite.rs` atomically and only when its contents change,
   prints every Cargo directive, and on failure prints one
   `cargo::error=<code>: <message>` line and exits with status 1.
3. A separate fallible API performs the same discovery and rendering against
   capability handles (`cap_std::fs_utf8::Dir`) supplied by the caller, with no
   process environment, no standard output, and no process exit:
   `theoremc::build::prepare_suite(&crate_root)` returns a `PreparedSuite` whose
   `contents()`, `cargo_directives()`, and `write_to(&out_dir)` give the
   future CLI commands everything the build script produces.
4. The root `build.rs` becomes the same one-line call (through
   `theoremc_build`, because a package cannot be its own build dependency). A
   shared fixture corpus owned by `theoremc-build` proves that the Cargo
   build-script path and the direct path produce byte-identical suites and
   identical directive streams.

Preserved behaviour: the discovery contract in `docs/theoremc-design.md` §7.1.1
(recursive `theorems/**/*.theorem`, forward-slash crate-relative paths,
deterministic order, the always-emitted `cargo::rerun-if-changed=theorems`
line, per-file rerun lines, write-if-changed suite output, and the
`theoremc_has_theorems` cfg). Deliberately changed behaviour, each recorded in
`Decision log` and the users' guide:

- build failures become one `cargo::error=` diagnostic with a stable code
  instead of a `Debug`-formatted `Err` returned from `main` (D6);
- a rerun path that cannot be written as one Cargo directive line is omitted
  and reported with a `cargo::warning=` line instead of corrupting the
  directive stream (D5);
- symbolic links under `theorems/`, previously skipped silently below the root
  and followed at the root, are rejected with a stable diagnostic (D8);
- path separators are always `/`, including on Windows hosts (D9);
- the suite file is replaced atomically and an unreadable previous suite is
  overwritten rather than failing every later build (D10);
- non-ASCII control characters and bidirectional formatting characters in
  theorem paths are escaped as `\u{...}` in the generated suite so the
  deny-by-default `text_direction_codepoint_in_literal` lint cannot reject it
  (D11).

This closes roadmap task 4.3.1. It does not implement any `cargo theorem`
command; tasks 4.3.4 and 4.3.5 consume this API.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- Follow `AGENTS.md`: no code file over 400 lines; every module starts with a
  `//!` comment; public items carry Rustdoc with examples; every `assert!`,
  `assert_eq!`, and `assert_ne!` carries a diagnostic message; no `unwrap` or
  `expect` outside tests; lints are silenced only with item-scoped
  `#[expect(lint, reason = "...")]`; en-GB Oxford spelling in prose and
  comments.
- Production code must not read the process environment or open directories
  with ambient authority except in one composition root,
  `theoremc_build::emit_suite_from_env_or_exit`. It passes `std::env::var_os`
  into an injected lookup closure under a single item-scoped
  `#[expect(clippy::disallowed_methods, reason = "...")]` and opens the crate
  root and `OUT_DIR` with `cap_std::ambient_authority()`. Library functions take
  `&cap_std::fs_utf8::Dir` capabilities. Tests inject closures and
  capabilities; no test mutates the process environment in-process. Child
  `cargo` processes may receive environment through `Command::env`.
- Filesystem access stays capability-oriented through `cap_std::fs_utf8` and
  `camino`. Do not introduce `std::fs` or `std::path` in production code.
- Do not share build-support source with `#[path]` or `include_str!` copies.
  ADR-0014 restates this rule from ADR-0008.
- The workspace minimum supported Rust version stays 1.89 (ADR-0010). The
  `cargo::error=` directive is stable from Cargo 1.84.
- `cargo-theorem` gains no new command, dependency, or behaviour in this task.
- Stay off the Kani vertical-slice path (ADR-0011 decision 7, roadmap
  sequencing rule 2): the `theorem_file_macro_bdd` fixtures, which `make kani`
  and `.github/workflows/proofs.yml` build, change only their dependency name
  and path, never their build-script call or dependency graph.
- The package rename is atomic: every caller, manifest, fixture, live
  document, and nextest filter changes in the same milestone. No alias crate,
  re-export shim, or deprecated name is introduced. The workspace has no
  release tag and no published crate, so no compatibility commitment exists.
- Persisted contracts consumed by `src/lib.rs` and adopters' `include!` sites
  must not change except as listed in `Purpose / big picture`: the file name
  `theorem_suite.rs`, the theorem root `theorems`, the cfg name
  `theoremc_has_theorems`, the line format `theorem_file!("<escaped path>");`,
  the empty-suite content (a single newline), the existing ASCII escape
  spellings (`\\`, `\"`, `\n`, `\r`, `\t`, `\xNN`), and the segment-wise path
  order (D12).
- Diagnostic codes are introduced as provisional stable identifiers. Their
  long-term compatibility policy belongs to roadmap task 8.1.2; message text is
  never a contract.
- Completed ExecPlans under `docs/execplans/` and accepted ADRs other than
  ADR-0008's status line are historical and must not be edited. ADR-0008 is
  marked `Superseded by ADR-0014` as the documentation style guide prescribes.
- Gates run sequentially: `make check-fmt`, `make lint`, `make test`, then the
  Markdown gates for documentation changes. Full gate runs are delegated to the
  `scrutineer` agent where practical.

## Tolerances (exception triggers)

All counts compare the working tree with the merge base of `main` (`4c05966` at
drafting time) and exclude every file under `docs/execplans/`. Each category is
enforced independently; spare capacity in one category does not offset another.
Counts measure net lines (additions minus deletions), because the rename and
the replacement of `build_script.rs` delete substantial existing code. A pure
`git mv` contributes zero net lines; a renamed file that Git no longer detects
as a rename counts as one deletion and one addition.

Classification rules:

- Code: `*.rs` files, `build.rs`, `.feature` files, `insta` snapshot files
  (`*.snap`), and `proptest-regressions/` files.
- Configuration: `Cargo.toml` files, `Cargo.lock`, `.config/nextest.toml`,
  `Makefile`, and `.github/**`.
- Documentation: `*.md` outside `docs/execplans/`, plus `typos.local.toml`.

Counting commands:

```bash
base=$(git merge-base HEAD origin/main)
git diff --numstat "$base" -- '*.rs' '*.feature' '*.snap' '**/proptest-regressions/**' \
  | awk '{n += $1 - $2} END {print "code", n}'
git diff --numstat "$base" -- '*Cargo.toml' Cargo.lock .config Makefile .github \
  | awk '{n += $1 - $2} END {print "config", n}'
git diff --numstat "$base" -- '*.md' typos.local.toml ':!docs/execplans/**' \
  | awk '{n += $1 - $2} END {print "docs", n}'
git diff --name-only -M "$base" -- ':!docs/execplans/**' | wc -l
```

Budget survey. The existing crate is 1,305 lines including tests
(`build_script.rs` 195, `build_script_tests.rs` 244, `discovery.rs` 267,
`discovery_tests.rs` 346, `suite.rs` 148, `suite_tests.rs` 90, `lib.rs` 15).
The plan replaces `build_script.rs` and its tests; adds production modules for
errors and codes, directive rendering, atomic writing, the Cargo boundary, and
the test-support corpus; splits `discovery_tests.rs` (already 346 lines) before
adding the representability and symlink tables; adds exhaustive character
checks, property tests, three consolidated snapshot files, a parity behavioural
suite, and a JSON-message Cargo helper; and deletes the five `include_str!`
source copies and the manifest rewriter in `tests/common/fixture_crate.rs`. The
planning estimate is about +1,350 net code lines across roughly 50 files, and
about 450 net documentation lines. Comparable history: the task 3.1.1 plan
budgeted 10 files or 600 net lines and the task 3.1.2 plan 12 files or 700 net
lines; the task 4.1.2 plan used scope-based tolerances only. None of the three
records a budget increase or its final size, so the history neither confirms
nor refutes those estimates. This task is broader than either, so the budgets
below carry about 15-20% contingency over the review-corrected estimate.

- Files: if more than 58 non-ExecPlan files change (detected renames count
  once), stop and escalate.
- Code: if code changes exceed 1,600 net lines, stop and escalate.
- Configuration: if configuration changes exceed 100 net lines, stop and
  escalate. `Cargo.lock` package-name churn from the rename is expected and
  counted.
- Documentation: if documentation changes exceed 600 net lines, stop and
  escalate.
- Interface: if any public item beyond those listed in
  `Interfaces and dependencies` is needed, or any existing public item of
  `theoremc-core`, `theoremc-macros`, or `cargo-theorem` must change, stop and
  escalate.
- Dependencies: the only authorized additions are listed in
  `Interfaces and dependencies`. Any other dependency, or any change of an
  existing version requirement, requires escalation.
- Architecture: if the work requires `theoremc-build` to depend on
  `theoremc-core`, `theoremc-macros`, `cargo-theorem`, `clap`, or
  `ortho_config` in its normal or test-support graph, or requires the CLI to
  own discovery or rendering logic, stop and escalate.
- Iterations: if a focused test still fails after three implementation
  attempts, stop and escalate.
- Time: if any milestone takes more than four hours of active work, stop and
  escalate.
- Ambiguity: if probe `EP-M0` contradicts `AXIOM-cargo-directive-parse` or
  `AXIOM-cargo-rerun`, or a reviewer requires the CLI command itself in this
  task, stop and present options.

## Risks

- Risk: Each real-Cargo fixture build adds serial time to the
  `nested-cargo-builds` nextest group, which is already near its 600-second
  per-test allowance on cold runs. Severity: medium. Likelihood: medium.
  Mitigation: only four new real-Cargo builds (two parity cases, one failure
  case, one adopter case); the full corpus runs in-process. The adopter fixture
  reuses the macro fixtures' dependency declarations so their compiled units
  are shared in the warm target (ADR-0012). Most host-side cost of the facade
  already exists because `theoremc-macros` depends on `theoremc-core`. D4 also
  removes today's per-fixture recompilation of the copied support crate. Record
  measured durations in `Artefacts and notes`; escalate before raising any
  nextest timeout.
- Risk: The new test binary runs outside the serialized nextest group and
  competes for Cargo's package-cache lock and compile slots. Severity: high.
  Likelihood: certain unless addressed. Mitigation: `EP-M3` adds
  `binary(=build_boundary_bdd)` to the group filter in `.config/nextest.toml`.
- Risk: Switching fixture crates from copied sources to an absolute path
  dependency on `crates/theoremc-build` changes how `[lints] workspace = true`
  is resolved for the dependency. Severity: low. Likelihood: low. Mitigation:
  the `theorem_file_macro_bdd` fixtures already path-depend on the workspace
  crate successfully, and Cargo caps lints for non-local dependencies.
- Risk: Cargo's handling of directive values differs from
  `AXIOM-cargo-directive-parse` on the pinned toolchain. Severity: medium.
  Likelihood: low. Mitigation: probe `EP-M0` before any implementation, with an
  explicit go/no-go.
- Risk: `proofs.yml` builds the Kani fixtures whose dependency name changes.
  Severity: medium. Likelihood: low. Mitigation: those fixtures keep their
  dependency graph and copied root `build.rs`; a green `proofs.yml` run on the
  pull request is an `EP-M3` acceptance item. The local `make kani` scenario is
  known to fail on a clean `main` because of the rustup library path, so local
  results are compared with `main` rather than treated as evidence.
- Risk: Rejecting symbolic links (D8) breaks a layout someone relies on.
  Severity: low. Likelihood: low (no release, no known adopter). Mitigation:
  the rejection names the link and a stable code; the users' guide documents
  the rule; relaxing it later to "follow links that stay inside the crate" is
  additive.
- Risk: The rename touches many live documents; a stale
  `theoremc-build-support` reference survives. Severity: low. Likelihood:
  medium. Mitigation: the stale-reference search in `EP-M4`, excluding
  completed ExecPlans and historical ADRs.
- Risk: `make check-fmt` does not detect committed Markdown problems because
  `mdtablefix --git` only inspects uncommitted files. Severity: low.
  Likelihood: medium. Mitigation: run `mdtablefix --check` and
  `markdownlint-cli2` explicitly on every edited Markdown file before each
  commit.

## Progress

- [x] (2026-11-16T19:02Z) Branch renamed, Lody session titled, skills loaded.
- [x] (2026-11-16T19:02Z) Wyvern reconnaissance of the build-support crate,
  facade, CLI, gates, documentation, and historical plans completed.
- [x] (2026-11-16T19:02Z) External facts gathered: Cargo build-script
  directives and parser behaviour, and `build-rs` prior art.
- [x] (2026-11-16T19:40Z) Initial draft written and committed.
- [x] (2026-11-16T20:30Z) Community-of-experts design review (structure and
  contracts, alternatives and viability, failure modes and cost, verification
  strategy) applied; verdict "proceed with conditions"; all conditions folded
  into this revision.
- [ ] Plan approved by the user.
- [ ] `EP-M0` directive-parsing probe recorded.
- [ ] `EP-M1` ADR-0014, atomic rename to `theoremc-build`, fixture path
  dependencies, nextest group.
- [ ] `EP-M2` capability-based direct API, test-support corpus, directive
  rendering, and the build-script boundary.
- [ ] `EP-M3` `theoremc::build` facade, parity and behavioural tests.
- [ ] `EP-M4` documentation closure and roadmap update.

## Surprises & discoveries

- Observation: the target crate name, public path, and visibility in roadmap
  4.3.1 contradict accepted ADR-0008 and `docs/developers-guide.md` §1.3.
  Evidence: ADR-0008 "Decision" says the support crate "does not make the
  package part of the public `theoremc` library API"; roadmap 4.3.1 asks for
  `theoremc::build::emit_suite_from_env_or_exit()`. Impact: ADR-0014 must
  supersede ADR-0008 before the rename lands (D1).
- Observation: root behavioural fixtures copy the build-support sources with
  `include_str!` into each temporary crate, contrary to the crate-level
  documentation that says fixtures depend on the crate. Evidence:
  `tests/common/fixture_crate.rs` constants `BUILD_SUPPORT_*_SOURCE` and
  `fixture_build_support_cargo_toml`. Impact: any module added to the crate
  silently breaks fixtures; `EP-M1` replaces the copies with a path dependency
  (D4).
- Observation: Verus is not wired into this repository (no `verus!` source,
  no Makefile target, no workflow); Kani runs only against generated harnesses
  in fixture crates. Evidence: reconnaissance of `Makefile`, `tools/`, and
  `.github/workflows/`. Impact: informs D7.
- Observation: Cargo trims the whole directive line, trims trailing whitespace
  from the value, and treats `cargo::error=` as a failure even when the script
  exits 0. Evidence: DeepWiki answer over `rust-lang/cargo`
  (`BuildOutput::parse`) and the Cargo Book build-script reference. Impact:
  `AXIOM-cargo-directive-parse`, D5, D6, and `EP-M0`.
- Observation: `Utf8PathBuf` orders paths segment by segment, not as strings,
  so `theorems/a/x.theorem` sorts before `theorems/a-b.theorem` although
  `"a-b" < "a/x"` as strings. Evidence: design review of
  `crates/theoremc-build-support/src/discovery.rs` (`sort_and_dedup`). Impact:
  the order contract is pinned as segment-wise (D12) and the property oracle
  uses segment vectors.
- Observation: discovery builds paths with `Utf8Path::join`, which uses `\` on
  Windows, contradicting the documented forward-slash contract; CI runs only on
  Linux. Evidence: `discovery.rs` `collect_entry`. Impact: D9.
- Observation: a symbolic link below `theorems/` is skipped silently, while a
  symbolic link at the root is followed. Evidence: `collect_entry` uses
  `DirEntry::file_type()`; `open_theorem_root` uses `Dir::metadata`. Impact: D8.
- Observation: `docs/rstest-bdd-users-guide.md`, listed among the required
  references, does not exist in this repository. Evidence: `ls docs`. Impact:
  Scenario Outline support was confirmed from the `rstest-bdd` 0.6 source in
  the Cargo registry and from the existing `tests/*_bdd.rs` suites; no other
  change.

## Decision log

- D1. Decision: rename `crates/theoremc-build-support` to
  `crates/theoremc-build` (package `theoremc-build`, library `theoremc_build`)
  and make it the reusable build-integration library. Record this in ADR-0014,
  "Reusable `theoremc-build` library and stable build-script boundary", which
  supersedes ADR-0008, restates ADR-0008's crate-boundary and no-`#[path]`
  rule, and explains its relation to ADR-0011 decision 7 (this task is off the
  vertical-slice path and does not make broad extraction a policy). ADR-0014 is
  written with status `Proposed` and becomes `Accepted` in `EP-M1` once `EP-M0`
  has confirmed D5 and D6. Rationale: CLI design §5 Table 1 assigns reusable
  discovery, suite rendering, and build-script entry points to
  `theoremc-build`, and the roadmap names that crate. Keeping
  `theoremc-build-support` beside a second crate would split one
  responsibility. A module inside the root facade was rejected because the root
  `build.rs` cannot build-depend on its own package. Amendment of ADR-0008 was
  rejected in review: ADR-0014 reverses its status summary, adapter location,
  API, and "no downstream contract" clause, which the style guide treats as
  supersession. Date/Author: 2026-11-16, planning agent after design review
  (pending user approval).
- D2. Decision: "the direct CLI path" in the success criterion means the
  fallible library service (`prepare_suite`, `PreparedSuite::write_to`,
  `PreparedSuite::cargo_directives`) that `cargo theorem build-script run`
  (4.3.4) and `generate --kind suite` (4.3.5) will call. No CLI command is
  added now, and `cargo-theorem` gains no dependency. Rationale: CLI design
  §10.1 says the fallible API "supports tests and direct CLI execution"; the
  commands belong to 4.3.4 and 4.3.5, which also need the mutation plan (4.3.2)
  and JSON envelope (4.1.4), and 4.3.5 already owns the byte-for-byte CLI
  criterion. The corpus lives in `theoremc-build` behind a `test-support`
  feature so later CLI tests reuse the same fixtures (D13). Date/Author:
  2026-11-16, planning agent (flagged for explicit user confirmation).
- D3. Decision: the stable adopter boundary is
  `theoremc::build::emit_suite_from_env_or_exit()`, re-exported from the root
  facade; the repository's own `build.rs` calls
  `theoremc_build::emit_suite_from_env_or_exit()`. Both names denote the same
  function. The 4.3.4 injector adds `theoremc` as the adopter's build
  dependency. Rationale: CLI design §10.1 fixes the adopter path and one
  dependency is simpler for adopters. A `build` feature on `theoremc` was
  rejected because `theoremc-core` already brings `camino`, `cap-std`, and
  `thiserror`, and a feature would make the stable call conditional. Having
  adopters build-depend on `theoremc-build` directly saves little, because
  `theoremc-macros` already compiles `theoremc-core` for the host. Date/Author:
  2026-11-16, planning agent.
- D4. Decision: fixture crates depend on the real workspace crate through an
  absolute path dependency instead of receiving copied sources. Rationale:
  removes the brittle file list, honours the no-copy rule, mirrors the existing
  `theorem_file_macro_bdd` fixtures, and lets the shared warm target compile
  the crate once. Date/Author: 2026-11-16, planning agent.
- D5. Decision: a rerun path is representable when it contains no line feed or
  carriage return and does not end with a `char::is_whitespace` character. An
  unrepresentable theorem file or watched directory stays in the suite and in
  `PreparedSuite::rerun_paths()`, but its `cargo::rerun-if-changed=` line is
  omitted and one `cargo::warning=build.unwatched_path: <escaped path>` line is
  emitted instead. Rationale: Cargo directives are line-oriented with trimmed
  values and no escape syntax (`AXIOM-cargo-directive-parse`), so such a path
  cannot be watched exactly and a line break would inject a directive.
  Rejecting the build, as the first draft proposed, rested on a false premise:
  the always-emitted root watch `theorems` already reruns the script for any
  change below it (`AXIOM-cargo-rerun`), so omitting the line loses no
  invalidation and breaks nobody. The warning keeps the omission visible.
  Date/Author: 2026-11-16, planning agent after design review.
- D6. Decision: build failures render as exactly one line,
  `cargo::error=<code>: <message>`, after which the helper flushes standard
  output and exits with status 1. `<code>` matches `^build\.[a-z_]+$`.
  `<message>` is the error's display text in which every path is rendered with
  `str::escape_debug` inside double quotes, every remaining `char::is_control`
  character is escaped as `\u{...}`, and trailing whitespace is escaped as
  `\u{...}`, so the line never contains a line break or ends with whitespace.
  Only the code is a contract; message text is not. Nothing else is printed on
  failure, even when discovery succeeded before a later write failed. Rationale:
  `cargo::error` is shown by Cargo without `-vv`; a stable code lets `check`
  (4.2.4) and `build-script run` (4.3.4) classify the same failures. Exiting
  non-zero makes failure independent of directive support. Codes follow the
  `schema.parse_failure` style in
  `crates/theoremc-core/src/schema/diagnostic.rs` and are provisional until
  roadmap task 8.1.2 defines the compatibility policy. Date/Author: 2026-11-16,
  planning agent after design review.
- D7. Decision: no Verus proof and no Kani harness. Character-level functions
  (`esc`, `sanitize`, `representable`) are verified by exhaustive enumeration
  over every Unicode scalar value, which is cheaper and stronger than a bounded
  Kani harness and can use `syn` as an oracle. Order and tree-shape properties
  use `proptest` with independent oracles; the Cargo integration uses real
  Cargo. Rationale: see `Formal verification decision`. Date/Author:
  2026-11-16, planning agent after design review.
- D8. Decision: any symbolic link at or below `theorems` is rejected with
  `build.symlink_unsupported`, naming the crate-relative link path. Rationale:
  the current behaviour is inconsistent and silently drops linked theorems
  below the root, which `AGENTS.md` forbids (silent absence). Following links
  safely needs a containment rule that is out of scope; rejection is explicit
  and can later be relaxed additively. Date/Author: 2026-11-16, planning agent
  after design review (pending user approval).
- D9. Decision: crate-relative paths are built by joining segments with a
  literal `/` through one pure helper, never with `Utf8Path::join`. Rationale:
  makes the documented forward-slash contract true on every host and keeps
  4.3.5 byte parity and 4.2.2 identifiers host-independent. Date/Author:
  2026-11-16, planning agent after design review.
- D10. Decision: `write_to` compares the previous suite as bytes, treats an
  unreadable or non-UTF-8 previous file as different, writes a temporary sibling
  (`.theorem_suite.rs.tmp`) in `OUT_DIR`, and renames it over
  `theorem_suite.rs`; on failure it removes the temporary file best-effort.
  Rationale: an interrupted in-place write today can leave a truncated,
  invalid-UTF-8 suite that fails every later build until `cargo clean`. Rename
  within one directory is atomic on supported platforms (`AXIOM-cap-std`).
  Cargo runs one instance of a package's build script per `OUT_DIR`, so a fixed
  temporary name cannot collide. Date/Author: 2026-11-16, planning agent after
  design review.
- D11. Decision: the suite escape keeps the existing ASCII spellings and also
  escapes non-ASCII `char::is_control` characters and the bidirectional
  formatting characters U+202A-U+202E and U+2066-U+2069 as `\u{...}`.
  Rationale: rustc's deny-by-default `text_direction_codepoint_in_literal` lint
  rejects raw bidirectional characters in string literals, so an unusual file
  name could otherwise break compilation of the generated suite. Date/Author:
  2026-11-16, planning agent after design review.
- D12. Decision: the order contract is segment-wise: paths are compared one
  `/`-separated segment at a time, each segment by its UTF-8 bytes. This is the
  existing `Utf8PathBuf` ordering for the forward-slash relative paths produced
  by discovery, and is now stated explicitly in design §7.1.1 and ADR-0014.
  Rationale: preserves today's persisted suite order while removing the
  ambiguity of "lexicographic". Date/Author: 2026-11-16, planning agent after
  design review.
- D13. Decision: the shared fixture corpus (`BuildCase` values and a
  `materialize(&Dir)` function) lives in `theoremc-build` behind a
  `test-support` Cargo feature, following the `theoremc-core` precedent, and is
  re-exported by `crates/test-helpers`. Rationale: the crate that owns the
  contract owns its corpus; dependencies still point downward; the root tests,
  the crate's own tests, and future `cargo-theorem` tests all use one corpus
  without a cycle and without pulling `theoremc-core` into the build crate's
  graph. A data-file corpus was rejected because it would need a parser
  dependency and cannot express symbolic links or file-in-place-of-directory
  cases. Date/Author: 2026-11-16, planning agent after design review.
- D14. Decision: the theorem root stays fixed at `theorems`. A configurable
  root (CLI design §10.2 "configured theorem root") will arrive later through a
  new, additive function taking an options value, not by changing
  `prepare_suite`. Rationale: no current consumer configures it; adding options
  now would be speculative. Date/Author: 2026-11-16, planning agent.
- D15. Decision: public errors are split by operation:
  `PrepareError` (discovery), `WriteError` (suite writing), and a crate-private
  `BuildScriptError` (environment and ambient directory opening, wrapping the
  other two) used only by the boundary. All carry `code()` returning a
  `#[non_exhaustive]` `BuildDiagnosticCode`; public enums and their struct
  variants are `#[non_exhaustive]`; a compile-time assertion checks
  `Send + Sync + 'static`. Rationale: direct callers never see environment
  variants they cannot trigger, while every boundary failure still has a stable
  code. Error sizes stay well below the `result_large_err` threshold, so no
  boxing is needed. Date/Author: 2026-11-16, planning agent after design review.

## Outcomes & retrospective

Not started. Complete at each milestone boundary and at completion, including
measured sizes against `Tolerances` and fixture build durations.

## Context and orientation

Inspected base revision: `4c05966` on `main` ("Chore(deps): bump rust-toolchain
(#255)"). Line references below refer to that revision; relocate symbols with
`rg -n '<symbol>' crates src tests build.rs`.

The workspace (`Cargo.toml`) has five members: `crates/test-helpers`,
`crates/theoremc-build-support`, `crates/cargo-theorem`,
`crates/theoremc-core`, and `crates/theoremc-macros`. The root package
`theoremc` (`src/lib.rs`, 41 lines) is the consumer-facing facade. It re-exports
`theoremc_core::{collision, mangle, schema}` and
`theoremc_macros::theorem_file`, and its private hidden module
`__theoremc_generated_suite` does:

```rust
#[cfg(theoremc_has_theorems)]
use theoremc_macros::theorem_file;
include!(concat!(env!("OUT_DIR"), "/theorem_suite.rs"));
```

`theoremc-build-support` is a `[build-dependencies]` entry of the root package
and depends only on `camino`, `cap-std` (`fs_utf8`), and `thiserror`. Its
modules:

- `src/discovery.rs`: `discover_theorem_inputs(manifest_dir)` opens the crate
  root with ambient authority and returns `BuildDiscovery` (sorted, deduplicated
  `theorem_files` and `watched_directories`) or
  `BuildDiscoveryError::{Io, TheoremRootNotDirectory}`. An absent `theorems`
  directory yields a root-only watch set. Entries are classified with
  `DirEntry::file_type()` (links not followed, so links are skipped); the root
  is checked with `Dir::metadata` (links followed). Paths are joined with
  `Utf8Path::join`. A non-UTF-8 entry name fails with `Io`.
- `src/suite.rs`: private `render_theorem_suite` emits one
  `theorem_file!("<escaped>");` line per file, or a single newline when empty;
  private `escape_rust_string` escapes `\`, `"`, `\n`, `\r`, `\t`, and other
  ASCII controls as `\xNN`, passing every other character through raw. Public
  `write_theorem_suite(out_dir: &Dir, discovery)` reads the old file as a
  string, fails on any read error except `NotFound`, and writes in place.
- `src/build_script.rs`: `prepare_build_script()` is the only environment
  reader (`std::env::var` under an item-scoped
  `#[expect(clippy::disallowed_methods)]`, passed into a private closure seam).
  It reads `CARGO_MANIFEST_DIR` and `OUT_DIR`, discovers, opens `OUT_DIR`,
  writes, and returns `BuildScriptOutput { has_theorems, rerun_paths }` or
  `BuildScriptError::{Environment, OpenOutputDirectory, Discovery, Suite}`.
- Unit tests live in `*_tests.rs` siblings using `rstest` and `tempfile`.

Behavioural tests are in the root package: `tests/build_discovery_bdd.rs` with
`tests/features/build_discovery.feature` (three scenarios) and
`tests/build_suite_bdd.rs` with `tests/features/build_suite.feature` (three
scenarios plus a shared-target isolation test). They build temporary fixture
crates with real `cargo build` through `test_helpers::FixtureCrate`
(`tests/common/fixture_crate.rs`), which copies the root `build.rs` and the
build-support sources into the fixture and rewrites its manifest to drop the
`[lints]` table. `tests/common/suite_layout.rs` locates a fixture's `OUT_DIR`
by matching package directories in the shared target, which can match a stale
directory. `crates/test-helpers/src/lib.rs` re-exports `tests/common/mod.rs`
through a `#[path]` module and depends on `theoremc-core`, not on `theoremc`.
All fixture crates share one warm target directory (ADR-0012) and run serially
in the nextest group `nested-cargo-builds` (`.config/nextest.toml`), whose
filter names test binaries explicitly; each test has a 600-second allowance and
the run a 30-minute global timeout.
`tests/theorem_file_macro_bdd/fixture_crate.rs` builds macro fixtures that
path-depend on the root package and on `crates/theoremc-build-support`;
`make kani` runs two of those scenarios under Kani in
`.github/workflows/proofs.yml`, whose path filter includes `build.rs`.

`crates/cargo-theorem` implements only `cargo theorem context --json`.
`build-script` and `generate` appear only in its configuration-key allowlist
(`crates/cargo-theorem/src/config/mod.rs`). It has no dependency on the
build-support crate.

Tooling: nightly `nightly-2026-11-16` (`rust-toolchain.toml`), MSRV 1.89
checked by `make msrv`. Test frameworks already configured in the workspace:
`rstest` 0.28, `rstest-bdd` 0.6 (Scenario Outline supported), `proptest` 1.11,
`googletest` 0.14, `pretty_assertions` 1.4, `insta` 1.47 (macros crate only),
`syn` 3 (root dev-dependency), `filetime` 0.2 (root dev-dependency). Gates:
`make check-fmt`, `make lint` (rustdoc, Clippy, Whitaker), `make test` (nextest
plus doctests), `make markdownlint`, `make nixie`, `make spelling`,
`make check-adr-rfc-names`. `mockable`, `mockall`, and `assert_cmd` are not
used anywhere; the environment seam convention here is a narrow closure, which
`AGENTS.md` accepts.

Documentation that names the build-support crate or its API and must change:
ADR-0008 (status line only), `docs/developers-guide.md` §§1.1-1.3, §2 layer
table, §2.3 "Build system API", and §3.6 "Extending the build system";
`docs/theoremc-design.md` overview diagram, the build sequence diagram, and
§§7.1-7.1.1; `docs/repository-layout.md`; `docs/contents.md`;
`docs/users-guide.md` "Build discovery"; `docs/cargo-theorem-cli-design.md`
§10.1; and `docs/roadmap.md` (the 4.3.1 checkbox and the 5.1 note naming
`theoremc-build-support`). ADR-0010 also names the old crate but is historical
and stays unchanged. `docs/CHANGELOG.md` is the imported OrthoConfig changelog,
not a project changelog, so it is not updated.

Prior art consulted: rust-lang's `build-rs` crate (0.3.5) offers typed Cargo
input and output, but its input functions such as
`build_rs::input::out_dir() -> PathBuf` panic on absence, which conflicts with
the fallible-API requirement; it is not adopted. `prost-build` and
`tonic-build` return `io::Result` from their compile functions and leave
process exit to the caller, which matches the fallible core and thin `_or_exit`
adapter split chosen here.

Skills and documents to keep open while implementing: `rust-router` (then
`rust-types-and-apis` for the public surface, `rust-errors` for the error enums,
`rust-unit-testing` and `proptest` for tests), `hexagonal-architecture` for
the port and adapter split, `execplans` for maintaining this plan,
`docs/rust-testing-with-rstest-fixtures.md`,
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`, and
`docs/developers-guide.md`. `docs/rstest-bdd-users-guide.md` is absent (see
`Surprises & discoveries`); use the existing `tests/*_bdd.rs` suites as the
house pattern. `docs/ortho-config-users-guide.md` is not needed because no CLI
configuration changes. `docs/theorem-file-specification.md` is relevant only
for the `.theorem` extension and the trivial fixture theorem.

## Conformance basis

Upstream artefacts at revision `4c05966`:

- `docs/roadmap.md` task 4.3.1 and its success criterion; sequencing rule 2.
- `docs/cargo-theorem-cli-design.md` §5 (Table 1 package responsibilities and
  dependency directions; cap-std for project services), §10.1 (managed call
  contract), §10.2 (`build-script run` uses "the same fallible build service
  directly"), §10.3 (`generate` determinism), §8.4 (exit taxonomy, for later
  CLI mapping).
- `docs/theoremc-design.md` §7.1 and §7.1.1 (discovery and invalidation
  contract).
- ADR-0008 (superseded here), ADR-0010 (MSRV), ADR-0011 decision 7 (stay off
  the vertical-slice path), ADR-0012 (shared warm target).
- No Terms of Reference document exists for this repository.

Requirements:

- `REQ-reusable-crate`: discovery and suite rendering live in a reusable
  `theoremc-build` library crate that depends on no application crate.
- `REQ-stable-boundary`: adopters' build scripts call
  `theoremc::build::emit_suite_from_env_or_exit()`, which owns environment
  reading, discovery, rerun output, suite writing, deterministic error
  rendering, and process termination (CLI design §10.1).
- `REQ-fallible-direct`: a fallible, capability-based API performs the same
  work without process environment, standard output, or exit (CLI design §§5,
  10.1-10.2).
- `REQ-shared-fixtures`: the root build script and the direct path use the
  same discovery and rendering fixtures and agree on outputs (roadmap success
  criterion).
- `REQ-preserve-contract`: the §7.1.1 discovery, ordering, invalidation, and
  file-name contract is unchanged except for the listed deliberate changes.

Gaps:

- `GAP-internal-only`: ADR-0008 forbids public reuse of the support crate.
- `GAP-adapter-in-root`: Cargo directive emission lives in the root
  `build.rs`, not in the library.
- `GAP-no-error-contract`: failures surface as a `Debug` print of `Err` from
  `main`, with no stable code.
- `GAP-directive-injection`: unrepresentable paths corrupt the directive
  stream.
- `GAP-copied-fixtures`: fixtures copy sources instead of depending on the
  crate.
- `GAP-silent-absence`: symbolic links below the root are skipped silently.
- `GAP-host-separators`: Windows hosts produce `\` separators.
- `GAP-non-atomic-write`: an interrupted write can wedge later builds.

Trace links:

```plaintext
Roadmap 4.3.1 -> REQ-reusable-crate -> GAP-internal-only -> D1, ADR-0014
  -> EP-M1 -> cargo tree -p theoremc-build shows no application crates
CLI §10.1 -> REQ-stable-boundary -> GAP-adapter-in-root, GAP-no-error-contract
  -> D3, D6, D15 -> EP-M2, EP-M3 -> emit_tests, build_boundary_bdd scenarios 2-3
CLI §§5, 10.2 -> REQ-fallible-direct -> D2, D15 -> EP-M2
  -> prepare and write unit tests on capability handles
Roadmap 4.3.1 success -> REQ-shared-fixtures -> GAP-copied-fixtures -> D4, D13
  -> EP-M1, EP-M2, EP-M3 -> corpus_parity tests, build_boundary_bdd scenario 1
DES §7.1.1 -> REQ-preserve-contract -> D9, D10, D11, D12
  -> INV-order, INV-literal, INV-write -> EP-M2 -> existing BDD suites green
Cargo protocol -> GAP-directive-injection -> D5, INV-directive-line
  -> EP-M0, EP-M2 -> injection regression test, directive checks
AGENTS.md silent absence -> GAP-silent-absence -> D8 -> EP-M2 -> symlink table
```

## Verification plan

### Abstract model

A theorem tree is a finite tree of entries below `theorems/` in a crate root.
Each entry has a name (a non-empty sequence of Unicode scalar values without
`/`) and a kind: directory, regular file, symbolic link, or other. Its
crate-relative path is `"theorems"` followed by `"/" + name` for each ancestor
and itself (D9). Define:

- `F`: the set of paths of regular files whose name ends in `.theorem`.
- `W`: `{"theorems"}` when `theorems` is absent; otherwise `"theorems"` and the
  path of every directory in the tree.
- `order`: compare paths segment by segment, each segment by UTF-8 bytes,
  a proper prefix first (D12); `sort(X)` sorts by `order` without duplicates.
- `esc(s)`: replace `\` by `\\`, `"` by `\"`, line feed by `\n`, carriage
  return by `\r`, tab by `\t`, other ASCII controls by `\x` and two upper-case
  hexadecimal digits, non-ASCII controls and bidirectional formatting
  characters by `\u{` hexadecimal `}` (D11); keep every other character.
- `representable(p)`: `p` contains no line feed or carriage return and does
  not end with a `char::is_whitespace` character (D5).
- `prepare(tree)`:
  - the error `build.symlink_unsupported` naming the first symbolic link in
    `order` if the root or any entry is a link (D8);
  - the error `build.theorem_root_not_directory` if `theorems` exists and is
    not a directory;
  - the error `build.non_utf8_entry` naming the parent directory if an entry
    name is not UTF-8;
  - otherwise `PreparedSuite` with `files = sort(F)`, `watched = sort(W)`,
    `contents` equal to the concatenation of
    `"theorem_file!(\"" + esc(p) + "\");\n"` for `p` in `files`, or `"\n"` when
    `files` is empty, and `has_theorems` equal to `files` being non-empty.
- `directives(S)` is the following ordered list, where
  `R = S.watched ++ S.files` (the rerun order):

  ```plaintext
  ["cargo::rustc-check-cfg=cfg(theoremc_has_theorems)"]
  ++ (["cargo::rustc-cfg=theoremc_has_theorems"] if S.has_theorems)
  ++ ["cargo::rerun-if-changed=" + p for p in R if representable(p)]
  ++ ["cargo::warning=build.unwatched_path: " + quote(p) for p in R if not representable(p)]
  ```

- `failure(e)` is the single line
  `"cargo::error=" + code(e) + ": " + sanitize(message(e))`, followed by exit
  status 1, where `quote(p)` is `"\"" + p.escape_debug() + "\""` and `sanitize`
  escapes every remaining `char::is_control` character and any trailing
  whitespace as `\u{...}`.
- `boundary(env, crate_root, out_dir)`: read `CARGO_MANIFEST_DIR` and
  `OUT_DIR`; open both; `S = prepare(crate_root)`; `S.write_to(out_dir)`; then
  print `directives(S)` as one block. Any error at any step prints only
  `failure(e)`.

Preconditions: Cargo supplies `CARGO_MANIFEST_DIR` and `OUT_DIR` as UTF-8 paths
and `OUT_DIR` exists. Violations produce `build.missing_environment`,
`build.non_utf8_environment`, or `build.open_directory` failures rather than
panics.

### Invariants and lemmas

- `INV-order` (REQ-preserve-contract, D12): `contents`, `rerun_paths`, and
  `directives` depend only on the tree, not on creation or directory-iteration
  order, and list paths in `order`.
- `INV-presence` (REQ-preserve-contract): `has_theorems` holds exactly when
  `files` is non-empty, `contents` is exactly `"\n"` when it is empty, and the
  cfg line appears exactly when `has_theorems` holds.
- `INV-literal` (REQ-preserve-contract, D11): for every string `s`,
  `"\"" + esc(s) + "\""` lexes as a Rust string literal whose value is exactly
  `s` and contains no raw bidirectional formatting character; the ASCII
  spellings are exactly those of the base revision.
- `INV-directive-line` (GAP-directive-injection, D5, D6): every line the
  boundary writes to standard output, on success or failure, starts with
  `cargo::`, contains no line feed or carriage return, and does not end with
  whitespace; success writes `|directives(S)|` lines and failure writes exactly
  one.
- `INV-watch-coverage` (D5): every path in `rerun_paths` is either emitted as
  a `rerun-if-changed` line or reported by exactly one `build.unwatched_path`
  warning, and `cargo::rerun-if-changed=theorems` is always emitted.
- `INV-classification` (D8, D9, GAP-silent-absence): discovery never skips a
  symbolic link silently, never returns a path containing `\`, and reports a
  non-UTF-8 entry with `build.non_utf8_entry`.
- `INV-write` (D10): `write_to` returns `Written` and leaves `contents` in
  `theorem_suite.rs` when the previous file was absent, different, or
  unreadable; returns `Unchanged` and performs no write when identical; leaves
  no temporary file; and changes no other entry of the output directory.
- `INV-late-failure` (D6): if `prepare` succeeds and opening or writing the
  output fails, the boundary prints exactly one `cargo::error=` line and no cfg
  or rerun line.
- `INV-parity` (REQ-shared-fixtures, REQ-stable-boundary): for every shared
  corpus case built through Cargo, the suite file in the `out_dir` reported by
  Cargo's `build-script-executed` message is byte-identical to
  `prepare(tree).contents`, and the directive lines in the sibling `output`
  file equal `directives(prepare(tree))`.
- `INV-codes` (D6, D15): every error variant maps to exactly one code; codes
  are pairwise distinct and match `^build\.[a-z_]+$`.
- `INV-env-seam` (constraints): only `emit_suite_from_env_or_exit` reads the
  process environment or uses ambient filesystem authority.
- `L-boundary-composes-direct`: `boundary` is the composition
  `lookup -> open -> prepare_suite -> write_to -> cargo_directives`, so
  `INV-parity` reduces to `INV-order` plus the fact that Cargo runs the
  boundary with the fixture's own paths. It is checked by an in-process test of
  the private runner against handwritten expected line vectors (not against a
  second call of the same functions) and by review.

### Axioms (trusted, not verified here)

- `AXIOM-cargo-directive-parse`: Cargo reads build-script standard output line
  by line, trims each line, splits at the first `=`, trims trailing whitespace
  from the value, keeps leading whitespace, treats `cargo::error=` as a failure
  even when the script exits 0, and fails the build when the script exits
  non-zero. Source: Cargo Book build-script reference and `BuildOutput::parse`
  in `rust-lang/cargo` (via DeepWiki). Boundary: `directives.rs`, `emit.rs`.
  Probed in `EP-M0` (exit 0 and exit 1, trailing space, leading space, `=`
  inside a value, and a nonexistent path).
- `AXIOM-cargo-rerun`: `cargo::rerun-if-changed=<dir>` reruns the script when
  anything below the directory changes, including creation of a missing
  directory; Cargo records no fingerprint for a failed build-script run, so it
  reruns next time. Source: Cargo Book; already exercised by
  `tests/features/build_discovery.feature`; probed in `EP-M0` for an edit below
  a directory whose name ends in a space.
- `AXIOM-syn-lexer`: `syn::LitStr::value()` implements Rust string-literal
  unescaping. Used only as a test oracle.
- `AXIOM-cap-std`: `cap_std::fs_utf8::Dir` operations behave as their
  documented POSIX counterparts; `symlink_metadata` does not follow links;
  `rename` within one directory is atomic; ambient opening is the only
  authority escape.
- `AXIOM-rustc-bidi-lint`: rustc's `text_direction_codepoint_in_literal` lint
  is deny-by-default and fires on raw bidirectional formatting characters in
  literals, including literals reached through `include!`. Exercised by the
  `mixed_tree` Cargo parity case, whose file names include U+202E.

### Methods per obligation

- Obligation: `INV-literal`.
  Method: exhaustive enumeration over every Unicode scalar value plus the
  existing named table. Rationale: `esc` works one character at a time, so
  checking every character (and adjacency within blocks) is complete for
  single-character behaviour and cheaper than Kani; `syn` is an independent
  lexer oracle. Domain: all 1,112,064 scalar values, in blocks of 4,096
  consecutive values escaped as one literal (about 272 parses); plus the named
  table pinning the exact ASCII spellings. Artefact:
  `crates/theoremc-build/src/escape_tests.rs`;
  `crates/theoremc-build/src/suite_tests.rs` (named table kept). Evidence:
  every block parses and round-trips; no rendered block contains a raw
  bidirectional formatting character; the named table passes unchanged.
  Non-vacuity: block count is asserted (272); seeded faults, each run once and
  reverted, must fail: drop the `"` arm, drop the `\\` arm, drop both the `\r`
  arm and the control arm (leaving a raw carriage return), and format `\x`
  without zero padding. The named table, not the round trip, catches a dropped
  `\t` arm, because `\x09` also round-trips; that division is deliberate.
- Obligation: `INV-directive-line` and `INV-watch-coverage`.
  Method: exhaustive enumeration over characters for `representable` and
  `sanitize`, a property test over generated trees, `insta` snapshots, and an
  injection regression test. Domain: for every character `c`,
  `representable("theorems/d" + c)` matches the specification and
  `sanitize("m" + c)` and `sanitize("m" + c + " ")` contain no line break and
  do not end with whitespace; generated trees include directory names with
  embedded line feeds and trailing spaces. Artefact:
  `crates/theoremc-build/src/directives_tests.rs`,
  `crates/theoremc-build/src/snapshots/` (one success snapshot per output
  family and one failure table snapshot). Evidence: predicates hold for all
  characters and generated cases; the injection regression test asserts that a
  directory named `x\ncargo::rustc-cfg=injected` yields one
  `build.unwatched_path` warning and no `rustc-cfg=injected` line. Non-vacuity:
  the regression test is written first against a stub renderer that formats
  like the base-revision `build.rs` and must fail with two lines; seeded fault
  "skip `sanitize`" must fail; generated-case classification records the share
  of trees with an unrepresentable path (must be non-zero).
- Obligation: `INV-order` and `INV-presence`.
  Method: property test with a tree generator and a segment oracle, plus named
  `rstest` rows. Rationale: creation order and tree shape span a large space;
  the oracle is independent of `Utf8PathBuf`. Domain: generated trees of depth
  1-3 with 0-12 files, segment names drawn from letters, digits, `-`, `_`, `.`,
  interior spaces, and non-ASCII letters, never starting or ending with
  whitespace; created in a generated permutation in a temporary directory; 256
  cases. Named rows pin `theorems/a/x.theorem` before `theorems/a-b.theorem` and
  `theorems/a b.theorem`. Artefact:
  `crates/theoremc-build/tests/prepare_properties.rs`; the named rows in
  `crates/theoremc-build/src/order_tests.rs`. Evidence: the suite parsed with
  `syn` into macro invocations yields literal values equal to the oracle, a
  `BTreeSet<Vec<String>>` of segments, in order;
  `has_theorems == !oracle.is_empty()`; contents are exactly `"\n"` when the
  oracle is empty. Non-vacuity: classification records empty trees,
  directories-only trees, and nested trees, each required to occur; seeded
  fault "remove the sort" must fail; `proptest-regressions/` is committed if a
  failure is ever found.
- Obligation: `INV-classification`.
  Method: parameterized `rstest` tables. Domain: symbolic link to a file, to a
  directory, to outside the crate, and as the root; a non-UTF-8 directory name
  and file name (Unix only, created with `std::os::unix::ffi::OsStrExt` in the
  test); nested paths (asserting no `\`); the D9 join helper on segments
  containing spaces and Unicode. Artefact:
  `crates/theoremc-build/src/classification_tests.rs` (new file so
  `discovery_tests.rs`, already 346 lines, stays under 400). Evidence: each row
  returns the expected `PrepareError` variant and code, naming the expected
  crate-relative path. Non-vacuity: each row asserts the offending entry
  exists, using `symlink_metadata`, before calling `prepare_suite`.
- Obligation: `INV-write`.
  Method: named unit tests. Artefact:
  `crates/theoremc-build/src/write_tests.rs`. Evidence: absent then `Written`;
  identical then `Unchanged` with a sentinel modification time preserved (set
  with `filetime`); different then `Written`; previous file containing invalid
  UTF-8 then `Written`; a directory in place of the file then `WriteError` with
  code `build.suite_write`; the directory listing before and after differs only
  in `theorem_suite.rs`, and no `.theorem_suite.rs.tmp` remains. Non-vacuity:
  seeded fault "write in place without rename" is caught by a test that injects
  a failing rename through a read-only temporary sibling and asserts the
  previous suite is intact.
- Obligation: `INV-late-failure`, `INV-codes`, and the exit contract.
  Method: in-process tests of the private runner with an injected lookup,
  capability directories, and in-memory or failing writers; a wildcard-free
  `match` over `BuildDiagnosticCode` in the test helper so a new variant fails
  to compile until it is tested; one consolidated `insta` snapshot of every
  code with a synthetic error's rendered line. Artefact:
  `crates/theoremc-build/src/emit_tests.rs`,
  `crates/theoremc-build/src/error_tests.rs`. Evidence: missing and non-UTF-8
  variables, an unopenable `OUT_DIR`, and a directory in place of
  `theorem_suite.rs` each produce one error line, the failure status, and no
  other line; a writer that fails produces the failure status; success returns
  `Success` with the handwritten expected lines. Non-vacuity: seeded fault
  "print directives before writing" must fail the late-failure case.
- Obligation: `INV-parity`.
  Method: in-process parity over the whole corpus, plus behavioural tests with
  real Cargo over two corpus cases. Rationale: `L-boundary-composes-direct`
  reduces parity to `INV-order` plus Cargo actually running the boundary; two
  real builds establish the latter without the cost of one build per case.
  Domain: corpus cases `no_theorem_root`, `empty_theorem_root`,
  `single_theorem`, `nested_with_ignored_files`, `unsorted_creation`,
  `unusual_names`, `unwatched_directory`, `mixed_tree` (nested, unsorted,
  ignored files, quotes, spaces, Unicode, and U+202E in a file name),
  `theorem_root_is_file`, and `symlinked_theorem` (Unix). Real Cargo runs
  `no_theorem_root` and `mixed_tree`. Artefact:
  `crates/theoremc-build/tests/corpus_parity.rs`; `tests/build_boundary_bdd.rs`,
  `tests/features/build_boundary.feature`. Evidence: in-process, the runner's
  output for each case equals the handwritten expectation stored with the case;
  through Cargo, the files at the `out_dir` reported by `--message-format=json`
  match the direct API exactly. Non-vacuity: each case asserts its expected
  theorem count before comparing; exactly one `output` file must exist beside
  `out_dir`; seeded fault "swap the watched and files order in the renderer"
  must fail the outline.
- Obligation: `INV-env-seam`.
  Method: lint and search. Evidence: `make lint` passes with exactly one
  `#[expect]` for `clippy::disallowed_methods` in `crates/theoremc-build`;
  `rg -n 'env::var|ambient_authority' crates/theoremc-build/src` lists only
  `emit.rs` outside `*_tests.rs` files (`test_support::materialize` writes into
  a `Dir` supplied by the caller and never opens ambient paths).

Assertion libraries: `pretty_assertions::assert_eq!` for whole suite contents
and directive vectors; `googletest` (`assert_that!`, `matches_pattern!`,
`elements_are!`) for error shapes, rerun-path lists, and `WriteOutcome`;
`prop_assert_eq!` inside property bodies; `insta` only for the three
consolidated snapshots, never for generated cases.

### Formal verification decision

No Verus proof and no Kani harness are planned (D7). Verus is warranted for
contractual logic whose guarantee must hold over an unbounded domain and that
has a small pure kernel independent of external semantics. The kernels here are
character-level (`esc`, `sanitize`, `representable`) or ordering functions. The
character-level kernels have a finite domain (every Unicode scalar value),
which the plan enumerates exhaustively, so a proof would add nothing over
complete enumeration; their correctness is also defined by agreement with
Rust's lexer and Cargo's parser, which a Verus model would have to restate as
unverified axioms. The ordering kernel delegates to standard library
comparison, whose correctness is an axiom, and the property test with an
independent segment oracle checks that the composition preserves it. Kani could
explore short strings, but it cannot run `syn`, and exhaustive character
enumeration dominates it here. Introducing Verus would also require new
toolchain and CI wiring with no other consumer. Residual gap: multi-character
interactions in `esc` are covered by contiguous blocks and the property test
rather than exhaustively; since `esc` is a per-character map with no state,
this is not a material gap. No proof milestone exists, so no Verus command is
part of acceptance.

### Suggested BDD features

Adopted, path `tests/features/build_boundary.feature`, runner
`cargo nextest run -p theoremc --test build_boundary_bdd`. Given steps
materialize corpus cases into a fixture, When steps run Cargo with
`--message-format=json` and keep the result in a step fixture, and Then steps
read the files Cargo reported; no step does the whole scenario.

```gherkin
Feature: Stable build-script boundary and direct suite generation
  Theorem adopters call one build-script function, and tools call a fallible
  service; both must produce the same generated suite and Cargo directives.

  Scenario Outline: The build script and direct generation agree
    Given the shared build corpus case "<case>"
    When Cargo builds the case through the theoremc-build boundary
    Then the generated suite equals the direct API's suite byte for byte
    And the recorded build-script directives equal the direct API's directives

    Examples:
      | case            |
      | no_theorem_root |
      | mixed_tree      |

  Scenario: An adopter crate compiles theorems through the facade call
    Given an adopter crate whose build script calls theoremc::build::emit_suite_from_env_or_exit
    And the crate has one theorem file
    When Cargo builds the adopter crate
    Then the build succeeds and the theorem suite is compiled

  Scenario: A misconfigured theorem root fails with a stable diagnostic
    Given the shared build corpus case "theorem_root_is_file"
    When Cargo builds the case through the theoremc-build boundary
    Then the build fails
    And the build log shows the diagnostic code "build.theorem_root_not_directory"
```

The exact rendering asserted by the failure scenario (Cargo's `error:` line for
a `cargo::error=` directive, or the `--- stdout` dump for a non-zero exit) is
fixed from the `EP-M0` transcript, not guessed. Existing features
`tests/features/build_discovery.feature` and
`tests/features/build_suite.feature` stay unchanged and must pass before and
after every milestone.

## Plan of work

### `EP-M0` directive-parsing probe (prototype, deleted)

Question: does Cargo on the pinned toolchain behave as
`AXIOM-cargo-directive-parse` and `AXIOM-cargo-rerun` state? In a scratch crate
under `/tmp/theoremc-probe/` (never inside the worktree), write a `build.rs`
that prints, in separate runs controlled by a file the script reads: a
`rerun-if-changed` line whose value ends in one space; a value with a leading
space; a value containing `=`; a nonexistent path; and
`cargo::error=probe: message`, once with exit status 0 and once with exit
status 1. Run each with
`cargo build -vv --message-format=json 2>&1 | tee /tmp/probe-theoremc-4-3-1.out`.
Record: whether the trailing space is removed in the recorded `output` file;
whether an edit below a directory whose name ends in a space reruns the script
through the root watch; how Cargo renders the error in each exit mode; and the
shape of the `build-script-executed` message (`out_dir`, `cfgs`). Go: implement
D5 and D6 as planned and fix the failure-scenario assertion from the
transcript. No-go: record the behaviour in `Surprises & discoveries`, revise D5
or D6, and escalate. Delete the scratch crate afterwards. The probe runs only
after approval.

### `EP-M1` ADR-0014, atomic rename, and fixture dependencies

Assigned: `REQ-reusable-crate`, `GAP-internal-only`, `GAP-copied-fixtures`, D1,
D4.

1. Write
   `docs/adrs/adr-0014-reusable-theoremc-build-library-and-stable-build-script-boundary.md`
   following the style guide template, recording D1-D3, D5, D6, D8, D12-D15,
   the rejected alternatives, the cap-std 4 public-surface trade-off, the
   relation to ADR-0011 decision 7, the reuse policy, and the note that
   emitting any `rerun-if-changed` line disables Cargo's default of rerunning
   the build script on any package file change (which the 4.3.4
   `build-script check` must consider). Status `Accepted` (the probe has passed
   by this point). Mark ADR-0008 `Superseded by ADR-0014`. Index ADR-0014 in
   `docs/contents.md`. Run `make check-adr-rfc-names`.
2. Before the rename, run `rg -n 'include_str!\("../../crates' tests` and
   record the matches (expected: the five source copies).
3. `git mv crates/theoremc-build-support crates/theoremc-build`; set
   `name = "theoremc-build"`; update the workspace member list, the root
   `[build-dependencies]` entry, `build.rs`
   (`theoremc_build::prepare_build_script()`; behaviour unchanged at this
   plateau), and every `theoremc_build_support` path in doctests and crate
   documentation. Run `cargo update -w`.
4. Replace the copied-source scaffold in `tests/common/fixture_crate.rs`:
   delete the `BUILD_SUPPORT_*` constants, `BUILD_DISCOVERY_SOURCE`,
   `BUILD_SUITE_SOURCE`, `fixture_build_support_cargo_toml`, and its tests; make
   `tests/common/build_support_fixture.rs` (renamed `build_fixture.rs`) emit
   `theoremc-build = { path = "<absolute workspace>/crates/theoremc-build" }`
   with the TOML escaping used by
   `tests/theorem_file_macro_bdd/fixture_crate.rs`; rename
   `build_support_fixture_manifest` to `build_fixture_manifest`; update the
   macro fixture manifest's dependency name and path only; update
   `crates/test-helpers` re-exports. Rerun the step 2 search (expected: no
   matches).
5. Update `.config/nextest.toml` comments that name the old crate.

Red/green: this milestone preserves behaviour, so there is no new failing test.
The regression net is the existing `build_discovery_bdd`, `build_suite_bdd`,
`theorem_file_macro_bdd`, and crate unit tests, run as a baseline before the
change and again after.

### `EP-M2` capability-based direct API, corpus, directives, and boundary

Assigned: `REQ-fallible-direct`, `REQ-stable-boundary` (library half),
`REQ-shared-fixtures` (corpus), `GAP-adapter-in-root`, `GAP-no-error-contract`,
`GAP-directive-injection`, `GAP-silent-absence`, `GAP-host-separators`,
`GAP-non-atomic-write`, D5-D15.

Red first, with real assertion failures rather than compile errors:

1. Land the public signatures from `Interfaces and dependencies` as stubs that
   delegate to the existing behaviour: `prepare_suite` wraps
   `discover_theorem_inputs` without symlink, representability, or separator
   changes; `cargo_directives` formats like the base-revision `build.rs`;
   `write_to` writes in place; error codes exist. Commit only after the next
   step's tests are in place and fail.
2. Add the tests from `Methods per obligation` (escape, directives, order,
   classification, write, emit, error, corpus parity) and the `test_support`
   corpus. Run them and record the expected failures: the injection regression
   test reports two lines; the symlink rows report silent skips; the
   invalid-UTF-8 write row reports an error; the late-failure test sees
   directives before the error; the bidirectional-character block contains a
   raw U+202E.

Green, module by module, keeping each file under 400 lines:

- `error.rs`: `PrepareError`, `WriteError`, crate-private `BuildScriptError`,
  `BuildDiagnosticCode` with `as_str` and `Display`, and the
  `Send + Sync + 'static` assertion.
- `path.rs`: the D9 join helper and the D5 `representable` predicate.
- `discovery.rs`: crate-private traversal using `symlink_metadata`, D8, D9,
  and `build.non_utf8_entry`.
- `escape.rs`: `esc` with D11.
- `suite.rs`: `prepare_suite`, `PreparedSuite`, rendering.
- `write.rs`: `write_to` and `WriteOutcome` with D10.
- `directives.rs`: `CargoDirectives`, success rendering with warnings, and
  failure rendering with `sanitize`.
- `emit.rs`: the private `run_build_script(lookup, writer) -> RunStatus` and
  `emit_suite_from_env_or_exit()`: render the whole block into one `String`,
  `write_all` and `flush` once on locked standard output (no `BufWriter`); on a
  writer error, write a one-line message to standard error and exit 1; never
  panic; return normally on success.
- `test_support.rs` (feature `test-support`): `BuildCase`, `CORPUS`, and
  `materialize(&Dir)`.
- Delete `build_script.rs` and `build_script_tests.rs`, migrating their
  still-relevant cases into `emit_tests.rs`. Replace the root `build.rs` body
  with `theoremc_build::emit_suite_from_env_or_exit();`.

Refactor: keep functions small, rerun the focused tests, run each seeded fault
once and revert it, then run all gates. The existing behavioural suites must
stay green without edits to their feature files.

### `EP-M3` facade, parity, and behavioural tests

Assigned: `REQ-stable-boundary` (adopter half), `REQ-shared-fixtures`, D2, D3.

1. Add `tests/common` support for `cargo build --message-format=json`: parse
   the `build-script-executed` message for the fixture package, return its
   `out_dir`, and require exactly one sibling `output` file. Use `serde_json`
   (already a root dev-dependency).
2. Red: add `tests/features/build_boundary.feature` and
   `tests/build_boundary_bdd.rs`, re-export the corpus from
   `crates/test-helpers`, and add `binary(=build_boundary_bdd)` to the
   `nested-cargo-builds` filter in `.config/nextest.toml`. The adopter scenario
   fails because rustc reports `E0433` for `theoremc::build`; assert that code
   in the red transcript.
3. Green: add to `src/lib.rs` a documented `pub mod build` that only
   re-exports the items listed in `Interfaces and dependencies`, and add
   `theoremc-build` to the root `[dependencies]`. The adopter fixture uses the
   macro fixtures' dependency declarations (`fixture_cargo_toml`) plus its own
   two-line `build.rs`, so compiled units are shared. The
   `theorem_file_macro_bdd` fixtures are not otherwise changed.
4. Run the parity seeded fault once and revert it. Run all gates. Push and
   confirm that `proofs.yml` and `ci.yml` pass on the pull request, recording
   the `proofs.yml` duration.

### `EP-M4` documentation closure

Assigned: all requirements (documentation), roadmap.

Update `docs/users-guide.md` ("Build discovery" becomes "Build integration":
the one-line adopter `build.rs`, the `[build-dependencies]` entry, the
`include!` pattern with `SUITE_FILE_NAME`, the `theoremc_has_theorems` cfg,
that emitting rerun directives disables Cargo's default rerun-on-any-change,
the diagnostic codes and their provisional status, the unwatched-path warning,
the symbolic-link and non-UTF-8 rules, segment-wise ordering, and the non-UTF-8
`OUT_DIR` limitation); `docs/developers-guide.md` §§1.1-1.3, §2 layer table,
§2.3 "Build system API", and §3.6 "Extending the build system" (crate
ownership; the reuse policy; the environment and capability seams; the corpus
and how to add a case; why only two corpus cases run through Cargo);
`docs/theoremc-design.md` overview and sequence diagrams and §7.1 (pin
segment-wise order in §7.1.1; add
`### 7.1.2 Implementation decisions (Task 4.3.1)` referencing ADR-0014, D5, D8,
D10, D11, and D12); `docs/cargo-theorem-cli-design.md` §10.1 (name the fallible
API and link ADR-0014); `docs/repository-layout.md`; `docs/contents.md`; and
`docs/roadmap.md` (mark 4.3.1 done; update the 5.1 note naming the old crate).
Search for stale references:

```bash
rg -n 'theoremc-build-support|theoremc_build_support|prepare_build_script|BuildScriptOutput' \
  --glob '!docs/execplans/**' --glob '!docs/adrs/**' .
rg -n 'theoremc-build-support' docs/adrs --glob '!adr-0008-*' --glob '!adr-0010-*' --glob '!adr-0014-*'
```

Expected: no matches from either command. Run `make markdownlint`, `make nixie`
(diagrams change), `make spelling`, and `mdtablefix --check` on each edited
file.

## Milestones and plateaus

- Identifier and outcome: `EP-M0`, recorded evidence that Cargo's directive
  handling matches the axioms.
  - Requirements and gaps: `GAP-directive-injection` (evidence only).
  - Acceptance evidence: transcript in `Artefacts and notes`.
  - Conformance check: no repository change; scratch crate deleted.
  - Recovery: rerun the probe; it is side-effect free.
  - Remaining gaps: all implementation.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M1`, ADR-0014 accepted, crate renamed, fixtures
  depend on the real crate; behaviour unchanged.
  - Requirements and gaps: `REQ-reusable-crate` (name and ownership),
    `GAP-internal-only`, `GAP-copied-fixtures`.
  - Acceptance evidence: all gates green; existing behavioural suites pass;
    `cargo tree -p theoremc-build --edges normal` lists only `camino`,
    `cap-std`, `thiserror` and their transitive dependencies; the
    `include_str!` search is empty.
  - Conformance check:
    - REQ-reusable-crate satisfied for naming; API reshaping remains.
    - ADR-0014 accepted; ADR-0008 marked superseded.
    - No upstream assumption falsified.
    - No new public interface; the rename is the authorized change.
    - No trust-boundary or persisted-format change.
    - Trace links current.
  - Recovery: revert the milestone commits; the rename is one coherent
    change.
  - Remaining gaps: every other gap.
  - Compatibility decision: none (unreleased, no external consumer).
- Identifier and outcome: `EP-M2`, the library owns the whole build-script
  boundary and the capability-based direct API; root `build.rs` is one call.
  - Requirements and gaps: `REQ-fallible-direct`, `REQ-stable-boundary`
    (library), `REQ-shared-fixtures` (corpus), `GAP-adapter-in-root`,
    `GAP-no-error-contract`, `GAP-directive-injection`,
    `GAP-silent-absence`, `GAP-host-separators`, `GAP-non-atomic-write`.
  - Acceptance evidence: new unit, enumeration, property, and snapshot tests
    pass; red failures and seeded-fault rejections recorded; existing
    behavioural suites pass unchanged.
  - Conformance check:
    - Requirements above satisfied.
    - CLI design §§5 and 10.1 split (capability-based service versus Cargo
      boundary) followed.
    - Axioms still consistent with `EP-M0`.
    - Public surface equals `Interfaces and dependencies`.
    - Persisted contract unchanged except for the listed deliberate changes.
    - Trace links current.
  - Recovery: revert to the `EP-M1` commit.
  - Remaining gaps: facade path and Cargo parity evidence.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M3`, adopters use
  `theoremc::build::emit_suite_from_env_or_exit()`; parity is proven through
  real Cargo.
  - Requirements and gaps: `REQ-stable-boundary` (adopter),
    `REQ-shared-fixtures`.
  - Acceptance evidence: `build_boundary_bdd` scenarios pass inside the
    serialized group; `theorem_file_macro_bdd` passes unchanged in behaviour;
    `ci.yml` and `proofs.yml` pass on the pull request.
  - Conformance check:
    - Requirements satisfied.
    - Facade re-exports only; no logic in the root package.
    - Kani fixtures unchanged apart from dependency name and path.
    - No new dependency beyond the authorized list.
    - Trace links current.
  - Recovery: revert to the `EP-M2` commit.
  - Remaining gaps: documentation.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M4`, documentation and roadmap reflect the
  shipped boundary.
  - Requirements and gaps: all, documentation only.
  - Acceptance evidence: stale-reference searches return nothing; Markdown
    gates pass; roadmap 4.3.1 checked.
  - Conformance check: upstream design documents reconciled with ADR-0014.
  - Recovery: documentation-only revert.
  - Remaining gaps: CLI commands in 4.3.4 and 4.3.5.
  - Compatibility decision: none.

## Concrete steps

Working directory: the repository root of the worktree. Use `set -o pipefail`
in every shell that pipes a gate through `tee`. Logs follow
`/tmp/$ACTION-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out`.

Baseline before `EP-M1`:

```bash
set -o pipefail
make test 2>&1 | tee /tmp/test-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
```

Expected: all tests pass; record the count.

Focused commands during `EP-M2`:

```plaintext
Red:   cargo nextest run -p theoremc-build --all-features
       -> assertion failures listed in EP-M2 step 2 (not compile errors)
Green: cargo nextest run -p theoremc-build --all-features -> all pass
Faults (each run once, then reverted): drop '"' arm; drop '\\' arm;
       drop '\r' and control arms; unpadded \x; remove sort; skip sanitize;
       in-place write; print directives before writing
Snapshots: cargo insta review (accept only after reading each snapshot)
```

Focused commands during `EP-M3`:

```plaintext
Red:   cargo nextest run -p theoremc --test build_boundary_bdd
       -> adopter scenario fails with E0433 for theoremc::build
Green: cargo nextest run -p theoremc --test build_boundary_bdd -> all scenarios pass
Fault: swap watched/files order in the renderer -> parity outline fails; revert
```

Gate sequence at the end of each milestone (sequential, via `scrutineer` where
possible):

```bash
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
make lint 2>&1 | tee /tmp/lint-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
make test 2>&1 | tee /tmp/test-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
make markdownlint 2>&1 | tee /tmp/markdownlint-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
make spelling 2>&1 | tee /tmp/spelling-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
make check-adr-rfc-names 2>&1 | tee /tmp/adr-names-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
```

`make nixie` runs whenever a Mermaid diagram changes (`EP-M4`). Known
host-specific behaviour: the Kani behavioural scenario can fail locally because
of the rustup library path even on `main`; CI does not install Kani for
`ci.yml`, so it self-skips there, and `proofs.yml` is the authority.

## Validation and acceptance

Behavioural acceptance:

- In a fresh crate with `[build-dependencies] theoremc = { path = ... }`,
  `[dependencies] theoremc = { path = ... }`, a `build.rs` containing only
  `theoremc::build::emit_suite_from_env_or_exit();`, a `src/lib.rs` with the
  documented `include!` pattern, and `theorems/smoke.theorem`, `cargo build`
  succeeds, and the build-script `output` file contains
  `cargo::rustc-cfg=theoremc_has_theorems` and
  `cargo::rerun-if-changed=theorems/smoke.theorem`.
- Replacing `theorems/` with a file makes `cargo build` fail and show
  `build.theorem_root_not_directory`.
- `theoremc::build::prepare_suite(&crate_root)` on the same tree returns
  contents byte-identical to `OUT_DIR/theorem_suite.rs` and
  `cargo_directives()` equal to the recorded directive lines.

Quality criteria:

- Tests: `make test` passes, including the new `theoremc-build` tests, the new
  `build_boundary_bdd` scenarios, and the unchanged `build_discovery_bdd`,
  `build_suite_bdd`, and `theorem_file_macro_bdd` suites.
- Verification: every obligation in `Methods per obligation` passes with its
  non-vacuity evidence and seeded-fault rejection recorded; no Verus or Kani
  obligation exists (D7).
- Lint and formatting: `make check-fmt` and `make lint` pass with no new
  `#[expect]` other than the single environment seam.
- CI: `ci.yml` and `proofs.yml` pass on the pull request.
- Documentation: Markdown gates pass and the stale-reference searches are
  empty.

## Idempotence and recovery

Every step is repeatable. The rename uses `git mv`; repeating it after success
fails harmlessly. `cargo update -w` is idempotent. Snapshot acceptance is
repeatable with `cargo insta test --review`. Seeded faults are applied and
reverted within one working session and never committed; `git diff` must be
empty of fault edits before each commit. Each milestone ends in a commit, so
recovery is `git revert` of that milestone's commits or a reset to the previous
milestone commit on this branch. The probe lives only under `/tmp` and is
deleted afterwards. Fixture crates are temporary directories cleaned
automatically; the shared fixture target follows ADR-0012.

## Artefacts and notes

Reconnaissance evidence (before implementation):

```plaintext
crates/theoremc-build-support/src: lib.rs 15, build_script.rs 195, build_script_tests.rs 244,
discovery.rs 267, discovery_tests.rs 346, suite.rs 148, suite_tests.rs 90 (total 1305)
git tag: (none)    publish flags: (none)    verus!: (no matches)
```

Cargo directive parsing (DeepWiki over `rust-lang/cargo`, to be confirmed by
`EP-M0`):

```rust
let mut iter = data.splitn(2, "=");
let key = iter.next();
let value = iter.next();
match (key, value) {
    (Some(a), Some(b)) => Ok((a, b.trim_end())),
```

Design review summary (2026-11-16): four reviewers. Structure and contracts:
capability handles instead of paths, split errors, `#[non_exhaustive]`,
explicit exit path, public directive output, D5 rationale corrected.
Alternatives and viability: supersede ADR-0008, larger file budget, split
`discovery_tests.rs`, keep Kani fixtures off the facade, Windows separators,
symbolic links. Failure modes and cost: atomic suite write, symbolic-link
policy, single flushed write, nextest group membership, two real-Cargo parity
cases, JSON messages for locating `OUT_DIR`. Verification: segment-wise
ordering oracle, effective seeded faults, exhaustive character enumeration,
late-failure obligation, bidirectional-character hazard, real red stages.

Implementation transcripts, seeded-fault results, fixture timings, and final
size counts are added here as milestones complete.

## Interfaces and dependencies

Crate `theoremc-build` (`crates/theoremc-build`), library `theoremc_build`. Its
normal dependencies are only `camino`, `cap-std` (`fs_utf8`), and `thiserror`.
Layering: pure kernels (`escape.rs`, `path.rs`, rendering in `suite.rs`,
`directives.rs`, `error.rs`) perform no I/O; driven ports are the injected
environment lookup closure, the `cap_std::fs_utf8::Dir` capabilities for the
crate root and output directory, and an `io::Write` sink; driving entry points
are `prepare_suite` with `PreparedSuite::write_to` and
`PreparedSuite::cargo_directives` (direct execution) and
`emit_suite_from_env_or_exit` (Cargo adapter and composition root). Public
surface at the end of `EP-M2` (doc comments abbreviated):

```rust
pub const THEOREM_ROOT: &str = "theorems";
pub const SUITE_FILE_NAME: &str = "theorem_suite.rs";
pub const HAS_THEOREMS_CFG: &str = "theoremc_has_theorems";

/// Discovers and renders the suite for one crate root without writing.
pub fn prepare_suite(crate_root: &cap_std::fs_utf8::Dir) -> Result<PreparedSuite, PrepareError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedSuite { /* private */ }
impl PreparedSuite {
    #[must_use] pub fn contents(&self) -> &str;
    #[must_use] pub const fn has_theorems(&self) -> bool;
    pub fn theorem_files(&self) -> impl Iterator<Item = &Utf8Path>;
    pub fn rerun_paths(&self) -> impl Iterator<Item = &Utf8Path>;
    #[must_use] pub fn cargo_directives(&self) -> CargoDirectives;
    /// Atomically writes SUITE_FILE_NAME under out_dir when its contents differ.
    pub fn write_to(&self, out_dir: &cap_std::fs_utf8::Dir) -> Result<WriteOutcome, WriteError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoDirectives { /* private */ }
impl CargoDirectives {
    pub fn lines(&self) -> impl Iterator<Item = &str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome { Written, Unchanged }

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PrepareError {
    #[non_exhaustive] Io { operation: &'static str, path: Utf8PathBuf, source: io::Error },
    #[non_exhaustive] TheoremRootNotDirectory { path: Utf8PathBuf },
    #[non_exhaustive] SymlinkUnsupported { path: Utf8PathBuf },
    #[non_exhaustive] NonUtf8Entry { parent: Utf8PathBuf },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum WriteError {
    #[non_exhaustive] Io { operation: &'static str, source: io::Error },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BuildDiagnosticCode {
    MissingEnvironment, NonUtf8Environment, OpenDirectory, DiscoveryIo,
    TheoremRootNotDirectory, SymlinkUnsupported, NonUtf8Entry, SuiteWrite,
    UnwatchedPath,
}
impl BuildDiagnosticCode { #[must_use] pub const fn as_str(self) -> &'static str; }
impl fmt::Display for BuildDiagnosticCode { /* writes as_str */ }
impl PrepareError { #[must_use] pub const fn code(&self) -> BuildDiagnosticCode; }
impl WriteError { #[must_use] pub const fn code(&self) -> BuildDiagnosticCode; }
// Codes: "build.missing_environment", "build.non_utf8_environment",
// "build.open_directory", "build.discovery_io", "build.theorem_root_not_directory",
// "build.symlink_unsupported", "build.non_utf8_entry", "build.suite_write",
// "build.unwatched_path" (warning only).

/// Cargo build-script entry point: reads Cargo's environment, prepares and
/// writes the suite, prints directives, and exits with status 1 on failure.
/// Emitting rerun directives disables Cargo's default rerun-on-any-change.
pub fn emit_suite_from_env_or_exit();

#[cfg(feature = "test-support")]
pub mod test_support {
    pub struct BuildCase { /* name, entries, expected theorem count, expected directives */ }
    pub const CORPUS: &[BuildCase];
    pub fn materialize(case: &BuildCase, root: &cap_std::fs_utf8::Dir) -> io::Result<()>;
}
```

Crate-private items: traversal, `BuildScriptError`, `RunStatus`,
`run_build_script`, `esc`, `sanitize`, `representable`, and the join helper.
Removed public items: `prepare_build_script`, `BuildScriptOutput`,
`BuildScriptError`, `BuildDiscovery`, `BuildDiscoveryError`,
`discover_theorem_inputs`, `write_theorem_suite`, `BuildSuiteError`. Their only
callers are the root `build.rs`, fixtures, and documentation, all updated in
the same milestones.

Root facade `theoremc` (`src/lib.rs`): a documented `pub mod build` that
re-exports `emit_suite_from_env_or_exit`, `prepare_suite`, `PreparedSuite`,
`CargoDirectives`, `WriteOutcome`, `PrepareError`, `WriteError`,
`BuildDiagnosticCode`, `SUITE_FILE_NAME`, `THEOREM_ROOT`, and
`HAS_THEOREMS_CFG`. It contains no logic and does not re-export `test_support`.

Reuse policy (recorded in ADR-0014 and `docs/developers-guide.md`): only
`theoremc-build` discovers theorem files, renders the suite, or renders Cargo
directives. `cargo-theorem` and adopters call `prepare_suite`, `write_to`,
`cargo_directives`, or the boundary; they must not re-implement traversal,
reparse directives, or open project directories with ambient authority outside
their own composition root. Read-only consumers such as `list` (4.2.3) use
`prepare_suite(..).theorem_files()`. New build-integration features (manifest
and `build.rs` integration plans for 4.3.4, a configurable root per D14) extend
this crate additively, not the CLI.

Authorized dependency changes are listed below. All versions already resolve in
`Cargo.lock`.

```toml
# Root Cargo.toml
[dependencies]
theoremc-build = { version = "0.1.0", path = "crates/theoremc-build" }

[build-dependencies]
theoremc-build = { version = "0.1.0", path = "crates/theoremc-build" }

# crates/theoremc-build/Cargo.toml
[features]
test-support = []

[dev-dependencies]
filetime = "0.2.29"
googletest = "0.14.3"
insta = "1.47.2"
pretty_assertions = "1.4.1"
proptest = { version = "1.11.0", default-features = false, features = ["std"] }
syn = { version = "3.0.3", default-features = false, features = ["parsing", "full", "printing"] }

# crates/test-helpers/Cargo.toml
[dependencies]
theoremc-build = { path = "../theoremc-build", features = ["test-support"] }
```

No other dependency is added or changed.

## Revision note

- 2026-11-16: initial draft from Wyvern reconnaissance, design documents, and
  external research.
- 2026-11-16: community-of-experts review applied. Changed: ADR-0008 is
  superseded rather than amended (D1); the public API takes `Dir` capabilities
  and splits errors by operation (D15); D5 now omits unrepresentable rerun
  lines with a warning instead of failing; new decisions D8-D14 (symbolic links,
  `/` joins, atomic writes, bidirectional escapes, segment-wise order, corpus
  ownership, fixed root); Kani fixtures stay off the facade; only two corpus
  cases run through Cargo, located via JSON messages; the new test binary joins
  the serialized nextest group; verification now uses exhaustive character
  enumeration, a segment-vector ordering oracle, effective seeded faults, real
  red stages, and a late-failure obligation; file and documentation budgets
  raised to 58 files and 600 lines after a file-by-file count. Effect on
  remaining work: milestone order unchanged; `EP-M2` scope grows by D8-D11;
  `EP-M3` real-Cargo work shrinks.
