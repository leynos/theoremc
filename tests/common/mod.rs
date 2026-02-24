//! Shared test helpers for integration tests.

use cap_std::{ambient_authority, fs_utf8::Dir};

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn load_fixture(name: &str) -> std::io::Result<String> {
    Dir::open_ambient_dir("tests/fixtures", ambient_authority())?.read_to_string(name)
}
