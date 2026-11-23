# Align the workspace lint baseline with `leynos/netsuke` (issue #98)

This ExecPlan is a living document. Keep it current as work proceeds, and
record progress and lessons here rather than relying on working memory.

## Purpose

Issue #98 asks TheoremC to raise its `[workspace.lints.*]` tables to the current
`leynos/netsuke` baseline, treating Netsuke as the minimum and keeping any
stricter TheoremC rule. It also asks for the `unsafe` environment-mutation test
helper to be removed rather than exempted.

Two earlier pull requests landed most of the policy, and a third is open:

- #233 added the zero-finding rules: `missing_assert_message`,
  `unknown_lints`, `renamed_and_removed_lints`, the six rustdoc rules, and
  `unexpected_cfgs` with `check-cfg` for `cfg(kani)` and `cfg(dylint_lib)`.
- #234 added `disallowed_methods` with the environment table in `clippy.toml`,
  `unsafe_code = "forbid"`, and deleted the `EnvGuard`/`ENV_LOCK` helper in
  favour of an injected reader. Every remaining boundary read carries a narrow,
  reasoned `#[expect]`.
- #235 added `missing_docs_in_private_items` after documenting the private
  items it reports.
- PR #261 (open, green) proves the policy is live with
  `tests/lint_policy_execution.rs`.

A byte-level comparison against `leynos/netsuke@main` (retrieved 2026-11-23)
now shows no missing Clippy, Rust, or rustdoc rule. At the time of that
comparison TheoremC additionally denied `cognitive_complexity`; #282 later
moved that gate to Whitaker's macro-aware rule at the same threshold, so the
tables now match the baseline with no Clippy addition at all.

What remains is the work the earlier layers deliberately deferred:

1. The generated Kani gate still emits `#[expect(unexpected_cfgs)]`. With
   `cfg(kani)` now registered in the workspace, an expect that a consumer
   fulfils becomes an *unfulfilled expectation*; under `-D warnings` that is an
   error. Generated code must compile under configurations the macro cannot
   know, so it needs a scoped `allow` with a reason.
2. No test proves an ordinary consumer compiles under `-D warnings` with
   `cfg(kani)` registered and with it unregistered.
3. The three tables carry no divergence comments, which the acceptance
   criteria require ("any deliberate divergence is documented next to the
   relevant setting").
4. No semantic contract test pins the minimum rule set, so a future deletion
   would only be caught by a behavioural test that covers some rules.
5. ADR-0013 still says the alignment "remains separate work", and no record
   states the baseline, its retrieval date, the divergences, or the generated
   Kani gate decision.

## Outcome

- All three root tables equal or exceed the Netsuke baseline, with each
  divergence explained by a comment beside the setting it affects.
- `unsafe_code = "forbid"` holds with no crate-wide exception, and the deleted
  guard helper is not mentioned anywhere as if it still existed.
- The generated Kani module compiles under `-D warnings` whether or not the
  invoking crate registers `cfg(kani)`, proved by two independent consumer
  builds in the BDD suite.
- `tests/lint_policy_contract.rs` parses the manifest and `clippy.toml`
  semantically and fails if a baseline rule is removed or downgraded.
- A new ADR records the baseline, its retrieval date, adopted rules,
  divergences, boundary expectations, and the generated-gate decision;
  ADR-0013, the developers' guide, and the design document are current.

## Scope

In scope: the root `Cargo.toml` lint tables, `clippy.toml` comments, the proc
macro's generated Kani gate and its golden tokens, the macro BDD fixtures and
steps, a new manifest contract test, and documentation.

Out of scope: adding `cfg(verus)` (no Verus cfg exists; ADR-0007 keeps the
backend planned), adding a `shell_quote` disallowed method (no such
dependency), and any change to the lint *levels* already adopted.

## Constraints

- Do not weaken a lint entry to make a gate pass. A finding is fixed at source
  or resolved with a narrow, reasoned `#[expect]`.
- No blanket `allow` attributes. The one `allow` this plan introduces is the
  generated Kani gate, which is emitted into consumer crates where no
  expectation can be fulfilled reliably.
- Every `assert!`/`assert_eq!` carries a diagnostic message with a reason.
- Files stay under 400 lines; Markdown prose wraps at 80 columns and code
  blocks at 120.
- The canonical gates are `make check-fmt`, `make lint`, `make test`,
  `make msrv`, `make markdownlint`, `make nixie`, `make spelling`, and
  `make typecheck`.

## Relevant documents

- `docs/adrs/adr-0013-workspace-complete-lint-gate.md`
- `docs/adrs/adr-0007-verus-backend-preservation-invariants.md`
- `docs/developers-guide.md` (lint policy and environment-injection passages)
- `docs/theoremc-design.md` (§7.2, generated cfg boundary)
- `docs/documentation-style-guide.md`

## Milestones

### Milestone 1: make the generated Kani gate configuration-independent

Acceptance:

- `crates/theoremc-macros/src/render.rs` emits a reasoned, module-scoped
  `allow(unexpected_cfgs)` in place of the `expect`.
- `crates/theoremc-macros/src/tests_support.rs` and the golden snapshot match.
- `cargo test -p theoremc-macros` passes, including trybuild.
- Clippy's `allow_attributes` does not fire on the generated output under
  `make lint`.

Acceptance commands: `cargo nextest run -p theoremc-macros`, `make lint`.

### Milestone 2: prove strict-warning consumers on both cfg configurations

Acceptance:

- The fixture manifest builder can emit `[lints.rust] unexpected_cfgs` with
  `check-cfg = ["cfg(kani)"]`.
- The Cargo runner can build a fixture with `RUSTFLAGS=-D warnings`, preserving
  status and diagnostics on failure.
- Two new BDD scenarios build independent consumers, one registering `cfg(kani)`
  and one not. Both pass. The existing Kani scenarios are unchanged.

Acceptance commands: `cargo nextest run --test theorem_file_macro_bdd`.

### Milestone 3: document the policy and lock it with a contract test

Acceptance:

- Each divergence carries a comment beside the setting in `Cargo.toml` and
  `clippy.toml`.
- `tests/lint_policy_contract.rs` parses the manifest and `clippy.toml` and
  asserts the baseline rule set, `unsafe_code = "forbid"`, parsed `check-cfg`
  elements, workspace inheritance for every package, and the seven `std::env`
  disallowed methods with reasons.
- `docs/adrs/adr-0014-netsuke-lint-baseline-alignment.md` records the
  decision; ADR-0013's "remains separate work" sentence now points at it.
- The developers' guide and design document describe the current policy.

Acceptance commands: the full gate list above.

## Progress log

- 2026-11-23: Oriented. Read the issue, the CodeRabbit plan, and the current
  tree. Confirmed by direct comparison with `leynos/netsuke@main` that no
  baseline rule is missing; `cognitive_complexity` was then the only extra.
  Confirmed
  `unsafe` code is gone and the guard helper deleted by #234. Remaining work is
  the generated gate, the consumer proof, divergence comments, the contract
  test, and documentation.
- 2026-11-23: Milestone 1 done in source. `render.rs` emits a reasoned,
  module-scoped `allow(unexpected_cfgs)`; the macro doc example, the expected
  tokens in `tests_support.rs`, and the golden snapshot agree. Added a fast
  unit test in `render_tests.rs` that fails if the gate reverts to `expect`, so
  the regression does not depend solely on the slow consumer builds.
- 2026-11-23: Milestone 2 done in source. `ConsumerCfgRegistration` selects
  whether a fixture consumer declares `check-cfg = ["cfg(kani)"]`;
  `cargo_run_denying_warnings` sets `RUSTFLAGS=-D warnings` on the child and
  clears `CARGO_ENCODED_RUSTFLAGS`, which Cargo would otherwise prefer. Two
  feature scenarios and their step bindings build independent consumers in both
  configurations. Unit tests cover the manifest builder in each mode.
- 2026-11-23: Milestone 3 partly done. Divergence comments added to
  `Cargo.toml` and `clippy.toml`; `toml = "1.1"` dev-dependency added;
  `tests/lint_policy_contract.rs` and its baseline fixture module written;
  ADR-0014 written; ADR-0013's superseded sentence and `docs/contents.md`
  updated; developers' guide and design document corrected.
- Lesson: `make check-fmt` runs `mdtablefix` over tracked Markdown, and
  `make fmt` regenerates `typos.toml`. Run both before committing documentation.

## Findings recorded during implementation

- The design document already described the `allow`-based gate while the
  generator still emitted `expect`, so the code was the drift, not the prose.
- The defect was reproduced outside the suite with `rustc` directly, using
  `--check-cfg=cfg(docsrs,test)` to match what Cargo always passes (without any
  `--check-cfg`, `unexpected_cfgs` stays dormant and `expect` looks fine):

  | generator | `cfg(kani)` registered              | unregistered |
  | --------- | ----------------------------------- | ------------ |
  | `expect`  | fails: unfulfilled lint expectation | passes       |
  | `allow`   | passes                              | passes       |

  This is why the paired BDD builds matter: a single build in one configuration
  would have missed the failure entirely.
- `panic_in_result_fn` is denied, and no existing `Result`-returning function in
  the workspace contains an assertion. The contract test therefore keeps its
  helpers assertion-free and puts every assertion in a test body, using
  `.expect` there, which `allow-expect-in-tests` permits.
- `tests/lint_policy_contract.rs` starts `cargo metadata`, so it joins the
  `nested-cargo-builds` group in `.config/nextest.toml` alongside the other
  compiler-spawning tests.
- Clippy's `allow_attributes` check skips attributes with
  `span.from_expansion()` or `is_from_proc_macro`, so the generated `allow` is
  exempt from the restriction that bans handwritten `allow` attributes. The
  fallback that would have required every consumer to register `cfg(kani)` is
  therefore unnecessary.
- Residual risk: no theorem files exist in this workspace, so the generated
  module is never compiled under the workspace lint tables and
  `missing_docs_in_private_items` cannot see it. Should a future `theorems/`
  directory expose a finding there, the fix belongs in `render.rs`, as the plan
  states, rather than at each call site.
- `tests/theorem_file_macro_bdd.rs` was already 482 lines on `main`, above the
  400-line ceiling. The two new scenario bindings add 39 lines. A split would
  move existing step wiring in the same change that alters behaviour, so it is
  recorded here as pre-existing drift rather than bundled into this fix.
- `toml = "1.1"` resolved to 1.1.8, whose tree (`toml_parser`, `toml_writer`,
  `serde_spanned`, `toml_datetime`, `indexmap`) declares Rust 1.85, inside the
  repository's 1.89 floor.

## Gate results

- `cargo fmt --all -- --check`: pass.
- `make markdownlint`: pass (after `markdownlint-cli2 --fix` corrected a table
  style in this plan).
- `mdtablefix --check --git --include-untracked ...`: pass.
- `make spelling`: pass (after "handwritten" replaced the hyphenated form).
- `make check-adr-rfc-names`: pass; 15 records and 101 local links checked.
- `cargo nextest run -p theoremc-macros`: 51 of 52 passed on the first run; the
  trybuild case hit nextest's 600 s allowance under host compile-queue
  contention and was re-run directly. No assertion failure occurred.
- Full `make lint`, `make test`, and `make msrv`: delegated to `scrutineer`
  once the targeted checks finished, because the shared compiler pool was
  saturated by other agents (hundreds of queued crates, waits over 150 s).
