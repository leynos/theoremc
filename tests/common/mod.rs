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
/// The constructor and accessor come from
/// [`theoremc_core::schema::StrNewtype`]; the trait is re-exported through
/// `test_helpers` so integration binaries can import it alongside this type.
///
/// These two methods were `pub const fn` before the trait existed. Routing
/// them through `StrNewtype` gives up `const`-callability, because a blanket
/// trait method cannot be `const` (trait `const fn` is unstable and `const
/// Trait` for the bound is unstable too). No `const`, `static`, `const fn`,
/// array-length, or const-generic context in this workspace constructs or
/// reads either wrapper, and `test-helpers` is a workspace-internal path
/// dependency rather than a published crate, so the capability is unused and
/// unobservable here. If that ever changes, restore an inherent `const` pair
/// on the affected wrapper — the precedent recorded for the const-capable
/// siblings in `docs/theoremc-design.md` §6.7.11 — rather than giving up the
/// deduplication for the other wrappers.
#[derive(Debug, Clone, Copy, Newtype, From, Into)]
pub struct FixtureName<'a>(&'a str);

/// Identifies an expected substring in fixture diagnostics or build logs.
///
/// Shares [`theoremc_core::schema::StrNewtype`] with [`FixtureName`] rather
/// than repeating the `new`/`as_str` pair, and gives up `const`-callability on
/// the same terms documented above.
#[derive(Debug, Clone, Copy, Newtype, From, Into)]
pub struct ExpectedFragment<'a>(&'a str);
