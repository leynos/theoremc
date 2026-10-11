//! Shared test helpers for integration tests.

use derive_more::{From, Into};
use the_newtype::Newtype;

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
///
/// The constructor and accessor come from [`StrNewtype`]; the trait is
/// re-exported through `test_helpers` so integration binaries can import it
/// alongside this type.
#[derive(Debug, Clone, Copy, Newtype, From, Into)]
pub struct FixtureName<'a>(&'a str);

/// Identifies an expected substring in fixture diagnostics or build logs.
///
/// Shares [`StrNewtype`] with [`FixtureName`] rather than repeating the
/// `new`/`as_str` pair.
#[derive(Debug, Clone, Copy, Newtype, From, Into)]
pub struct ExpectedFragment<'a>(&'a str);
