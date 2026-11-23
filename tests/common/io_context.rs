//! Consistent operation-and-path context for fixture failures.
//!
//! A fixture that fails with only `No such file or directory (os error 2)`
//! cannot be diagnosed from a CI log. Every fallible filesystem step therefore
//! routes its error through [`path_error`], which names the operation and the
//! affected path without changing the `Result<_, String>` fixture contract.

use std::io;

use camino::Utf8Path;

/// Formats `"<operation> <path>: <io error>"`.
///
/// This is the single context shape fixture helpers use, so a captured failure
/// can be read as "what was attempted, where, and why it failed". The
/// underlying [`io::Error`] display is kept verbatim, including its kind-derived
/// text such as `No such file or directory (os error 2)`.
///
/// # Examples
///
/// ```
/// let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
/// let message = format!(
///     "opening {}: {missing}",
///     camino::Utf8Path::new("/tmp/fixture"),
/// );
/// assert!(message.starts_with("opening /tmp/fixture: "));
/// ```
pub(super) fn path_error(operation: &str, path: &Utf8Path, error: &io::Error) -> String {
    format!("{operation} {}: {error}", path.as_str())
}

#[cfg(test)]
#[path = "io_context_tests.rs"]
mod tests;
