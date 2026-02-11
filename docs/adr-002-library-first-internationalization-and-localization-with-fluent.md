# Architectureal Decision Record (ADR) 002: library-first internationalization and localization with Fluent

- Status: proposed
- Date: 2026-02-11
- Deciders: theoremc maintainers
- Technical story: localizable diagnostics for theoremc as a reusable library

## Context

`theoremc` is a library crate first. Consumers embed it into different
applications, each with their own locale negotiation policy, fallback rules,
and runtime environment constraints.

We reviewed three existing approaches:

- `rstest-bdd` ships Fluent catalogues inside the crate and exposes helpers for
  switching locales at runtime. It provides strong defaults and broad locale
  coverage, but it also relies on mutable global and thread-local loader state.
- `cucumber-rs/gherkin` localizes parsing keywords (for example, `Feature`,
  `Given`, and `Then`) by loading static language definitions and honouring the
  `# language:` directive. It does not localize runtime diagnostics.
- `ortho-config` defines a library-safe `Localizer` trait and a
  `FluentLocalizer` implementation that layers consumer catalogues over
  embedded defaults. It avoids global localisation state, preserves fallback
  behaviour, and allows deterministic error handling.

`theoremc` currently emits English diagnostics through `Display` strings in
typed error enums. This is deterministic but not localizable.

For theoremc, we need:

- library-safe localisation without process-wide mutable state,
- stable machine-readable diagnostics for tooling and CI integration,
- Fluent-based message rendering for human-facing output,
- deterministic fallback behaviour when translations are missing or malformed,
- no hidden locale negotiation inside the library.

## Decision

### 1. Adopt a library-first localisation contract

`theoremc` will adopt an injected localizer model, not a global loader model.

The library will expose a `Localizer` trait (or equivalent) for rendering
diagnostic message IDs and interpolation arguments. Locale negotiation remains
the consumer application's responsibility.

No theoremc API will read environment locale variables or mutate global locale
state.

### 2. Separate structured diagnostics from rendered text

`theoremc` diagnostics will be modelled with:

- a stable diagnostic code (machine-readable),
- structured arguments (machine-readable),
- an English fallback text (human-readable baseline).

Errors remain typed and inspectable. Localized rendering is applied at display
boundaries, not as the source of truth for programmatic error handling.

### 3. Use Fluent as the default localisation backend

`theoremc` will ship embedded Fluent resources for `en-US` as the required
baseline locale and may add further locales incrementally.

The crate will expose its Fluent assets so consumers with an existing
`FluentLanguageLoader` can load theoremc catalogues into their own localization
context.

To support turnkey adoption, theoremc will also provide an optional
Fluent-backed localizer implementation that:

- layers consumer-provided resources over theoremc defaults,
- falls back to theoremc defaults when consumer messages are absent,
- reports formatting failures and then falls back to the next catalogue.

### 4. Keep determinism for compile-time and machine-facing outputs

Compile-time diagnostics emitted by macros and code generation remain English
and deterministic.

Machine-facing artefacts (for example, JSON reports) use stable diagnostic
codes and structured fields; localized human text is optional metadata.

### 5. Scope parser internationalization separately

Unlike `gherkin`, theoremc will not localize schema keywords (such as
`Theorem`, `Given`, `Prove`) in this decision. The theorem schema remains
canonical and language-stable for now.

Internationalization of theorem syntax is deferred to a future ADR, after
validating demand and migration implications.

## Consequences

Positive consequences:

- Consumers keep full control over locale selection and fallback policy.
- Theoremc remains composable as a library in multi-crate applications.
- Diagnostics become localizable without sacrificing typed error handling.
- CI and tooling remain stable through diagnostic codes and English fallback.

Costs and trade-offs:

- The implementation is more complex than static English `Display` strings.
- Message IDs and argument schemas become part of the compatibility surface.
- Translation catalogues require governance and regression testing.
- Consumers must explicitly wire a localizer to see localized output.

## Alternatives considered

### Use a global Fluent loader inside theoremc

Rejected. This mirrors part of the `rstest-bdd` ergonomics but introduces
shared mutable localization state that is awkward for library composition,
parallel tests, and host applications with existing i18n infrastructure.

### Localize only parser keywords, following gherkin

Rejected for now. Theoremc's immediate pain point is user-facing diagnostics,
not theorem syntax keywords. Localizing schema keys increases parser complexity
and migration burden without solving diagnostic localization.

### Keep English-only diagnostics

Rejected. This blocks localization for downstream products and pushes
translation work into every consumer, creating inconsistency and duplicate
effort.

## Related documents

- `docs/localizable-rust-libraries-with-fluent.md`
- `docs/theoremc-design.md`
- `docs/users-guide.md`
- `docs/adr-0001-theorem-symbol-stability-and-non-vacuity-policy.md`
