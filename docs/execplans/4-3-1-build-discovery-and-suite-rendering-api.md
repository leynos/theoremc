# 4.3.1 Extract build discovery and suite rendering behind a reusable API

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances (exception triggers)`, `Risks`, `Progress`,
`Surprises & discoveries`, `Decision log`, `Outcomes & retrospective`,
`Conformance basis`, and `Verification plan` must be kept up to date as work
proceeds.

Status: DRAFT

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
have no reusable service to call.

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
   `OUT_DIR/theorem_suite.rs` only when its contents change, prints every Cargo
   directive, and on failure prints one deterministic
   `cargo::error=<code>: <message>` line and exits with status 1.
3. A separate fallible API, `theoremc::build::prepare_suite(crate_root)`
   returning a `PreparedSuite`, plus `PreparedSuite::write_to(out_dir)`,
   performs the same discovery and rendering with explicit paths, no process
   environment, no standard output, and no process exit. This is the "direct
   execution" path that the future CLI commands will call.
4. The root `build.rs` becomes the same one-line call (through
   `theoremc_build`, because a package cannot be its own build dependency), and
   a shared fixture corpus proves that the Cargo build-script path and the
   direct path produce byte-identical suites, identical rerun paths, and the
   same theorem-presence flag.

Preserved behaviour: the discovery contract in `docs/theoremc-design.md` §7.1.1
(recursive `theorems/**/*.theorem`, forward-slash crate-relative paths,
lexicographic order, the always-emitted `cargo::rerun-if-changed=theorems`
line, per-file rerun lines, write-if-changed suite output, and the
`theoremc_has_theorems` cfg). Deliberately changed behaviour: build failures
become one stable `cargo::error=` diagnostic with a machine-readable code
instead of a `Debug`-formatted `Err` returned from `main`, and theorem-tree
entries whose crate-relative path cannot be written as a single Cargo directive
line are rejected with a stable diagnostic instead of silently corrupting the
directive stream (see `Decision log` D5).

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
- Production code must not read the process environment except at one
  composition-root function, `theoremc_build::emit_suite_from_env_or_exit`,
  which passes `std::env::var_os` into an injected lookup closure under a
  single item-scoped `#[expect(clippy::disallowed_methods, reason = "...")]`.
  Tests inject a closure; no test mutates the process environment in-process.
  Child `cargo` processes may receive environment through `Command::env`.
- Filesystem access stays capability-oriented through `cap_std::fs_utf8` and
  `camino`, as in the existing crate. Do not introduce `std::fs` or `std::path`
  in production code.
- Do not share build-support source with `#[path]` or `include_str!` copies;
  ADR-0008's crate-boundary decision remains in force.
- The workspace minimum supported Rust version stays 1.89 (ADR-0010). The
  `cargo::error=` directive is stable from Cargo 1.84, so it is available.
- `cargo-theorem` gains no new command, dependency, or behaviour in this task.
  Commands that call the direct API belong to roadmap tasks 4.3.4 and 4.3.5.
- The package rename is atomic: every caller, manifest, fixture, live
  document, and nextest filter changes in the same milestone. No alias crate,
  re-export shim, or deprecated name is introduced. The workspace has no
  release tag and no published crate, so no compatibility commitment exists.
- The generated file name `theorem_suite.rs`, the theorem root `theorems`, the
  cfg name `theoremc_has_theorems`, the suite line format
  `theorem_file!("<escaped path>");`, and the empty-suite content (a single
  newline) are persisted contracts consumed by `src/lib.rs` and adopters'
  `include!` sites. They must not change.
- Completed ExecPlans under `docs/execplans/` are historical and must not be
  edited. ADR-0008 is preserved; it receives only an `Amended by` banner and an
  `## Amendments` entry pointing to the new ADR, as
  `docs/documentation-style-guide.md` prescribes.
- Gates run sequentially: `make check-fmt`, `make lint`, `make test`, then the
  Markdown gates for documentation changes. Full gate runs are delegated to the
  `scrutineer` agent where practical.

## Tolerances (exception triggers)

All counts compare the working tree with the merge base of `main` (`4c05966` at
drafting time) and exclude every file under `docs/execplans/`. Each category is
enforced independently; spare capacity in one category does not offset another.
Counts measure net lines (additions minus deletions), because the rename and
the replacement of `build_script.rs` delete substantial existing code. A pure
`git mv` contributes zero net lines.

Classification rules:

- Code: `*.rs` files, `build.rs`, `.feature` files, and `insta` snapshot files
  (`*.snap`).
- Configuration: `Cargo.toml` files, `Cargo.lock`, `.config/nextest.toml`,
  `Makefile`, and `.github/**`.
- Documentation: `*.md` outside `docs/execplans/`, plus `typos.local.toml`.

Counting commands:

```bash
base=$(git merge-base HEAD origin/main)
git diff --numstat "$base" -- '*.rs' '*.feature' '*.snap' \
  | awk '{n += $1 - $2} END {print "code", n}'
git diff --numstat "$base" -- '*Cargo.toml' Cargo.lock .config Makefile .github \
  | awk '{n += $1 - $2} END {print "config", n}'
git diff --numstat "$base" -- '*.md' typos.local.toml ':!docs/execplans/**' \
  | awk '{n += $1 - $2} END {print "docs", n}'
git diff --name-only "$base" -- ':!docs/execplans/**' | wc -l
```

Budget survey. The existing crate is 1,305 lines including tests
(`build_script.rs` 195, `build_script_tests.rs` 244, `discovery.rs` 267,
`discovery_tests.rs` 346, `suite.rs` 148, `suite_tests.rs` 90, `lib.rs` 15).
The plan replaces `build_script.rs` and its tests, adds four small production
modules (Cargo environment, error codes, directive rendering, emission), adds
property tests, snapshot tests, a shared fixture corpus, a parity test and a
behavioural feature, and deletes the five `include_str!` source copies and the
manifest rewriter in `tests/common/fixture_crate.rs`. The estimate is roughly
+1,250 net code lines. Comparable history: the task 3.1.1 plan budgeted 10
files or 600 net lines and the task 3.1.2 plan 12 files or 700 net lines; the
task 4.1.2 plan used scope-based tolerances only. None of the three records a
budget increase, and none records its final size, so the history neither
confirms nor refutes those estimates. This task is broader than either (rename,
new public boundary, parity fixtures, and new verification), so the budgets
below carry about 20% contingency over the estimate.

- Files: if more than 45 non-ExecPlan files change (renames count once), stop
  and escalate.
- Code: if code changes exceed 1,500 net lines, stop and escalate.
- Configuration: if configuration changes exceed 100 net lines, stop and
  escalate. `Cargo.lock` package-name churn from the rename is expected and
  counted.
- Documentation: if documentation changes exceed 550 net lines, stop and
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
  `ortho_config`, or requires the CLI to own discovery or rendering logic, stop
  and escalate.
- Iterations: if a focused test still fails after three implementation
  attempts, stop and escalate.
- Time: if any milestone takes more than four hours of active work, stop and
  escalate.
- Ambiguity: if probe `EP-M0` contradicts `AXIOM-cargo-directive-parse`, or a
  reviewer requires the CLI command itself in this task, stop and present
  options.

## Risks

- Risk: Fixture crates that build-depend on `theoremc` (the facade) compile
  `theoremc-core` and `theoremc-macros` for the host, lengthening the
  `nested-cargo-builds` test group that is already near its 600-second
  allowance. Severity: medium. Likelihood: medium. Mitigation: only the
  adopter-path scenarios depend on the facade; the discovery, suite, and parity
  fixtures depend on `theoremc-build` alone. All fixtures share the warm target
  from ADR-0012. Record measured durations in `Artefacts and notes`; escalate
  before raising any nextest timeout.
- Risk: Switching fixture crates from copied sources to an absolute path
  dependency on `crates/theoremc-build` changes how `[lints] workspace = true`
  is resolved for the dependency. Severity: low. Likelihood: low. Mitigation:
  the `theorem_file_macro_bdd` fixtures already path-depend on the workspace
  crate successfully (`tests/theorem_file_macro_bdd/fixture_crate.rs`), so the
  mechanism is proven. Cargo caps lints for non-local dependencies.
- Risk: Cargo's directive parser trims values, so a watched directory whose
  name ends in whitespace would be watched under the wrong path, and a path
  containing a line break would inject a second directive. Severity: medium.
  Likelihood: low (unusual names). Mitigation: the representability rule (D5),
  its unit and property tests, and probe `EP-M0`.
- Risk: `cargo kani` in `.github/workflows/proofs.yml` builds the
  `theorem_file_macro_bdd` fixtures, whose build script changes to the facade
  call. Severity: medium. Likelihood: low. Mitigation: `make kani` is run
  locally in `EP-M3` if Kani is installed. The memory note on the local Kani
  behavioural test failing on a clean `main` applies, so a local failure is
  compared against `main` before it is treated as a regression.
- Risk: The rename touches many live documents; a stale
  `theoremc-build-support` reference survives. Severity: low. Likelihood:
  medium. Mitigation: the stale-reference search in `EP-M4` with an explicit
  exclusion for completed ExecPlans and the amended ADR-0008 text.
- Risk: `make check-fmt` does not detect committed Markdown problems because
  `mdtablefix --git` only inspects uncommitted files. Severity: low.
  Likelihood: medium. Mitigation: run `mdtablefix --check` and
  `make markdownlint` explicitly on every edited Markdown file before
  committing.

## Progress

- [x] (2026-11-16T19:02Z) Branch renamed, Lody session titled, skills loaded.
- [x] (2026-11-16T19:02Z) Wyvern reconnaissance of the build-support crate,
  facade, CLI, gates, documentation, and historical plans completed.
- [x] (2026-11-16T19:02Z) External facts gathered: Cargo build-script
  directives and parser behaviour, and `build-rs` prior art.
- [x] (2026-11-16) Initial draft written.
- [ ] Community-of-experts design review applied.
- [ ] Plan approved by the user.
- [ ] `EP-M0` directive-parsing probe recorded.
- [ ] `EP-M1` atomic rename to `theoremc-build`, ADR-0014, fixture path
  dependencies.
- [ ] `EP-M2` fallible direct API, directive rendering, and the build-script
  boundary.
- [ ] `EP-M3` `theoremc::build` facade, shared fixture corpus, parity and
  behavioural tests.
- [ ] `EP-M4` documentation closure and roadmap update.

## Surprises & discoveries

- Observation: the target crate name, public path, and visibility in roadmap
  4.3.1 contradict accepted ADR-0008 and `docs/developers-guide.md` §1.3.
  Evidence: ADR-0008 "Decision" says the support crate "does not make the
  package part of the public `theoremc` library API"; roadmap line 455 asks for
  `theoremc::build::emit_suite_from_env_or_exit()`. Impact: a new ADR must
  amend ADR-0008 before the rename lands (D1).
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
  `.github/workflows/`. Impact: informs the formal-verification decision (D7).
- Observation: Cargo trims the whole directive line and trims trailing
  whitespace from the value, and `cargo::error=` fails the build even when the
  script exits 0. Evidence: DeepWiki answer over `rust-lang/cargo`
  (`BuildOutput::parse`) and the Cargo Book build-script reference. Impact:
  `AXIOM-cargo-directive-parse`, D5, and `EP-M0`.

## Decision log

- D1. Decision: rename `crates/theoremc-build-support` to
  `crates/theoremc-build` (package `theoremc-build`, library `theoremc_build`)
  and make it the reusable build-integration library; record this in a new
  ADR-0014, "Reusable `theoremc-build` library and stable build-script
  boundary", which amends ADR-0008's visibility and reuse decision while
  keeping its crate-boundary decision. Rationale:
  `docs/cargo-theorem-cli-design.md` §5 Table 1 assigns reusable discovery,
  suite rendering, and build-script entry points to `theoremc-build`, and the
  roadmap names that crate. Keeping `theoremc-build-support` and adding a second
  `theoremc-build` crate on top would create two crates for one
  responsibility. A feature-grouped module inside the root facade was rejected
  because the root `build.rs` cannot build-depend on its own package. The
  workspace has no release, so the rename needs no alias. Date/Author:
  2026-11-16, planning agent (pending user approval).
- D2. Decision: in this task, "direct CLI path" means the fallible library
  service `prepare_suite` plus `PreparedSuite::write_to`, which
  `cargo theorem build-script run` (4.3.4) and `generate --kind suite` (4.3.5)
  will call. No CLI command is added now. Rationale: the roadmap assigns those
  commands to 4.3.4 and 4.3.5, which also require the mutation plan (4.3.2) and
  the JSON envelope (4.1.4). Adding a dependency from `cargo-theorem` with no
  caller would trip unused-dependency checks and pre-empt those tasks. The
  success criterion is met by proving that the build-script path and the direct
  service consume the same fixtures and produce identical output. Date/Author:
  2026-11-16, planning agent (pending user approval; flagged for explicit
  confirmation).
- D3. Decision: the stable adopter boundary is
  `theoremc::build::emit_suite_from_env_or_exit()`, a re-export from the root
  facade. The repository's own `build.rs` calls
  `theoremc_build::emit_suite_from_env_or_exit()` because Cargo forbids a
  package from build-depending on itself; both names denote the same function.
  Rationale: the CLI design §10.1 fixes the adopter-facing path; the facade
  keeps adopters on one dependency. A `build` Cargo feature on `theoremc` to
  avoid compiling `theoremc-build` for normal builds was rejected: the crate's
  dependencies (`camino`, `cap-std`, `thiserror`) are already in the graph, and
  a feature would make the stable call conditional. Date/Author: 2026-11-16,
  planning agent.
- D4. Decision: fixture crates depend on the real workspace crate through an
  absolute path dependency instead of receiving copied sources. Rationale:
  removes the brittle file list, honours ADR-0008's no-copy intent, and mirrors
  the existing `theorem_file_macro_bdd` fixtures. Date/Author: 2026-11-16,
  planning agent.
- D5. Decision: discovery rejects a theorem-tree entry whose crate-relative
  path contains a line feed or carriage return, or (for watched directories)
  ends with whitespace, returning `BuildError::UnrepresentablePath` with code
  `build.unrepresentable_path`. Rationale: Cargo directives are line-oriented
  and their values are trimmed (`AXIOM-cargo-directive-parse`). Such a path
  cannot be watched correctly, and a line break would let a file name inject an
  arbitrary directive such as `cargo::rustc-link-arg`. Escaping is impossible
  because Cargo defines no escape syntax; silently skipping the path would
  break the guarantee that edits rerun the build. `.theorem` files cannot end
  in whitespace because they must end with the `.theorem` extension. Rejection
  is a deliberate behaviour change recorded in the users' guide. Date/Author:
  2026-11-16, planning agent (pending user approval).
- D6. Decision: build failures render as exactly one line,
  `cargo::error=<code>: <message>`, where `<code>` is a stable `BuildErrorCode`
  string and `<message>` is the error's display text with control characters
  escaped; the helper then exits with status 1. Rationale: `cargo::error` is
  stable from Cargo 1.84 and is shown by Cargo without `-vv`; a stable code lets
  `cargo theorem check` (4.2.4) and `build-script run` (4.3.4) classify the
  same failures. Exiting non-zero as well as emitting the directive makes
  failure independent of directive support. Codes follow the existing
  `schema.parse_failure` style in
  `crates/theoremc-core/src/schema/diagnostic.rs`. Date/Author: 2026-11-16,
  planning agent.
- D7. Decision: no Verus proof and no Kani harness for this task; use
  parameterized tests, `proptest` properties with independent oracles, `insta`
  snapshots, and real-Cargo behavioural tests. Rationale: see
  `Formal verification decision`. The invariants are about agreement with two
  external grammars (Rust string-literal lexing and Cargo's directive parser);
  a Verus model would have to axiomatize exactly those grammars, so a proof
  would rest on an unverified restatement of the risk. Testing against `syn`
  and against real Cargo exercises the real interfaces. Date/Author:
  2026-11-16, planning agent.

## Outcomes & retrospective

Not started. Complete at each milestone boundary and at completion, including
measured sizes against `Tolerances`.

## Context and orientation

Inspected base revision: `4c05966` on `main` ("Chore(deps): bump rust-toolchain
(#255)"). Line numbers below refer to that revision; relocate symbols with
`rg -n '<symbol>' crates src tests build.rs`.

The workspace (`Cargo.toml`) has five members: `crates/test-helpers`,
`crates/theoremc-build-support`, `crates/cargo-theorem`,
`crates/theoremc-core`, and `crates/theoremc-macros`. The root package
`theoremc` (`src/lib.rs`, 41 lines) is the consumer-facing facade. It re-exports
`theoremc_core::{collision, mangle, schema}` and
`theoremc_macros::theorem_file`, and its private hidden module
`__theoremc_generated_suite` (`src/lib.rs:25-41`) does:

```rust
#[cfg(theoremc_has_theorems)]
use theoremc_macros::theorem_file;
include!(concat!(env!("OUT_DIR"), "/theorem_suite.rs"));
```

`theoremc-build-support` is a `[build-dependencies]` entry of the root package
(`Cargo.toml:25`) and has these modules:

- `src/discovery.rs`: `discover_theorem_inputs(manifest_dir)` opens the crate
  root with ambient authority, returns `BuildDiscovery` (sorted, deduplicated
  `theorem_files` and `watched_directories`, forward-slash crate-relative), or
  `BuildDiscoveryError::{Io, TheoremRootNotDirectory}`. An absent `theorems`
  directory yields a root-only watch set. Classification uses
  `DirEntry::file_type()`, so symbolic links inside the tree are skipped
  (untested).
- `src/suite.rs`: private `render_theorem_suite` emits one
  `theorem_file!("<escaped>");` line per file, or a single newline when empty;
  private `escape_rust_string` escapes `\`, `"`, `\n`, `\r`, `\t`, and other
  ASCII controls as `\xNN`. Public `write_theorem_suite(out_dir, discovery)`
  writes `theorem_suite.rs` only when the contents differ, with error
  `BuildSuiteError::Io`.
- `src/build_script.rs`: `prepare_build_script()` is the only environment
  reader. It passes `std::env::var` (under an item-scoped
  `#[expect(clippy::disallowed_methods)]`) into a private
  `prepare_build_script_with_env` closure seam, reads `CARGO_MANIFEST_DIR` and
  `OUT_DIR`, discovers, opens `OUT_DIR`, writes, and returns
  `BuildScriptOutput { has_theorems, rerun_paths }`. Errors are
  `BuildScriptError::{Environment, OpenOutputDirectory, Discovery, Suite}`.
- Unit tests live in `*_tests.rs` siblings using `rstest` and `tempfile`.

Behavioural tests are in the root package: `tests/build_discovery_bdd.rs` with
`tests/features/build_discovery.feature` (three scenarios) and
`tests/build_suite_bdd.rs` with `tests/features/build_suite.feature` (three
scenarios plus a shared-target isolation test). They build temporary fixture
crates with real `cargo build` through `test_helpers::FixtureCrate`
(`tests/common/fixture_crate.rs`), which copies the root `build.rs` and the
build-support sources into the fixture and rewrites its manifest to drop the
`[lints]` table. `crates/test-helpers/src/lib.rs` re-exports
`tests/common/mod.rs` through a `#[path]` module. All fixture crates share one
warm target directory (ADR-0012) and run serially in the nextest group
`nested-cargo-builds` (`.config/nextest.toml`), each with a 600-second
allowance. `tests/theorem_file_macro_bdd/fixture_crate.rs` builds macro
fixtures that path-depend on the root package and on
`crates/theoremc-build-support`; `make kani` runs two of those scenarios under
Kani in `.github/workflows/proofs.yml`.

`crates/cargo-theorem` implements only `cargo theorem context --json`.
`build-script` and `generate` appear only in its configuration-key allowlist
(`crates/cargo-theorem/src/config/mod.rs:46-63`). It has no dependency on the
build-support crate.

Tooling: nightly `nightly-2026-11-16` (`rust-toolchain.toml`), MSRV 1.89
checked by `make msrv`. Test frameworks already configured in the workspace:
`rstest` 0.28, `rstest-bdd` 0.6, `proptest` 1.11, `googletest` 0.14,
`pretty_assertions` 1.4, `insta` 1.47 (macros crate only), `syn` 3 (root
dev-dependency). Gates: `make check-fmt`, `make lint` (rustdoc, Clippy,
Whitaker), `make test` (nextest plus doctests), `make markdownlint`,
`make nixie`, `make spelling`. `mockable`, `mockall`, and `assert_cmd` are not
used anywhere; the environment seam convention here is a narrow closure, which
`AGENTS.md` accepts.

Documentation that names the build-support crate and must change: ADR-0008
(banner only), `docs/developers-guide.md` §§1.1-1.3 and §2 layer table,
`docs/theoremc-design.md` overview diagram and §§7.1-7.1.1,
`docs/repository-layout.md`, `docs/contents.md`, `docs/users-guide.md` "Build
discovery", and `docs/roadmap.md` (the 4.3.1 checkbox and the 5.1 note that
names `theoremc-build-support`). `docs/cargo-theorem-cli-design.md` already
uses the target name. `docs/CHANGELOG.md` is the imported OrthoConfig
changelog, not a project changelog, so it is not updated.

Prior art consulted: rust-lang's `build-rs` crate (0.3.5) offers typed Cargo
input and output, but its input functions such as
`build_rs::input::out_dir() -> PathBuf` panic on absence, which conflicts with
the fallible-API requirement; it is not adopted. `prost-build` and
`tonic-build` return `io::Result` from their compile functions and leave
process exit to the caller, which matches the fallible-core and thin `_or_exit`
adapter split chosen here.

Skills and documents to keep open while implementing: `rust-router` (then
`rust-types-and-apis` for the public surface, `rust-errors` for the error enum,
`rust-unit-testing` and `proptest` for tests), `hexagonal-architecture` for the
port and adapter split, `execplans` for maintaining this plan, and
`docs/rust-testing-with-rstest-fixtures.md`, `docs/rstest-bdd-users-guide.md`,
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`, and
`docs/developers-guide.md`. `docs/ortho-config-users-guide.md` is not needed
because no CLI configuration changes. `docs/theorem-file-specification.md` is
relevant only for the `.theorem` extension and the trivial fixture theorem.

## Conformance basis

Upstream artefacts at revision `4c05966`:

- `docs/roadmap.md` task 4.3.1 and its success criterion.
- `docs/cargo-theorem-cli-design.md` §5 (Table 1 package responsibilities and
  dependency directions), §10.1 (managed call contract), §10.2
  (`build-script run` uses "the same fallible build service directly"), §10.3
  (`generate` determinism), §8.4 (exit taxonomy, for later CLI mapping).
- `docs/theoremc-design.md` §7.1 and §7.1.1 (discovery and invalidation
  contract).
- ADR-0008 (crate boundary; amended here), ADR-0010 (MSRV), ADR-0012 (shared
  warm target), ADR-0011 decision 7 (build-support extraction stays off the
  Kani vertical-slice path; this task does not block slice work).
- No Terms of Reference document exists for this repository.

Requirements:

- `REQ-reusable-crate`: discovery and suite rendering live in a reusable
  `theoremc-build` library crate that depends on no application crate.
- `REQ-stable-boundary`: adopters' build scripts call
  `theoremc::build::emit_suite_from_env_or_exit()`, which owns environment
  reading, discovery, rerun output, suite writing, deterministic error
  rendering, and process termination (CLI design §10.1).
- `REQ-fallible-direct`: a fallible API performs the same work with explicit
  inputs, without process environment, standard output, or exit (CLI design
  §10.1-10.2).
- `REQ-shared-fixtures`: the root build script and the direct path use the
  same discovery and rendering fixtures and agree on outputs (roadmap success
  criterion).
- `REQ-preserve-contract`: the §7.1.1 discovery, ordering, invalidation, and
  file-name contract is unchanged.

Gaps:

- `GAP-internal-only`: ADR-0008 forbids public reuse of the support crate.
- `GAP-adapter-in-root`: Cargo directive emission lives in the root
  `build.rs`, not in the library.
- `GAP-no-error-contract`: failures surface as a `Debug` print of `Err` from
  `main`, with no stable code.
- `GAP-directive-injection`: unrepresentable paths can corrupt the directive
  stream.
- `GAP-copied-fixtures`: fixtures copy sources instead of depending on the
  crate.

Trace links:

```plaintext
Roadmap 4.3.1 -> REQ-reusable-crate -> GAP-internal-only -> D1, ADR-0014
  -> EP-M1 -> cargo tree -p theoremc-build shows no application crates
CLI §10.1 -> REQ-stable-boundary -> GAP-adapter-in-root, GAP-no-error-contract
  -> D3, D6 -> EP-M2, EP-M3 -> build_boundary_bdd scenarios 2 and 3
CLI §10.2 -> REQ-fallible-direct -> D2 -> EP-M2
  -> theoremc-build prepare tests and write tests
Roadmap 4.3.1 success -> REQ-shared-fixtures -> GAP-copied-fixtures -> D4
  -> EP-M1, EP-M3 -> build_boundary_bdd scenario 1 (parity)
DES §7.1.1 -> REQ-preserve-contract -> INV-order, INV-literal, INV-write-idempotent
  -> EP-M2 -> existing BDD suites unchanged and green
Cargo protocol -> GAP-directive-injection -> D5, INV-directive-line
  -> EP-M0, EP-M2 -> directive property tests
```

## Verification plan

### Abstract model

Let a theorem tree be a finite set `E` of crate-relative entries below
`theorems/`, each a directory or a file, with forward-slash paths. Define:

- `F(E)`: the set of regular files in `E` whose name ends in `.theorem`.
- `W(E)`: `{"theorems"}` when `theorems` is absent; otherwise `"theorems"`
  together with every directory in `E`.
- `representable(p)`: `p` contains no `\n` or `\r`, and `p` does not end with
  a Unicode whitespace character.
- `prepare(E)`: if any `p` in `F(E)` or `W(E)` is not representable, the error
  `build.unrepresentable_path`; else `PreparedSuite` with `files = sort(F(E))`,
  `watched = sort(W(E))`, `contents = concat(line(p) for p in files)` or `"\n"`
  when empty, where `line(p) = "theorem_file!(\"" + esc(p) + "\");\n"`, and
  `has_theorems = files non-empty`.
- `directives(S)` is the following ordered list:

  ```plaintext
  ["cargo::rustc-check-cfg=cfg(theoremc_has_theorems)"]
  ++ (["cargo::rustc-cfg=theoremc_has_theorems"] if S.has_theorems)
  ++ ["cargo::rerun-if-changed=" + p for p in S.watched ++ S.files]
  ```

- `failure(e) = ["cargo::error=" + code(e) + ": " + sanitize(msg(e))]` with
  exit status 1, where `sanitize` escapes every control character.

Preconditions: Cargo supplies `CARGO_MANIFEST_DIR` and `OUT_DIR` as UTF-8
paths, and `OUT_DIR` exists. Violations produce `build.missing_environment`,
`build.non_utf8_environment`, or `build.output_directory` failures rather than
panics.

### Invariants and lemmas

- `INV-order` (REQ-preserve-contract): `prepare(E).contents` and the directive
  list depend only on the set `E`, not on creation or directory-iteration order.
- `INV-literal` (REQ-preserve-contract): for every representable path `p`,
  `esc(p)` inside double quotes lexes as a Rust string literal whose value is
  exactly `p`.
- `INV-directive-line` (GAP-directive-injection): every line the build-script
  boundary writes to standard output, on success or failure, starts with
  `cargo::`, contains no `\n` or `\r`, and does not end with whitespace; the
  number of lines equals `|directives(S)|` on success and 1 on failure.
- `INV-representable` (D5): `prepare` rejects exactly the trees containing an
  unrepresentable file or watched directory path, naming the first such path in
  sorted order.
- `INV-write-idempotent` (REQ-preserve-contract): `write_to` creates or
  replaces `theorem_suite.rs` with `contents`, returns `Written` when the
  previous file was absent or different, returns `Unchanged` and performs no
  write when identical, and a second call always returns `Unchanged`.
- `INV-parity` (REQ-shared-fixtures, REQ-stable-boundary): for every shared
  fixture case, the suite file Cargo's run of the build script leaves in
  `OUT_DIR`, the rerun paths in Cargo's recorded build-script output, and the
  presence of the cfg equal `prepare(E).contents`, `watched ++ files`, and
  `has_theorems` from the direct API.
- `INV-failure-contract` (D6): every `BuildError` maps to exactly one stable
  code; the boundary emits exactly one `cargo::error=` line beginning with that
  code and exits with status 1; the direct API returns the same error value
  without writing to standard output.
- `INV-env-seam` (constraints): only `emit_suite_from_env_or_exit` reads the
  process environment.
- `L-boundary-composes-direct`: the build-script boundary is the composition
  `lookup -> CargoBuildEnv -> prepare_suite -> write_to -> directives`, so
  `INV-parity` reduces to `INV-order` plus the fact that Cargo runs the
  boundary with the fixture's own paths. This is a structural lemma checked by
  code review and by the in-process boundary unit test that compares its output
  with `directives(prepare_suite(...))`.

### Axioms (trusted, not verified here)

- `AXIOM-cargo-directive-parse`: Cargo reads build-script standard output line
  by line, trims each line, splits at the first `=`, trims trailing whitespace
  from the value, treats `cargo::error=` as a build failure even when the
  script exits 0, and fails the build when the script exits non-zero. Source:
  the Cargo Book build-script reference and `BuildOutput::parse` in
  `rust-lang/cargo` (via DeepWiki). Boundary: `directives.rs` and `emit.rs`.
  Probed in `EP-M0`; exercised by the behavioural failure scenario.
- `AXIOM-cargo-rerun`: `cargo::rerun-if-changed=<dir>` reruns the script when
  anything under the directory changes, including a missing directory being
  created. Source: Cargo Book; already exercised by
  `tests/features/build_discovery.feature`.
- `AXIOM-syn-lexer`: `syn::LitStr::value()` implements Rust string-literal
  unescaping. Used only as a test oracle for `INV-literal`.
- `AXIOM-cap-std`: `cap_std::fs_utf8::Dir` read and write operations behave as
  their documented POSIX counterparts, and `DirEntry::file_type()` does not
  follow symbolic links.

### Methods per obligation

- Obligation: `INV-order`.
  Method: property test. Rationale: the input space is sets of paths with
  arbitrary creation order; sampled permutations with an independent oracle
  give proportionate confidence. Domain: 1-12 distinct relative paths drawn
  from a small segment alphabet (letters, digits, `-`, `_`, space, Unicode
  letters) at depth 1-3, created in a generated permutation in a temporary
  directory. Artefact: `crates/theoremc-build/tests/prepare_properties.rs`.
  Evidence: `cargo nextest run -p theoremc-build --test prepare_properties`
  passes 256 cases. Non-vacuity: the oracle is a `BTreeSet` of the generated
  paths, independent of discovery; the suite is parsed with `syn` into macro
  invocations whose literal values must equal the oracle in order. A seeded
  fault that removes `sort_and_dedup` must fail (run once, recorded, reverted).
  Generator classification records cases with nesting and with more than one
  file.
- Obligation: `INV-literal`.
  Method: property test plus the existing named table. Rationale: escaping is a
  pure function over an open input space. Domain: arbitrary strings from
  `proptest`'s `\PC*` plus injected ASCII controls, quotes, backslashes, and
  astral characters, filtered to representable paths (filter rate recorded;
  must stay below 5%). Artefact: `crates/theoremc-build/src/suite_tests.rs`
  (named table kept) and `crates/theoremc-build/tests/prepare_properties.rs`
  (property). Evidence: `syn::parse_str::<syn::LitStr>` succeeds and `.value()`
  equals the input for all cases. Non-vacuity: the generator must hit every
  escape branch; a seeded fault that drops the `\t` arm or the `\x` arm must
  fail.
- Obligation: `INV-directive-line`.
  Method: property test plus `insta` snapshots. Rationale: the property covers
  arbitrary representable paths and arbitrary error text; snapshots pin the
  exact order and spelling. Domain: generated `PreparedSuite` values (through
  generated trees) and generated error messages containing arbitrary
  characters, including `\n`, `\r`, and trailing spaces. Artefact:
  `crates/theoremc-build/src/directives_tests.rs` and
  `crates/theoremc-build/src/snapshots/`. Evidence: every rendered line
  satisfies the predicate; snapshot files for "empty", "theorems present", and
  each `BuildErrorCode`. Non-vacuity: generated messages must include line
  breaks in at least 10% of cases (classified); a seeded fault that skips
  `sanitize` must fail.
- Obligation: `INV-representable`.
  Method: parameterized `rstest` table. Rationale: a finite partition (clean,
  line feed, carriage return, trailing space in a directory, trailing tab,
  leading space accepted, space inside accepted). Artefact:
  `crates/theoremc-build/src/discovery_tests.rs`. Evidence: rejected cases
  return `UnrepresentablePath` naming the path; accepted cases discover
  normally. Non-vacuity: each rejected row creates the offending entry on the
  host filesystem before calling `prepare_suite`; the test asserts the entry
  exists first.
- Obligation: `INV-write-idempotent`.
  Method: named unit tests. Artefact:
  `crates/theoremc-build/src/suite_tests.rs`. Evidence: absent then `Written`;
  identical then `Unchanged` with unchanged modification time; different then
  `Written`; directory in place of the file then `build.suite_write` error.
  Non-vacuity: the `Unchanged` case first writes a sentinel modification time
  with `filetime` (already a root dev-dependency; added as a dev-dependency of
  `theoremc-build`) and asserts it is preserved.
- Obligation: `INV-parity`.
  Method: behavioural test with real Cargo over the shared fixture corpus.
  Rationale: the property concerns the real Cargo integration, which no
  in-process test can stand in for. Domain: shared cases `no_theorem_root`,
  `empty_theorem_root`, `single_theorem`, `nested_with_ignored_files`,
  `unsorted_creation`, `unusual_names` (spaces, quotes, Unicode). Artefact:
  `tests/build_boundary_bdd.rs`, `tests/features/build_boundary.feature`, and
  the corpus in `tests/common/build_cases.rs`. Evidence: the parity scenario
  passes for every case. Non-vacuity: each case asserts its own expected
  theorem count before comparing, so an empty corpus or a build script that
  never ran fails; the comparison reads Cargo's recorded `output` file, not the
  direct API.
- Obligation: `INV-failure-contract`.
  Method: exhaustive `rstest` table over `BuildErrorCode` plus a behavioural
  scenario. Artefact: `crates/theoremc-build/src/error_tests.rs`,
  `crates/theoremc-build/src/emit_tests.rs`, and the failure scenario in
  `tests/features/build_boundary.feature`. Evidence: codes are distinct and
  stable (snapshot of the full code list); the in-process boundary runner
  returns failure status and writes one line; the Cargo fixture with `theorems`
  as a file fails with `cargo::error=build.theorem_root_not_directory` visible
  in the build log. Non-vacuity: the fixture asserts the build failed and that
  the log contains the code, so a passing build or a missing directive fails
  the scenario.
- Obligation: `INV-env-seam`.
  Method: lint (`clippy::disallowed_methods`, configured in `clippy.toml`) plus
  review. Evidence: `make lint` passes with exactly one `#[expect]` for that
  lint in `crates/theoremc-build`; `rg -n 'env::var' crates/theoremc-build/src`
  shows one site.
- Obligation: `L-boundary-composes-direct`.
  Method: in-process unit test of the private boundary runner with an injected
  lookup and an in-memory writer, compared against `prepare_suite` on the same
  temporary tree. Artefact: `crates/theoremc-build/src/emit_tests.rs`.

### Formal verification decision

No Verus proof and no Kani harness are planned (D7). Verus is warranted for
contractual logic whose guarantee must hold for all inputs and which has a
small pure kernel independent of external semantics. The kernels here are a
string-escape function and a line-rendering function whose correctness is
defined entirely by agreement with Rust's lexer and Cargo's parser. A Verus
proof would require modelling both as specifications; those models would be
unverified axioms equivalent to the property at risk, so the proof would add
ceremony without adding assurance. The escape function also predates this task
(task 3.1.2). Kani could exhaust short character sequences, but `syn`-oracle
property tests reach the same equivalence classes, including astral characters
that a small Kani bound would exclude, and run in the ordinary `make test`
gate. Introducing Verus would also require new toolchain and CI wiring with no
other consumer. The residual gap is that property tests sample rather than
exhaust the input space; the escape function has seven branches, each forced by
the generator and guarded by a seeded-fault check. No proof milestone exists,
so no Verus command is part of acceptance.

### Suggested BDD features

Adopted, path `tests/features/build_boundary.feature`, runner
`cargo nextest run -p theoremc --test build_boundary_bdd`:

```gherkin
Feature: Stable build-script boundary and direct suite generation
  Theorem adopters call one build-script function, and tools call a fallible
  service; both must produce the same generated suite.

  Scenario Outline: The build script and direct generation agree
    Given the shared build fixture "<case>"
    When Cargo builds the fixture through the theoremc-build boundary
    And the same fixture is prepared through the direct API
    Then the generated suite files are byte-identical
    And the rerun paths and theorem cfg agree

    Examples:
      | case                      |
      | no_theorem_root           |
      | single_theorem            |
      | nested_with_ignored_files |
      | unsorted_creation         |
      | unusual_names             |

  Scenario: An adopter crate compiles theorems through the facade call
    Given an adopter crate whose build script calls theoremc::build::emit_suite_from_env_or_exit
    And the crate has one theorem file
    When Cargo builds the adopter crate
    Then the build succeeds and the theorem suite is compiled

  Scenario: A misconfigured theorem root fails with a stable diagnostic
    Given the shared build fixture "theorem_root_is_file"
    When Cargo builds the fixture through the theoremc-build boundary
    Then the build fails
    And the build log contains "cargo::error=build.theorem_root_not_directory"
```

Existing features `tests/features/build_discovery.feature` and
`tests/features/build_suite.feature` stay unchanged and must pass before and
after every milestone (`REQ-preserve-contract`).

## Plan of work

### `EP-M0` directive-parsing probe (prototype, deleted)

Question: does Cargo on the pinned toolchain behave as
`AXIOM-cargo-directive-parse` states? In a scratch crate under
`/tmp/theoremc-probe/` (never inside the worktree), write a `build.rs` that
prints `cargo::rerun-if-changed=<directory name ending in one space>` and
`cargo::error=probe: message`, then exits 0. Run
`cargo build -vv 2>&1 | tee /tmp/probe-theoremc-4-3-1.out`. Expected: the build
fails with "build script logged errors", and the fingerprint file under
`target/debug/build/*/output` shows the trailing space removed. Go: implement
D5 and D6 as planned. No-go: record the observed behaviour in
`Surprises & discoveries`, adjust D5/D6, and escalate. Delete the scratch crate
afterwards. This probe runs only after approval.

### `EP-M1` atomic rename, ADR-0014, and fixture dependencies

Assigned: `REQ-reusable-crate`, `GAP-internal-only`, `GAP-copied-fixtures`, D1,
D4.

1. Write
   `docs/adrs/adr-0014-reusable-theoremc-build-library-and-stable-build-script-boundary.md`
   (status Accepted on plan approval) recording D1, D2, D3, D5, and D6 in
   Y-statement form, with the rejected alternatives. Add the
   `Amended by ADR-0014 (2026-11-16)` banner and an `## Amendments` section to
   ADR-0008. Index ADR-0014 in `docs/contents.md`. Run
   `make check-adr-rfc-names`.
2. `git mv crates/theoremc-build-support crates/theoremc-build`; set
   `name = "theoremc-build"` in its `Cargo.toml`; update the workspace member
   list and the root `[build-dependencies]` entry; update `build.rs` to call
   `theoremc_build::prepare_build_script()` (behaviour unchanged at this
   plateau); update every `theoremc_build_support` path in doctests and crate
   documentation. Run `cargo update -w` to refresh `Cargo.lock`.
3. Replace the copied-source scaffold in `tests/common/fixture_crate.rs`:
   delete the `BUILD_SUPPORT_*` constants, `BUILD_DISCOVERY_SOURCE`,
   `BUILD_SUITE_SOURCE`, `fixture_build_support_cargo_toml`, and its tests;
   change `tests/common/build_support_fixture.rs` to emit
   `theoremc-build = { path = "<absolute workspace>/crates/theoremc-build" }`
   using the same TOML escaping helper as
   `tests/theorem_file_macro_bdd/fixture_crate.rs`; update the macro fixture
   manifest and `crates/test-helpers` re-exports. Rename
   `build_support_fixture_manifest` to `build_fixture_manifest`.
4. Update `.config/nextest.toml` comments that name the old crate.

Red/green: this is a behaviour-preserving rename, so there is no new failing
test. The existing suites `build_discovery_bdd`, `build_suite_bdd`,
`theorem_file_macro_bdd`, and the crate's unit tests are the regression net;
they must pass before the change (baseline) and after. A deliberate check that
the copy is gone: `rg -n 'include_str!\("../../crates' tests` returns nothing.

### `EP-M2` fallible direct API, directive rendering, and boundary

Assigned: `REQ-fallible-direct`, `REQ-stable-boundary` (library half),
`GAP-adapter-in-root`, `GAP-no-error-contract`, `GAP-directive-injection`, D5,
D6.

Red first. Add, before production code, the focused tests that specify the new
surface and confirm they fail to compile or fail for the intended reason:

1. `crates/theoremc-build/src/error_tests.rs`: code table over every
   `BuildErrorCode` and a snapshot of the code list (fails: type absent).
2. `crates/theoremc-build/src/discovery_tests.rs`: the `INV-representable`
   table (fails: no rejection).
3. `crates/theoremc-build/src/suite_tests.rs`: the `INV-write-idempotent`
   tests against `PreparedSuite::write_to` (fails: method absent).
4. `crates/theoremc-build/src/directives_tests.rs`: snapshots and the
   `INV-directive-line` property (fails: module absent).
5. `crates/theoremc-build/src/emit_tests.rs`: the in-process boundary runner
   with injected lookup and `Vec<u8>` writer, covering missing variables,
   non-UTF-8 variables (via `OsString::from_vec` on Unix), success output equal
   to `directives(prepare_suite(..))`, and failure status with one line.
6. `crates/theoremc-build/tests/prepare_properties.rs`: `INV-order` and
   `INV-literal` properties (fails: `prepare_suite` absent).

Green. Implement the modules listed in `Interfaces and dependencies`:

- `error.rs`: `BuildError` (flattened from the three existing error enums)
  and `BuildErrorCode` with `as_str`.
- `cargo_env.rs`: `CargoBuildEnv::from_lookup`.
- `discovery.rs`: keep traversal; make `BuildDiscovery` crate-private; add the
  representability check; return `BuildError`.
- `suite.rs`: `TheoremSuite` rendering (pure), `PreparedSuite`,
  `prepare_suite`, `write_to`, `WriteOutcome`.
- `directives.rs`: pure rendering of success and failure directive lines.
- `emit.rs`: private `run_build_script(lookup, writer) -> BuildScriptStatus`
  and public `emit_suite_from_env_or_exit()`.
- Delete `build_script.rs` and `build_script_tests.rs`; migrate their
  still-relevant cases into `emit_tests.rs`.
- Replace the root `build.rs` body with
  `theoremc_build::emit_suite_from_env_or_exit();`.

Refactor: keep each file under 400 lines, keep functions small, rerun the
focused tests, then all gates. The existing behavioural suites must stay green
without edits to their feature files.

### `EP-M3` facade, shared fixture corpus, parity and behavioural tests

Assigned: `REQ-stable-boundary` (adopter half), `REQ-shared-fixtures`, D2, D3.

1. Red: add `tests/features/build_boundary.feature` and
   `tests/build_boundary_bdd.rs` with the three scenarios above; add the shared
   corpus `tests/common/build_cases.rs` (`BuildCase` with a name, a list of
   entries, and an expected theorem count, plus `materialize(&Dir)`), and
   re-export it from `crates/test-helpers`. The adopter scenario fails because
   `theoremc::build` does not exist.
2. Green: add to `src/lib.rs` a documented `pub mod build` that re-exports
   `theoremc_build::{emit_suite_from_env_or_exit, prepare_suite,
   PreparedSuite, WriteOutcome, BuildError, BuildErrorCode, SUITE_FILE_NAME}`,
   and add `theoremc-build` to the root `[dependencies]`. Switch the
   `theorem_file_macro_bdd` fixtures to build-depend on `theoremc` and to write
   their own two-line `build.rs` calling
   `theoremc::build::emit_suite_from_env_or_exit()`, so the macro suite and
   Kani scenarios exercise the adopter boundary.
3. Make `crates/theoremc-build` unit tests for discovery consume the same
   `BuildCase` corpus through a dev-dependency on `test-helpers` where the
   cases overlap, so "same fixtures" holds below the Cargo layer too. If this
   introduces a dependency cycle or pulls `theoremc-core` into the build
   crate's normal graph, keep the corpus in `test-helpers` only and record the
   decision.
4. Refactor and run all gates, plus `make kani` when Kani is installed.

### `EP-M4` documentation closure

Assigned: all requirements (documentation), roadmap.

Update `docs/users-guide.md` ("Build discovery" becomes "Build integration":
the one-line adopter `build.rs`, the `[build-dependencies]` entry, the
`include!` pattern with `SUITE_FILE_NAME`, the `theoremc_has_theorems` cfg, the
failure codes, the representability rule, and migration guidance for the
repository's own former adapter); `docs/developers-guide.md` §§1.1-1.3 and §2
(crate ownership, reuse policy: the CLI and adopters call `prepare_suite` and
the boundary, never re-implement discovery or rendering; the environment seam;
the fixture corpus and how to add a case); `docs/theoremc-design.md` overview
diagram and §7.1 (new `### 7.1.2 Implementation decisions (Task 4.3.1)`
referencing ADR-0014); `docs/cargo-theorem-cli-design.md` §10.1 (name the
fallible API and link ADR-0014); `docs/repository-layout.md` and
`docs/contents.md`; and `docs/roadmap.md` (mark 4.3.1 done; update the 5.1 note
naming the old crate). Search for stale references:

```bash
rg -n 'theoremc-build-support|theoremc_build_support|prepare_build_script|BuildScriptOutput' \
  --glob '!docs/execplans/**' --glob '!docs/adrs/adr-0008-*' .
```

Expected: no matches. Run the Markdown gates.

## Milestones and plateaus

- Identifier and outcome: `EP-M0`, recorded evidence that Cargo's directive
  parsing matches `AXIOM-cargo-directive-parse`.
  - Requirements and gaps: `GAP-directive-injection` (evidence only).
  - Acceptance evidence: transcript in `Artefacts and notes`.
  - Conformance check: no repository change; scratch crate deleted.
  - Recovery: rerun the probe; it is side-effect free.
  - Remaining gaps: all implementation.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M1`, crate renamed, ADR-0014 accepted, fixtures
  depend on the real crate; behaviour unchanged.
  - Requirements and gaps: `REQ-reusable-crate` (name and ownership),
    `GAP-internal-only` (decision recorded), `GAP-copied-fixtures`.
  - Acceptance evidence: all gates green; existing behavioural suites pass;
    `cargo tree -p theoremc-build --edges normal` lists only `camino`,
    `cap-std`, `thiserror` and their transitive dependencies.
  - Conformance check:
    - REQ-reusable-crate satisfied for naming; API reshaping remains.
    - ADR-0008 crate-boundary decision still followed; ADR-0014 accepted.
    - No upstream assumption falsified.
    - No new public interface; the rename is the authorized change.
    - No trust-boundary or persisted-format change.
    - Trace links current.
  - Recovery: revert the milestone commits; the rename is a single coherent
    change.
  - Remaining gaps: `GAP-adapter-in-root`, `GAP-no-error-contract`,
    `GAP-directive-injection`.
  - Compatibility decision: none (unreleased, no external consumer).
- Identifier and outcome: `EP-M2`, the library owns the whole build-script
  boundary and the fallible direct API; root `build.rs` is one call.
  - Requirements and gaps: `REQ-fallible-direct`, `REQ-stable-boundary`
    (library), `GAP-adapter-in-root`, `GAP-no-error-contract`,
    `GAP-directive-injection`.
  - Acceptance evidence: new unit, property, and snapshot tests pass; seeded
    faults recorded; existing behavioural suites pass unchanged.
  - Conformance check:
    - Requirements above satisfied.
    - CLI design §10.1 split (boundary versus fallible service) followed.
    - AXIOM-cargo-directive-parse still consistent with EP-M0.
    - Public surface equals `Interfaces and dependencies`.
    - Persisted contract (file name, line format, cfg) unchanged, checked by
      the unchanged behavioural suites and snapshots.
    - Trace links current.
  - Recovery: revert to the `EP-M1` commit.
  - Remaining gaps: facade path and parity evidence.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M3`, adopters use
  `theoremc::build::emit_suite_from_env_or_exit()`; parity over the shared
  corpus is proven through real Cargo.
  - Requirements and gaps: `REQ-stable-boundary` (adopter),
    `REQ-shared-fixtures`.
  - Acceptance evidence: `build_boundary_bdd` scenarios pass;
    `theorem_file_macro_bdd` passes using the facade; `make kani` passes or is
    shown to fail identically on `main`.
  - Conformance check:
    - Requirements satisfied.
    - Facade re-exports only; no logic in the root package.
    - No new dependency beyond the authorized list.
    - Trace links current.
  - Recovery: revert to the `EP-M2` commit.
  - Remaining gaps: documentation.
  - Compatibility decision: none.
- Identifier and outcome: `EP-M4`, documentation and roadmap reflect the
  shipped boundary.
  - Requirements and gaps: all, documentation only.
  - Acceptance evidence: stale-reference search returns nothing; Markdown
    gates pass; roadmap 4.3.1 checked.
  - Conformance check: upstream design documents reconciled with ADR-0014.
  - Recovery: documentation-only revert.
  - Remaining gaps: CLI commands in 4.3.4 and 4.3.5.
  - Compatibility decision: none.

## Concrete steps

Working directory: the repository root of the worktree. Use `set -o pipefail`
in every shell that pipes a gate through `tee`. Log names follow
`/tmp/$ACTION-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out`.

Baseline before `EP-M1`:

```bash
set -o pipefail
make test 2>&1 | tee /tmp/test-theoremc-4-3-1-build-discovery-and-suite-rendering-api.out
```

Expected: all tests pass (record the count; the local Kani behavioural scenario
may self-skip when Kani is absent).

Focused commands during `EP-M2`:

```plaintext
Red:   cargo nextest run -p theoremc-build -> compile errors naming prepare_suite, BuildErrorCode, PreparedSuite
Green: cargo nextest run -p theoremc-build -> all unit, property, and snapshot tests pass
Fault: remove sort_and_dedup call -> prepare_properties::order_is_independent_of_creation fails; revert
Fault: skip sanitize in failure rendering -> directives_tests property fails; revert
Accept snapshots: cargo insta review (only after reading each snapshot)
```

Focused commands during `EP-M3`:

```plaintext
Red:   cargo nextest run -p theoremc --test build_boundary_bdd -> adopter scenario fails: unresolved module theoremc::build
Green: cargo nextest run -p theoremc --test build_boundary_bdd -> all scenarios pass
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
of the rustup library path even on `main`; CI does not install Kani, so it
self-skips there. A local Kani failure is compared against `main` before being
treated as a regression.

## Validation and acceptance

Behavioural acceptance:

- In a fresh crate with `[build-dependencies] theoremc = { path = ... }`,
  `[dependencies] theoremc = { path = ... }`, a `build.rs` containing only
  `theoremc::build::emit_suite_from_env_or_exit();`, a `src/lib.rs` with the
  documented `include!` pattern, and `theorems/smoke.theorem`, `cargo build`
  succeeds, and the build output file contains
  `cargo::rustc-cfg=theoremc_has_theorems` and
  `cargo::rerun-if-changed=theorems/smoke.theorem`.
- Replacing `theorems/` with a file makes `cargo build` fail with a line
  containing `build.theorem_root_not_directory`.
- `theoremc::build::prepare_suite(crate_root)` on the same tree returns
  contents byte-identical to `OUT_DIR/theorem_suite.rs`.

Quality criteria:

- Tests: `make test` passes, including the new `build_boundary_bdd` scenarios
  and the unchanged `build_discovery_bdd`, `build_suite_bdd`, and
  `theorem_file_macro_bdd` suites.
- Verification: every obligation in `Methods per obligation` passes with its
  non-vacuity evidence recorded; no Verus or Kani obligation exists (D7).
- Lint and formatting: `make check-fmt` and `make lint` pass with no new
  `#[expect]` other than the single environment seam.
- Documentation: Markdown gates pass and the stale-reference search is empty.

## Idempotence and recovery

Every step is repeatable. The rename uses `git mv`; repeating it after success
is a no-op error that can be ignored. `cargo update -w` is idempotent. Snapshot
acceptance is repeatable with `cargo insta test --review`. Each milestone ends
in a commit, so recovery is `git revert` of that milestone's commits or a reset
to the previous milestone commit on this branch. The probe lives only under
`/tmp` and is deleted afterwards. Fixture crates are temporary directories
cleaned automatically; the shared fixture target directory follows ADR-0012 and
needs no manual cleanup.

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

Implementation transcripts, seeded-fault results, fixture timing, and final
size counts are added here as milestones complete.

## Interfaces and dependencies

Crate `theoremc-build` (`crates/theoremc-build`), library `theoremc_build`. It
depends only on `camino`, `cap-std` (`fs_utf8`), and `thiserror`. Layering:
pure rendering (`suite.rs` rendering half, `directives.rs`, `error.rs`) has no
I/O; driven ports are the injected environment lookup closure, the
`cap_std::fs_utf8::Dir` capabilities for the crate root and output directory,
and an `io::Write` sink; driving entry points are `prepare_suite` with
`PreparedSuite::write_to` (direct execution) and `emit_suite_from_env_or_exit`
(Cargo adapter and composition root). Public surface at the end of `EP-M2`:

```rust
/// Crate-relative directory scanned for theorem files.
pub const THEOREM_ROOT: &str = "theorems";
/// File name written under the output directory.
pub const SUITE_FILE_NAME: &str = "theorem_suite.rs";
/// Cfg emitted when at least one theorem file exists.
pub const HAS_THEOREMS_CFG: &str = "theoremc_has_theorems";

/// Discovers and renders the suite for one crate root without writing.
pub fn prepare_suite(crate_root: &Utf8Path) -> Result<PreparedSuite, BuildError>;

pub struct PreparedSuite { /* discovery result and rendered contents */ }
impl PreparedSuite {
    pub fn contents(&self) -> &str;
    pub fn has_theorems(&self) -> bool;
    pub fn theorem_files(&self) -> impl Iterator<Item = &Utf8Path>;
    pub fn rerun_paths(&self) -> impl Iterator<Item = &Utf8Path>;
    /// Writes `SUITE_FILE_NAME` under `out_dir` when its contents differ.
    pub fn write_to(&self, out_dir: &Utf8Path) -> Result<WriteOutcome, BuildError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome { Written, Unchanged }

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    MissingEnvironment { variable: &'static str },
    NonUtf8Environment { variable: &'static str },
    DiscoveryIo { operation: &'static str, path: Utf8PathBuf, source: io::Error },
    TheoremRootNotDirectory { path: Utf8PathBuf },
    UnrepresentablePath { path: Utf8PathBuf },
    OutputDirectory { path: Utf8PathBuf, source: io::Error },
    SuiteWrite { path: Utf8PathBuf, source: io::Error },
}
impl BuildError { pub const fn code(&self) -> BuildErrorCode; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildErrorCode { /* one variant per BuildError variant */ }
impl BuildErrorCode { pub const fn as_str(self) -> &'static str; }
// "build.missing_environment", "build.non_utf8_environment", "build.discovery_io",
// "build.theorem_root_not_directory", "build.unrepresentable_path",
// "build.output_directory", "build.suite_write"

/// Cargo build-script entry point: reads Cargo's environment, prepares and
/// writes the suite, prints directives, and exits with status 1 on failure.
pub fn emit_suite_from_env_or_exit();
```

Crate-private items: `BuildDiscovery`, `CargoBuildEnv`, `run_build_script`,
`BuildScriptStatus`, directive rendering functions. Removed public items:
`prepare_build_script`, `BuildScriptOutput`, `BuildScriptError`,
`BuildDiscovery`, `BuildDiscoveryError`, `discover_theorem_inputs`,
`write_theorem_suite`, `BuildSuiteError`. Their only callers are the root
`build.rs`, the fixtures, and documentation, all updated in the same milestones.

Root facade `theoremc` (`src/lib.rs`): a documented `pub mod build` that
re-exports `emit_suite_from_env_or_exit`, `prepare_suite`, `PreparedSuite`,
`WriteOutcome`, `BuildError`, `BuildErrorCode`, `SUITE_FILE_NAME`,
`THEOREM_ROOT`, and `HAS_THEOREMS_CFG`. It contains no logic.

Reuse policy (recorded in ADR-0014 and `docs/developers-guide.md`): only
`theoremc-build` discovers theorem files, renders the suite, or renders Cargo
directives. `cargo-theorem` and adopters call `prepare_suite`, `write_to`, or
the boundary; they must not reparse directives or re-implement traversal. New
build-integration features (manifest and `build.rs` integration plans for
4.3.4) extend this crate, not the CLI.

Authorized dependency changes are listed below. All versions already resolve in
`Cargo.lock`. The `test-helpers` entry applies only if `EP-M3` step 3 holds.

```toml
# Root Cargo.toml
[dependencies]
theoremc-build = { version = "0.1.0", path = "crates/theoremc-build" }

[build-dependencies]
theoremc-build = { version = "0.1.0", path = "crates/theoremc-build" }

# crates/theoremc-build/Cargo.toml
[dev-dependencies]
filetime = "0.2.29"
googletest = "0.14.3"
insta = "1.47.2"
pretty_assertions = "1.4.1"
proptest = { version = "1.11.0", default-features = false, features = ["std"] }
syn = { version = "3.0.3", default-features = false, features = ["parsing", "full", "printing"] }
test-helpers = { path = "../test-helpers" }
```

No other dependency is added or changed.

## Revision note

- 2026-11-16: initial draft from Wyvern reconnaissance, design documents, and
  external research. Awaiting community-of-experts review and user approval.
