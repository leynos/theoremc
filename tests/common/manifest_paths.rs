//! Formats repository paths for generated fixture manifests.
//!
//! A fixture manifest names the checkout by path, so the same path has to be
//! written the same way whatever host produced it. These helpers normalize
//! separators and escape the result for a TOML basic string, so a checkout under
//! a path with a backslash, a quote or a control character still produces a
//! manifest Cargo accepts.

use camino::{Utf8Path, Utf8PathBuf};
use theoremc_core::path_format::{normalize_path_separators, toml_basic_string_value};

/// The directory, relative to the checkout root, that holds build support.
const BUILD_SUPPORT_DIR: &str = "crates/theoremc-build-support";

/// Returns the repository checkout root.
///
/// This module is compiled into the `test-helpers` crate, whose manifest
/// directory is `<checkout>/crates/test-helpers`; the checkout is two levels
/// above it.
#[must_use]
pub fn checkout_root() -> Utf8PathBuf {
    let crate_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .unwrap_or(crate_dir)
        .to_path_buf()
}

/// Formats `path` as a quoted TOML basic string with forward-slash separators.
///
/// The result is ready to sit on the right-hand side of a `path = ...` entry.
///
/// # Examples
///
/// ```
/// use test_helpers::toml_path;
///
/// let value = toml_path(camino::Utf8Path::new("/work/checkout"));
/// assert_eq!(value, "\"/work/checkout\"");
/// ```
#[must_use]
pub fn toml_path(path: &Utf8Path) -> String {
    let normalized = normalize_path_separators(path.as_str());
    format!("\"{}\"", toml_basic_string_value(&normalized))
}

/// Returns the dependency line that points a fixture at the checkout's build
/// support crate.
///
/// Referencing the checkout rather than copying the crate into each fixture gives
/// that crate one identity in the shared fixture target, so it compiles once and
/// every later fixture reuses it.
#[must_use]
pub fn build_support_dependency() -> String {
    build_support_dependency_for(&checkout_root())
}

/// Returns the dependency line for the build-support crate under `checkout_root`.
///
/// This is the injectable form, so a test can assert the generated path without
/// depending on where the repository happens to sit.
#[must_use]
pub fn build_support_dependency_for(checkout_root: &Utf8Path) -> String {
    format!(
        "theoremc-build-support = {{ path = {} }}",
        toml_path(&checkout_root.join(BUILD_SUPPORT_DIR))
    )
}
