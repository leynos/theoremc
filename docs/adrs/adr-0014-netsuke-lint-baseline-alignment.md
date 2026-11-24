# Architectural decision record (ADR) 0014: Netsuke lint baseline alignment

## Status

Accepted. TheoremC's three workspace lint tables now enforce the
`leynos/netsuke` lint policy as a minimum, keep any stricter TheoremC rule, and
record every deliberate divergence beside the setting it affects. At this
decision no stricter Clippy rule remains; the divergences are cfg registrations
and disallowed methods. The generated Kani gate is an `allow` scoped to one
module rather than an `expect`.

## Date

2026-11-23

## Context and problem statement

Issue #98 asks TheoremC to treat the `leynos/netsuke` workspace lint tables as
a floor: every applicable rule is enforced here, and any stricter TheoremC rule
survives the alignment. Before this decision the tables carried a subset. The
gaps were the rule families the earlier layers deferred because each needed
remediation first:

- the Netsuke Clippy denials for `disallowed_methods`,
  `missing_assert_message`, and `missing_docs_in_private_items`;
- the Rust denials `unknown_lints`, `renamed_and_removed_lints`, and
  `unsafe_code = "forbid"`, plus `unexpected_cfgs` with the `check-cfg`
  declarations Kani and Dylint need;
- the complete Netsuke rustdoc set beyond `missing_crate_level_docs`.

The policy landed in layers: #233 added the zero-finding rules, #234 added the
environment restrictions and `unsafe_code = "forbid"` after removing the only
`unsafe` code, and #235 added `missing_docs_in_private_items` after documenting
the private items it reports. A layer-per-rule approach kept every commit
green, but it left three obligations: the policy was not compared against a
pinned baseline, its divergences were undocumented, and nothing prevented a
future deletion.

## Decision drivers

- Treat Netsuke as a minimum rather than a template, so a rule TheoremC needs
  is added without dropping the ones it already has.
- Never weaken an entry to make a gate pass; resolve the finding at source, or
  constrain the exception to the smallest scope that works.
- Keep the reason for every exception where a reader will meet it: beside the
  setting in `Cargo.toml` or `clippy.toml`, or on the cited item.
- Make removal visible. A deleted rule should fail a test, not wait for a
  violation that may never occur.

## Baseline

The comparison used `leynos/netsuke` at commit
`871b609d4228ac3be714ad7ead1ddb0fb4ac17f6` (`main`), retrieved on 2026-11-23.
Every rule in its `[workspace.lints.clippy]`, `[workspace.lints.rust]`, and
`[workspace.lints.rustdoc]` tables is present in TheoremC at the same or a
stricter level, and TheoremC adds none. The rule set is pinned by
`tests/lint_policy_contract.rs`, so a later removal fails the suite.

## Decision

1. **The three tables match the baseline.** The Clippy table denies the
   baseline's 45 rules under its own group headings and keeps
   `pedantic = { level = "warn", priority = -1 }`, so an explicit denial always
   wins over the group. The Rust table denies `unknown_lints`,
   `renamed_and_removed_lints`, and `missing_docs`, forbids `unsafe_code`, and
   declares `unexpected_cfgs` at `warn` with `check-cfg` for `cfg(kani)` and
   `cfg(dylint_lib, values(any()))`; the gates supply `-D warnings`, so a new
   undeclared `cfg` fails them. The rustdoc table denies all seven baseline
   rules.
2. **Cognitive complexity stays with Whitaker, as in the baseline.** The
   Clippy rule is not enabled, because it scores code after macro expansion and
   so counts branching a macro generated. Whitaker's experimental
   `macro_aware_cognitive_complexity` gates it at 9, CodeScene's ceiling, and
   `dylint.toml` opts in. This matches the baseline rather than diverging from
   it.
3. **Three omissions are recorded rather than silently dropped.**
   `cfg(verus)` is not registered, because no Verus cfg exists yet and ADR-0007
   keeps that backend planned. `cfg(theoremc_has_theorems)` stays registered by
   the package's `build.rs`, because that script travels into every consumer
   crate and owns that package's discovery result. The two non-environment
   disallowed methods (`shell_quote::QuoteRefExt::quoted` and
   `std::io::IsTerminal::is_terminal`) are omitted because TheoremC has no
   shell-quoting dependency and performs no terminal detection.
4. **Environment access is injected, not exempted.** Production code reads the
   environment through an injected reader; tests pass a stub. The seven
   `std::env` methods are disallowed in `clippy.toml` with reasons that name
   the remedy. The few boundary seams that must inherit Cargo's or the host's
   values carry `#[expect(clippy::disallowed_methods, reason = "...")]`, so the
   expectation warns once the site is migrated and the backlog removes itself.
   `unsafe_code = "forbid"` is workspace-wide, so no expectation can re-open
   it, and the guard helper that once mutated the process environment is
   deleted.
5. **The generated Kani gate is an `allow`, not an `expect`.** The proc macro
   emits the gate into the invoking crate, whose configuration it cannot know.
   A crate that registers `cfg(kani)` itself leaves an `expect` unfulfilled, and
   `-D warnings` turns that into an error; a crate that does not register it
   needs the `cfg` suppressed. Only `allow` is correct under both. It is scoped
   to the generated per-file module, names one lint, and carries a reason. The
   action and referenced-type probes are emitted inside that module, so they
   share the scope; narrowing it further would mean emitting them elsewhere,
   which the current emission does not do. Two BDD scenarios build independent
   consumers with `RUSTFLAGS=-D warnings`, one registering `cfg(kani)` and one
   not, and both must pass.

## Consequences

- A rule can no longer be deleted or downgraded unnoticed: the semantic
  contract test fails on the manifest, and the behavioural test fails on the
  stage that would have caught the violation.
- The policy is auditable in one place. Each divergence carries its reason
  beside the setting, so a reader does not have to reconstruct intent from
  history.
- The generated gate no longer participates in the expectation economy. That
  is a deliberate exception to the "`allow` is never used" rule: an expectation
  in generated code cannot be fulfilled reliably, so the alternative would be
  to require every consumer to register `cfg(kani)`, which pushes a macro
  detail into consumer manifests.
- Answers to "why is this rule here?" and "why is that one missing?" live with
  the rule. The cost is that the tables are longer; the benefit is that a
  future contributor does not need this record to avoid re-adding an
  inapplicable rule.

## Related documents

- [ADR-0005: Rust build standard](adr-0005-adopt-the-rust-build-standard-for-development-builds.md)
- [ADR-0007: Verus backend preservation invariants](adr-0007-verus-backend-preservation-invariants.md)
- [ADR-0013: workspace-complete lint gate](adr-0013-workspace-complete-lint-gate.md)
- [Developer's guide](../developers-guide.md)
- [Theoremc design](../theoremc-design.md)
- [Issue #98](https://github.com/leynos/theoremc/issues/98)
