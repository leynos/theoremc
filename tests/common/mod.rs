//! Shared test helpers for integration tests.
#![allow(
    dead_code,
    reason = "integration test crates include this module and use different helper subsets"
)]

use cap_std::{ambient_authority, fs_utf8::Dir};
use theoremc::schema::{SourceId, TheoremDoc, load_theorem_docs};

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Errors
///
/// Returns an I/O error when the fixture cannot be read.
pub fn load_fixture(name: &str) -> std::io::Result<String> {
    Dir::open_ambient_dir("tests/fixtures", ambient_authority())?.read_to_string(name)
}

/// Returns the source identifier used by diagnostics for a fixture.
#[must_use]
pub fn fixture_source_id(fixture_name: &str) -> SourceId {
    SourceId::new(format!("tests/fixtures/{fixture_name}"))
}

/// Returns the diagnostic source string used by diagnostics for a fixture.
#[must_use]
pub fn fixture_source(fixture_name: &str) -> String {
    fixture_source_id(fixture_name).as_str().to_owned()
}

/// Loads a fixture and parses it as theorem documents.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or the theorem documents
/// fail to load.
pub fn load_fixture_docs(fixture_name: &str) -> Result<Vec<TheoremDoc>, String> {
    let yaml =
        load_fixture(fixture_name).map_err(|error| format!("failed to load fixture: {error}"))?;
    load_theorem_docs(&yaml).map_err(|error| format!("fixture should load: {error}"))
}

/// Asserts that loading a fixture succeeds.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or loaded.
pub fn assert_fixture_loads(fixture_name: &str) -> Result<(), String> {
    load_fixture_docs(fixture_name).map(|_| ())
}

/// Loads a fixture and returns the theorem loading error message.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read or unexpectedly loads.
pub fn fixture_error_message(fixture_name: &str) -> Result<String, String> {
    let yaml =
        load_fixture(fixture_name).map_err(|error| format!("failed to load fixture: {error}"))?;
    match load_theorem_docs(&yaml) {
        Ok(_) => Err(format!("fixture should fail: {fixture_name}")),
        Err(error) => Ok(error.to_string()),
    }
}

/// Asserts that loading a fixture fails with an error message fragment.
///
/// # Errors
///
/// Returns an error when the fixture cannot be read, unexpectedly loads, or the
/// error message omits the expected fragment.
pub fn assert_fixture_error_contains(
    fixture_name: &str,
    expected_fragment: &str,
) -> Result<(), String> {
    let message = fixture_error_message(fixture_name)?;

    if message.contains(expected_fragment) {
        return Ok(());
    }

    Err(format!(
        "expected '{expected_fragment}' in error for {fixture_name}, got: {message}"
    ))
}
