# Migrating to 0.2.0

This guide covers what changes for someone who builds `theoremc` from a
checkout when upgrading from 0.1.x to 0.2.0.

## Build defaults

The repository's Cargo configuration now makes the parallel `rustc` frontend
(`-Zthreads=8`) the default for every build, and the `mold` linker the default
on Linux. The pinned nightly toolchain accepts `-Zthreads=8`; macOS and Windows
keep their platform linker.

- On Linux, install `mold` before building. A build without it fails at link
  time.
- The Makefile development targets (`make test`, `make lint`, `make typecheck`
  and the debug build) compose the standard flags onto any `RUSTFLAGS` already
  in the environment, so existing flags keep working.
- `make release` and the coverage build assign their own `RUSTFLAGS`, which
  displaces the configuration's flags, so a shipped artefact keeps the platform
  linker and a measurement does not depend on the fast flags.

## Building without the defaults

An assigned `RUSTFLAGS`, even an empty one, displaces every flag in
`.cargo/config.toml`. To use the platform linker and the stable frontend for a
single command, assign an empty value:

```bash
RUSTFLAGS="" cargo build --release
```

The Rust 1.88 validation path needs this form, because a stable toolchain
refuses `-Zthreads=8`:

```bash
RUSTFLAGS="" cargo +1.88 check --workspace
```

## Borrowed-string newtypes

0.2.0 adds `theoremc::schema::StrNewtype`, a public trait supplying `new` and
`as_str` for `&'a str`-backed newtypes. It is additive: no existing public type
changed shape, so an upgrading caller needs no code change.

It matters for a crate that defines its own borrowed-string wrapper. Such a
wrapper now derives its conversions instead of writing the pair out, and
callers must bring the trait into scope to reach the two methods:

```rust
use derive_more::{From, Into};
use theoremc::schema::StrNewtype;
use the_newtype::Newtype;

#[derive(Newtype, From, Into, Copy, Clone)]
struct Marker<'a>(&'a str);

let marker = Marker::new("witness");
```

Defining a wrapper this way requires `the-newtype` and `derive_more` as direct
dependencies of the defining crate, because Cargo does not re-export them. The
[trait bounds and wrapper recipe](developers-guide.md) and the
[wrapper API contract](users-guide.md) are documented in full.

Two consequences worth knowing before adopting it. `as_str` takes `&self`
rather than `self` by value, because Clippy's `wrong_self_convention` rejects a
by-value receiver on a trait method; `Copy` wrappers are unaffected. And the
methods are not `const`, so a wrapper that needs `const` construction or access
must keep an inherent `const fn` pair alongside the derive — a blanket trait
method cannot be `const`.
