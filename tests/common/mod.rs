//! Shared test helpers for integration tests.

use derive_more::{From, Into};
use the_newtype::Newtype;
mod bounded_command;
mod build_log;
mod build_support_fixture;
mod fixture_command;
mod fixture_crate;
mod fixture_isolation;
mod fixture_lock;
mod hermetic_cargo;
mod manifest_paths;
mod schema_fixtures;
mod suite_layout;

pub use bounded_command::{CommandFailure, CommandOutput, run_within_deadline};
pub use build_log::{BuildLog, toml_section};
pub use build_support_fixture::build_support_fixture_manifest;
pub use fixture_command::{
    DEFAULT_FIXTURE_COMMAND_TIMEOUT, FIXTURE_TIMEOUT_ENV, FixtureOutcome, fixture_command_timeout,
    run_fixture_command, run_fixture_command_within,
};
pub use fixture_crate::{BUILD_SCRIPT_SOURCE, FixtureCrate, TRIVIAL_THEOREM};
pub use fixture_isolation::{shared_target_dir, shared_target_sources_dir};
pub use fixture_lock::{DEFAULT_LOCK_WAIT, FixtureLock};
pub use hermetic_cargo::hermetic_cargo;
pub use manifest_paths::{
    build_support_dependency, build_support_dependency_for, checkout_root, toml_path,
};
pub use schema_fixtures::{
    FIXTURES_DIR, assert_diagnostic_failure, assert_fixture_error_contains, assert_fixture_fails,
    assert_fixture_loads, fixture_error_message, fixture_source_id, load_fixture,
    load_fixture_docs, load_fixture_text,
};

/// Identifies a fixture file under `tests/fixtures/`.
///
/// The constructor and accessor come from
/// [`theoremc_core::schema::StrNewtype`]; the trait is
/// re-exported through `test_helpers` so integration binaries can import it
/// alongside this type.
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
