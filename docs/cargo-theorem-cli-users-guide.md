# `cargo theorem` user's guide

- Status: proposed
- Audience: users, integrators, and automation authors

This guide describes how to use the `cargo theorem` command-line interface
(CLI). It covers the tasks a caller performs: installing and first use,
initialising a project, creating and inspecting theorems, checking a project,
installing and pinning proof backends, running proofs, reading reports, and
migrating from `rust-prover-tools`.

The command surface is **planned, not shipped**. It is defined normatively in
the [`cargo theorem` CLI design](cargo-theorem-cli-design.md); where this guide
and that design disagree, the design wins. Nothing here is a compatibility
promise for a released binary.

## Before you begin

`cargo theorem` is a Cargo external subcommand, so it needs two things already
in place:

- Cargo, which ships with a Rust toolchain; and
- Rust, installed through [Rustup](https://rustup.rs/), at a version that
  satisfies the CLI's own minimum supported Rust version (MSRV).

The CLI is its own package, `cargo-theorem`, and it declares its own MSRV. The
CLI requires Rust 1.89, because `ortho_config` 0.9.0 requires it, while the
theoremc library packages stay on Rust 1.88 until a separate compatibility
decision raises theirs. Continuous integration (CI) tests the two contracts
separately:

```console
cargo +1.88 check --workspace --exclude cargo-theorem
cargo +1.89 check -p cargo-theorem
```

Adding the CLI must not silently increase the compiler version required by
theoremc library consumers, which is why the split is tested explicitly.

## Running the command

Every workflow is reached through one entry point:

```console
cargo theorem <command> [options]
```

The distributable package and executable are both named `cargo-theorem`. Cargo
discovers that executable on `PATH` and exposes it as `cargo theorem`. Direct
invocation as `cargo-theorem` is supported for tests and unusual execution
environments, but documentation and diagnostics use `cargo theorem`.

Commands that inspect or mutate a Cargo project accept these global selectors:

- `--manifest-path <path>` selects a Cargo manifest;
- `--package <package-id>` selects one workspace member;
- `--workspace` selects all eligible members; and
- `--exclude <package-id>` excludes members from workspace selection.

Selection uses the stable `cargo metadata` JSON protocol. Ambiguous package
names fail with an error that lists the valid package identifiers. Mutation
commands require one unambiguous target unless their contract explicitly
supports workspace-wide changes.

## Initialising a project

`cargo theorem init` calculates and applies one project integration plan. The
plan may:

- create `theoremc.toml` when no project configuration exists;
- create `theorems/` and `.theoremc/` support directories;
- add `.theoremc/` runtime artefacts to `.gitignore`;
- add the theoremc runtime and build dependencies to the selected manifest;
- install the managed build-script call;
- create or connect `src/theorem_actions.rs`; and
- optionally add a small compiling example when `--example` is present.

The default is additive: existing files are preserved unless theoremc can prove
that a managed fragment can be updated safely.

`--dry-run` returns a structured plan containing create, edit, unchanged,
conflict, and manual-action entries, and writes nothing. `--force` authorises
only the specific conflicts named in that plan. It does not turn arbitrary
source rewriting into an accepted operation.

Applying the same plan twice results in no content changes. Every file write
uses a temporary sibling, a flush, and an atomic rename. Multi-file changes
acquire a workspace-local lock and retain a recovery journal until all renames
complete.

## Creating and inspecting theorems

The theorem resource uses the canonical top-level `list`, `get`, and `create`
verbs, so the command does not stutter as `cargo theorem theorem list`:

```text
cargo theorem list
cargo theorem get <theorem-id>
cargo theorem create <theorem-name>
cargo theorem action list
cargo theorem action get <action-name>
cargo theorem action create <action-name>
```

`list` returns stable theorem IDs, paths, tags, configured backends, and a
short status summary. `get` returns one theorem's parsed document, canonical
ID, aliases, generated harness names, referenced actions, and evidence policy.
Source text is omitted from JSON unless `--include-source` is explicit.

`action list` and `action get` expose canonical names, mangled exports,
declared signatures, source theorems, and binding-probe status. They do not
attempt Rust reflection beyond the compile-time contracts theoremc already owns.

### Creating a theorem

`cargo theorem create <theorem-name>` creates one valid `.theorem` document.
The positional value is the theorem's name (`T`), not its canonical external
ID. It accepts `--about`, `--backend`, `--tag`, `--path`, and `--template`.

Without an explicit path, the scaffold is written to
`theorems/<normalized-name>.theorem`. The canonical external ID is then
`{normalized_path(P)}#<theorem-name>`, where `P` is the created path. Callers
use that ID with `get` and with stable-ID selectors.

An explicit `--path` is resolved relative to the selected workspace when it is
not absolute. Before any file is written, theoremc canonicalises the workspace
root and the path, resolving existing symlinks, and then verifies that the
target remains within the canonical workspace. The default `theorems/` path
receives the same containment check. External paths are not supported, so there
is no escape hatch from this guard.

The scaffold contains no invented business property. It includes explicit
placeholders represented as schema-valid documentary text, and it refuses to
claim that a proof exists. `--template example` may create a complete
pedagogical example, but ordinary creation does not generate assertions from
the theorem name.

`action create` can add a theorem-side signature declaration and a
deterministic re-export entry for an existing Rust function. Function-body
generation requires an explicit `--stub` mode and a complete parameter and
return signature. By default it adds no `todo!()`, panic stubs, or lint
suppressions to consumer code.

## Checking a project

`cargo theorem check` composes non-installing validations:

- schema validation;
- alias validation;
- action-signature checks;
- build-script checks; and
- configured backend pin checks.

The default command is read-only. `check --compile` is an explicit effectful
boundary: it invokes Cargo, and Cargo may execute the package's arbitrary
`build.rs`. Its plan and agent context are therefore classified separately from
read-only validation.

Theoremc itself performs no network access or backend installation in either
mode. A project `build.rs` invoked by `check --compile` may independently
access the network or install tools; those effects are outside theoremc's
control and are reported as project build effects.

## Installing and pinning backends

Backends are managed under the `backend` subcommand:

```text
cargo theorem backend list
cargo theorem backend get <backend>
cargo theorem backend install <backend>
cargo theorem backend check <backend>
cargo theorem backend sync
cargo theorem backend update <backend>
cargo theorem backend delete <backend>
cargo theorem backend run <backend>
```

**Table:** Backend commands and their effects

| Command           | Effect                                                                       | Installs tools | Requires `--force` |
| ----------------- | ---------------------------------------------------------------------------- | -------------- | ------------------ |
| `backend list`    | Returns registered providers and configured or installed state               | No             | No                 |
| `backend get`     | Returns one descriptor, pin, installation, capability, and health record     | No             | No                 |
| `backend install` | Installs the locked or explicitly requested release                          | Yes            | No                 |
| `backend check`   | Compares the installed tool with the resolved lock and runs usability probes | No             | No                 |
| `backend sync`    | Checks every selected backend and installs only missing or mismatched pins   | Yes            | No                 |
| `backend update`  | Changes a project pin and lock entry; installs only with `--install`         | Only with flag | No                 |
| `backend delete`  | Removes a specific theoremc-managed cache entry                              | No             | Yes                |
| `backend run`     | Low-level compatibility and diagnostics path for a proof file or harness     | No             | No                 |

*Table 1: Backend subcommands, effects, and their installation and force rules.*

The most important rule in this section is a negative one: `check`,
`build-script run`, and top-level `run` never install or update tools.
Installation is always explicit. A command that finds a missing backend reports
`backend_unavailable` rather than fetching it.

Two further rules are easy to get wrong:

- `backend install` never rewrites project pins unless `--update-lock` is
  given; and
- `backend delete` removes a theoremc-managed cache entry and requires
  `--force`, and it still respects active-install leases. A cache entry that is
  in use is not evicted.

Project intent lives in `theoremc.toml`, and resolved state lives in
`theoremc.lock`. The lock records resolved versions, release source, checksum,
required Rust toolchain, and provider schema version. It does not record
machine-specific installation paths. Installations live in the platform cache
directory by default and may be redirected with configuration.

`backend run <backend>` is the low-level compatibility and diagnostics path.
Ordinary theorem execution uses top-level `run` so that evidence, identity,
policy, and reports remain connected. `backend run` offers
`--passthrough-exit-code` for migration scripts; that flag disables the stable
application exit-code promise and is marked as an advanced compatibility option
in agent context.

## Running proofs

```console
cargo theorem run
```

A run performs these stages:

1. Resolve workspace, configuration, profile, and theorem selection.
2. Validate theorem sources and stable IDs.
3. Check required backend installations and pins without changing them.
4. Materialise an immutable execution plan and input digest.
5. Create a run directory and ledger record.
6. Execute backend jobs with bounded parallelism.
7. Parse backend-native results into canonical theorem outcomes.
8. Apply expected-status, witness, vacuity, and incomplete-result policy.
9. Persist the canonical run record before rendering summaries or delivery.
10. Return a stable application status and artefact references.

### Waiting for a run

Runs wait by default. `--no-wait` starts a durable detached job, returns the
job ID, and leaves all output in the run directory; `--wait` is the canonical
positive flag in metadata and may be set explicitly by automation. Both forms
write a durable record under `.theoremc/runs/<run-id>/`.

`--timeout` and `--jobs` control bounded execution. `--keep-going` continues
after a failure, and `--fail-fast` stops at the first one. Those last two are
mutually exclusive.

### Selecting theorems

`run`, `generate`, and report-oriented inspection share one selection model:

- exact stable IDs through repeated `--id`;
- path globs through repeated `--path`;
- tags through `--tag` and `--exclude-tag`;
- package and workspace selectors;
- backend through `--backend auto|all|<name>`; and
- optional changed-file selection through `--changed-since <git-revision>`.

Selectors combine by intersection across categories and union within a repeated
category. The resolved selection is sorted by stable theorem ID.

An empty selection fails by default and explains the active filters.
`--allow-empty` turns it into a successful no-op for conditional CI jobs.

`--backend auto` runs each theorem's declared evidence backends and never falls
back from one verifier to another. `--backend all` requests every configured
backend supported by the theorem and reports unsupported combinations
explicitly.

### Idempotency

An optional `--idempotency-key` prevents duplicate active submissions. The
input digest covers the execution plan and the protected provider state that
each protected secret binds to. Because a protected value is never persisted,
that state contributes a stable, non-redeemable one-way fingerprint, or an
explicit provider-state version when the provider supplies one, and never the
value, a lookup key, or a capability.

Reusing a key returns the existing job only when the input digest and the
protected provider state both match. When the provider state behind a reference
is rotatable and neither a fingerprint nor a version can be derived, reuse is
rejected rather than assumed. Reusing a key with different inputs fails and
reports both digests. `--force-new` creates a new job while retaining the
relationship to the prior run.

`--backend-arg NAME=VALUE` accepts public values only. Undeclared or
secret-typed arguments supplied through this option are rejected rather than
guessed; providers declare protected secret inputs separately. Secret bytes are
never placed in argv, command metadata, provenance, tracing logs, run records,
or test fixtures.

## Reading reports and replaying failures

```text
cargo theorem report create <run-id>
cargo theorem replay <run-id> [--id <theorem-id>]
```

`report create <run-id>` renders one or more formats:

```text
json
markdown
html
junit
cucumber
```

Multiple `--format` values require `--deliver` to a directory or a filename
pattern. Machine-facing formats retain stable codes and English fallback text;
localised strings are optional projections and do not replace invariant fields.

`replay <run-id>` selects a failed theorem and delegates to the owning
backend's replay capability. With one replayable failure, the selector is
implicit. When the run has multiple replayable failures, `--id <theorem-id>` is
required; omitting it fails with a bounded list of valid failed IDs. An ID that
is not in that list is also a bounded usage diagnostic.

The result is a child run with links to the original counterexample, generated
playback source, command line, and logs. Replaying never rewrites application
source unless a future explicit `--apply` workflow is separately designed. Kani
concrete playback is the first implementation.

## Managing jobs

Every run, including a foreground run, has a durable record under
`.theoremc/runs/<run-id>/`. That directory contains `run.json`, `plan.json`,
bounded command metadata, backend stdout and stderr logs, generated reports,
and replay artefacts. A workspace-local append-only index supports bounded job
listing over those records.

Four subcommands operate on the ledger:

```text
cargo theorem jobs list
cargo theorem jobs get <job-id>
cargo theorem jobs cancel <job-id>
cargo theorem jobs prune
```

**Table:** `jobs` subcommands and their effects

| Command       | Effect                      | Mutation guard  |
| ------------- | --------------------------- | --------------- |
| `jobs list`   | Lists runs from the index   | Read-only       |
| `jobs get`    | Returns one run summary     | Read-only       |
| `jobs cancel` | Records cancellation intent | Explicit target |
| `jobs prune`  | Prunes the run ledger       | Explicit target |

*Table 2: `jobs` subcommands, their effects, and their mutation guards.*

`jobs list` accepts status, backend, theorem ID, time-range, and parent-run
filters, and applies the standard bounded list contract described under Bounded
responses.

`jobs get <job-id>` returns one run summary plus optional bounded log excerpts.
Full backend logs are stored as artefacts and are retrieved through a specific
`jobs get` request with an explicit byte limit, rather than being embedded in
the summary.

`jobs cancel <job-id>` records cancellation intent and terminates only a
theoremc-owned active process.

`jobs prune` supports `--before`, `--status`, `--dry-run`, and `--force`. It
never deletes the newest record for an idempotency key without naming that
consequence in the plan.

## JSON output and exit codes

Data-returning commands support `--json`. The two stdout and stderr contracts
are exact:

- success writes exactly one JSON document to stdout and nothing to stderr;
- failure writes no stdout and exactly one JSON diagnostic document to stderr.

`result` is present only for a successful command and contains command data.
`error` is present only for a failed command and contains its diagnostic. The
two fields are mutually exclusive and absent from envelopes whose status is
neither `success` nor `error`. Progress, subprocess output, and tracing logs
are captured rather than inherited, and the result includes paths to full logs
and artefacts instead of embedding unbounded output.

Schema identifiers, diagnostic codes, exit classes, backend names, and status
values are never localised.

The stable application exit codes are:

**Table:** Stable application exit classes

| Code | Class                     | Meaning                                                                                                 |
| ---- | ------------------------- | ------------------------------------------------------------------------------------------------------- |
| `0`  | `success`                 | The command completed and all selected evidence matched policy                                          |
| `2`  | `usage`                   | Arguments or a selected value were invalid                                                              |
| `3`  | `configuration`           | Configuration could not be discovered, merged, or validated                                             |
| `4`  | `project_state`           | Cargo package or theoremc integration state was unsuitable                                              |
| `5`  | `backend_unavailable`     | A required backend was missing or did not match its pin                                                 |
| `6`  | `verification_mismatch`   | Actual proof status differed from declared evidence policy                                              |
| `7`  | `verification_incomplete` | A selected proof was unreachable, undetermined, timed out, or cancelled without an accepted expectation |
| `8`  | `external_tool_failure`   | Cargo, Rustup, or a backend failed outside a parsed proof result                                        |
| `9`  | `delivery_failure`        | Execution succeeded but an explicitly requested delivery target failed                                  |
| `10` | `internal`                | The CLI violated an invariant or encountered an unclassified failure                                    |

*Table 3: Stable application exit classes and their process exit codes.*

Backend-native exit codes are recorded in run data and diagnostics; the
high-level CLI never leaks them as its own unstable taxonomy.

## Bounded responses

Every list command defaults to 50 entries and accepts `--limit`. The maximum
accepted limit is 500, and an error caused by an excessive limit states the
maximum valid value. When more data exists, the response returns an opaque
`next_cursor` that can be passed back as `--cursor`.

Human summaries include at most ten failing theorem entries before pointing to
the canonical run artefact. JSON summaries contain bounded arrays and explicit
counts. Full backend logs are stored as artefacts and retrieved through a
specific `jobs get` request with an explicit byte limit.

The following are hard default resource and retention limits. Configuration may
lower a limit, but may not raise one until a future compatibility decision
changes this contract:

- a run may select and persist at most 500 theorem outcomes;
- each backend process may retain at most 16 MiB of stdout and 16 MiB of
  stderr;
- a run may retain at most 256 MiB and 1,000 artefacts in total, and each
  generated report, replay artefact, or partial artefact is limited to 64 MiB;
- the run ledger retains at most 10,000 runs, 10 GiB, or 30 days, whichever is
  reached first; and
- the backend cache defaults to 10 GiB and evicts non-active entries by
  least-recently-used (LRU) or age.

## Delivery

Commands that create artefacts accept:

```text
--deliver stdout
--deliver file:<path>
--deliver webhook:<url>
```

`--deliver stdout` is valid only in human mode. Combining it with `--json` is a
usage error, because JSON mode reserves stdout for exactly one JSON document;
JSON callers must select `file:<path>` or another supported non-stdout target.

`stdout` is a validated stream with partial-write semantics: the artefact is
fully validated before the first byte is written, and an interrupted terminal
or pipe write leaves a truncated stream that cannot be rolled back. Callers
that require atomic bytes must use `file:<path>`, which writes through a
temporary sibling, flushes, and renames.

`webhook:<url>` is deferred. The local implementation must ship before webhook
delivery. Unknown schemes enumerate the supported values.

## Configuration and precedence

The canonical project configuration file is `theoremc.toml` at the Cargo
workspace root. An explicit `--config <path>` suppresses discovery.

Resolved values follow this precedence, from lowest to highest:

```text
built-in defaults
< discovered configuration files
< selected profile
< THEOREMC_* environment variables
< command-line arguments
```

The selected subcommand receives only its relevant merged configuration.
Configuration for `backend install` must not leak into `run`, and a `run`
profile must not alter project mutation commands unless it declares those
fields explicitly.

User-level configuration may provide renderer and installation-cache defaults,
but project proof policy must live in the project file or in theorem evidence.

## Working with profiles

A profile is a named configuration overlay. It sits between discovered
configuration files and `THEOREMC_*` environment variables in the precedence
order listed under Configuration and precedence, so it can specialise a project
default without editing `theoremc.toml` and without exporting variables. The
selected subcommand receives only its relevant merged configuration, and a
profile applies only to the commands it declares fields for.

Four subcommands manage the profile store:

```text
cargo theorem profile list
cargo theorem profile get <profile>
cargo theorem profile save <profile>
cargo theorem profile delete <profile>
```

`profile list` is paginated. `save` and `delete` support `--dry-run`, and
`delete` requires `--force`.

Secrets are redacted. `profile` output and `context --json` expose profile
names and non-secret fields only, and never secret values.

Profile read-modify-write transactions are atomic under the profile lock, so a
concurrent reader observes either the previous state or the complete new state.
Configuration for one subcommand must not leak into another: configuration for
`backend install` must not leak into `run`, and a `run` profile must not alter
project mutation commands unless it declares those fields explicitly.

## Migrating from `rust-prover-tools`

`cargo theorem` absorbs the durable behaviour of `leynos/rust-prover-tools`
without preserving its Python implementation or its command vocabulary as a
second public API.

`theoremd` never shipped as a compatibility surface, so there is no binary to
deprecate. `cargo theorem` is the surface from the outset.

**Table:** Migration from `rust-prover-tools`

| Legacy invocation                            | `cargo theorem` equivalent                            |
| -------------------------------------------- | ----------------------------------------------------- |
| `prover-tools kani install`                  | `cargo theorem backend install kani`                  |
| `prover-tools kani check-version`            | `cargo theorem backend check kani`                    |
| `prover-tools verus install`                 | `cargo theorem backend install verus`                 |
| `prover-tools verus run --proof-file <path>` | `cargo theorem backend run verus --proof-file <path>` |

*Table 4: `rust-prover-tools` commands and their `cargo theorem` equivalents.*

Legacy `KANI`, `VERUS_*`, and `INPUT_*` environment variables are accepted only
by the migration adapter, only for the corresponding backend command, and they
emit a deprecation diagnostic in human mode. In JSON mode the legacy source is
recorded in a non-localised `warnings` array rather than contaminating stdout
with prose.

The legacy `--kani-command` override becomes a typed executable plus argument
vector in configuration. Shell parsing is not part of the new stable contract,
because providers construct argument vectors rather than shell command strings.

`rust-prover-tools` is retired only after cross-platform parity gates pass.
Archival cannot occur until Kani and Verus parity fixtures pass on every
supported host and migration guidance has shipped. Until then the Python tool
remains available.

## Where to go next

- [`cargo theorem` CLI design](cargo-theorem-cli-design.md) — the normative
  definition of every command, contract, and boundary summarised here.
- [Theoremc development roadmap](roadmap.md) — the implementation sequence,
  including Phase 3.1 for the CLI foundation and Step 5.4 for migration
  completion.
- [Theorem file format](theorem-file-specification.md) — the `.theorem` schema
  that `create`, `list`, and `check` operate on.
- [User's guide](users-guide.md) — the library application programming
  interface (API) guide for `theoremc` consumers.
