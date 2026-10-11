# Include provenance in macro action-signature conflict diagnostics

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`,
`Outcomes & Retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: IN PROGRESS (EP-M1 to EP-M3 complete; EP-M4 review and PR close-out
remaining)

## Purpose / big picture

A `.theorem` file may hold several theorem documents separated by `---`, and
each document declares the Rust signatures it expects for the actions it calls.
Two documents may legitimately repeat the same action; when their declarations
disagree, `theorem_file!` refuses to expand and the crate fails to compile.

Today that failure says only which action disagreed:

```plaintext
error: referenced action `account.deposit` has conflicting Actions signatures
```

A theorem author working in a multi-document file cannot tell which two
documents disagree, nor how their signatures differ, without reading every
`Actions` block in the file by hand.

After this change the same failure names both disagreeing documents and renders
both declared signatures:

```plaintext
error: referenced action `account.deposit` has conflicting Actions signatures:
theorem `FirstConflict` declares `fn(account: u64) -> bool`, but theorem
`SecondConflict` declares `fn(account: u32) -> bool`
```

Observable success:

1. Expanding a `.theorem` file whose documents declare incompatible signatures
   for one action fails with a diagnostic naming both theorem names and both
   signature summaries.
2. The diagnostic also covers conflicts between declarations that are not
   referenced by any `Let` or `Do` step, because the index still reports them.
3. Files whose repeated declarations are semantically equivalent (differing
   only in insignificant whitespace, such as `Vec<u8>` versus `Vec <u8>`)
   continue to expand without error.
4. Every existing deterministic gate (`make check-fmt`, `make lint`,
   `make test`) passes.

## Constraints

- `crates/theoremc-macros/src/lib.rs` must not exceed 400 lines; the repository
  caps code files at that length (`AGENTS.md`, "Keep file size manageable").
- `MacroExpansionError::ConflictingActionSignature` is private to the
  `theoremc-macros` crate, so this is an internal change with no public API
  compatibility obligation. No compatibility shim is introduced.
- The diagnostic span must stay at the macro call-site string literal.
  `MacroExpansionError::to_compile_error` receives a `Span`, and `theorem_file`
  already passes `path_literal.span()`; no source span survives from the
  `.theorem` load path, so nothing else is available.
- The rendered message must be deterministic: fixed clause order, no
  iteration over unordered collections, so the trybuild golden file is stable.
- Comments and documentation use en-GB-oxendict spelling.
- Markdown paragraphs are wrapped at 80 columns; code blocks at 120.
- Do not change `equivalent_action_signatures` or `signature_drift` snapshot
  fixtures: the equivalence path and the rustc `E0308` backstop are out of
  scope.
- Do not silence lints. `clippy.toml` caps function arguments at four and sets
  a cognitive-complexity ceiling of nine.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than eight files or 250
  net lines of code, stop and escalate.
- Interface: if a public API signature of `theoremc-core` must change, stop and
  escalate. (The planned change touches only private items in
  `theoremc-macros`.)
- Dependencies: if a new external dependency is required, stop and escalate.
- Iterations: if the trybuild golden file still mismatches after three
  regeneration attempts, stop and escalate.
- Rustfmt: if the rendered signature summary cannot be written without
  rustfmt splitting `#[error(...)]` into a form that fails `make check-fmt`,
  shorten the summary (drop parameter names) before escalating; if a second
  rewrite is still needed, stop and escalate.
- Ambiguity: if the conflict message format materially changes the evidence
  required by issue #51, stop and present options.
- Time: if a milestone takes more than four hours of wall-clock work, stop and
  escalate.

## Risks

- Risk: `#[error(...)]` line-breaking fights rustfmt, making `make check-fmt`
  fail while the code still compiles. Severity: medium. Likelihood: medium.
  Mitigation: the long literal shares one source line with `#[error(` and ends
  at a `"` before the closing `)]`, so rustfmt leaves the attribute alone;
  verified by running `make check-fmt` before the trybuild regeneration. If it
  fights, drop parameter names from the summary to shorten it.
- Risk: the trybuild golden `.stderr` is brittle and the message length may
  produce an unstable render. Severity: low. Likelihood: low. Mitigation: keep
  the message a single deterministic clause, and regenerate with
  `TRYBUILD=overwrite` then review the diff by eye.
- Risk: carrying the declaring `TheoremName` changes the `ActionSignatureIndex`
  map value type and could disturb probe generation ordering. Severity: medium.
  Likelihood: low. Mitigation: the index remains a `BTreeMap<&str, _>` filtered
  by the same `referenced_actions` order; the
  `expansion_emits_typed_action_probe_for_referenced_action` assertion pins the
  exact generated probe tokens.
- Risk: a build queued behind the shared compile-slot pool makes gates look
  slow. Severity: low. Likelihood: high. Mitigation: expect the
  `[build-limits] CRATE: waiting for a compile slot` line and allow for it; do
  not start parallel builds.

## Progress

- [x] (2026-10-11T02:07Z) Reconnaissance: read the macro expansion pipeline,
      the action-signature index, the trybuild fixtures, the snapshot pattern,
      `clippy.toml`, the `Makefile` gates, and ADR-004.
- [x] (2026-10-11T02:07Z) Probed `thiserror` attribute forms and rustc
      `compile_error!` rendering of long single-line messages.
- [x] (2026-10-11T02:12Z) Wrote this ExecPlan; branch renamed and Lody session
      title set.
- [x] (2026-10-11T02:14Z) EP-M1 red: the tests for the four provenance fields
      failed with `variant MacroExpansionError::ConflictingActionSignature does
      not have fields named first_theorem, conflicting_theorem,
      first_signature, conflicting_signature` (`E0026`), which is the expected
      red failure.
- [x] (2026-10-11T02:20Z) EP-M1 green: implementation committed as `2f67bef`.
      It landed in `crates/theoremc-macros/src/lib.rs` and now lives, after the
      split below, in `crates/theoremc-macros/src/action_signature_index.rs`
      (`DeclaredSignature`, `render_signature_summary`,
      `conflicting_signature_error`, extended error variant).
- [x] (2026-10-11T02:20Z) EP-M2: `rstest` provenance case
      (`expansion_names_both_declaring_theorems`) with parameter-type and
      return-type drift cases.
- [x] (2026-10-11T02:20Z) EP-M2: strengthened index test
      (`action_signature_index_reports_conflicting_signatures_with_provenance`)
      with `googletest` field matchers over both conflict kinds, plus a
      message-substring assertion.
- [x] (2026-10-11T02:20Z) EP-M2: regenerated trybuild golden `.stderr`,
      reviewed by eye; the only change is the extended message, and the span
      `tests/expand/conflicting_action_signatures.rs:5:15` is unchanged.
- [x] (2026-10-11T02:27Z) Refactor: extracted
      `crates/theoremc-macros/src/action_signature_index.rs` and
      `crates/theoremc-macros/src/action_probe_tests/conflicting_signatures.rs`
      to restore the 400-line file cap. Commit `bcaf572`.
- [x] (2026-10-11T02:28Z) EP-M3: macro rustdoc table row, users' guide
      paragraph, and ADR-004 rule update.
- [x] (2026-10-11T02:34Z) EP-M3: `make fmt`, then `make check-fmt`,
      `make markdownlint`, `make nixie` all passed on the documentation commit
      `2fd8213`.
- [x] (2026-10-11T02:41Z) EP-M3: `scrutineer` commit-gate run. Four of five
      gates passed (`check-fmt`, `lint` 281s, `markdownlint`, `nixie`).
      `make test` reported 781/782 with
      `a_valid_theorem_file_exposes_a_kani_proof_harness` failing, which is the
      documented `RUSTC_WRAPPER`/Kani environment interaction, not a product
      defect; a focused re-run reproduced the identical `rustversion` error.
      The doctest half was re-run on its own and passed (5 binaries, 0 failed),
      since `make` had aborted before reaching it.
- [ ] EP-M4: CodeRabbit `--agent` review passes; findings addressed.
- [ ] EP-M4: push and open the draft PR.

## Surprises & Discoveries

- Observation: `thiserror` rejects `#[error(concat!(...))]`; it requires a
  literal, `transparent`, or `fmt`. Evidence:
  `error: expected one of: string literal, transparent, fmt` from a throwaway
  crate under `/tmp/terrprobe`. Impact: the long message needs a backslash
  continuation inside the literal, not a `concat!` expression. Rustfmt leaves
  long single-string attributes alone; this was confirmed at Milestone 3.
- Observation: repo-level `clippy::cognitive_complexity` is denied at a
  threshold of nine, and source positions in the generated `.stderr` change
  whenever lines are added above the error construction in `lib.rs`. Evidence:
  `clippy.toml`; the golden file records
  `--> tests/expand/conflicting_action_signatures.rs:5:15` and the trybuild
  fixture's own line numbers. Impact: `lib.rs` edits shift the
  `Span::call_site()` position, so the golden `.stderr` must be regenerated and
  reviewed after implementation, not crafted by hand.
- Observation: at the time of implementation, GitHub REST (`gh api`) was
  returning `API rate limit exceeded` for the token identity, while SSH
  authenticated as `leynos` and could fetch and push. Evidence:
  `/tmp/ghclean.sh /usr/bin/gh api user` returned HTTP 403; the same script's
  `ssh -T git@github.com` returned `Hi leynos!`. Impact: PR creation may need
  `gh` to be retried after the rate limit window; SSH push is unaffected.
- Observation: `make test` reports one failure on this machine that is not a
  product defect.
  `theoremc::theorem_file_macro_bdd::a_valid_theorem_file_exposes_a_kani_proof_harness`
  fails because Kani's nested toolchain build invokes `rustc --version` through
  `RUSTC_WRAPPER=build-limits-rustc`, receives an empty string, and aborts in
  `rustversion`'s build script with `Error: unexpected output from`rustc
  --version`: ""`. The scenario's own skip guard,
  `is_unusable_kani_environment` in `tests/theorem_file_macro_bdd.rs`, matches
  only `error while loading shared libraries`,
  `cannot open shared object file`, and `Broken pipe`, so it does not cover
  this case. Evidence: `/tmp/test-...-issue-51-...out` and the focused re-run
  `/tmp/kani-focused-issue-51.out`, which reproduced the identical error in
  8.8s. Impact: `make test` cannot be made green locally without unsetting
  `RUSTC_WRAPPER`, which the compile-admission policy forbids; CI does not set
  this wrapper, and every other test passes. Recorded so the next reader does
  not misdiagnose it as a regression from this change. The Kani-related
  scenario is untouched by this work: the change affects the conflict
  diagnostic only, and the failing fixture is `theorems/single.theorem` with no
  conflicting actions.

## Decision Log

- Decision: carry the declaring `TheoremName` through the index as a private
  `DeclaredSignature { theorem, signature }` value type rather than a tuple.
  Rationale: the named struct keeps `insert_signature` and
  `conflicting_signature_error` at three parameters each, inside the
  `too-many-arguments-threshold = 4` ceiling, and names the two fields at use
  sites. The signature index is built once per expansion inside
  `ActionSignatureIndex::for_actions`, so the extra struct costs nothing at
  runtime. Date/Author: 2026-10-11, implementation agent.
- Decision: render a signature summary as
  `fn(<name>: <type>, ...) -> <return>`, reusing the declared parameter names.
  Rationale: those names are exactly the keys the author wrote under
  `Actions.params`, so the summary maps onto the YAML the user edits, and
  ADR-004's documented probe shape shows parameter names. Parameter order is
  the YAML order, which is already significant for the generated probe.
  Date/Author: 2026-10-11, implementation agent.
- Decision: keep `ConflictingActionSignature` as a struct variant with five
  `String` fields and put the rendered message in `#[error(...)]`, adding
  `#[error("{}", .message)]`-style indirection only if the attribute fails to
  format. Rationale: the message is short enough to read from the attribute,
  and the verified probe shows the attribute accepts a long single-line
  literal. The structured fields carry all typed data the tests assert on.
  Date/Author: 2026-10-11, implementation agent.
- Decision: keep the diagnostic span at the macro call site.
  Rationale: no `Span` for a `.theorem` document survives loading;
  `SchemaDiagnostic` renders locations into message text instead, and only
  schema failures use that path. Issue #51 asks for provenance in the message,
  not a new span source. Date/Author: 2026-10-11, implementation agent.
- Decision: `insert_signature` reports the fully populated error variant, so
  `to_compile_error` and its `path_literal.span()` call site stay unchanged.
  Rationale: minimal blast radius, and the type stays the single source of
  truth for the conflict message. Date/Author: 2026-10-11, implementation agent.
- Decision: no `insta` snapshot test for the message.
  Rationale: the message is two clauses and one rendered signature pair, well
  within direct substring assertions, so a snapshot would duplicate the
  trybuild golden file without adding review value. Recorded here per the
  issue's "use snapshot tests if the format becomes substantial" guidance.
  Date/Author: 2026-10-11, implementation agent.
- Decision: extract the index and its provenance into
  `crates/theoremc-macros/src/action_signature_index.rs`, and the conflict
  tests into
  `crates/theoremc-macros/src/action_probe_tests/conflicting_signatures.rs`,
  rather than growing `lib.rs` and `action_probe_tests.rs` past the 400-line
  cap. Rationale: the cap is a hard repository rule in `AGENTS.md`, and the
  extracted index is a coherent unit ("first-seen signatures with provenance")
  that reads better on its own than as a section of the macro entry point.
  `lib.rs` is 379 lines and `action_probe_tests.rs` 135 after the split.
  Date/Author: 2026-10-11, implementation agent.

## Outcomes & Retrospective

The purpose above is met. A theorem author who declares one action differently
in two documents of the same `.theorem` file now reads, at the macro call site:

```plaintext
error: referenced action `account.deposit` has conflicting Actions signatures:
theorem `FirstConflict` declares `fn(account: u64) -> bool`, but theorem
`SecondConflict` declares `fn(account: u32) -> bool`
```

That message names the action, both declaring theorems, and both signatures, so
the author can go straight to the disagreement. Before this change the message
stopped after "conflicting Actions signatures" and named only the action; in a
file with several documents and several actions, locating the two declarations
meant searching the file by hand.

The typed data behind the message is asserted directly. Unit tests in
`crates/theoremc-macros/src/action_probe_tests/action_signature_index.rs` and
`conflicting_signatures.rs` match all five fields for both conflict kinds
(parameter-type drift and return-type drift) and assert the same five values
appear in the rendered message, so the typed error and what the user sees
cannot drift apart silently. The public compile error is pinned by the trybuild
golden file
`crates/theoremc-macros/tests/expand/conflicting_action_signatures.stderr`,
whose only change is the extended message; the recorded span
`tests/expand/conflicting_action_signatures.rs:5:15` did not move.

Deliberately out of scope, and recorded rather than silently dropped:

- No source span inside the `.theorem` file. No `Span` survives the load path,
  so every failure in `theorem_file!` still points at the macro call site; this
  change carries provenance in the message text instead. A schema-level
  location facility would be a separate piece of work.
- No shared cross-file signature manifest or import mechanism. A conflict is
  defined within one `theorem_file!` expansion; two different `.theorem` files
  may still disagree, and ordinary Rust type checking rejects whichever file no
  longer matches the export. ADR-004 records this.
- No `insta` snapshot, per the `Decision Log` entry above.

Lessons for similar work:

- The 400-line cap is easy to breach when a diagnostic grows typed fields,
  because the fields, the renderer, and the constructor all land near the
  existing error type. Check `wc -l` on the touched code files before running
  the gates, not after they fail. The split here also improved the design: the
  index is a coherent unit that reads better on its own.
- `thiserror` refuses `concat!` inside `#[error(...)]`, so a long message must
  be a single literal. Rustfmt leaves a long single-string attribute alone, so
  no formatting conflict arises — but this could only be established by
  probing, and the discrepancy between the plan's assumption and the
  attribute's actual behaviour is worth remembering.
- A trybuild golden file couples the diagnostic text to the source line of the
  macro call site. Regenerating it is safe, but the diff must be read by eye to
  confirm the span did not move while the message was extended.
- On this machine, `make test` is red for reasons unrelated to any change: the
  Kani BDD scenario cannot survive `RUSTC_WRAPPER`. Confirm that failure
  reproduces identically in isolation before attributing it to the diff.

## Context and orientation

`theorem_file!` is a procedural macro crate:
`crates/theoremc-macros/src/lib.rs`. Given a crate-relative path such as
`"theorems/account.theorem"`, it loads the file through `theoremc_core`, then
emits a private module containing `include_str!` anchoring, compile-time action
probes, referenced-type probes, and `#[cfg(kani)]` harness stubs. Any failure
becomes a `compile_error!` invocation at the macro call site.

The relevant pipeline inside `lib.rs`:

- `theorem_file` (proc-macro entry) → `expand_theorem_file` →
  `expand_theorem_file_at` (loads documents) → `render_expansion` →
  `generated_action_probes` → `ActionSignatureIndex::for_actions`.
- `ActionSignatureIndex` is a private `BTreeMap` wrapper built once per
  expansion. `for_actions` takes the loaded `TheoremDoc` slice and the ordered
  list of referenced canonical action names from `theoremc_core::collision`,
  walks every document and every declared action in document order, and records
  the first declaration of each action. `insert_signature` compares each later
  declaration with the recorded one using
  `ActionSignature::is_semantically_equivalent` (which canonicalizes Rust type
  strings through `syn::Type`, so `Vec<u8>` and `Vec <u8>` compare equal) and
  raises `MacroExpansionError::ConflictingActionSignature` when they differ.
  `signature_for` then serves the recorded signature for probe generation and
  raises `MissingActionSignature` for unreferenced actions that were never
  declared.
- `MacroExpansionError` is a private `thiserror` enum. `to_compile_error`
  converts any variant to a `compile_error!` at a caller-supplied span.

Tests live in three places: focused unit tests under
`crates/theoremc-macros/src/`, trybuild compile-fail fixtures under
`crates/theoremc-macros/tests/expand/` driven by
`crates/theoremc-macros/tests/expand.rs`, and behavioural tests at the
repository root under `tests/` driven by `rstest-bdd`. The behavioural suite
covers fixture-crate builds and schema diagnostics; it does not currently
exercise action-signature conflicts, and this plan does not add such a scenario
because the conflict is pure in-process data with no observable build behaviour
beyond the compile error, which trybuild already pins.

Key terms:

- *Canonical action name*: the dot-separated key written under `Actions`, for
  example `account.deposit`.
- *Signature summary*: the rendered, function-signature-like string this plan
  introduces, for example `fn(account: u64) -> bool`.
- *Trybuild golden file*: the committed `.stderr` file that records expected
  compiler output for a compile-fail fixture.
- *Span*: the source region rustc highlights for a diagnostic.
- *Red-Green-Refactor*: write the failing test first, make it pass, then tidy.

## Conformance basis

Upstream artefacts:

- Issue #51, "Include provenance in macro action-signature conflict
  diagnostics" (leynos/theoremc). Its "Proposed resolution" is "carry the first
  and conflicting declaration provenance through the macro query layer and
  render both theorem names and relevant signature summaries in the compile
  error"; its "Validation expectations" are unit tests for the typed conflict
  data, trybuild coverage for the public compile error, and snapshot tests only
  if the format becomes substantial.
- `docs/adr-004-action-signature-specification.md` (ADR-004), which rules that
  a signature declared more than once within one `theorem_file!` expansion must
  be identical, and that probes are emitted as anonymous `const _` items.
- `docs/reviews/rfc-0001-selective-adoption-and-vertical-slice-preservation.md`
  and `docs/rfcs/` for repository governance style.
- `AGENTS.md` (code style, doc maintenance, committing, Rust guidance) and
  `docs/documentation-style-guide.md`.

There is no Terms of Reference revision or technical-design revision that names
this diagnostic; the repository's plan set under `docs/execplans/` tracks
roadmap steps, and issue #51 is a post-roadmap quality item. That is stated
here rather than inventing an identifier.

Trace links:

```plaintext
ISSUE-51-PROPOSED-RESOLUTION -> EP-M1 (typed provenance in the index and error)
ISSUE-51-VALIDATION-UNIT    -> EP-M2 (rstest cases and index field assertions)
ISSUE-51-VALIDATION-TRYBUILD-> EP-M2 (regenerated golden .stderr)
ADR-004-RULE-IDENTICAL      -> EP-M1 (equivalence path unchanged)
AGENTS-DOC-MAINTENANCE      -> EP-M3 (macro rustdoc table row, users' guide)
```

## Verification plan

This change introduces one non-trivial cross-cutting invariant and no lemmas
requiring a formal prover: the data flow is a straight-line walk over an
ordered slice, and every obligation is decidable by example with a finite,
enumerable case set. Bounded model checking and formal proof are therefore not
proportionate; parameterized tests plus a compile-fail golden file discharge
every obligation. Each confirmation names the fixtures that *do* reach the
conflict path. The equivalent-signature case acts as the negative control: it
must be accepted, with no conflict reported, while still generating the action
probe for the agreed signature. Together the two directions show the check is
not passing everything or rejecting everything.

Claims under test:

- C1 (data propagation): the index stores the declaring `TheoremName` with the
  first-seen signature, so a conflict can name both documents.
- C2 (case identity): both named documents are the two that declared the
  action.
- C3 (summary fidelity): the rendered summaries describe the two disagreeing
  declarations, parameter names in declaration order.
- C4 (bare insertion): a conflict between two declarations with no `Let` or
  `Do` reference is still reported.
- C5 (position independence): moving the conflict to deeper references in the
  later document leaves both the error and the success path unchanged.
- C6 (non-equivalence fidelity): the reported message matches the state of the
  underlying signature data.

Obligation V1 — conflict provenance names both declarations.

- Method: `rstest` parameterized unit test over the two conflict kinds
  (parameter-type drift and return-type drift), each with distinct theorem
  names on either side.
- Rationale: the case set is finite and semantic, and the parameterization is
  what proves the provenance is not accidentally position-specific.
- Domain: two `TheoremDoc`s per expansion, sharing `account.deposit`; cases
  `(u64, bool) vs (u32, bool)` and `(u64, bool) vs (u64, u32)`, first theorem
  `FirstProvenanceThesis`, second `SecondProvenanceThesis`.
- Artefact: `crates/theoremc-macros/src/action_probe_tests.rs`.
- Evidence: `cargo nextest run -p theoremc-macros expansion_names_both`.
  Initially fails: `ConflictingActionSignature` is a unit-shaped variant with
  no provenance field, so the pattern cannot compile. Discharge condition:
  `first_theorem`, `conflicting_theorem`, `first_signature`, and
  `conflicting_signature` hold the expected values for both cases.
- Non-vacuity: each case constructs a real conflict, evidenced by the
  pre-change `expansion_rejects_conflicting_signatures_for_shared_action`
  reaching the same error variant; the case input must match the reported
  summaries, or the test fails.

Obligation V2 — the index reports both conflict kinds with provenance.

- Method: parameterized `#[rstest]` cases with `googletest` field matchers.
- Rationale: this is the layer below the macro entry point, so failures here
  separate index logic from expansion plumbing.
- Domain: `ActionSignatureIndex::for_actions` over the parameter-drift case
  (`account.deposit`, `u64` vs `u32`) and a return-drift case on a different
  action (`payload.write`, `u64` vs `String`) so the file does not encode one
  fixture twice.
- Artefact:
  `crates/theoremc-macros/src/action_probe_tests/action_signature_index.rs`.
- Evidence: `cargo nextest run -p theoremc-macros action_signature_index`.
  Discharge condition: the pattern matches and each field equals the expected
  string, and `error.to_string()` contains both theorem names and both
  summaries.
- Non-vacuity: the pre-existing test at the same location already reaches this
  variant; the added assertions are what the pre-change code cannot satisfy.

Obligation V3 — the public compile error renders both declarations.

- Method: trybuild compile-fail fixture with a regenerated golden `.stderr`.
- Rationale: only a real rustc run proves that the rendered message reaches the
  user unchanged through `syn::Error::to_compile_error` and `compile_error!`.
- Domain: `crates/theoremc-macros/tests/expand/conflicting_action_signatures.rs`
  plus its `.theorem` fixture (`FirstConflict` with `account: u64`,
  `SecondConflict` with `account: u32`).
- Artefact:
  `crates/theoremc-macros/tests/expand/conflicting_action_signatures.stderr`.
- Evidence: `cargo nextest run -p theoremc-macros --test expand`. Discharge
  condition: the golden file records one `error:` line naming both
  `FirstConflict` and `SecondConflict` with `fn(account: u64) -> bool` and
  `fn(account: u32) -> bool`, and the run passes.
- Non-vacuity: the golden file is regenerated from the new output and reviewed
  by eye against the compiled message; a stale golden file fails the run.

Obligation V4 — typed fields and rendered message agree.

- Method: direct assertion on `error.to_string()` alongside the field
  assertions in V1 and V2.
- Rationale: the message is derived from the fields; a disagreement between
  them would be a defect a field-only test would miss.
- Domain: the V1 and V2 fixtures.
- Artefact: the two test modules named above.
- Evidence: substring assertions for both theorem names and both summaries.
- Non-vacuity: parameter names are removed from the expected values, so a
  mismatch in type strings still fails.

Non-trivial axioms:

- `ActionSignature::is_semantically_equivalent` is the repository's canonical
  equivalence test; this plan does not re-verify `syn`-based type comparison.
- `CompileError::to_compile_error` renders the message via
  `Display` on the error, which `thiserror` derives; formatting fidelity
  therefore reduces to the `#[error(...)]` literal, which the trybuild golden
  file pins.
- rustc renders a long `compile_error!` message on one line regardless of
  length; verified once during reconnaissance with a standalone `rustc` run.

Negative control: the unchanged
`whitespace_only_signature_drift_does_not_conflict` test asserts that `Vec<u8>`
versus `Vec <u8>` does **not** produce "conflicting Actions signatures". It
must keep passing, proving the new provenance path did not turn equivalence
into conflict.

Residual gaps: neither the unit tests nor the golden file can prove that the
provenance survives into every editor's diagnostic rendering; rustc's own
message rendering is axiomatic here.

## Plan of work

Stage A (no code changes): reconnaissance, reported in `Progress`.

Stage B (red): extend the two test modules so they fail for the expected
reason, and add the `rstest` provenance case in
`crates/theoremc-macros/src/action_probe_tests.rs` next to the existing
`expansion_rejects_conflicting_signatures_for_shared_action`, then run
`cargo nextest run -p theoremc-macros` and record the failure. The failure must
be a compile error on the missing struct fields, not an assertion failure, and
the red stage is therefore enforced by the type system rather than an
expected-failure marker.

Stage C (green): change `crates/theoremc-macros/src/lib.rs`, moving the index
into `crates/theoremc-macros/src/action_signature_index.rs` as steps 1 to 8 are
completed, because `lib.rs` was already near the 400-line cap before this work.

1. Add `TheoremName` to the `use theoremc_core::{...}` block, following the
   existing `ActionSignature` entry. `TheoremName` is re-exported from
   `theoremc_core::schema`.
2. Add a private `DeclaredSignature<'a>` struct holding
   `theorem: &'a TheoremName` and `signature: &'a ActionSignature`, deriving
   `Debug, Clone, Copy`.
3. Change `ActionSignatureIndex::signatures` to
   `BTreeMap<&'a str, DeclaredSignature<'a>>`.
4. Change `for_actions` to build each `DeclaredSignature` from `doc.theorem`
   and the current pair, preserving document and declaration order.
5. Change `insert_signature` to take a `DeclaredSignature` and compare
   `incoming.signature` against `first.signature`; on a non-equivalent
   signature, build the error from both declarations.
6. Change `signature_for` to return `declared.signature`, leaving
   `MissingActionSignature` unchanged.
7. Add `fn render_signature_summary(signature: &ActionSignature) -> String`
   rendering `fn(<name>: <type>, ...) -> <return>` in declared parameter order.
8. Add `fn conflicting_signature_error(...)` taking the action name and both
   `DeclaredSignature`s, returning the populated variant (see
   `Interfaces and dependencies` for the exact signature).
9. Extend the `ConflictingActionSignature` variant with `action`,
   `first_theorem`, `conflicting_theorem`, `first_signature`, and
   `conflicting_signature`, and write the message in `#[error(...)]` with a
   backslash continuation so the literal stays on one source line.
10. Leave `to_compile_error` and its `path_literal.span()` call site unchanged.

Stage D (refactor, documentation, wider validation): add the conflicting-action
row to the `theorem_file` rustdoc diagnostic table in `lib.rs`, and document
the diagnostic in `docs/users-guide.md`. Regenerate the trybuild golden file
with `TRYBUILD=overwrite cargo nextest run -p theoremc-macros --test expand`,
review the diff by eye, then run the full deterministic gates.

## Milestones and plateaus

Milestone EP-M1: the index and error type carry provenance.

- Identifier and outcome: EP-M1 leaves the workspace compiling with the typed
  provenance in place and the red tests turning green; no user-visible change
  yet.
- Requirements and gaps: discharges `ISSUE-51-PROPOSED-RESOLUTION` and
  `ADR-004-RULE-IDENTICAL`.
- Acceptance evidence: `cargo nextest run -p theoremc-macros` passes, including
  the new provenance case and the unchanged whitespace-equivalence control.
- Conformance check:
  - Requirements satisfied: provenance is carried and rendered.
  - Design still followed: the index remains a single-pass build over
    document order, and probes are still anonymous `const _` items.
  - Upstream assumptions still valid: ADR-004's "identical declarations"
    rule is unchanged; equivalence still uses
    `is_semantically_equivalent`.
  - No unapproved public interface or dependency change: the variant is
    private to `theoremc-macros`; no dependency is added.
  - No trust boundary or persisted-format change.
  - Trace links current.
- Recovery: revert the single `lib.rs` commit; the tests fail again as
  expected.
- Remaining gaps: the trybuild golden file and the documentation.
- Compatibility decision: none — the type is private and pre-1.0.

Milestone EP-M2: tests and compile-fail coverage pin the new behaviour.

- Identifier and outcome: EP-M2 leaves the test suite proving both conflict
  kinds, the provenance fields, the rendered message, and the public compile
  error.
- Requirements and gaps: discharges `ISSUE-51-VALIDATION-UNIT` and
  `ISSUE-51-VALIDATION-TRYBUILD`.
- Acceptance evidence: `cargo nextest run -p theoremc-macros` passes and the
  regenerated golden file names both documents and both summaries.
- Conformance check:
  - Requirements satisfied: unit and trybuild expectations met; the snapshot
    option is deliberately not used (see `Decision Log`).
  - Design still followed: tests stay in the existing locations.
  - Upstream assumptions still valid: the trybuild golden file remains the
    single source of truth for the public message.
  - No unapproved public interface, dependency, trust-boundary, or persisted
    format change.
  - Trace links current.
- Recovery: `TRYBUILD=overwrite` re-derives the golden file from the
  implementation; the fixture `.rs` and `.theorem` files are untouched.
- Remaining gaps: documentation and the full gate run.
- Compatibility decision: none.

Milestone EP-M3: documentation and deterministic gates are green.

- Identifier and outcome: EP-M3 leaves documentation describing the diagnostic
  and every deterministic gate passing.
- Requirements and gaps: discharges `AGENTS-DOC-MAINTENANCE`.
- Acceptance evidence: `make check-fmt`, `make lint`, `make test`,
  `make markdownlint`, and `make nixie` all pass, with logs under `/tmp`.
- Conformance check:
  - Requirements satisfied: docs updated.
  - Design still followed.
  - Upstream assumptions still valid.
  - No unapproved public interface, dependency, trust-boundary, or persisted
    format change.
  - Trace links current.
- Recovery: documentation commits are independent of the code commits.
- Remaining gaps: CodeRabbit review and PR creation.
- Compatibility decision: none.

Milestone EP-M4: CodeRabbit review and draft PR.

- Identifier and outcome: EP-M4 leaves the branch pushed with a draft pull
  request titled with `(#51)`, a body stating `Closes #51`, and a clean
  CodeRabbit `--agent` result.
- Requirements and gaps: delivers the change for review.
- Acceptance evidence: `coderabbit review --agent` reports no unresolved
  concerns, and the PR exists as a draft.
- Conformance check: the PR references issue #51 and the Lody session.
- Recovery: the branch is pushed only after the deterministic gates pass.
- Remaining gaps: none.
- Compatibility decision: none.

## Concrete steps

All commands run from the worktree root
`/home/leynos/.lody/repos/github---leynos---theoremc/worktrees/f007b632-b9ef-4d68-b617-b879d0bb1900`.

Every command below is piped through `tee` so its output survives truncation. A
pipeline's exit status is that of its last command, so `tee` would mask a
failing gate. Run these snippets in a shell with `set -o pipefail` enabled, or
inspect `${PIPESTATUS[0]}` afterwards, before treating a gate as passed. The
gate results recorded in `Progress` and `Outcomes & Retrospective` were read
that way.

Red stage:

```sh
cargo nextest run -p theoremc-macros 2>&1 | tee /tmp/nextest-issue-51-red.out
```

Expected: compile errors in the two test modules on missing struct-variant
fields such as `first_theorem`.

Green stage:

```sh
cargo build -p theoremc-macros 2>&1 | tee /tmp/build-issue-51.out
cargo nextest run -p theoremc-macros 2>&1 | tee /tmp/nextest-issue-51-green.out
```

Expected: the build succeeds, and the trybuild test fails only on the golden
file mismatch.

Golden-file regeneration:

```sh
TRYBUILD=overwrite cargo nextest run -p theoremc-macros --test expand 2>&1 \
  | tee /tmp/nextest-issue-51-trybuild.out
git diff -- crates/theoremc-macros/tests/expand/
```

Expected diff (shape; the wrapped diagnostic text is one `error:` line):

```plaintext
 error: referenced action `account.deposit` has conflicting Actions signatures:
 theorem `FirstConflict` declares `fn(account: u64) -> bool`, but theorem
 `SecondConflict` declares `fn(account: u32) -> bool`
```

Gates:

```sh
make fmt                     2>&1 | tee /tmp/fmt-issue-51.out
make check-fmt               2>&1 | tee /tmp/check-fmt-issue-51.out
make lint                    2>&1 | tee /tmp/lint-issue-51.out
make test                    2>&1 | tee /tmp/test-issue-51.out
make markdownlint            2>&1 | tee /tmp/markdownlint-issue-51.out
make nixie                   2>&1 | tee /tmp/nixie-issue-51.out
```

Run them sequentially, never in parallel: the machine shares a compile-slot
pool and a Cargo package-cache lock.

CodeRabbit:

```sh
coderabbit review --agent 2>&1 | tee /tmp/coderabbit-issue-51-pass1.out
```

If the rate limit is exceeded, wait with `vsleep $(shuf -i 45-90 -n 1)m` and
retry.

## Validation and acceptance

Behaviour to observe, in order:

1. `expansion_names_both_declaring_theorems` (new, in
   `crates/theoremc-macros/src/action_probe_tests.rs`) fails before the
   `lib.rs` change with a compile error on the missing fields, and passes
   afterwards for both the parameter-drift and return-drift cases.
2. `action_signature_index_reports_conflicting_signatures_with_provenance`
   (extended in
   `crates/theoremc-macros/src/action_probe_tests/action_signature_index.rs`)
   asserts both theorem names and both summaries for both conflict kinds.
3. `cargo nextest run -p theoremc-macros --test expand` passes against the
   regenerated golden file.
4. `make test` passes workspace-wide;
   `whitespace_only_signature_drift_does_not_conflict` still passes as the
   negative control.
5. `make check-fmt` and `make lint` pass with no new warnings.

Quality criteria:

- Tests: every `theoremc-macros` test passes, including the new cases.
- Verification: obligations V1–V4 discharged as described in
  `Verification plan`.
- Lint/typecheck: `make lint` and `make check-fmt` clean.
- Performance: not applicable; the index is built once per expansion and the
  added allocation is two `String`s per conflict.
- Security: not applicable; no new input surface.

## Idempotence and recovery

Every step is repeatable. The trybuild golden file is regenerated from the
implementation, so re-running `TRYBUILD=overwrite` after a further change is
safe and produces a fresh, reviewable diff. Nothing here writes outside the
worktree except the `/tmp` logs. If a gate fails, fix the cause and re-run that
gate; do not re-run gates in parallel.

## Artefacts and notes

Rendered conflict message, to be confirmed against the golden file:

```plaintext
error: referenced action `account.deposit` has conflicting Actions signatures:
theorem `FirstConflict` declares `fn(account: u64) -> bool`, but theorem
`SecondConflict` declares `fn(account: u32) -> bool`
```

Pre-change golden file, for comparison:

```plaintext
error: referenced action `account.deposit` has conflicting Actions signatures
 --> tests/expand/conflicting_action_signatures.rs:5:15
```

## Interfaces and dependencies

No new dependency. The provenance items live in
`crates/theoremc-macros/src/action_signature_index.rs`, added to keep `lib.rs`
under the 400-line file cap. That module declares:

```rust
#[derive(Debug, Clone, Copy)]
struct DeclaredSignature<'a> {
    theorem: &'a TheoremName,
    signature: &'a ActionSignature,
}

fn render_signature_summary(signature: &ActionSignature) -> String;

fn conflicting_signature_error(
    action: &str,
    first: DeclaredSignature<'_>,
    conflicting: DeclaredSignature<'_>,
) -> MacroExpansionError;
```

`MacroExpansionError` itself stays in `lib.rs` (it is the crate's shared error
type), and `ConflictingActionSignature` is a struct variant with the fields
`action`, `first_theorem`, `conflicting_theorem`, `first_signature`, and
`conflicting_signature`, all `String`. The index module reaches the error type
through `use super::MacroExpansionError`, so the variant and its
`#[error(...)]` attribute remain in one place.

`ActionSignatureIndex::signature_for` keeps its signature
`fn signature_for(&self, canonical: &str) -> Result<&'a ActionSignature, MacroExpansionError>`
so `generated_action_probes` and `action_probe` are untouched.

The provenance unit tests live in
`crates/theoremc-macros/src/action_probe_tests/conflicting_signatures.rs`, and
the focused index tests in
`crates/theoremc-macros/src/action_probe_tests/action_signature_index.rs`, both
reached from `crates/theoremc-macros/src/action_probe_tests.rs` by `#[path]`
module declarations.

## Revision note (2026-10-11)

Initial draft, written after reconnaissance and two throwaway probes
(`thiserror` attribute forms; rustc rendering of a long `compile_error!`). Sets
the design decisions that stage C implements and records why no `insta`
snapshot is added.

Revision (2026-10-11T02:28Z): the provenance implementation first landed inside
`crates/theoremc-macros/src/lib.rs`, which grew to 478 lines and breached the
400-line file cap in `AGENTS.md`; `action_probe_tests.rs` reached 413 lines for
the same reason. The implementation and its tests were extracted into the two
new modules named above, and `lib.rs` returned to 379 lines. This section now
describes the delivered layout rather than the drafted one. No design decision
changed: the error type, the message format, the span, and the absence of an
`insta` snapshot are all as originally recorded in `Decision Log`.
