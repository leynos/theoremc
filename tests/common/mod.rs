//! Shared test helpers for integration tests.

/// Loads a fixture file from the `tests/fixtures/` directory.
///
/// # Panics
///
/// Panics if the file cannot be read.
pub fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}"))
        .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}
