//! Shared test helpers for integration tests.

mod fixture_crate;
mod schema_fixtures;

pub use fixture_crate::{
    BUILD_DISCOVERY_SOURCE, BUILD_SCRIPT_SOURCE, BUILD_SUITE_SOURCE, BuildLog, FixtureCrate,
    TRIVIAL_THEOREM, toml_section,
};
pub use schema_fixtures::{
    FIXTURES_DIR, assert_diagnostic_failure, assert_fixture_error_contains, assert_fixture_fails,
    assert_fixture_loads, fixture_error_message, load_fixture, load_fixture_docs,
    load_fixture_text,
};

/// Identifies a fixture file under `tests/fixtures/`.
#[derive(Debug, Clone, Copy)]
pub struct FixtureName<'a>(&'a str);

impl<'a> FixtureName<'a> {
    /// Creates a fixture-name wrapper.
    #[must_use]
    pub const fn new(value: &'a str) -> Self {
        Self(value)
    }

    /// Returns the wrapped fixture name.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

/// Identifies an expected substring in fixture diagnostics or build logs.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedFragment<'a>(&'a str);

impl<'a> ExpectedFragment<'a> {
    /// Creates an expected-fragment wrapper.
    #[must_use]
    pub const fn new(value: &'a str) -> Self {
        Self(value)
    }

    /// Returns the wrapped expected fragment.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}
