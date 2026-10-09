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
