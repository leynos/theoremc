//! Shared helpers for integration and macro tests.

#[path = "../../../tests/common/mod.rs"]
mod integration;

pub use integration::{
    BUILD_DISCOVERY_SOURCE, BUILD_SCRIPT_SOURCE, BUILD_SUITE_SOURCE, BuildLog, ExpectedFragment,
    FIXTURES_DIR, FixtureCrate, FixtureLock, FixtureName, LockPackage, TRIVIAL_THEOREM,
    assert_diagnostic_failure, assert_fixture_error_contains, assert_fixture_fails,
    assert_fixture_loads, build_support_fixture, build_support_fixture_manifest,
    fixture_cargo_home, fixture_error_message, fixture_source_id, hermetic_cargo, load_fixture,
    load_fixture_docs, load_fixture_text, lock_packages, seeded_lock, toml_section,
};
pub use theoremc_core::schema::StrNewtype;
