//! Proc-macro expansion for compile-time theorem integration.

use camino::Utf8Path;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{LitStr, parse_macro_input};

/// Semantic query phase: loads theorem files and derives expansion inputs.
mod query;

/// Token rendering phase: converts expansion inputs into Rust tokens.
mod render;

/// Expansion failures and their `compile_error!` diagnostics.
mod error;

use error::MacroExpansionError;

/// Expands a crate-relative `.theorem` file into a stable private module.
///
/// # Input
///
/// The macro accepts a single string-literal argument: a path to a `.theorem`
/// file relative to the consuming crate's manifest directory
/// (`CARGO_MANIFEST_DIR`). The path must be relative, must not begin with a
/// drive prefix (e.g. `C:`), and must not contain `..` components.
///
/// ```ignore
/// theorem_file!("theorems/my_theorem.theorem");
/// ```
///
/// # Generated output
///
/// Each invocation emits a deterministic, private module whose name is derived
/// from the theorem file path via [`theoremc_core::mangle::mangle_module_path`].
/// Inside that module:
///
/// - A `const _: &str = include_str!(…)` anchors the theorem source to
///   `CARGO_MANIFEST_DIR` so the file is tracked as a compile-time dependency.
/// - A `#[cfg(kani)] pub(super) mod kani` sub-module contains one
///   `#[kani::proof]` and `#[kani::unwind(n)]` `pub(crate) fn` per theorem
///   document, named via
///   [`theoremc_core::mangle::mangle_theorem_harness`].
/// - A cfg-gated const array of `fn()` pointers sized to the harness count
///   anchors all generated symbols when Kani is compiling the crate.
///
/// Document order is preserved: the first theorem document in the file
/// produces the first harness function.
///
/// # Errors
///
/// All failures are reported as `compile_error!` at the macro call site:
///
/// | Cause | Diagnostic |
/// | --- | --- |
/// | `CARGO_MANIFEST_DIR` is not set | macro configuration error message |
/// | Path is absolute or contains `..` or a drive prefix | `InvalidTheoremPath` message |
/// | Theorem file cannot be read | `ReadTheoremFile` message with IO code |
/// | File contains no theorem documents | `EmptyTheoremFile` message |
/// | Schema parsing or validation fails | rendered `SchemaDiagnostic` (includes source location) |
/// | A theorem omits `Evidence.kani` | theorem `<name>` does not declare required `Evidence.kani` configuration |
/// | Two theorems declare one action with inequivalent signatures | both theorem names and both signature summaries |
///
/// # Panics
///
/// This macro does not panic. All error conditions are converted to
/// `compile_error!` invocations so failures surface as ordinary Rust compiler
/// diagnostics.
///
/// # Example
///
/// Given a file `theorems/my_theorem.theorem` containing:
///
/// ```text
/// Schema: 1
/// Theorem: MyLemma
/// About: A simple proof obligation
/// Prove:
///   - assert: "true"
///     because: trivial
/// Evidence:
///   kani:
///     unwind: 1
///     expect: SUCCESS
/// ```
///
/// The invocation:
///
/// ```ignore
/// theorem_file!("theorems/my_theorem.theorem");
/// ```
///
/// expands to (approximately):
///
/// ```ignore
/// mod theorems__my_theorem__h<hash> {
///     const _: &str = include_str!(
///         concat!(env!("CARGO_MANIFEST_DIR"), "/", "theorems/my_theorem.theorem")
///     );
///     #[expect(unexpected_cfgs, reason = "Kani sets cfg(kani) when compiling proof harnesses")]
///     #[cfg(kani)]
///     pub(super) mod kani {
///         #[kani::proof]
///         #[kani::unwind(1)]
///         pub(crate) fn theorem__my_lemma__h<hash>() {}
///     }
///     #[cfg(kani)]
///     const _: [fn(); 1] = [kani::theorem__my_lemma__h<hash>];
/// }
/// ```
#[proc_macro]
pub fn theorem_file(input: TokenStream) -> TokenStream {
    let path_literal = parse_macro_input!(input as LitStr);
    match expand_theorem_file(&path_literal) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error(path_literal.span()).into(),
    }
}

/// Expands a theorem file relative to the invoking crate's manifest directory.
fn expand_theorem_file(path_literal: &LitStr) -> Result<TokenStream2, MacroExpansionError> {
    let manifest_dir = query::manifest_dir_from_env()?;
    expand_theorem_file_at(&manifest_dir, path_literal)
}

/// Expands a theorem file against an explicit manifest directory.
///
/// This is the single seam the private expansion tests drive directly, so the
/// public proc-macro entry point stays a thin wrapper around it.
fn expand_theorem_file_at(
    manifest_dir: &Utf8Path,
    path_literal: &LitStr,
) -> Result<TokenStream2, MacroExpansionError> {
    let plan = query::plan_expansion(manifest_dir, path_literal)?;
    Ok(render::render_expansion(&plan))
}

/// Fixture and assertion helpers consumed by the private macro expansion tests.
#[cfg(test)]
mod tests_support;

/// Private expansion tests that exercise this module through `tests_support`.
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Error-reporting tests for the private expansion pipeline.
#[cfg(test)]
#[path = "error_reporting_tests.rs"]
mod error_reporting_tests;

/// Private expansion tests for compile-time action probe generation.
#[cfg(test)]
#[path = "action_probe_tests.rs"]
mod action_probe_tests;

/// Private expansion tests for compile-time referenced-type probe generation.
#[cfg(test)]
#[path = "type_probe_tests.rs"]
mod type_probe_tests;
