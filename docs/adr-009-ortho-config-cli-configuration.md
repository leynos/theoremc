# Architectural Decision Record (ADR) 009: OrthoConfig CLI configuration

- Status: accepted
- Date: 2026-10-26
- Deciders: theoremc maintainers
- Technical story: configuration boundary for the planned `cargo theorem` app

## Context

The merged `cargo theorem` design assigns command parsing and configuration to
the dedicated `crates/cargo-theorem/` application package. Issue #200 adopts
the published `ortho_config` 0.10.0 release as part of roadmap Phase 3.1. That
release requires Rust 1.89.0; the facade, `theoremc-core`, `theoremc-macros`,
and build-support packages currently declare Rust 1.88.

OrthoConfig v0.10.0 provides profile overlays, injected environment and file
sources, Cargo external-subcommand entry support, command behaviour metadata,
and Fluent-backed parsing. Its default features include TOML and the JSON merge
representation. Disabling default features is unsupported in this release.

ADR-002 specifies library-first localization through an injected `Localizer`
and Fluent. The reusable theoremc crates do not currently contain a Fluent
runtime or OrthoConfig dependency. The application can use the localizer
re-exported by OrthoConfig without raising the library MSRV.

## Decision

### 1. Keep OrthoConfig in the application package

Only `cargo-theorem` depends on `ortho_config = "0.10.0"`, with default
features enabled. The root facade, `theoremc-core`, `theoremc-macros`, and
build-support crates remain configuration-free and retain Rust 1.88. The CLI
package declares Rust 1.89.0. CI and local validation check the library
workspace on Rust 1.88 and the application package on Rust 1.89.

The CLI uses the OrthoConfig facade's Fluent APIs. Reusable libraries do not
adopt a direct `ortho_l10n` dependency for this change and do not raise their
MSRV. Their localization boundary remains the injected, library-owned
`Localizer` contract from ADR-002. If a library later needs an independent
Fluent runtime, that dependency and its MSRV impact require a separate
compatibility decision.

### 2. Define file discovery and precedence

Configuration files use TOML. Automatic discovery stacks system, user, and
project scopes in that order. On Unix and Redox, system files use each
`$XDG_CONFIG_DIRS` entry (default `/etc/xdg`) and user files use
`$XDG_CONFIG_HOME`, falling back to `$HOME/.config`; both search for
`theoremc/theoremc.toml`. User discovery also checks `~/.theoremc.toml`. On
other platforms OrthoConfig uses the home directory and native configuration
directory supplied by its `EnvSource`, where available. The project file is
`theoremc.toml` at the Cargo workspace root. Cargo starts external subcommands
in the caller's current directory without passing the workspace root, so
project commands resolve it with `cargo metadata` and pass the result to
OrthoConfig. `--manifest-path` selects the manifest used for that resolution.
`context --json` remains standalone and does not inspect the workspace.

An explicit `--config <path>` or `THEOREMC_CONFIG_PATH` selects one required,
exclusive file and suppresses automatic discovery; `--config` takes precedence
if both selectors are set. Missing optional files during automatic discovery
are ignored; malformed, unreadable, or explicitly missing files produce
actionable configuration errors. TOML `extends` entries layer parent files,
with relative paths resolved from the declaring file. Unknown keys are errors.

Resolved settings follow this order:

```text
application defaults
< discovered files
< selected named profile
< THEOREMC_* environment values
< explicit command-line values
```

This full order applies to global configuration. OrthoConfig 0.10.0 rejects
`cmds` and `inherits` inside profile bodies, so a profile cannot contain
command-specific settings. The selected subcommand receives only its relevant
merged configuration, whose precedence is defaults < files < environment <
explicit command-line values. `cli_default_as_absent` is used wherever a Clap
parser default must remain below file and environment values. Per-command
settings live under `cmds.<command>` and are merged only for the selected
command.

### 3. Offer named profiles

The application opts into OrthoConfig profiles. `[profile.<name>]` values sit
between files and environment values for global settings. `--profile <name>` and
`THEOREMC_PROFILE` select a profile; the global selector does not conflict
with the later `profile` management subcommand. In v0.10.0, profile bodies
cannot contain `cmds` or `inherits`, so profiles cannot override
command-specific settings. Secrets remain outside ordinary configuration and
generated agent context.

### 4. Preserve application-owned localization

Help and parse failures use OrthoConfig's localized parsing boundary with an
application-owned localizer. The foundation retains English Clap text as its
fallback; translated command catalogues and locale negotiation are later work.
Machine-readable diagnostic identifiers, exit classes, and JSON fields remain
non-localized. Libraries do not read process locale variables or install global
localization state.

### 5. Use the v0.10.0 Cargo and agent metadata

The entry point uses the single-variant `#[command(subcommand)]` wrapper so
`cargo theorem ...` and `cargo-theorem theorem ...` reach the same command
parser. Command metadata declares interaction and mutation behaviour. The
foundation runs OrthoConfig's `cargo orthohelp --check-agent-native` policy
check against the CLI package; it does not infer behaviour from command names.

## Consequences

- The application package has a separate Rust 1.89 MSRV; reusable consumers
  remain on Rust 1.88.
- The workspace lockfile contains the CLI's OrthoConfig dependency graph, but
  the library package check must still succeed on Rust 1.88.
- Configuration loading is testable without process-global environment
  mutation by using OrthoConfig's `_with_sources` entry points and `MapEnv`.
- Profiles and configuration namespaces are part of the planned CLI contract.
- Only the Phase 3.1 foundation ships in this slice. Backend execution,
  project mutation, profile management, and translated catalogues remain later
  roadmap work.

## Alternatives considered

### Put OrthoConfig in the facade

Rejected. It would expose application configuration and Rust 1.89 to reusable
library consumers, contrary to the approved package boundary.

### Raise every library to Rust 1.89 and depend on `ortho_l10n`

Rejected for this change. No current library needs an independent Fluent
runtime, and raising the library MSRV would affect consumers without an
implemented localization service that justifies the change.

### Keep configuration unimplemented until later CLI commands land

Rejected. Phase 3.1 owns the configuration and command spine. Building the
configuration boundary now prevents later handlers from inventing incompatible
discovery, environment, precedence, or command namespace rules.

## Related documents

- [ADR-002: library-first Fluent localization](adr-002-library-first-internationalization-and-localization-with-fluent.md)
- [`cargo theorem` CLI design](cargo-theorem-cli-design.md)
- [Theoremc development roadmap](roadmap.md)
- [OrthoConfig v0.10.0 provenance](ortho-config-v0-10-0-provenance.md)
