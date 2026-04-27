# Theorem Compiler

theoremc is a Rust workspace for compile-time theorem validation via
[Kani](https://github.com/model-checking/kani).

## Crate layout

| Crate | Purpose |
| --- | --- |
| `theoremc` | Root facade — re-exports `theoremc-core` modules and the `theorem_file!` proc-macro |
| `theoremc-core` | Schema parsing, name mangling, collision detection, and the crate-relative theorem loader |
| `theoremc-macros` | The `theorem_file!` proc-macro implementation |

## `theorem_file!` proc-macro

The `theorem_file!` macro expands a crate-relative `.theorem` file into a
deterministic private module at compile time:

```rust
theorem_file!("theorems/my_theorem.theorem");
```

Invalid theorem files cause the build to fail with an actionable schema
diagnostic pointing to the source location of the error.

## Programmatic theorem loading

`theoremc-core` exports `load_theorem_file_from_manifest_dir` for loading and
validating `.theorem` files outside the proc-macro context:

```rust
use camino::Utf8Path;
use theoremc_core::load_theorem_file_from_manifest_dir;

let docs = load_theorem_file_from_manifest_dir(
    Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
    Utf8Path::new("tests/fixtures/my_theorem.theorem"),
)?;
```

See [`docs/users-guide.md`](docs/users-guide.md) for full API documentation.

## Workspace split (since Step 3.2.1)

Prior to Step 3.2.1, `theorem_file!` was a `macro_rules!` bridge in
`src/lib.rs`. The real proc-macro now lives in `crates/theoremc-macros` and all
schema, mangling, and IO logic lives in `crates/theoremc-core`. The root
`theoremc` crate re-exports both. Import paths from `theoremc` are unchanged.
